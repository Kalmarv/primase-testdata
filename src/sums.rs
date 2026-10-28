//! The text of a manifest: what it records for an input ([`Entry`]), read from TOML ([`parse`]) and written as TOML
//! ([`render`]). The sums themselves are made by [`crate::digest`].
//!
//! A manifest file is a TOML document with `format = 1` and a list of `[[entry]]` tables, each naming an input by the key
//! a query for it has (`installs.sekiro`, `tools.oodle6`, `oracles.corpus.sekiro/havok`) and recording what a good copy
//! of it holds:
//!
//! ```toml
//! format = 1
//!
//! [[entry]]
//! key = "tools.oodle6"            # a file
//! kind = "file"
//! size = 1085952
//! blake3 = "…"                    # absent for a file larger than the hash limit
//!
//! [[entry]]
//! key = "installs.sekiro"         # a set of files below an install folder
//! kind = "files"
//! files = [
//!   { path = "sekiro.exe", size = 68005144, blake3 = "…" },
//!   { path = "Data5.bdt", size = 6830280950 },
//! ]
//!
//! [[entry]]
//! key = "oracles.corpus.sekiro/havok"   # a folder
//! kind = "tree"
//! count = 174                     # files
//! bytes = 228777184
//! shape = "…"                     # the shape digest
//! blake3 = "…"                    # the content digest
//! check = "warn"                  # optional: a mismatch warns instead of failing the test
//! ```
//!
//! This file uses only the standard library and `toml`, so the manifest example and the tool that wrote the shipped
//! manifests both read it as it is.
use std::fmt::Write as _;

/// What a mismatch does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// The test that asked for the input fails.
    Fail,
    /// One warning line is printed: for an input whose exact version does not decide the tests (a system tool).
    Warn,
}

/// One file of a `files` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSum {
    /// Its path below the install folder, `/`-separated.
    pub path: String,
    /// Its size in bytes.
    pub size: u64,
    /// Its BLAKE3 hash; absent when the file is larger than the hash limit.
    pub blake3: Option<String>,
}

/// What a good copy of an input holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sums {
    /// A file.
    File {
        /// Its size in bytes.
        size: u64,
        /// Its BLAKE3 hash; absent when the file is larger than the hash limit.
        blake3: Option<String>,
    },
    /// Named files below a folder (an install).
    Files(Vec<FileSum>),
    /// A folder: its listing's digests.
    Tree {
        /// The number of files.
        count: usize,
        /// Their sizes added.
        bytes: u64,
        /// The shape digest of the listing.
        shape: String,
        /// The content digest of the listing.
        content: String,
    },
}

/// One input's entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The key a query for the input has.
    pub key: String,
    /// What a good copy holds.
    pub sums: Sums,
    /// What a mismatch does.
    pub severity: Severity,
}

const ENTRY_KEYS: &[&str] = &[
    "key", "kind", "size", "blake3", "files", "count", "bytes", "shape", "check",
];

fn text<'a>(table: &'a toml::Table, key: &str, what: &str) -> Result<&'a str, String> {
    table
        .get(key)
        .and_then(toml::Value::as_str)
        .ok_or_else(|| format!("{what}: `{key}` is missing or not a string"))
}

fn number(table: &toml::Table, key: &str, what: &str) -> Result<u64, String> {
    table
        .get(key)
        .and_then(toml::Value::as_integer)
        .and_then(|n| u64::try_from(n).ok())
        .ok_or_else(|| format!("{what}: `{key}` is missing or not a whole number"))
}

fn hash(table: &toml::Table, key: &str, what: &str) -> Result<Option<String>, String> {
    match table.get(key) {
        None => Ok(None),
        Some(toml::Value::String(h))
            if h.len() == 64
                && h.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
        {
            Ok(Some(h.clone()))
        }
        Some(_) => Err(format!("{what}: `{key}` is not 64 lowercase hex digits")),
    }
}

