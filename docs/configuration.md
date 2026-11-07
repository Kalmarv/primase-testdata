# Configuration, guarantees and manifests

## The configuration file

`GAME_DATA` names a TOML file. One file serves every crate on the machine; [game-data.example.toml](../game-data.example.toml) is a sample.

```toml
[installs]
sekiro = 'D:/games/Sekiro'
[keys]
gta = 'D:/keys/gta'
[tools]
oodle6 = 'D:/games/Sekiro/oo2core_6_win64.dll'
[oracles]
root = 'D:/oracles'               # <root>/<set>/<game>/<file>
[scratch]
root = 'D:/scratch'
[run]
require = 'gate'                  # or 'all': an unset input fails instead of skipping
gate = 'smoke'                    # or 'full': how much the heavy tests cover
seed = '20384'                    # the smoke sample's seed (the day's by default)
threads = '1'                     # the threads a heavy test's work may use
verify = 'quick'                  # or 'content' or 'off': how much of a copy is compared with its manifest
```

Every section and key is optional, and every value is a string. The keys of `[installs]`, `[keys]` and `[tools]` are not checked, because one file serves crates that declare different inputs. `GAME_DATA_DEBUG='key=value;key=value'` sets the debug switches a crate declares.

Machines can share one oracle home: point each machine's `[oracles] root` at the same copy on a network share, and every machine checks the same bytes against the same manifests, while installs stay in each machine's own `[installs]`. A `[tools]` entry is checked as a file whatever runs it, so a Windows build (fxc, dxc) named on Linux, to run under Wine, passes its manifest check there too.

A test starts a tool with `program(Tool::X)`, a `Program` whose `command()` gives a `ToolCommand` for each run: the program itself, or, where `[tools] wine` names a launcher, a Windows program (`.exe`) as `<launcher> <program> args...`. Its path arguments go through `path_arg`, which under a launcher writes an absolute path as a Wine path (`/tmp/a.dds` as `Z:\tmp\a.dds`: a Windows program reads a leading `/` as an option); other arguments pass unchanged. So `[tools] texconv` names the real `texconv.exe` on Linux too, and its manifest check holds that file. The launcher takes the program and its arguments and runs it with this process's standard streams: on Linux, Wine without a pseudo-terminal (primase-oracles' `tools/wine/wine`; `wine = '/home/me/opt/wine/bin/wine'` in `[tools]`, a key every version of this crate accepts, so one file serves checkouts on older versions too). A Windows program asked for where the machine runs none itself and no launcher is set fails, naming the setting.

"Gate" has three meanings in this crate: an input declared `gate: true` must be present in a complete run; `[run] gate = 'smoke'|'full'` says how much a heavy test covers; and `gate::GateReport` is a comparison against reference output.

## What it guarantees

- A typo does not skip quietly: an unknown section, an unknown key of `[oracles]`, `[scratch]` or `[run]`, a value that is not a string, and a flat map with no sections are errors that name the file and the key.
- A configured path that does not exist fails the test. An oracle set whose folder the machine does not hold (`<oracles.root>/<set>`) counts as not configured instead.
- A query for something the crate has not declared fails, and so does a declaration that does not say what the input is and where it comes from (`Registry::problems`).
- Unset inputs skip with one stderr line, `primase-testdata: SKIP <test> (<key> not configured ...)`. With `[run] require = 'gate'`, inputs declared `gate: true` (and every `Blocked` file) fail instead of skipping; `'all'` fails on every unset input. An input declared `Unheld` skips whatever `require` says.
- A copy is checked before a test trusts it: every path a query finds is compared with the BLAKE3 sums in this crate's `manifests/`, and a copy that differs fails the test that asked, naming the input and what differs. `[run] verify = 'off'` runs on whatever is there.
- `check_declared` fails on a query in the crate's sources whose game, set, path or debug key is not declared or not a literal, on a source that reads `GAME_DATA` itself, on a README whose generated inputs table (`primase_testdata::doc::tables`, between its markers) differs from the registry, and on a live `gate: true` install, tool or file with no manifest entry.
- `gate::GateReport` counts items that match a reference exactly, within a tolerance, or not at all; a gate that checked nothing has not passed. `gate::compare_json` compares two JSON documents number by number.

## Manifests

`manifests/<game>.toml` (and `tools.toml`) hold, for each input a query can name, what a good copy of it is: a file by its size and BLAKE3 hash, the identifying files of an install by the same, a folder by the digests of its listing (a folder of an install that the game writes into, settings or run history, by its other files, named like an install's: `add <key> --except <path>`) (`src/sums.rs` shows the format, `src/digest.rs` how the sums are made). They are embedded in the library, so every consumer that pins this crate checks against the same sums. An oracle folder may be stored as one archive, `<rel>.zip` in place of the folder (never both), recorded as a `zip` entry: the archive's size and its members' count, bytes, shape and content digests, the same digests the folder's `tree` entry would have, so a record shows the archive holds the folder's files byte for byte. The archive is a plain zip (stored members, no compression, zip64 where it needs it; primase-oracles' `tools/oracle-home/pack_zip.py` packs one reproducibly); a compressed or encrypted member, a data descriptor and a name that leaves the folder are refused, naming the member. `quick` reads the archive's size and central directory (one file, against a folder's listing of every file); `content` also hashes every member and checks its CRC-32. `oracle_tree` reads members in place; `Tree::extract` writes them under the scratch root, at most a given number of bytes. Archive keys are never in a manifest.

`[run] verify` says how much a run reads. `quick` (the default) compares sizes everywhere, hashes every file up to 256 MiB that an entry lists, and compares a folder's file names and sizes; `content` also hashes every file up to 256 MiB in a folder; `off` compares nothing. A file larger than 256 MiB is identified by its size alone. Each key is checked once per process. An entry marked `check = "warn"` (a tool installed with the system) prints one warning line instead of failing.

`cargo run --example manifest -- add <key>` writes an entry (`--except <path>`, repeatable, records an install's folder by every file but those under the paths, `--warn` marks a mismatch as a warning, `--dir` writes to another folder), `list <key> [--listing]` prints what the configured copy holds, and `verify [<key>...]` compares the configured copies with the shipped entries, reading every file whatever `[run] verify` says. A new `gate: true` input needs an entry before its crate's `check_declared` passes; commit the changed `manifests/*.toml` here and move the consumers' pins.

## What it does not do

- It ships no game data, keys or tools, and does not download or extract any. Every input is the user's own.
- It does not parse game files; it finds paths, builds the command that starts a configured tool (`program`), and reports comparisons.
- The self-check reads source text, not syntax trees: it sees queries written in the forms it knows (called through `inputs::`, imported by name, renamed with `as`, imported by a glob, or through a renamed module) and says so for a call it cannot read. It cannot follow a query function re-exported from the inputs module, so a `pub use` of one is itself a problem it reports.
