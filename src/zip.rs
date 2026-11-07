//! A reader for the archives oracle entries may be stored as: zip files whose members are stored (not compressed), read
//! in place with positional reads, so one open file serves every thread.
//!
//! An oracle set of a hundred thousand small files costs a test more in opening files than in reading them, most of
//! all over a network share; the same files as one archive cost one open. The reader takes only what a store's packer
//! writes ([`crate::sums`] describes the record of such an entry), and refuses the rest naming the member:
//!
//! - a member that is compressed, encrypted or followed by a data descriptor;
//! - a folder entry, a name that is absolute, has a `\`, an empty part, `.` or `..`, or one listed twice;
//! - an archive split over several disks.
//!
//! zip64 is read (the end-of-central-directory locator and record, and the 64-bit sizes and offset of a member's
//! extra field), since a set may hold more than 65,535 members or more than 4 GiB. The central directory is read once,
//! into a name-sorted index; a member's local header is read when the member is.
//!
//! This file uses only the standard library.
use std::fs::File;
use std::io;
use std::path::Path;

const END: u32 = 0x0605_4b50;
const END64: u32 = 0x0606_4b50;
const LOCATOR64: u32 = 0x0706_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const LOCAL: u32 = 0x0403_4b50;
/// The end-of-central-directory record is 22 bytes, followed by a comment of at most 65,535.
const END_SEARCH: u64 = 22 + 65_535;

/// One member of an archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member {
    /// Its name: a relative, `/`-separated path.
    pub name: String,
    /// Its size in bytes (stored, so also its size in the archive).
    pub size: u64,
    /// The CRC-32 the central directory records for it.
    pub crc32: u32,
    /// Where its local header starts.
    offset: u64,
}

/// An archive opened for reading: its members, sorted by name byte by byte, and the file.
#[derive(Debug)]
pub struct Archive {
    file: File,
    len: u64,
    members: Vec<Member>,
}

fn invalid(what: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2).map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    b.get(at..at + 8).map_or(0, |s| {
        u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
    })
}

/// Fill `buf` from `file` at `offset`, whatever the platform.
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
    }
    #[cfg(windows)]
    {
        let mut done = 0;
        while done < buf.len() {
            let n = std::os::windows::fs::FileExt::seek_read(file, &mut buf[done..], offset + done as u64)?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            done += n;
        }
        Ok(())
    }
}

fn read_vec(file: &File, offset: u64, len: u64) -> io::Result<Vec<u8>> {
    let len = usize::try_from(len).map_err(|_| invalid(format!("{len} bytes do not fit in memory")))?;
    let mut buf = vec![0; len];
    read_at(file, &mut buf, offset)?;
    Ok(buf)
}

/// Whether `name` is a member name the reader takes: relative, `/`-separated, with no empty, `.` or `..` part.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && !name.contains(':')
        && name.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

impl Archive {
    /// Open the archive at `path` and read its central directory.
    pub fn open(path: &Path) -> io::Result<Self> {
        let at = |e: io::Error| io::Error::new(e.kind(), format!("{}: {e}", path.display()));
        let file = File::open(path).map_err(at)?;
        let len = file.metadata().map_err(at)?.len();
        let members = Self::directory(&file, len).map_err(at)?;
        Ok(Self { file, len, members })
    }

