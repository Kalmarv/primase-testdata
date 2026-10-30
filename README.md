# primase-testdata

Test support for crates whose tests read files that cannot be shipped (a game's own data, a third-party program, reference output made from them): the crate declares what its tests read, one TOML file per machine says where those files are, and a test whose input is not configured skips with one line naming the key.

It was written for the real-data tests of format readers for game files: it resolves installs of games such as Sekiro and Grand Theft Auto V, their archive-key folders, tools such as Oodle libraries or Microsoft's WARP rasterizer, and "oracle" sets (output of an independent reader to compare against). Nothing in it depends on a game, so it works for any suite that reads machine-local files.

Use it only as a dev-dependency. "Gate" in this crate's vocabulary has three senses: an input declared `gate: true` must be present in a complete run; `[run] gate = 'smoke'|'full'` says how much a heavy test covers; and `gate::GateReport` is a comparison against reference output.

## Example

A crate declares its inputs once, in `tests/inputs/mod.rs`. This is `examples/declare_inputs.rs`, which `cargo run --example declare_inputs` runs (with `GAME_DATA` unset it prints one `SKIP` line and stops) and which is also the crate's doc test:

```rust
mod inputs {
    primase_testdata::inputs! {
        games {
            Sekiro = "sekiro",
        }
        installs {
            Sekiro {
                about: "Sekiro's install folder",
                source: "the user's own install",
                gate: true,
            },
        }
        keys {}
        tools {
            Oodle6 = "oodle6" {
                about: "an Oodle version-6 DLL",
                source: "the game's own DLL from the user's install",
                gate: true,
            },
        }
        oracle_sets {
            Corpus = "corpus" {
                about: "the games' own files cut from their installs",
                source: "the user's own installs: files extracted as stored",
            },
        }
        declared {
            Query::Oracle(OracleSet::Corpus, Game::Sekiro, "dcx") => {
                status: Live,
                gate: false,
                about: "Sekiro's DCX files as stored",
                source: "the user's own install",
            },
        }
        debug {
            "oodle.dump" => "a folder under the scratch root the Oodle gate writes to",
        }
    }
}

use inputs::{Game, OracleSet};

fn main() {
    assert!(inputs::REGISTRY.problems().is_empty());
    let Some(install) = inputs::install(Game::Sekiro) else {
        return; // `installs.sekiro` is not configured here: the SKIP line names it
    };
    println!("Sekiro is installed at {}", install.display());
    if let Some(folder) = inputs::oracle(OracleSet::Corpus, Game::Sekiro, "dcx") {
        println!("DCX files are in {}", folder.display());
    }
}
```

The macro generates the enums `Game`, `Tool` and `OracleSet`, a typed `Query`, a `REGISTRY` of everything declared, and the queries `install`, `install_file`, `keys`, `tool`, `oodle`, `oracle`, `resolve`, `debug` and `expect_found`, beside `scratch`, `own_scratch`, `warp_exclusive`, `depth`, `seed`, `threads` and `read_input`. Integration tests use the module with `mod inputs;`; unit tests with `#[cfg(test)] #[path = "../tests/inputs/mod.rs"] mod inputs;` in the crate root.

Each crate also runs the self-check as one test:

```rust
mod inputs;

#[test]
fn real_data_inputs_are_declared() {
    primase_testdata::check_declared(&inputs::REGISTRY, env!("CARGO_MANIFEST_DIR"));
}
```

## The configuration file

`GAME_DATA` names a TOML file. One file serves every crate on the machine; [game-data.example.toml](game-data.example.toml) is a sample.

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

Every section and key is optional, and every value is a string. `GAME_DATA_DEBUG='key=value;key=value'` sets the debug switches a crate declares.

## What it guarantees

- A typo does not skip quietly: an unknown section, an unknown key of `[oracles]`, `[scratch]` or `[run]`, a value that is not a string, and a flat map with no sections are errors that name the file and the key.
- A configured path that does not exist fails the test. An oracle set whose folder the machine does not hold (`<oracles.root>/<set>`) counts as not configured instead.
- A query for something the crate has not declared fails, and so does a declaration that does not say what the input is and where it comes from (`Registry::problems`).
- Unset inputs skip with one stderr line, `primase-testdata: SKIP <test> (<key> not configured ...)`. With `[run] require = 'gate'`, inputs declared `gate: true` (and every `Blocked` file) fail instead of skipping; `'all'` fails on every unset input. An input declared `Unheld` skips whatever `require` says.
- A copy is compared with what a good one holds before a test trusts it: every path a query finds is checked against the BLAKE3 sums in this crate's `manifests/` (see below), and a copy that differs fails the test that asked, naming the input and what differs. `[run] verify = 'off'` runs on whatever is there.
- `check_declared` fails on a query in the crate's sources whose game, set, path or debug key is not declared or not a literal, on a source that reads `GAME_DATA` itself, on a README whose generated inputs table (`primase_testdata::doc::tables`, between its markers) differs from the registry, and on a live `gate: true` install, tool or file with no manifest entry.
- `gate::GateReport` counts items that match a reference exactly, within a tolerance, or not at all; a gate that checked nothing has not passed. `gate::compare_json` compares two JSON documents number by number.

## Manifests

`manifests/<game>.toml` (and `tools.toml`) hold, for each input a query can name, what a good copy of it is: a file by its size and BLAKE3 hash, the identifying files of an install by the same, a folder by the digests of its listing (`src/sums.rs` shows the format, `src/digest.rs` how the sums are made). They are embedded in the library, so every consumer that pins this crate checks against the same sums. Archive keys are never in a manifest.

`[run] verify` says how much a run reads. `quick` (the default) compares sizes everywhere, hashes every file up to 256 MiB that an entry lists, and compares a folder's file names and sizes; `content` also hashes every file up to 256 MiB in a folder; `off` compares nothing. A file larger than 256 MiB is identified by its size alone. Each key is checked once per process. An entry marked `check = "warn"` (a tool installed with the system) prints one warning line instead of failing.

On a machine that holds the inputs, `cargo run --example manifest` writes and checks entries:

```text
cargo run --example manifest -- add oracles.corpus.sekiro/havok          # a file or a folder
cargo run --example manifest -- add installs.sekiro sekiro.exe Data1.bhd # the files that identify an install
cargo run --example manifest -- add tools.dxc --warn                     # a mismatch only warns
cargo run --example manifest -- verify                                   # every shipped entry against this machine
```

A new input needs an entry before its crate's `check_declared` passes (if it is a `gate: true` input); commit the changed `manifests/*.toml` here and move the consumers' pins.

## What it does not do

- It ships no game data, keys or tools, and does not download or extract any. Every input is the user's own.
- It does not run tools or parse game files; it only finds paths and reports comparisons.
- The self-check reads source text, not syntax trees: it sees queries written in the forms it knows (called through `inputs::`, imported by name, renamed with `as`, imported by a glob, or through a renamed module) and says so for a call it cannot read. It cannot follow a query function re-exported from the inputs module, so a `pub use` of one is itself a problem it reports.
- It is for tests; do not use it as a normal dependency.

## Running the tests

`cargo test` runs this crate's own tests and doc test; they need no game files. To run a consuming crate's real-data tests against your own install, copy `game-data.example.toml` outside the repository, replace the placeholder paths with your own, set `GAME_DATA` to its absolute path and run that crate's `cargo test`. The inputs a crate reads, and what each is, are listed in its own `tests/inputs/README.md` (a table made from the registry by `primase_testdata::doc::tables` between two marker lines, so it cannot drift from what the tests ask for). Without `GAME_DATA` those tests skip.

The embedded manifests also cover UnityPy's player colour-space and URP object readings, the Unity editor's
PEAK light-volume output, and the CLF5 cloth captures used by the Havok tests. The tool manifests identify the REF host with its vertex mode and the DDS worker that writes decoded levels in
64 KiB pieces. Updating an oracle's copy requires
updating its entry here before consumers move their pin; the manifests ship identifying sums, never those assets.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md), which also says how to report a file a reader cannot read.

The asset-conversion library [primase](https://github.com/Kalmarv/primase) uses this crate for its tests.

## Check the examples

From the repository root, `cargo test -p primase-testdata --doc` checks the crate documentation, and
`cargo build -p primase-testdata --examples` builds its example programs. Examples that read paths need the caller's files; the repository ships no game assets.

## License

GPL-3.0-or-later, see [LICENSE](LICENSE).
