# Changelog

## 0.1.11 (2026-11-06)

- A folder of an install (`installs.<game>/<path>`) may be recorded as a `files` entry, like an install: each file with its size and hash, for a folder the game writes into, whose settings or run history would fail a `tree` entry after one play session. `cargo run --example manifest -- add <key> --except <path>` (repeatable) names every file of the folder but those under the paths; `measure::files_except` lists them, and `measure::entry_for` records an install's folder given files as `files`. Without files it is still a `tree`.
- Manifests: `installs.gungeon` (Enter the Gungeon), with `installs.gungeon/EtG_Data` (198 files, 310,466,547 bytes) and its `StreamingAssets/Assets` (30 bundles, 69,839,404 bytes); `installs.schedule1` (Schedule I), with `installs.schedule1/Schedule I_Data` (104 files, 6,664,718,013 bytes); `installs.ror2` (Risk of Rain 2), with `installs.ror2/Risk of Rain 2_Data` as `files` but for `Config` and `RunReports`, and its `StreamingAssets/aa` (1,725 files, 2,887,437,250 bytes). unity-formats' player tests read them.
- Manifests: UnityPy 1.25.4's readings (unity-formats 3bd855fe42a8 `tests/oracles`): `oracles.unitypy.gungeon/{census.json, bundles.jsonl, player-settings.json, clips.json}`, `oracles.unitypy.ror2/{textures.json, clips.json}` and `oracles.unitypy.schedule1/{textures.json, clips.json}`.

## 0.1.10 (2026-11-06)

- Manifests: `tools.warp` is the WARP oracle build of primase-oracles fa44129 (448,000 bytes), which adds two lines to
  its `d3d12draw` case form: `topology` (point and line lists, line and triangle strips; a triangle list when absent)
  and `adapter` (the first hardware adapter instead of WARP). It answers the 2,545 cases of the earlier forms (514 case
  files) byte for byte as the build the entry named before (446,464 bytes, blake3 `31c9f67f...`).

## 0.1.9 (2026-11-06)

