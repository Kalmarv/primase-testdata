//! Manifests: the sums of the real-data inputs a good copy holds, checked before a test trusts the copy it was given.
//!
//! The tests of a crate that reads a game's files compare what they read against counts measured on one build of the
//! game, with the oracle files one version of a tool produced. A copy that differs (another patch of the game, an
//! oracle folder made by another tool) makes those tests fail with a wrong count or a refused entry and no hint why. A
//! manifest says what the good copy is: this crate ships one file per game under `manifests/` (`sekiro.toml`, …, and
//! `tools.toml`), embedded in the library, with one entry for each input a query can name ([`crate::sums`] shows the
//! format; [`crate::digest`] says how the sums are made).
//!
//! The check is part of every query: [`crate::resolve`] (and so every typed query a consumer's [`crate::inputs!`] gives)
//! compares the path it found with the entry of the same key, once for each key in a process, and fails the test that
//! asked, naming the key and what differs, when the copy is not the good one. What it reads depends on `[run] verify`
//! in the `GAME_DATA` file:
//!
//! | `verify` | a file | files below an install | a folder |
//! |---|---|---|---|
//! | `quick` (the default) | size, and the hash when it is no larger than [`HASH_LIMIT`] | the same for each | the number of files, their sizes and names (the shape digest) |
//! | `content` | the same | the same | and the hash of every file no larger than [`HASH_LIMIT`] |
//! | `off` | nothing | nothing | nothing |
//!
//! A key with no entry is not checked. Archive keys are never in a manifest. Entries are written with the `manifest`
//! example (`cargo run --example manifest -- add <key>` on a machine that holds the input), which also lists what a copy
//! holds and checks a configuration against the shipped sums.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::digest::{self, HASH_LIMIT};
use crate::query::Query;
use crate::registry::{Registry, Status};
pub use crate::sums::{Entry, FileSum, Severity, Sums};

/// How much of a copy a run reads to compare it with its manifest entry (`[run] verify`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verify {
    /// Nothing: run on whatever is there.
    Off,
    /// Sizes everywhere, hashes of files up to the hash limit, the shape of folders: the default.
    Quick,
    /// Quick, and the hash of every file in a folder up to the hash limit.
    Content,
}

impl Verify {
    /// The mode `[run] verify` names.
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "off" => Some(Self::Off),
            "quick" => Some(Self::Quick),
            "content" => Some(Self::Content),
            _ => None,
        }
    }
}

/// The manifests this crate ships: `(file name, text)`. A new file under `manifests/` is listed here too (a test fails
/// when one is not).
pub(crate) const EMBEDDED: &[(&str, &str)] = &[
    ("bigwalk", include_str!("../manifests/bigwalk.toml")),
    ("elden", include_str!("../manifests/elden.toml")),
    ("general", include_str!("../manifests/general.toml")),
    ("gta", include_str!("../manifests/gta.toml")),
    ("peak", include_str!("../manifests/peak.toml")),
    ("riders", include_str!("../manifests/riders.toml")),
    ("sekiro", include_str!("../manifests/sekiro.toml")),
    ("shaunwhite", include_str!("../manifests/shaunwhite.toml")),
    ("shred", include_str!("../manifests/shred.toml")),
    ("skate", include_str!("../manifests/skate.toml")),
    ("skatebird", include_str!("../manifests/skatebird.toml")),
    ("sledding", include_str!("../manifests/sledding.toml")),
    ("stuntboost", include_str!("../manifests/stuntboost.toml")),
    ("tools", include_str!("../manifests/tools.toml")),
];

/// All shipped entries, by key.
#[derive(Debug)]
pub struct Manifest {
    entries: BTreeMap<String, Entry>,
}

