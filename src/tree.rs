//! An oracle entry read as a set of named files, whether the store holds it as a folder or as one archive
//! ([`crate::zip`]): what `oracle_tree` gives a test, so a test is written once and the store chooses the form.
use std::io;
use std::path::{Path, PathBuf};

use crate::digest;
use crate::zip::Archive;

/// The files of an oracle entry, by their paths below it (`/`-separated, sorted byte by byte).
///
/// A folder is listed once when the tree is opened; an archive's central directory is read once. Reading a member of
/// an archive is a positional read of the one open file, so threads can share a tree.
#[derive(Debug)]
pub struct Tree {
    inner: Inner,
}

#[derive(Debug)]
enum Inner {
    Folder {
        root: PathBuf,
        files: Vec<(String, u64)>,
    },
    Zip {
        file: PathBuf,
        archive: Archive,
    },
}

impl Tree {
    /// The folder at `root`, listed as [`digest::list`] lists it.
    pub fn folder(root: &Path) -> io::Result<Self> {
        let files = digest::list(root)?
            .into_iter()
            .map(|f| (f.path, f.size))
            .collect();
        Ok(Self {
            inner: Inner::Folder {
                root: root.to_owned(),
                files,
            },
        })
    }

    /// The archive at `file` ([`crate::zip`] says which archives are read).
    pub fn zip(file: &Path) -> io::Result<Self> {
        Ok(Self {
            inner: Inner::Zip {
                file: file.to_owned(),
                archive: Archive::open(file)?,
            },
        })
    }

    /// Where the entry is: the folder, or the archive file.
    pub fn path(&self) -> &Path {
        match &self.inner {
            Inner::Folder { root, .. } => root,
            Inner::Zip { file, .. } => file,
        }
    }

    /// The folder, for an entry held as a folder; `None` for an archive.
    pub fn as_folder(&self) -> Option<&Path> {
        match &self.inner {
            Inner::Folder { root, .. } => Some(root),
            Inner::Zip { .. } => None,
        }
    }

    /// The paths of its files, sorted byte by byte.
    pub fn names(&self) -> Vec<&str> {
        match &self.inner {
            Inner::Folder { files, .. } => files.iter().map(|(n, _)| n.as_str()).collect(),
            Inner::Zip { archive, .. } => archive.members().iter().map(|m| m.name.as_str()).collect(),
        }
    }

    /// The number of files.
    pub fn len(&self) -> usize {
        match &self.inner {
            Inner::Folder { files, .. } => files.len(),
            Inner::Zip { archive, .. } => archive.members().len(),
        }
    }

    /// Whether it has no files.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The size of the file `name`; `None` when there is no such file.
    pub fn size(&self, name: &str) -> Option<u64> {
        match &self.inner {
            Inner::Folder { files, .. } => files
                .binary_search_by(|(n, _)| n.as_str().cmp(name))
                .ok()
                .and_then(|i| files.get(i))
                .map(|(_, s)| *s),
            Inner::Zip { archive, .. } => archive.member(name).map(|m| m.size),
        }
    }

    /// The bytes of the file `name`. A name the tree does not hold is an error of kind `NotFound`.
    pub fn read(&self, name: &str) -> io::Result<Vec<u8>> {
        let missing = || {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{}: no file {name}", self.path().display()),
            )
        };
        match &self.inner {
            Inner::Folder { root, .. } => {
                if self.size(name).is_none() {
                    return Err(missing());
                }
                let path: PathBuf = std::iter::once(root.as_os_str())
                    .chain(name.split('/').map(std::ffi::OsStr::new))
                    .collect();
                crate::read_input(path)
            }
            Inner::Zip { file, archive } => {
                let member = archive.member(name).ok_or_else(missing)?;
                archive
                    .read(member)
                    .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", file.display())))
            }
        }
    }

    /// The tree as a folder, for a tool that needs real paths: the folder itself, or the archive's files written once
    /// under `scratch` in a folder named by the archive's shape digest (a later call with the same archive reuses it).
    /// An archive whose files add up to more than `limit` bytes is refused rather than written out.
    pub fn extract(&self, scratch: &Path, limit: u64) -> io::Result<PathBuf> {
        let (file, archive) = match &self.inner {
            Inner::Folder { root, .. } => return Ok(root.clone()),
            Inner::Zip { file, archive } => (file, archive),
        };
        let total: u64 = archive.members().iter().map(|m| m.size).sum();
        if total > limit {
            return Err(io::Error::other(format!(
                "{}: its {} files hold {total} bytes, more than the {limit} allowed to write out: read it in place",
                file.display(),
                archive.members().len()
            )));
        }
        let shape = digest::shape_of(archive.members().iter().map(|m| (m.name.as_str(), m.size)));
        // the folder is complete once its marker exists beside it (inside, it would be one of its files)
        let trees = scratch.join("oracle-trees");
        let dir = trees.join(&shape);
        let done = trees.join(format!("{shape}.complete"));
        if done.try_exists()? {
            return Ok(dir);
        }
        let partial = trees.join(format!("{shape}.partial"));
        if partial.try_exists()? {
            std::fs::remove_dir_all(&partial)?;
        }
        for member in archive.members() {
            let at: PathBuf = std::iter::once(partial.as_os_str())
                .chain(member.name.split('/').map(std::ffi::OsStr::new))
                .collect();
            if let Some(parent) = at.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&at, archive.read(member)?)?;
        }
        if dir.try_exists()? {
            std::fs::remove_dir_all(&dir)?;
        }
        std::fs::rename(&partial, &dir)?;
        std::fs::write(&done, b"")?;
        Ok(dir)
    }
}
