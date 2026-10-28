//! Real-data inputs for tests: where a test finds a game's install, archive keys, a third-party tool or the reference
//! output it compares against, without any path in the test itself.
//!
//! A crate whose tests read files nobody can ship (a game's own data) declares what they read once, in
//! `tests/inputs/mod.rs`, with [`inputs!`]. The macro gives the tests typed queries (`install`, `install_file`, `keys`,
//! `tool`, `oodle`, `oracle`, `debug`). A person who has the files writes one TOML file, names it in the `GAME_DATA`
//! environment variable ([`config`] shows its layout) and the same file serves every crate that uses this one.
//!
//! - An entry the file does not set makes the test print one `SKIP` line naming its configuration key and return,
//!   unless `[run] require` says a complete run must have it.
//! - A path the file sets that does not exist fails the test.
//! - A file the crate has not declared fails the test: a mistyped path never skips quietly.
//! - A path a query finds is compared with the BLAKE3 sums of a good copy in this crate's manifests ([`manifest`]) before a
//!   test trusts it; `[run] verify = 'off'` skips the comparison.
//! - [`check_declared`] runs as one test in each consumer and checks its sources against its declarations; [`doc`] makes
//!   the table of inputs for its README.
//! - What a test compares against reference output it counts in a [`gate::GateReport`].
//!
//! - "gate" names three things, each its own: `gate: true` on a declaration says a complete run must have that input
//!   (`[run] require = 'gate'` enforces it); `[run] gate = 'smoke'|'full'` is how much a heavy test covers ([`Depth`]);
//!   and a [`gate::GateReport`] is a comparison against reference output.
//!
//! Add this crate only under `[dev-dependencies]`.
//!
//! ```
//! mod inputs {
//!     primase_testdata::inputs! {
//!         games {
//!             Sekiro = "sekiro",
//!         }
//!         installs {
//!             Sekiro {
//!                 about: "Sekiro's install folder",
//!                 source: "the user's own install",
//!                 gate: true,
//!             },
//!         }
//!         keys {}
//!         tools {
//!             Oodle6 = "oodle6" {
//!                 about: "an Oodle version-6 DLL",
//!                 source: "the game's own DLL from the user's install",
//!                 gate: true,
//!             },
//!         }
//!         oracle_sets {
//!             Corpus = "corpus" {
//!                 about: "the games' own files cut from their installs",
//!                 source: "the user's own installs: files extracted as stored",
//!             },
//!         }
//!         declared {
//!             Query::Oracle(OracleSet::Corpus, Game::Sekiro, "dcx") => {
//!                 status: Live,
//!                 gate: false,
//!                 about: "Sekiro's DCX files as stored",
//!                 source: "the user's own install",
//!             },
//!         }
//!         debug {
//!             "oodle.dump" => "a folder under the scratch root the Oodle gate writes to",
//!         }
//!     }
//! }
//!
//! use inputs::{Game, OracleSet};
//!
//! fn main() {
//!     assert!(inputs::REGISTRY.problems().is_empty());
//!     let Some(install) = inputs::install(Game::Sekiro) else {
//!         return; // `installs.sekiro` is not configured here: the SKIP line names it
//!     };
//!     println!("Sekiro is installed at {}", install.display());
//!     if let Some(folder) = inputs::oracle(OracleSet::Corpus, Game::Sekiro, "dcx") {
//!         println!("DCX files are in {}", folder.display());
//!     }
//! }
//! ```
#![deny(missing_docs)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

mod check;
pub mod config;
pub mod digest;
pub mod doc;
pub mod gate;
mod inputs;
pub mod manifest;
pub mod measure;
mod query;
mod registry;
pub mod sums;
pub use check::{check_declared, problems};
pub use query::Query;
pub use registry::{Declared, Entry, Names, Registry, SCRATCH, Set, Status};

use config::Config;

fn environment(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

/// The configuration, read and parsed on the first query and kept for the process: `GAME_DATA` is read once, so every
/// query of a run sees the same file. A named but unreadable or invalid file fails each query with the same message.
#[track_caller]
fn load() -> &'static Config {
    static CONFIG: std::sync::OnceLock<Result<Config, String>> = std::sync::OnceLock::new();
    cached(&CONFIG, || {
        Config::load(&environment, |path| std::fs::read_to_string(path))
    })
}

