//! The sums a manifest records, and how they are made: BLAKE3 of a file, and the two digests of a folder.
//!
//! This file uses only the standard library and `blake3`, so the manifest example and the tool that wrote the shipped
//! manifests both read it as it is.
//!
//! - A **file** is identified by its size and, when it is no larger than [`HASH_LIMIT`], the BLAKE3 hash of its bytes
//!   ([`file_hash`]: 64 lowercase hex digits).
//! - A **folder** is its listing ([`list`]): every regular file below it, links followed, named by its path below the
//!   folder with `/` separators, sorted by that name byte by byte. Two digests are made of the listing. The *shape*
//!   ([`shape_digest`]) is the BLAKE3 hash of one line `path NUL size LF` for each file (sizes in decimal), so it needs
//!   nothing but the folder's directory entries. The *content* digest ([`content_digest`]) is the hash of one line
//!   `path NUL size NUL hash LF` for each file, where `hash` is the file's [`file_hash`], or `-` for a file larger than
//!   [`HASH_LIMIT`].
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Files larger than this (256 MiB) are identified by their size alone: hashing a multi-gigabyte archive on every test
/// run would cost more than the check is worth.
pub const HASH_LIMIT: u64 = 256 * 1024 * 1024;

/// How deep a listing follows folders: a link loop ends in an error here instead of running forever.
const DEPTH_LIMIT: usize = 32;

/// BLAKE3 of the bytes of the file at `path`, as 64 lowercase hex digits.
pub fn file_hash(path: &Path) -> io::Result<String> {
    let file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(file)?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// One file of a folder's listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    /// Its path below the folder, `/`-separated.
    pub path: String,
    /// Its size in bytes.
    pub size: u64,
    /// Where it is.
    pub file: PathBuf,
}

/// Every regular file below `dir`, sorted by path. Links are followed (an oracle folder is often a junction to the folder
/// that holds the data); a name that is not UTF-8, a link that leads nowhere and a nesting deeper than 32 folders are
/// errors.
pub fn list(dir: &Path) -> io::Result<Vec<Listed>> {
    let mut out = Vec::new();
    walk(dir, "", 0, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk(dir: &Path, prefix: &str, depth: usize, out: &mut Vec<Listed>) -> io::Result<()> {
    if depth > DEPTH_LIMIT {
        return Err(io::Error::other(format!(
            "{}: more than {DEPTH_LIMIT} folders deep (a link loop?)",
            dir.display()
        )));
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().into_string().map_err(|name| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{}: a name that is not UTF-8: {name:?}", dir.display()),
            )
        })?;
        let path = entry.path();
        let meta = std::fs::metadata(&path)?;
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        if meta.is_dir() {
            walk(&path, &rel, depth + 1, out)?;
        } else if meta.is_file() {
            out.push(Listed {
                path: rel,
                size: meta.len(),
                file: path,
            });
        }
    }
    Ok(())
}

/// The shape digest of a listing: names and sizes.
pub fn shape_digest(listing: &[Listed]) -> String {
    let mut hasher = blake3::Hasher::new();
    for file in listing {
        hasher.update(file.path.as_bytes());
        hasher.update(&[0]);
        hasher.update(file.size.to_string().as_bytes());
        hasher.update(b"\n");
    }
    hasher.finalize().to_hex().to_string()
}

/// The content digest of a listing, from the hashes of its files in listing order (`None` for a file larger than
/// [`HASH_LIMIT`], which [`hash_files`] leaves out).
pub fn content_digest(listing: &[Listed], hashes: &[Option<String>]) -> String {
    let mut hasher = blake3::Hasher::new();
    for (file, hash) in listing.iter().zip(hashes) {
        hasher.update(file.path.as_bytes());
        hasher.update(&[0]);
        hasher.update(file.size.to_string().as_bytes());
        hasher.update(&[0]);
        hasher.update(hash.as_deref().unwrap_or("-").as_bytes());
        hasher.update(b"\n");
    }
    hasher.finalize().to_hex().to_string()
}

/// The [`file_hash`] of every file of `listing` that is no larger than [`HASH_LIMIT`] (`None` for the others), computed on
/// `threads` threads.
pub fn hash_files(listing: &[Listed], threads: usize) -> io::Result<Vec<Option<String>>> {
    let next = AtomicUsize::new(0);
    let hashes: Mutex<Vec<Option<String>>> = Mutex::new(vec![None; listing.len()]);
    let failure: Mutex<Option<io::Error>> = Mutex::new(None);
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(file) = listing.get(i) else { break };
                    if file.size > HASH_LIMIT {
                        continue;
                    }
                    match file_hash(&file.file) {
                        Ok(hash) => {
                            if let Ok(mut all) = hashes.lock()
                                && let Some(slot) = all.get_mut(i)
                            {
                                *slot = Some(hash);
                            }
                        }
                        Err(e) => {
                            if let Ok(mut first) = failure.lock() {
                                first.get_or_insert(io::Error::new(
                                    e.kind(),
                                    format!("{}: {e}", file.file.display()),
                                ));
                            }
                            break;
                        }
                    }
                }
            });
        }
    });
    if let Ok(mut first) = failure.lock()
        && let Some(e) = first.take()
    {
        return Err(e);
    }
    hashes
        .into_inner()
        .map_err(|_| io::Error::other("a hashing thread panicked"))
}

/// What a folder's listing comes to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSums {
    /// The number of files.
    pub files: usize,
    /// Their sizes added.
    pub bytes: u64,
    /// The [`shape_digest`].
    pub shape: String,
    /// The [`content_digest`], when it was asked for.
    pub content: Option<String>,
}

/// The sums of `listing`; the content digest (which reads every file of at most [`HASH_LIMIT`] bytes) only when
/// `content`.
pub fn tree_sums(listing: &[Listed], threads: usize, content: bool) -> io::Result<TreeSums> {
    let content = if content {
        Some(content_digest(listing, &hash_files(listing, threads)?))
    } else {
        None
    };
    Ok(TreeSums {
        files: listing.len(),
        bytes: listing.iter().map(|f| f.size).sum(),
        shape: shape_digest(listing),
        content,
    })
}
