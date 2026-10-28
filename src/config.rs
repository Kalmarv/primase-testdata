//! The real-data configuration: the TOML file `GAME_DATA` names, read into one `Config`. One file serves every
//! consumer, each with its own registry, so its sections are checked and the keys of `[installs]`, `[keys]` and
//! `[tools]` are not: what one consumer does not declare another may.
//!
//! ```toml
//! [installs]
//! sekiro = 'D:/placeholder/Sekiro'
//! [keys]
//! gta = 'D:/placeholder/gta-keys'
//! [tools]
//! oodle6 = 'D:/placeholder/oo2core_6_win64.dll'
//! [oracles]
//! root = 'D:/placeholder/oracles'   # <root>/<set>/<game>/<file>
//! [scratch]
//! root = 'D:/placeholder/scratch'
//! [run]
//! require = 'gate'                  # or 'all': an unset input fails instead of skipping
//! gate = 'smoke'                    # or 'full': how much the heavy real-data tests cover (smoke by default)
//! seed = '20384'                    # the smoke sample's seed (the day's by default), to rerun one exactly
//! threads = '1'                     # the threads a heavy real-data test's work may use (one by default)
//! verify = 'quick'                  # or 'content' or 'off': how much of a copy is compared with its manifest
//! ```
//!
//! Every section and key is optional. An unknown section is an error (a typo must not skip quietly), and so is an
//! unknown key of `[oracles]`, `[scratch]` and `[run]`, a value that is not a string, and a flat map of names to
//! strings with no sections.
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// What the configuration file says.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Config {
    /// The file it was read from; `None` when `GAME_DATA` is unset.
    pub file: Option<PathBuf>,
    /// The structured sections, by full key (`installs.sekiro`, `tools.oodle6`, `oracles.root`, `scratch.root`).
    pub roots: BTreeMap<String, PathBuf>,
    /// `[run] require`: `gate` or `all`.
    pub require: Option<String>,
    /// `[run] gate`: `smoke` or `full`.
    pub gate: Option<String>,
    /// `[run] seed`: a smoke sample's seed.
    pub seed: Option<u64>,
    /// `[run] threads`: the threads a heavy real-data test's work may use.
    pub threads: Option<std::num::NonZeroUsize>,
    /// `[run] verify`: `off`, `quick` or `content`.
    pub verify: Option<String>,
}

/// Why the configuration file could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file `GAME_DATA` names could not be read.
    #[error("cannot read GAME_DATA file {}: {source}", path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// The I/O error.
        source: std::io::Error,
    },
    /// The file is not a valid configuration: `detail` says which section, key or value and what was expected.
    #[error("GAME_DATA file {}: {detail}", path.display())]
    Invalid {
        /// The file (`(text)` for [`validate`]).
        path: PathBuf,
        /// What is wrong with it.
        detail: String,
    },
}

/// The sections of the structured format and the keys each may hold; `None` for a section whose keys are the
/// consumers' (installs, keys and tools), where any key is accepted.
fn section_keys(section: &str) -> Option<Option<&'static [&'static str]>> {
    match section {
        "installs" | "keys" | "tools" => Some(None),
        "oracles" | "scratch" => Some(Some(&["root"])),
        "run" => Some(Some(&["require", "gate", "seed", "threads", "verify"])),
        _ => None,
    }
}

/// `a, b or c`.
fn or_list(words: &[&str]) -> String {
    match words.split_last() {
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        None => String::new(),
    }
}

impl Config {
    /// The configuration `GAME_DATA` (read through `environment`) names, read with `read`; empty when unset.
    pub(crate) fn load(
        environment: &impl Fn(&str) -> Option<OsString>,
        read: impl Fn(&Path) -> std::io::Result<String>,
    ) -> Result<Config, ConfigError> {
        let mut config = Config::default();
        if let Some(path) = environment("GAME_DATA").map(PathBuf::from) {
            let text = read(&path).map_err(|source| ConfigError::Read {
                path: path.clone(),
                source,
            })?;
            config.parse(&path, &text)?;
            config.file = Some(path);
        }
        Ok(config)
    }

    fn parse(&mut self, path: &Path, text: &str) -> Result<(), ConfigError> {
        let invalid = |detail: String| ConfigError::Invalid {
            path: path.to_owned(),
            detail,
        };
        let table: toml::Table = toml::from_str(text).map_err(|e| invalid(e.to_string()))?;
        if !table.is_empty() && table.values().all(|v| v.is_str()) {
            return Err(invalid(
                "a flat map of names with no sections: write the structured sections".into(),
            ));
        }
        for (section, body) in &table {
            let Some(keys) = section_keys(section) else {
                return Err(invalid(format!(
                    "unknown section [{section}]: expected installs, keys, tools, oracles, scratch or run"
                )));
            };
            let body = body
                .as_table()
                .ok_or_else(|| invalid(format!("[{section}] is not a table")))?;
            for (key, value) in body {
                if let Some(keys) = keys
                    && !keys.contains(&key.as_str())
                {
                    return Err(invalid(format!(
                        "unknown key {key} in [{section}]: expected {}",
                        or_list(keys)
                    )));
                }
                let value = value
                    .as_str()
                    .ok_or_else(|| invalid(format!("{section}.{key} is not a string")))?;
                if section == "run" {
                    match key.as_str() {
                        "require" if matches!(value, "gate" | "all") => self.require = Some(value.to_owned()),
                        "require" => {
                            return Err(invalid(format!("run.require = '{value}': 'gate' or 'all'")));
                        }
                        "gate" if matches!(value, "smoke" | "full") => self.gate = Some(value.to_owned()),
                        "gate" => return Err(invalid(format!("run.gate = '{value}': 'smoke' or 'full'"))),
                        "verify" if matches!(value, "off" | "quick" | "content") => {
                            self.verify = Some(value.to_owned())
                        }
                        "verify" => {
                            return Err(invalid(format!(
                                "run.verify = '{value}': 'off', 'quick' or 'content'"
                            )));
                        }
                        "seed" => {
                            self.seed = Some(
                                value
                                    .parse()
                                    .map_err(|_| invalid(format!("run.seed = '{value}': a whole number")))?,
                            )
                        }
                        _ => {
                            self.threads = Some(value.parse().map_err(|_| {
                                invalid(format!("run.threads = '{value}': a whole number, at least 1"))
                            })?)
                        }
                    }
                } else {
                    self.roots
                        .insert(format!("{section}.{key}"), PathBuf::from(value));
                }
            }
        }
        Ok(())
    }
}

/// Check `text` as a configuration file (the example file's check): its sections and keys known, its values strings,
/// `[run] require` one of `gate` and `all`, `[run] gate` one of `smoke` and `full`, `[run] seed` a whole number, `[run]
/// threads` a whole number from 1. A flat map with no sections is refused.
pub fn validate(text: &str) -> Result<(), ConfigError> {
    Config::default().parse(Path::new("(text)"), text)
}

/// The value `GAME_DATA_DEBUG` (read through `environment`) gives `key` (`key=value;key=value`).
pub(crate) fn debug(environment: &impl Fn(&str) -> Option<OsString>, key: &str) -> Option<String> {
    let all = environment("GAME_DATA_DEBUG")?;
    all.to_string_lossy().split(';').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k.trim() == key).then(|| v.trim().to_owned())
    })
}