impl Manifest {
    /// The manifests `files` (name and text), read together; a key listed twice is an error.
    pub fn from_texts(files: &[(&str, &str)]) -> Result<Self, String> {
        let mut entries = BTreeMap::new();
        for (name, text) in files {
            for entry in crate::sums::parse(name, text)? {
                if let Some(first) = entries.insert(entry.key.clone(), entry) {
                    return Err(format!("{name}: {} is listed twice", first.key));
                }
            }
        }
        Ok(Self { entries })
    }

    /// The entry for `key`.
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.get(key)
    }

    /// Every entry, by key.
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.values()
    }
}

/// The manifests this crate ships, read once. A malformed one is a defect of the crate: it fails on first use.
pub fn shipped() -> &'static Manifest {
    static SHIPPED: OnceLock<Manifest> = OnceLock::new();
    SHIPPED.get_or_init(|| {
        Manifest::from_texts(EMBEDDED)
            .unwrap_or_else(|e| panic!("the manifests shipped with primase-testdata: {e}"))
    })
}

/// The name of the manifest file a key's entry belongs in: its game for an install or an oracle, `tools` for a tool;
/// `None` for a key no manifest holds (archive keys, the scratch folder, debug keys).
pub fn file_of(key: &str) -> Option<String> {
    if key.starts_with("tools.") {
        return Some("tools".to_owned());
    }
    if let Some(rest) = key.strip_prefix("installs.") {
        return Some(rest.split('/').next().unwrap_or(rest).to_owned());
    }
    // oracles.<set>.<game>/<path>
    let rest = key.strip_prefix("oracles.")?;
    let (head, _) = rest.split_once('/')?;
    let (_, game) = head.rsplit_once('.')?;
    Some(game.to_owned())
}

/// The configuration key a query key is resolved from, and the path below it: `installs.sekiro/sekiro.exe` is
/// (`installs.sekiro`, `sekiro.exe`), `oracles.corpus.sekiro/havok` is (`oracles.root`, `corpus/sekiro/havok`).
pub fn root_of(key: &str) -> Option<(String, PathBuf)> {
    let split = |s: &str| -> PathBuf { s.split('/').filter(|p| !p.is_empty()).collect() };
    if let Some(rest) = key.strip_prefix("installs.") {
        let (game, rel) = rest.split_once('/').unwrap_or((rest, ""));
        return Some((format!("installs.{game}"), split(rel)));
    }
    if key.starts_with("tools.") {
        return Some((key.to_owned(), PathBuf::new()));
    }
    let rest = key.strip_prefix("oracles.")?;
    let (head, rel) = rest.split_once('/')?;
    let (set, game) = head.rsplit_once('.')?;
    Some(("oracles.root".to_owned(), split(&format!("{set}/{game}/{rel}"))))
}

/// Compare the copy at `path` with `entry`: `Ok` when it is the good one, else what differs. `mode` is `Quick` or
/// `Content` (`Off` checks nothing); `threads` is how many threads hashing a folder's files may use.
pub fn verify(entry: &Entry, path: &Path, mode: Verify, threads: usize) -> Result<(), String> {
    if mode == Verify::Off {
        return Ok(());
    }
    let io = |what: &Path, e: std::io::Error| format!("{}: {e}", what.display());
    match &entry.sums {
        Sums::File { size, blake3 } => file_matches(path, *size, blake3.as_deref()),
        Sums::Files(files) => {
            for file in files {
                let at = path.join(file.path.split('/').collect::<PathBuf>());
                file_matches(&at, file.size, file.blake3.as_deref())
                    .map_err(|e| format!("{}: {e}", file.path))?;
            }
            Ok(())
        }
        Sums::Tree {
            count,
            bytes,
            shape,
            content,
        } => {
            let listing = digest::list(path).map_err(|e| io(path, e))?;
            let total: u64 = listing.iter().map(|f| f.size).sum();
            if listing.len() != *count || total != *bytes {
                return Err(format!(
                    "{} files of {total} bytes, where the good copy has {count} of {bytes}",
                    listing.len()
                ));
            }
            if digest::shape_digest(&listing) != *shape {
                return Err(format!(
                    "{} files of {total} bytes, as many as the good copy has, but their names or sizes differ (the shape digest differs)",
                    listing.len()
                ));
            }
            if mode == Verify::Content {
                let hashes = digest::hash_files(&listing, threads).map_err(|e| io(path, e))?;
                if digest::content_digest(&listing, &hashes) != *content {
                    let small = hashes.iter().flatten().count();
                    return Err(format!(
                        "the names and sizes agree but the content differs (content digest over {small} hashed files)"
                    ));
                }
            }
            Ok(())
        }
    }
}