- `[tools] wine`: a launcher that runs Windows programs, for a machine that runs none itself (Linux: primase-oracles' `tools/wine/wine`). A `[tools]` key, which every version of this crate accepts, so a configuration that sets it still loads in checkouts on older versions. `program(Tool::X)` (and `primase_testdata::program`) gives a configured tool's `Program`, whose `command()` is a `ToolCommand` for one run: the program itself, or a Windows program (`.exe`) through the launcher, with path arguments (`path_arg`) as Wine paths (`Z:\...`), since a Windows program reads a leading `/` as an option. `[tools]` then names the real `.exe` on Linux, and its manifest check holds that file, where a hand-written wrapper script failed it (and needed `[run] verify = 'off'`). A Windows program asked for on a machine with neither fails, naming the setting. `wine_path` is public.
- `cargo run --example manifest -- add` keeps the comment lines written inside a manifest's entries (it rewrites the whole file, and dropped them before: the note on `oracles.capture.elden/havok-cloth` was lost that way and is restored). `sums::notes` reads them, `sums::render_noted` writes them back.
- Manifests: shader corpora that were held on one machine only, now in the shared oracle home. `oracles.corpus.<game>/d3d11-shaders`, locum's D3D11 shader dumps (`<hash>.<stage>.dxbc`), for celeste (11 files), hollowknight (34), kovaaks (100), lethalcompany (97), motortown (418), ror2 (325), sekiro (9,680, 209,977,516 bytes) and ultrakill (32), and `oracles.corpus.general/d3d11-shaders-unnamed/{map,menu,title}` (61, 341, 156) for three dumps whose game is not recorded; `oracles.corpus.motortown/d3d12-shaders-gfxr` (3,013 files, 41,531,536 bytes), the programs of two locum runs of Motor Town under D3D12 (3,010 DXBC, 3 DXIL); `oracles.corpus.elden/locum-extra` (30), the programs of locum's Elden Ring runs that `corpus.elden/shaders` lacks; `oracles.corpus.sekiro/shader-binders` (22,478 files, 415,588,500 bytes), Sekiro's shader binders and the 248 bundles of `shaderbdle.shaderbdlebnd.dcx` extracted as stored, with their `.metaparam`; `oracles.corpus.riders/anvil-programs` (13,790 files, 91,671,512 bytes), Riders Republic's SM5.0 programs named by their sha256; and `oracles.corpus.gta/shader-database-extracted` (346 files, 6,219,739 bytes), programs and listings taken out of `corpus.gta/shader-database.awc`. New manifest files for the games new here.
- Manifests: `oracles.corpus.general/unitypy-samples` (4 files, 1,450,688 bytes), UnityPy's test bundles (UnityPy 417d59522a99 `tests/samples`); `oracles.unitypy.general/samples.json` (5,349 bytes), UnityPy 1.25.4's reading of them; and `oracles.corpus.general/assetripper-tpk` (4 files, 2,299,958 bytes), AssetRipper/Tpk's type-tree packages uncompressed and in LZ4, LZMA and Brotli. unity-formats' public-sample tests read them.

## 0.1.8 (2026-11-06)

- A configured path whose existence cannot be told (an IO error: a network share out of resources, a permission refused) fails naming the error instead of reading as absent; absent is only what the system says is not found. A test whose input sits on an unreadable path now fails where it used to skip. `cargo run --example manifest -- verify` reports such a path as FAIL.
- Manifests: `oracles.dxc.elden/shader-listings` (50,283 files, 3,996,701,341 bytes), dxc 1.9.2609's `-dumpbin` listing of each of Elden Ring's 50,283 DXIL programs (`<path>.txt`, mirroring `oracles.corpus.elden/shaders`); dx's DXIL listing tests compare with them.
- Manifests: `oracles.d3d-disassembler.sekiro/programs` (10,368 files, 153,015,626 bytes), Sekiro's 432 shader programs (390 `.ppo`, 42 `.vpo`) with `D3DDisassemble`'s listing of each under 23 flag words (`NAME.ppo.asm`, `NAME.ppo.flagNN.asm`), and `oracles.d3d-disassembler.sledding/corpus-unity` (9,708 files, 210,878,281 bytes), the Sledding Game Unity player's 4,854 programs with their listings (`NNNNN.dxbc.asm`), both made by d3dcompiler_47 10.0.26100.4654; dx's listing tests compare with them.
- Manifests: `oracles.smithbox.elden/param-meta`, Smithbox fdde3225's Elden Ring Param Meta (179 files, 615,530 bytes), the per-param XML whose field attributes name the fields that refer to effect files; fromsoft-formats' effect census is cross-checked against it.
- Manifests: re-recorded `oracles.capture.elden/havok-cloth` (6 files, 19,537,771 bytes), with 1,499 wind/simulation steps and 1,500 paired skin calls. The saved chest item 871100 names protector model 1361, whose installed definition matches the recording bit for bit; the former 1360 label named a different asset. The original `rearrope-cap1.clf5` is unchanged, with its legacy 1360 source identity explicitly unverified pending its own save-to-protector-to-model check.
- Manifests: `oracles.gbxnet.tmnf/tmx-2026-top` re-recorded (101 files, 99,136,431 bytes): the old record also held `maps.jsonl.bak-before-tmunlimiter`, an earlier harness version's `maps.jsonl` kept aside beside the result; that file is not part of the oracle (no test reads it), and the GBX.NET dump makes the other 101 files byte for byte.
- `manifest::file_of` (and so `cargo run --example manifest -- add`) names a game's manifest file by its key without hyphens: `oracles.<set>.shaun-white/…` belongs in `shaunwhite.toml` with `installs.shaunwhite`, the file the crate embeds, not in a `shaun-white.toml` it would never load.
- Manifests: records for 16 oracles the tests declared without one: Elden Ring's UV-program listing (dxc), Soulstruct's TAE event classes, the walk save ER0000.sl2 and its walk, both games' sampler pairs, Shaun White's sdb2 disassembly, and TrackMania's stock, TMX and GBX.NET corpora (corpus.tmnf, gbxnet.tmnf). The frame recordings (scripts.sekiro/frames, scripts.elden/frames) and corpus.gta/legacy-effects still have none.
- Manifests: `oracles.corpus.elden/havok-cloth` re-recorded (815 files, 283,161,466 bytes): the cut that made the old record named a tagfile its binders list under several names by whichever thread finished first, so it could not be made again; the cut now takes each tagfile's last member in a fixed walk order (archive entries by name, members as their binders store them), and two runs give the same tree. The files' contents are as before; only those names differ.

## 0.1.7 (2026-10-31)

- Manifests: `installs.fh6`, Forza Horizon 6 (Steam build 25588841) identified by its executable, game database, `NerdData.json` and the Tokyo track's four GeoChunks with their chunk maps and contents lists; `installs.fh6/media/Tracks/Brio`, that track's folder (217 files); and `oracles.forzatools.fh6/minizip`, Nenkai ForzaTools' reading of its GeoChunks (10 files). forzatech-formats' minizip tests read them.
- Manifests: `installs.tmnf`, TrackMania Nations Forever (Steam) identified by TmForever.exe and its three packs and pack list; primase-trackmania's tests read its campaign maps and block models.

## 0.1.6 (2026-10-30)

- Manifests: `oracles.unitypy.<game>/urp.json` for PEAK, Sledding Game and Big Walk, what UnityPy reads of each player's Universal Render Pipeline objects (pipeline assets, renderers and their features, every camera's URP data); unity-formats' URP test compares its own readers with it.

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
