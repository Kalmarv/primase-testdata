//! Measuring a copy: the manifest entry for the file, the named files or the folder at a path. The manifest example and the
//! tool that wrote the shipped manifests both make their entries here.
//!
//! This file uses only the standard library, [`crate::digest`] and [`crate::sums`].
use std::path::{Path, PathBuf};

use crate::digest::{self, HASH_LIMIT};
use crate::sums::{Entry, FileSum, Severity, Sums};

/// The entry for the copy of `key` at `path`.
///
/// An install (`installs.<game>`, no path below it) is recorded as the named `files`, relative to its folder and
/// `/`-separated: the folder holds files that vary between machines (logs, mods), which an entry must not list. Any other
/// key is a file, recorded by its size and hash, or a folder, recorded by its listing's digests. A file larger than the hash
/// limit is recorded by its size alone. `threads` is how many threads hashing a folder's files may use. Archive keys
/// (`keys.*`) are refused: their digests would be digests of keys.
pub fn entry_for(
    key: &str,
    path: &Path,
    files: &[String],
    severity: Severity,
    threads: usize,
) -> Result<Entry, String> {
    if key.starts_with("keys.") {
        return Err("archive keys have no manifest entry".into());
    }
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let hash_of = |file: &Path, size: u64| -> Result<Option<String>, String> {
        if size > HASH_LIMIT {
            return Ok(None);
        }
        digest::file_hash(file)
            .map(Some)
            .map_err(|e| format!("{}: {e}", file.display()))
    };
    let sums = if key.starts_with("installs.") && !key.contains('/') {
        if files.is_empty() {
            return Err("an install needs the files that identify it".into());
        }
        let mut sums = Vec::with_capacity(files.len());
        for rel in files {
            let at = path.join(rel.split('/').collect::<PathBuf>());
            let size = std::fs::metadata(&at)
                .map_err(|e| format!("{}: {e}", at.display()))?
                .len();
            sums.push(FileSum {
                path: rel.clone(),
                size,
                blake3: hash_of(&at, size)?,
            });
        }
        Sums::Files(sums)
    } else if meta.is_file() {
        Sums::File {
            size: meta.len(),
            blake3: hash_of(path, meta.len())?,
        }
    } else {
        let listing = digest::list(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let all =
            digest::tree_sums(&listing, threads, true).map_err(|e| format!("{}: {e}", path.display()))?;
        Sums::Tree {
            count: all.files,
            bytes: all.bytes,
            shape: all.shape,
            content: all.content.unwrap_or_default(),
        }
    };
    Ok(Entry {
        key: key.to_owned(),
        sums,
        severity,
    })
}