/// A file's size, and its hash when `blake3` is given.
fn file_matches(path: &Path, size: u64, blake3: Option<&str>) -> Result<(), String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a file", path.display()));
    }
    if meta.len() != size {
        return Err(format!(
            "is {} bytes, where the good copy's is {size}",
            meta.len()
        ));
    }
    if let Some(want) = blake3.filter(|_| size <= HASH_LIMIT) {
        let got = digest::file_hash(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if got != want {
            return Err(format!(
                "has the BLAKE3 hash {got}, where the good copy's is {want}"
            ));
        }
    }
    Ok(())
}

type Verdicts = Mutex<BTreeMap<String, Arc<OnceLock<Result<(), String>>>>>;

/// Check the copy `query` was resolved to against the shipped entry for its key, once for each key in this process.
/// A mismatch fails the calling test, or prints one warning line for an entry marked `check = "warn"`. Archive keys, the
/// scratch folder and debug keys are never checked, nor a key with no entry, nor anything under `Verify::Off`.
#[track_caller]
pub(crate) fn enforce(query: Query, path: &Path, mode: Verify, threads: usize) {
    if mode == Verify::Off || matches!(query, Query::Keys(_) | Query::Scratch(_) | Query::Debug(_)) {
        return;
    }
    let key = query.key();
    let Some(entry) = shipped().get(&key) else { return };
    static VERDICTS: Verdicts = Mutex::new(BTreeMap::new());
    let cell = {
        let mut all = VERDICTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(all.entry(key.clone()).or_default())
    };
    let verdict = cell.get_or_init(|| {
        let verdict = verify(entry, path, mode, threads);
        if let (Err(why), Severity::Warn) = (&verdict, entry.severity) {
            eprintln!("primase-testdata: WARN {}", describe(&key, path, why));
        }
        verdict
    });
    if let (Err(why), Severity::Fail) = (verdict, entry.severity) {
        panic!("{}", describe(&key, path, why));
    }
}

/// The message for a copy that is not the good one.
fn describe(key: &str, path: &Path, why: &str) -> String {
    format!(
        "{key} at {} is not the copy this crate's manifests describe: {why}. The tests compare it with counts \
         measured on that copy, so a different build fails them in confusing ways. Use the matching build, or set \
         `[run] verify = 'off'` in the GAME_DATA file to run on it anyway.",
        path.display()
    )
}

/// The inputs of `registry` that a manifest should list and does not, as (required, optional) lists of messages. Required:
/// a live input a complete run must have (`gate: true`: an install, a tool, a declared file); optional: a live input
/// that is not. Archive keys, blocked and unheld inputs are never listed.
pub fn unlisted(registry: &Registry) -> (Vec<String>, Vec<String>) {
    let manifest = shipped();
    let (mut required, mut optional) = (Vec::new(), Vec::new());
    let mut check = |key: String, gate: bool| {
        if manifest.get(&key).is_none() {
            let line = format!(
                "{key}: no manifest entry (add one with primase-testdata's `manifest` example, `add {key}`, where the input is held)"
            );
            (if gate { &mut required } else { &mut optional }).push(line);
        }
    };
    for (key, entry) in registry.entries() {
        if key.starts_with("installs.") || key.starts_with("tools.") {
            check(key, entry.gate);
        }
    }
    for declared in registry.declared {
        if declared.status == Status::Live {
            check(declared.query.key(), declared.gate);
        }
    }
    (required, optional)
}
