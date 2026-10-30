# Changelog

## 0.1.5 (2026-10-30)

- Manifests: `oracles.scripts.sekiro/dds` is the DDS worker that writes each decoded level to stdout in 64 KiB pieces (one write of a whole large level can fail with ERROR_NO_SYSTEM_RESOURCES when the machine is short of memory). Its output is unchanged for all 9326 Sekiro area textures; the folder's bytes, shape and content sums change because DdsWorker.dll and its .pdb do.

## 0.1.4 (2026-10-29)

- Manifests: `oracles.unity-editor.peak/light-volume-level4`, the Unity editor's read of PEAK level 4's light volume (4 files), which primase's PEAK light volume test compares its RGBAHalf slices with.

## 0.1.3 (2026-10-29)

- Manifests: `oracles.unitypy.<game>/player-settings.json` for SkateBIRD, Sledding Game, PEAK and Big Walk, the colour space (`m_ActiveColorSpace`) each player's PlayerSettings states, read by UnityPy; unity-formats' player settings test compares its own read with it.

## 0.1.2 (2026-10-29)

- Manifests: the havok-cloth captures (`oracles.capture.sekiro/havok-cloth`, `oracles.capture.elden/havok-cloth`) are CLF5 frames files, which also hold the cloth instances' volume and transition states; the Sekiro set's sleeve capture is extracted again with its action calls. Only these sums change; the file counts (7 and 2) are the same.

## 0.1.1 (2026-10-29)

- Manifests: `manifests/*.toml` (embedded) hold the BLAKE3 sums of the real-data inputs a good copy holds; every query that finds a path (`resolve`, and so `install`, `install_file`, `tool`, `oodle`, `oracle`) compares it with the entry of the same key, once per process, and fails the test that asked when the copy differs. New `[run] verify = 'off' | 'quick' | 'content'`, `verify_mode`, `locate`, the modules `manifest`, `digest`, `sums` and `measure`, the `manifest` example (`add`, `list`, `verify`), and a `check_declared` that also requires an entry for each live `gate: true` install, tool and declared file.

## 0.1.0 (2026-10-27)

- First release: declare the game installs, tools and reference outputs tests read; one TOML file per machine (`GAME_DATA`) says where they are, and an input it does not configure skips its test.