/// The entries of the manifest `text` (named `name` in errors).
pub fn parse(name: &str, text_of_file: &str) -> Result<Vec<Entry>, String> {
    let table: toml::Table = toml::from_str(text_of_file).map_err(|e| format!("{name}: {e}"))?;
    if table.get("format").and_then(toml::Value::as_integer) != Some(1) {
        return Err(format!("{name}: `format = 1` is missing"));
    }
    if let Some(unknown) = table.keys().find(|k| !matches!(k.as_str(), "format" | "entry")) {
        return Err(format!("{name}: unknown key `{unknown}`"));
    }
    let list = match table.get("entry") {
        None => return Ok(Vec::new()),
        Some(toml::Value::Array(list)) => list,
        Some(_) => return Err(format!("{name}: `entry` is not a list of tables")),
    };
    let mut entries = Vec::with_capacity(list.len());
    for (i, item) in list.iter().enumerate() {
        let t = item
            .as_table()
            .ok_or_else(|| format!("{name}: entry {i} is not a table"))?;
        let key = text(t, "key", &format!("{name}: entry {i}"))?.to_owned();
        let what = format!("{name}: {key}");
        if let Some(unknown) = t.keys().find(|k| !ENTRY_KEYS.contains(&k.as_str())) {
            return Err(format!("{what}: unknown key `{unknown}`"));
        }
        let severity = match t.get("check").map(toml::Value::as_str) {
            None | Some(Some("fail")) => Severity::Fail,
            Some(Some("warn")) => Severity::Warn,
            _ => return Err(format!("{what}: `check` is `fail` or `warn`")),
        };
        let sums = match text(t, "kind", &what)? {
            "file" => Sums::File {
                size: number(t, "size", &what)?,
                blake3: hash(t, "blake3", &what)?,
            },
            "files" => {
                let files = t
                    .get("files")
                    .and_then(toml::Value::as_array)
                    .ok_or_else(|| format!("{what}: `files` is missing or not a list"))?;
                let mut sums = Vec::with_capacity(files.len());
                for (n, file) in files.iter().enumerate() {
                    let f = file
                        .as_table()
                        .ok_or_else(|| format!("{what}: file {n} is not a table"))?;
                    let at = format!("{what}: file {n}");
                    if let Some(unknown) = f
                        .keys()
                        .find(|k| !matches!(k.as_str(), "path" | "size" | "blake3"))
                    {
                        return Err(format!("{at}: unknown key `{unknown}`"));
                    }
                    sums.push(FileSum {
                        path: text(f, "path", &at)?.to_owned(),
                        size: number(f, "size", &at)?,
                        blake3: hash(f, "blake3", &at)?,
                    });
                }
                Sums::Files(sums)
            }
            "tree" => Sums::Tree {
                count: usize::try_from(number(t, "count", &what)?)
                    .map_err(|_| format!("{what}: `count` is too large"))?,
                bytes: number(t, "bytes", &what)?,
                shape: hash(t, "shape", &what)?.ok_or_else(|| format!("{what}: `shape` is missing"))?,
                content: hash(t, "blake3", &what)?.ok_or_else(|| format!("{what}: `blake3` is missing"))?,
            },
            other => return Err(format!("{what}: unknown kind `{other}`")),
        };
        entries.push(Entry { key, sums, severity });
    }
    Ok(entries)
}

/// A TOML basic string.
fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The manifest text for `entries`, sorted by key, under the comment lines `header` (each without its `#`): the text
/// [`parse`] reads back to the same entries.
pub fn render(header: &[&str], entries: &[Entry]) -> String {
    let mut sorted: Vec<&Entry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.key.cmp(&b.key));
    let mut out = String::new();
    for line in header {
        let _ = writeln!(out, "# {line}");
    }
    out.push_str("format = 1\n");
    for entry in sorted {
        let _ = write!(out, "\n[[entry]]\nkey = {}\n", quoted(&entry.key));
        match &entry.sums {
            Sums::File { size, blake3 } => {
                let _ = writeln!(out, "kind = \"file\"\nsize = {size}");
                if let Some(hash) = blake3 {
                    let _ = writeln!(out, "blake3 = \"{hash}\"");
                }
            }
            Sums::Files(files) => {
                out.push_str("kind = \"files\"\nfiles = [\n");
                for file in files {
                    let _ = write!(out, "  {{ path = {}, size = {}", quoted(&file.path), file.size);
                    if let Some(hash) = &file.blake3 {
                        let _ = write!(out, ", blake3 = \"{hash}\"");
                    }
                    out.push_str(" },\n");
                }
                out.push_str("]\n");
            }
            Sums::Tree {
                count,
                bytes,
                shape,
                content,
            } => {
                let _ = writeln!(
                    out,
                    "kind = \"tree\"\ncount = {count}\nbytes = {bytes}\nshape = \"{shape}\"\nblake3 = \"{content}\""
                );
            }
        }
        if entry.severity == Severity::Warn {
            out.push_str("check = \"warn\"\n");
        }
    }
    out
}
