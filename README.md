# primase-testdata

[![crates.io](https://img.shields.io/crates/v/primase-testdata)](https://crates.io/crates/primase-testdata) [![docs.rs](https://img.shields.io/docsrs/primase-testdata)](https://docs.rs/primase-testdata) [![CI](https://img.shields.io/github/actions/workflow/status/Kalmarv/primase-testdata/ci.yml?branch=main&label=CI)](https://github.com/Kalmarv/primase-testdata/actions/workflows/ci.yml)

A Rust test-support crate that tells a crate's tests where to find files that cannot be shipped with it: game installs, archive keys, third-party tools and reference output made from them. Each machine lists those paths in one TOML file, and a test whose input is not listed skips.

## Features

- Declares a crate's real-data inputs once with the `inputs!` macro, which generates typed `Game`, `Tool` and `OracleSet` enums and query functions
- Resolves game installs, files inside them, archive key folders, tools and oracle files (output of an independent reader to compare against)
- Reads one configuration file, named by `GAME_DATA`, that serves every crate on the machine
- Skips a test whose input is not configured with one `primase-testdata: SKIP` line naming the missing key
- Turns those skips into failures for required inputs with `[run] require = 'gate'`, or for all inputs with `'all'`
- Checks every resolved file or folder against BLAKE3 manifests embedded in the crate, with `[run] verify = 'quick'`, `'content'` or `'off'`
- Starts a configured tool with `program`, running a Windows program through the `[tools] wine` launcher on Linux, with its path arguments as Wine paths
- Ships manifests for Sekiro, Elden Ring, GTA V, Forza Horizon 6, TrackMania Nations Forever, Riders Republic, Shaun White, SHRED, skate., SkateBIRD, PEAK, Sledding Game, Big Walk, STUNTBOOST, the shader programs of Celeste, Hollow Knight, KovaaK's, Lethal Company, Motor Town, Risk of Rain 2 and ULTRAKILL, and a set of tools
- Provides a `check_declared` self-check that finds undeclared or non-literal queries in a crate's sources and stale inputs tables in its README
- Generates the inputs table for a crate's README with `doc::tables`
- Counts comparisons against reference output with `gate::GateReport`, and compares JSON documents number by number with `gate::compare_json`
- Includes helpers for per-process scratch folders, a machine-wide lock on the WARP rasterizer, and smoke or full test depth with a daily seed
- Fails on unknown sections, unknown keys, configured paths that do not exist, and queries for inputs the crate did not declare
- Contains no game data, keys or tools, and does not download or extract any

## Getting Started

### Requirements

- Rust 1.99 or newer (edition 2024)
- For real-data runs, your own copies of the games, tools and oracle files a consuming crate declares

### Installation

Add it as a dev-dependency only:

```sh
cargo add --dev primase-testdata
```

### Usage

A crate declares its inputs in `tests/inputs/mod.rs`. This is [examples/declare_inputs.rs](examples/declare_inputs.rs); with `GAME_DATA` unset, `cargo run --example declare_inputs` prints one `SKIP` line and stops.

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

An oracle folder can be stored as one archive (`<rel>.zip`, its manifest entry `kind = "zip"`), which a network share
reads far faster than many small files. `inputs::oracle_tree(set, game, "rel")` reads either form: a `Tree` lists the
files (`names`), gives their sizes and reads one (`read`), and `extract` writes them out for a tool that needs real
paths. `inputs::oracle` fails, naming `oracle_tree`, for an entry stored as an archive.

Integration tests use the module with `mod inputs;`; unit tests add `#[cfg(test)] #[path = "../tests/inputs/mod.rs"] mod inputs;` to the crate root. Each crate also runs the self-check as one test:

```rust
mod inputs;

#[test]
fn real_data_inputs_are_declared() {
    primase_testdata::check_declared(&inputs::REGISTRY, env!("CARGO_MANIFEST_DIR"));
}
```

To run a consuming crate's real-data tests, copy [game-data.example.toml](game-data.example.toml) outside the repository, replace the placeholder paths, and set `GAME_DATA` to its absolute path:

```toml
[installs]
sekiro = 'D:/games/Sekiro'
[tools]
oodle6 = 'D:/games/Sekiro/oo2core_6_win64.dll'
[oracles]
root = 'D:/oracles'               # <root>/<set>/<game>/<file>
[run]
require = 'gate'                  # or 'all': an unset input fails instead of skipping
```

On a machine that holds the inputs, the `manifest` example writes and checks manifest entries:

```sh
cargo run --example manifest -- add oracles.corpus.sekiro/havok          # a file or a folder
cargo run --example manifest -- add installs.sekiro sekiro.exe Data1.bhd # the files that identify an install
cargo run --example manifest -- add 'installs.ror2/Risk of Rain 2_Data' --except Config # an install folder's files but what the game writes
cargo run --example manifest -- list tools.oodle6 --listing              # print an entry without writing it
cargo run --example manifest -- verify                                   # every shipped entry against this machine
```

The full configuration reference, the guarantees, and the manifest format are in [docs/configuration.md](docs/configuration.md).

`cargo test` runs the crate's own tests, which need no game files. See [CONTRIBUTING.md](CONTRIBUTING.md) for the other checks and for reporting a file the crate cannot read. Licensed under GPL-3.0-or-later.
