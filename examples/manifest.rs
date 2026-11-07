//! Writes and checks the manifests (`manifests/*.toml`): the sums of the real-data inputs a good copy holds.
//!
//! ```text
//! cargo run --example manifest -- list <key> [--listing]     what the configured copy holds, as an entry
//! cargo run --example manifest -- add <key> [FILE...] [--except PATH...] [--warn] [--zip] [--dir manifests]
//!                                                            write the entry for <key> into its manifest file
//! cargo run --example manifest -- verify [<key>...]          compare the configured copies with the shipped entries
//! ```
//!
//! A key is the one a query for the input has: `installs.sekiro`, `installs.sekiro/sekiro.exe`, `tools.oodle6`,
//! `oracles.corpus.sekiro/havok`. The copy is found where the `GAME_DATA` file puts it. `add` on an install (`installs.<game>`
//! with no path) needs the files that identify it, relative to the install folder (`sekiro.exe Data1.bhd ...`): the
//! folder holds files that vary between machines (logs, mods), which a manifest must not list. A folder of an install
//! (`installs.<game>/<path>`) that the game writes into is recorded the same way: `add --except Config --except
//! RunReports` names every file of it but those paths and what is under them. `add --warn` marks an
//! entry whose mismatch only warns (a tool installed with the system). `add --zip` on an oracle folder records its archive
//! `<rel>.zip` instead (`kind = "zip"`: the archive's size and its members' sums, which are the folder's); once shipped,
//! queries for the key read the archive. `verify` reads every file, whatever `[run] verify`
//! says. Archive keys (`keys.*`) have no entry.
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use primase_testdata::digest::{self, HASH_LIMIT};
use primase_testdata::manifest::{self, Verify};
use primase_testdata::measure;
use primase_testdata::sums::{self, Entry, Severity};

fn usage() -> ExitCode {
    eprintln!(
        "usage: manifest list <key> [--listing] | add <key> [FILE...] [--except PATH...] [--warn] [--zip] [--dir manifests] | verify [<key>...]"
    );
    ExitCode::from(2)
}

fn threads() -> usize {
    primase_testdata::threads().get()
}

/// The entry for the copy of `key` this machine holds; with `zip`, its archive `<rel>.zip` (an oracle folder stored as
/// one archive).
fn measure_key(key: &str, files: &[String], severity: Severity, zip: bool) -> Result<Entry, String> {
    let path = primase_testdata::locate(key)
        .ok_or_else(|| format!("{key} is not configured in the GAME_DATA file"))?;
    let path = if zip && path.extension().is_none_or(|e| e != "zip") {
        primase_testdata::zip_beside(path)
    } else {
        path
    };
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
    let entry = measure_key(key, &[], Severity::Fail, false)?;
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

fn add(
    key: &str,
    files: &[String],
    except: &[String],
    severity: Severity,
    zip: bool,
    dir: &Path,
) -> Result<(), String> {
    let name = manifest::file_of(key).ok_or_else(|| format!("{key}: no manifest holds this key"))?;
    let entry = if except.is_empty() {
        measure_key(key, files, severity, zip)?
    } else {
        if !(key.starts_with("installs.") && key.contains('/')) || !files.is_empty() {
            return Err(format!(
                "{key}: --except is for a folder of an install (installs.<game>/<path>), without FILE arguments"
            ));
        }
        let path = primase_testdata::locate(key)
            .ok_or_else(|| format!("{key} is not configured in the GAME_DATA file"))?;
        let files = measure::files_except(&path, except)?;
        measure::entry_for(key, &path, &files, severity, threads())?
    };
    let target = dir.join(format!("{name}.toml"));
    // the notes a person wrote inside entries are kept: the entries are parsed and written again
    let (mut entries, notes) = match std::fs::read_to_string(&target) {
        Ok(text) => (sums::parse(&name, &text)?, sums::notes(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Vec::new(), Default::default()),
        Err(e) => return Err(format!("{}: {e}", target.display())),
    };
    entries.retain(|e| e.key != key);
    entries.push(entry);
    let header = header(&name);
    let lines: Vec<&str> = header.iter().map(String::as_str).collect();
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::write(&target, sums::render_noted(&lines, &entries, &notes))
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
        // absent is only what the system says is not found: an IO error (a share out of resources) is no absent copy
        match path.try_exists() {
            Ok(true) => {}
            Ok(false) => {
                println!("SKIP {} ({} does not exist)", entry.key, path.display());
                continue;
            }
            Err(e) => {
                all_good = false;
                println!(
                    "FAIL {}: cannot tell whether {} exists: {e}",
                    entry.key,
                    path.display()
                );
                continue;
            }
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
    except: Vec<String>,
    warn: bool,
    zip: bool,
    listing: bool,
    dir: PathBuf,
}

fn parse(args: impl Iterator<Item = String>) -> Option<Args> {
    let mut parsed = Args {
        words: Vec::new(),
        except: Vec::new(),
        warn: false,
        zip: false,
        listing: false,
        dir: PathBuf::from("manifests"),
    };
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--warn" => parsed.warn = true,
            "--zip" => parsed.zip = true,
            "--listing" => parsed.listing = true,
            "--dir" => parsed.dir = PathBuf::from(args.next()?),
            "--except" => parsed.except.push(args.next()?),
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
    if !args.except.is_empty() && args.words.first().map(String::as_str) != Some("add") {
        return usage();
    }
    let result = match args.words.split_first() {
        Some((command, rest)) if command == "list" && rest.len() == 1 => list(&rest[0], args.listing),
        Some((command, rest)) if command == "add" && !rest.is_empty() => {
            let severity = if args.warn { Severity::Warn } else { Severity::Fail };
            add(&rest[0], &rest[1..], &args.except, severity, args.zip, &args.dir)
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
