# Contributing

## Build and test

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
```

The crate's own tests and doc test need no game files and no `GAME_DATA`. They use a fake environment and temporary folders. `cargo run --example declare_inputs` runs the example from the README.

`game-data.example.toml` must stay a file the reader accepts (a test checks it with `config::validate`), and the example in the README must stay the same code as `examples/declare_inputs.rs` (a test checks that too). A change to what `src/config.rs` accepts changes both files and the README's configuration section.

## Reporting a file or configuration it cannot read

Open an issue with the "unreadable file" template. It asks for:

- the version of this crate (or the commit) and your platform;
- your `GAME_DATA` file with the paths shortened to what matters (`installs.sekiro = '.../Sekiro'`), or the smallest part of it that still fails;
- the exact message: the test's panic or its `primase-testdata: SKIP ...` line;
- what the path is: a file, a folder, a symlink, a network drive.

Do not attach game files, archives, keys or anything extracted from a game: they are not ours to redistribute, and the report does not need them. Describe the file instead (its name, its size, what the reader that refused it said).

## Changing the code

Keep the public items documented (`missing_docs` is on), keep refusals naming the key, section or path involved, and add a test next to the behaviour you change.