/// What `cell` holds, filled by `load` on the first call: a failure is kept as its message and fails every call.
#[track_caller]
fn cached(
    cell: &std::sync::OnceLock<Result<Config, String>>,
    load: impl FnOnce() -> Result<Config, config::ConfigError>,
) -> &Config {
    cell.get_or_init(|| load().map_err(|e| e.to_string()))
        .as_ref()
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Whether a complete run must have what `query` names: a declared file's own flag ([`Registry::declared`]), else
/// its configuration key's ([`Registry::installs`], [`Registry::keys`], [`Registry::tools`]). A file in an install or
/// an oracle set that is not declared fails, and so does an install, keys or a tool the registry does not declare: a
/// mistyped path must not skip quietly.
#[track_caller]
fn gate_of(registry: &Registry, query: Query) -> bool {
    match query {
        Query::Install(_, Some(_)) | Query::Oracle(..) => {
            registry
                .declaration(query)
                .unwrap_or_else(|| {
                    panic!(
                        "{} is not declared in this crate's tests/inputs/mod.rs (declared): declare it there, with \
                         what it holds and where it comes from",
                        query.key()
                    )
                })
                .gate
        }
        Query::Install(..) | Query::Keys(_) | Query::Tool(_) => {
            let key = query.key();
            registry
                .entry(&key)
                .unwrap_or_else(|| panic!("{key} is not declared in this crate's tests/inputs/mod.rs"))
                .gate
        }
        Query::Scratch(_) | Query::Debug(_) => false,
    }
}

/// Where `query` points, if configured: its configuration key's path with the query's relative path below it.
fn configured(config: &Config, query: Query) -> Option<PathBuf> {
    let (key, rel) = query.root()?;
    // an install, a tool or keys configured as a file is that file: joining an empty path would add a separator
    config.roots.get(&key).map(|root| {
        if rel.as_os_str().is_empty() {
            root.clone()
        } else {
            root.join(rel)
        }
    })
}

/// What [`resolve`] does with a query: the path, a skip or a failure, with why.
#[derive(Debug, PartialEq)]
enum Decision {
    Found(PathBuf),
    /// Not configured on this machine (and not required): the reason the SKIP line gives.
    Skip(String),
    Fail(String),
}

/// The decision for `query` under `config`: a configured path that exists is found and one that does not fails, except
/// an oracle file whose set this machine does not hold (no `<oracles.root>/<set>` folder in a root that exists): that
/// set is not configured here, as an unset key is. What is not configured skips, or fails where `[run] require` needs
/// it (`all`, or `gate` and `gate` says a complete run must have it).
fn decide(registry: &Registry, config: &Config, query: Query, gate: bool) -> Decision {
    // an input no machine here holds: used where it is there, else skipped naming why, whatever require says
    if let Some(Declared {
        status: Status::Unheld { why },
        ..
    }) = registry.declaration(query)
    {
        return match configured(config, query).filter(|p| p.exists()) {
            Some(path) => Decision::Found(path),
            None => Decision::Skip(format!("{} is held by no machine here: {why}", query.key())),
        };
    }
    let require = match config.require.as_deref() {
        Some("all") => true,
        Some("gate") => gate,
        _ => false,
    };
    let file = || {
        config.file.as_ref().map_or_else(
            || " (GAME_DATA unset)".to_owned(),
            |f| format!(" in {}", f.display()),
        )
    };
    let unset = match (query, configured(config, query)) {
        (_, None) => format!("{} not configured{}", query.key(), file()),
        (Query::Oracle(set, ..), Some(path)) => {
            let root = config.roots.get("oracles.root");
            if let Some(root) = root.filter(|r| !r.exists()) {
                return Decision::Fail(format!(
                    "oracles.root = {} is set but does not exist",
                    root.display()
                ));
            }
            match root.map(|r| r.join(set)) {
                Some(folder) if !folder.exists() => {
                    format!("oracles.{set} not on this machine: no {}", folder.display())
                }
                _ if path.exists() => return Decision::Found(path),
                _ => {
                    return Decision::Fail(format!(
                        "{} = {} is set but does not exist",
                        query.key(),
                        path.display()
                    ));
                }
            }
        }
        (_, Some(path)) if path.exists() => return Decision::Found(path),
        (_, Some(path)) => {
            return Decision::Fail(format!(
                "{} = {} is set but does not exist",
                query.key(),
                path.display()
            ));
        }
    };
    if require {
        Decision::Fail(format!("{unset}, and [run] require needs it"))
    } else {
        Decision::Skip(unset)
    }
}

/// Resolve `query`, which `registry` must declare, to an existing path, or skip: what is not configured prints one SKIP
/// line naming the test and the configuration key, unless the configuration's `[run] require` is `all`, or `gate` and
/// the entry is one a complete run must have (a blocked file always is). An oracle set this machine does not hold
/// (no `<oracles.root>/<set>` folder) counts as not configured; a configured path that does not exist fails (inside a
/// set that is there: a typo or a broken tree), and so does an undeclared query.
#[track_caller]
pub fn resolve(registry: &Registry, query: Query) -> Option<PathBuf> {
    let gate = gate_of(registry, query);
    match decide(registry, load(), query, gate) {
        Decision::Found(path) => {
            manifest::enforce(query, &path, verify_mode(), threads().get());
            Some(path)
        }
        Decision::Fail(why) => panic!("{why}"),
        Decision::Skip(why) => {
            let test = std::thread::current().name().unwrap_or("?").to_owned();
            eprintln!("primase-testdata: SKIP {test} ({why})");
            None
        }
    }
}

/// Where a test gets its Oodle codec: the game's own library on Windows, else nothing to configure.
#[derive(Clone, Debug)]
pub struct OodleLibrary(Option<PathBuf>);

impl OodleLibrary {
    /// The configured game library: `Some` on Windows, where the test loads it as the codec; `None` elsewhere, where
    /// the test uses a portable decoder instead.
    pub fn path(&self) -> Option<&Path> {
        self.0.as_deref()
    }
}

/// The Oodle codec for a game's data (`version`: the tool's key, `oodle5`, `oodle6` or `oodle9`): on Windows the game's
/// library, which `tools.<version>` names (a test skips, or fails under `[run] require`, where it is not configured);
/// elsewhere there is nothing to configure and nothing to skip for, and the result holds no path.
#[track_caller]
pub fn oodle(registry: &Registry, version: &'static str) -> Option<OodleLibrary> {
    if cfg!(windows) {
        resolve(registry, Query::Tool(version)).map(|path| OodleLibrary(Some(path)))
    } else {
        Some(OodleLibrary(None))
    }
}

/// Where a real-data test writes what it keeps: the configured scratch root, else the OS temporary folder.
#[track_caller]
pub fn scratch() -> PathBuf {
    configured(load(), Query::Scratch(None)).unwrap_or_else(std::env::temp_dir)
}

/// A folder `name` of [`scratch`] that only this process uses ([`own_scratch`]), removed when it is dropped unless the test
/// is failing (what it wrote stays to be looked at).
#[derive(Debug)]
pub struct OwnScratch(PathBuf);

impl std::ops::Deref for OwnScratch {
    type Target = std::path::Path;
    fn deref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for OwnScratch {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            // best effort: a folder another process still holds open is left for the next sweep of the scratch root
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

/// A folder `name` under [`scratch`] this call alone writes to (`<scratch>/<name>/p<pid>-<call>`, made): a test that
/// exchanges files with an external program (a compiler, a rasterizer) under fixed names writes them here, so two runs of
/// it at once (two checkouts, a CI job beside a local run) cannot overwrite or truncate each other's files while the program
/// reads them.
#[track_caller]
pub fn own_scratch(name: &str) -> OwnScratch {
    // one folder a call: two tests of one process each own theirs, and one's drop does not take the other's files
    static CALLS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = scratch()
        .join(name)
        .join(format!("p{}-{call}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    OwnScratch(dir)
}

/// The machine-wide lock on the WARP rasterizer program ([`warp_exclusive`]), released when it is dropped.
#[derive(Debug)]
pub struct WarpLock(std::fs::File);

impl Drop for WarpLock {
    fn drop(&mut self) {
        // closing the file would release it too; this says so
        let _ = self.0.unlock();
    }
}

/// Takes the lock on Microsoft's WARP software rasterizer program (`warp.exe`, a tool a consumer may configure),
/// waiting for whoever holds it: a test runs the program only while it holds the lock.
///
/// Several runs of that program at once (two test binaries, two checkouts) were observed not to give each run its own
/// results: four copies of one comparison test started together mostly failed (an empty results file, a cut line),
/// where a single copy passed every time. The lock is `warp/.oracle.lock` in the [`scratch`] root, held with the
/// operating system's own file lock, so it is released if the holder dies.
#[track_caller]
pub fn warp_exclusive() -> WarpLock {
    let dir = scratch().join("warp");
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let path = dir.join(".oracle.lock");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    file.lock()
        .unwrap_or_else(|e| panic!("locking {}: {e}", path.display()));
    WarpLock(file)
}

/// A debug key's value (`GAME_DATA_DEBUG=key=value;key=value`); `None` when unset. The key must be one `registry`
/// declares ([`Registry::debug`]).
#[track_caller]
pub fn debug(registry: &Registry, key: &str) -> Option<String> {
    assert!(
        registry.debug.iter().any(|(k, _)| *k == key),
        "debug key {key} is not declared in this crate's tests/inputs/mod.rs"
    );
    config::debug(&environment, key)
}

/// How much of a copy is compared with its manifest entry (`[run] verify`): [`manifest::Verify::Quick`] when unset.
pub fn verify_mode() -> manifest::Verify {
    load()
        .verify
        .as_deref()
        .and_then(manifest::Verify::parse)
        .unwrap_or(manifest::Verify::Quick)
}

/// Where the configuration puts the input a manifest key names (`installs.sekiro`, `installs.sekiro/sekiro.exe`,
/// `tools.oodle6`, `oracles.corpus.sekiro/havok`), whether or not it exists; `None` when the configuration does not set
/// it or the key is not one a manifest holds. The manifest example reads and writes entries through it.
pub fn locate(key: &str) -> Option<PathBuf> {
    let (root, rel) = manifest::root_of(key)?;
    let base = load().roots.get(&root)?;
    Some(if rel.as_os_str().is_empty() {
        base.clone()
    } else {
        base.join(rel)
    })
}

/// How much a heavy real-data test covers (`[run] gate`): a sample, or everything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    /// A sample, drawn with [`seed`]: the default.
    Smoke,
    /// The whole input.
    Full,
}

/// `[run] gate`: [`Depth::Full`] for `full`, else [`Depth::Smoke`].
pub fn depth() -> Depth {
    if load().gate.as_deref() == Some("full") {
        Depth::Full
    } else {
        Depth::Smoke
    }
}

/// The seed a smoke sample is drawn with: `[run] seed` when given (to rerun one sample exactly), else the day (days
/// since 1970-01-01, UTC), so the sample rotates and its coverage accumulates across runs. A test prints the seed it
/// used and names it in its failures.
pub fn seed() -> u64 {
    load().seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() / 86_400)
    })
}

/// The threads a heavy real-data test's work may use (`[run] threads`, one by default), so a machine opts into the
/// load. For work whose result is the same on any thread count only.
pub fn threads() -> std::num::NonZeroUsize {
    load().threads.unwrap_or(std::num::NonZeroUsize::MIN)
}

/// The bytes of a real-data input file: `std::fs::read` of the path a query resolved, or of a file inside the folder
/// it resolved.
pub fn read_input(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    std::fs::read(path)
}

/// What the registry says `query` names: a declared file's description, else its configuration key's.
fn about_of(registry: &Registry, query: Query) -> &'static str {
    if let Some(d) = registry.declaration(query) {
        return d.about;
    }
    let key = query.root().map(|(key, _)| key).unwrap_or_default();
    registry.entry(&key).map_or("", |e| e.about)
}

/// A configured input that yields nothing is a failure, not a pass: call it after enumerating what `query` gave
/// (`what`: "scene folders", "drawables"), with how many were found.
#[track_caller]
pub fn expect_found(registry: &Registry, query: Query, what: &str, n: usize) {
    if n == 0 {
        let about = about_of(registry, query);
        panic!("{} is set but yielded 0 {what} (expected: {about})", query.key());
    }
}

#[cfg(test)]
mod tests;