    fn directory(file: &File, len: u64) -> io::Result<Vec<Member>> {
        let tail_len = len.min(END_SEARCH);
        let tail = read_vec(file, len - tail_len, tail_len)?;
        let end = (0..tail.len().saturating_sub(21))
            .rev()
            .find(|&i| u32_at(&tail, i) == END && i + 22 + usize::from(u16_at(&tail, i + 20)) == tail.len())
            .ok_or_else(|| invalid("not a zip archive: no end-of-central-directory record".into()))?;
        let end_at = len - tail_len + end as u64;
        if u16_at(&tail, end + 4) != 0 || u16_at(&tail, end + 6) != 0 {
            return Err(invalid("an archive split over several disks".into()));
        }
        let (mut count, mut size, mut offset) = (
            u64::from(u16_at(&tail, end + 10)),
            u64::from(u32_at(&tail, end + 12)),
            u64::from(u32_at(&tail, end + 16)),
        );
        // zip64: a locator just before the end record points at the zip64 end record
        if end_at >= 20 {
            let locator = read_vec(file, end_at - 20, 20)?;
            if u32_at(&locator, 0) == LOCATOR64 {
                let record_at = u64_at(&locator, 8);
                let record = read_vec(file, record_at, 56)?;
                if u32_at(&record, 0) != END64 {
                    return Err(invalid(format!("no zip64 end record at {record_at}")));
                }
                count = u64_at(&record, 32);
                size = u64_at(&record, 40);
                offset = u64_at(&record, 48);
            }
        }
        if offset.checked_add(size).is_none_or(|e| e > end_at) {
            return Err(invalid(format!(
                "the central directory ({size} bytes at {offset}) runs past the end record"
            )));
        }
        let dir = read_vec(file, offset, size)?;
        let mut members = Vec::with_capacity(usize::try_from(count).unwrap_or(0));
        let mut at = 0usize;
        for n in 0..count {
            if u32_at(&dir, at) != CENTRAL || at + 46 > dir.len() {
                return Err(invalid(format!("central directory entry {n} is not one")));
            }
            let flags = u16_at(&dir, at + 8);
            let method = u16_at(&dir, at + 10);
            let crc32 = u32_at(&dir, at + 16);
            let mut packed = u64::from(u32_at(&dir, at + 20));
            let mut size = u64::from(u32_at(&dir, at + 24));
            let name_len = usize::from(u16_at(&dir, at + 28));
            let extra_len = usize::from(u16_at(&dir, at + 30));
            let comment_len = usize::from(u16_at(&dir, at + 32));
            let mut local = u64::from(u32_at(&dir, at + 42));
            let name_bytes = dir
                .get(at + 46..at + 46 + name_len)
                .ok_or_else(|| invalid(format!("central directory entry {n} runs past the directory")))?;
            let name = String::from_utf8(name_bytes.to_vec())
                .map_err(|_| invalid(format!("member {n}: a name that is not UTF-8")))?;
            let extra = dir
                .get(at + 46 + name_len..at + 46 + name_len + extra_len)
                .unwrap_or_default();
            // the zip64 extra field holds, in this order, the values the entry sets to all ones
            let mut e = 0;
            while e + 4 <= extra.len() {
                let (id, len) = (u16_at(extra, e), usize::from(u16_at(extra, e + 2)));
                if id == 1 {
                    let mut v = e + 4;
                    for field in [&mut size, &mut packed, &mut local] {
                        if *field == u64::from(u32::MAX) {
                            *field = u64_at(extra, v);
                            v += 8;
                        }
                    }
                }
                e += 4 + len;
            }
            let refuse = |why: &str| invalid(format!("{name}: {why}"));
            if name.ends_with('/') {
                return Err(refuse("a folder entry (a packed set lists files only)"));
            }
            if !plain(&name) {
                return Err(refuse("not a relative `/`-separated path without `.` or `..`"));
            }
            if method != 0 {
                return Err(refuse(&format!(
                    "compressed (method {method}): only stored members are read"
                )));
            }
            if flags & 1 != 0 {
                return Err(refuse("encrypted"));
            }
            if flags & 8 != 0 {
                return Err(refuse("followed by a data descriptor"));
            }
            if packed != size {
                return Err(refuse(&format!(
                    "stored, but {packed} bytes in the archive for {size}"
                )));
            }
            members.push(Member {
                name,
                size,
                crc32,
                offset: local,
            });
            at += 46 + name_len + extra_len + comment_len;
        }
        members.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(w) = members.windows(2).find(|w| w[0].name == w[1].name) {
            return Err(invalid(format!("{}: listed twice", w[0].name)));
        }
        Ok(members)
    }

    /// The archive file's size in bytes.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Whether the archive has no members.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Its members, sorted by name byte by byte.
    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// The member called `name`.
    pub fn member(&self, name: &str) -> Option<&Member> {
        self.members
            .binary_search_by(|m| m.name.as_str().cmp(name))
            .ok()
            .and_then(|i| self.members.get(i))
    }

    /// The bytes of `member`, which must be one of this archive's.
    pub fn read(&self, member: &Member) -> io::Result<Vec<u8>> {
        let header = read_vec(&self.file, member.offset, 30)?;
        if u32_at(&header, 0) != LOCAL {
            return Err(invalid(format!(
                "{}: no local header at {}",
                member.name, member.offset
            )));
        }
        let data = member.offset + 30 + u64::from(u16_at(&header, 26)) + u64::from(u16_at(&header, 28));
        if data.checked_add(member.size).is_none_or(|e| e > self.len) {
            return Err(invalid(format!(
                "{}: runs past the end of the archive",
                member.name
            )));
        }
        read_vec(&self.file, data, member.size)
    }
}

/// The CRC-32 (IEEE, as zip records it) of `bytes`.
pub fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, slot) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *slot = c;
        }
        t
    });
    !bytes.iter().fold(!0u32, |c, &b| {
        table[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8)
    })
}
