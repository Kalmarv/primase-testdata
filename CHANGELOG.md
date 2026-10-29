# Changelog

## 0.1.1 (2026-10-29)

- Manifests: `manifests/*.toml` (embedded) hold the BLAKE3 sums of the real-data inputs a good copy holds; every query that finds a path (`resolve`, and so `install`, `install_file`, `tool`, `oodle`, `oracle`) compares it with the entry of the same key, once per process, and fails the test that asked when the copy differs. New `[run] verify = 'off' | 'quick' | 'content'`, `verify_mode`, `locate`, the modules `manifest`, `digest`, `sums` and `measure`, the `manifest` example (`add`, `list`, `verify`), and a `check_declared` that also requires an entry for each live `gate: true` install, tool and declared file.

## 0.1.0 (2026-10-27)

- First release: declare the game installs, tools and reference outputs tests read; one TOML file per machine (`GAME_DATA`) says where they are, and an input it does not configure skips its test.
