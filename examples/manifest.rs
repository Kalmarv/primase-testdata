//! Writes and checks the manifests (`manifests/*.toml`): the sums of the real-data inputs a good copy holds.
//!
//! ```text
//! cargo run --example manifest -- list <key> [--listing]     what the configured copy holds, as an entry
//! cargo run --example manifest -- add <key> [FILE...] [--warn] [--dir manifests]
//!                                                            write the entry for <key> into its manifest file
//! cargo run --example manifest -- verify [<key>...]          compare the configured copies with the shipped entries
//! ```
//!
//! A key is the one a query for the input has: `installs.sekiro`, `installs.sekiro/sekiro.exe`, `tools.oodle6`,
//! `oracles.corpus.sekiro/havok`. The copy is found where the `GAME_DATA` file puts it. `add` on an install (`installs.<game>`
//! with no path) needs the files that identify it, relative to the install folder (`sekiro.exe Data1.bhd ...`): the
//! folder holds files that vary between machines (logs, mods), which a manifest must not list. `add --warn` marks an
//! entry whose mismatch only warns (a tool installed with the system). `verify` reads every file, whatever `[run] verify`
//! says. Archive keys (`keys.*`) have no entry.
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use primase_testdata::digest::{self, HASH_LIMIT};
use primase_testdata::manifest::{self, Verify};
use primase_testdata::measure;
use primase_testdata::sums::{self, Entry, Severity};

fn usage() -> ExitCode {
    eprintln!(
        "usage: manifest list <key> [--listing] | add <key> [FILE...] [--warn] [--dir manifests] | verify [<key>...]"
    );
    ExitCode::from(2)
}

fn threads() -> usize {
    primase_testdata::threads().get()
}

/// The entry for the copy of `key` this machine holds.
fn measure_key(key: &str, files: &[String], severity: Severity) -> Result<Entry, String> {
    let path = primase_testdata::locate(key)
        .ok_or_else(|| format!("{key} is not configured in the GAME_DATA file"))?;
    measure::entry_for(key, &path, files, severity, threads())
}

fn header(name: &str) -> Vec<String> {
    vec![
        format!("primase-testdata manifest `{name}`: BLAKE3 sums of the real-data inputs a good copy holds."),
        "Written by `cargo run --example manifest -- add <key>`; src/sums.rs shows the format, src/digest.rs how the sums are made."
            .to_owned(),
    ]
}

fn list(key: &str, listing: bool) -> Result<(), String> {
    let entry = measure_key(key, &[], Severity::Fail)?;
    print!("{}", sums::render(&[], std::slice::from_ref(&entry)));
    if listing && let Some(path) = primase_testdata::locate(key) {
        for file in digest::list(&path).map_err(|e| e.to_string())? {
            let hash = if file.size <= HASH_LIMIT {
                digest::file_hash(&file.file).map_err(|e| e.to_string())?
            } else {
                "-".to_owned()
            };
            println!("{}\t{}\t{hash}", file.path, file.size);
        }
    }
    Ok(())
}

fn add(key: &str, files: &[String], severity: Severity, dir: &Path) -> Result<(), String> {
    let name = manifest::file_of(key).ok_or_else(|| format!("{key}: no manifest holds this key"))?;
    let entry = measure_key(key, files, severity)?;
    let target = dir.join(format!("{name}.toml"));
    let mut entries = match std::fs::read_to_string(&target) {
        Ok(text) => sums::parse(&name, &text)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(format!("{}: {e}", target.display())),
    };
    entries.retain(|e| e.key != key);
    entries.push(entry);
    let header = header(&name);
    let lines: Vec<&str> = header.iter().map(String::as_str).collect();
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::write(&target, sums::render(&lines, &entries))
        .map_err(|e| format!("{}: {e}", target.display()))?;
    println!("{key}: written to {}", target.display());
    Ok(())
}

fn verify(keys: &[String]) -> bool {
    let shipped = manifest::shipped();
    let wanted: Vec<&Entry> = if keys.is_empty() {
        shipped.entries().collect()
    } else {
        keys.iter().filter_map(|k| shipped.get(k)).collect()
    };
    let mut all_good = true;
    for entry in wanted {
        let Some(path) = primase_testdata::locate(&entry.key) else {
            println!("SKIP {} (not configured)", entry.key);
            continue;
        };
        if !path.exists() {
            println!("SKIP {} ({} does not exist)", entry.key, path.display());
            continue;
        }
        match manifest::verify(entry, &path, Verify::Content, threads()) {
            Ok(()) => println!("OK   {}", entry.key),
            Err(why) => {
                all_good = entry.severity == Severity::Warn && all_good;
                println!("DIFF {}: {why}", entry.key);
            }
        }
    }
    all_good
}

/// The words and flags of the command line.
struct Args {
    words: Vec<String>,
    warn: bool,
    listing: bool,
    dir: PathBuf,
}

fn parse(args: impl Iterator<Item = String>) -> Option<Args> {
    let mut parsed = Args {
        words: Vec::new(),
        warn: false,
        listing: false,
        dir: PathBuf::from("manifests"),
    };
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--warn" => parsed.warn = true,
            "--listing" => parsed.listing = true,
            "--dir" => parsed.dir = PathBuf::from(args.next()?),
            flag if flag.starts_with("--") => return None,
            _ => parsed.words.push(arg),
        }
    }
    Some(parsed)
}

fn main() -> ExitCode {
    let Some(args) = parse(std::env::args().skip(1)) else {
        return usage();
    };
    let result = match args.words.split_first() {
        Some((command, rest)) if command == "list" && rest.len() == 1 => list(&rest[0], args.listing),
        Some((command, rest)) if command == "add" && !rest.is_empty() => {
            let severity = if args.warn { Severity::Warn } else { Severity::Fail };
            add(&rest[0], &rest[1..], severity, &args.dir)
        }
        Some((command, rest)) if command == "verify" => {
            return if verify(rest) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            };
        }
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}
