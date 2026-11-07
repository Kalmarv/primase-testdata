# Changelog

## 0.2.5 (2026-11-07)

- Manifests: `oracles.dxc.elden/shader-listings` is recorded as one archive (`kind = "zip"`, `shader-listings.zip`, 4,008,774,993 bytes), with the same 50,283 files, 3,996,701,341 bytes and digests as the folder's record. Breaking for a reader that still calls `oracle` for it: read it through `oracle_tree` (dx does from 0.10). `oracles.corpus.elden/shaders` stays a folder.

## 0.2.4 (2026-11-07)

- Manifests: `oracles.d3d-disassembler.general/fxc-sweep-zi` and `fxc-sweep-zod` are recorded as one archive each (`kind = "zip"`; `fxc-sweep-zi.zip` 347,039,829 bytes, `fxc-sweep-zod.zip` 478,350,701 bytes), with the same counts, bytes and digests as their folders' records. Breaking for a reader that still calls `oracle` for them: read them through `oracle_tree` (dx does from 0.10).

## 0.2.3 (2026-11-07)

- Manifests: `oracles.corpus.riders/anvil-programs` is recorded as one archive (`kind = "zip"`, `anvil-programs.zip`, 94,705,334 bytes): the same 13,790 files of 91,671,512 bytes and the same digests as the folder's record. No crate reads the set; read it through `oracle_tree`.

## 0.2.2 (2026-11-07)

- Manifests: `oracles.corpus.general/slim` is re-recorded (16,566 files, 180,659,652 bytes; 8,287 programs). iridia's code after the 0.2.1 record reaches 12 llvm-cov regions and 3 branch sides (its constant-buffer layouts in `iridia-dxbc/src/reflection.rs`) that only the rest of the pool reached; the coverage fill adds the one program that reaches them, `vkd3d__cbuffer__2__ps_5_0` from the fxc sweep (`60e54bed….dxbc` with its listing), and its `slim.tsv` row. The 0.2.1 record of the set no longer matches the folder.

## 0.2.1 (2026-11-07)

- Manifests: `oracles.corpus.general/slim` is re-recorded (16,564 files, 180,644,672 bytes): three rows of its `slim.tsv` named a Direct3D 9 comment block's kind by its raw bytes (control characters and NULs inside a text file); they now read `comment:0x1c000000` and `comment:0x0f000002`. The programs and listings are unchanged; the 0.1.17 and 0.2.0 record of the set no longer matches the folder.

## 0.2.0 (2026-11-07)

- Oracle folders can be stored as one archive: an entry `kind = "zip"` records `<rel>.zip` by its size and its members' count, bytes, shape and content digests, the digests the folder would have. `oracle_tree(set, game, rel)` (in `inputs!`, beside `oracle`) gives a `Tree` that reads a folder and an archive alike: `names`, `size`, `read`, `len`, `as_folder` and `extract` (to the scratch root, with a byte limit). The archive reader (`zip`) is the crate's own, no new dependency: stored members only, zip64, every refusal naming the member. `quick` verification reads the central directory instead of listing the folder; `content` hashes every member and checks its CRC-32.
- Breaking: `oracle` fails for an entry recorded as an archive, naming `oracle_tree`, instead of returning a path that is no folder. Entries already recorded keep their form until their consumers read them through `oracle_tree`.
- `check_declared` reports a folder or entry of the crate it cannot read (an IO error) instead of passing over its sources.
- `cargo run --example manifest -- add <key> --zip` records a configured archive.

## 0.1.17 (2026-11-07)

- Manifests: `oracles.corpus.general/slim` (16,564 files, 180,644,654 bytes), the slim shader set: 8,286 DXBC, DXIL and Direct3D 9 programs chosen from 108,394 distinct ones (the oracle home's game corpora and disassembler sets, ten outside projects' test corpora, FidelityFX SDK v1.1.4) by a set cover of their disassembly features, plus the programs that reach the llvm-cov regions and branch sides the set alone did not reach in dx and iridia, so the set reaches all the full pool reaches in both. Each program `programs/<fam>/<sha256>.<fam>` beside Microsoft's listing (`.dxil.txt` from dxc 1.9.2609 -dumpbin, `.dxbc.asm` and `.d3d9.asm` from D3DDisassemble 10.0.26100.4654, flags 0, LF line ends, the closing NUL stripped), `slim.tsv` (why each program is there and where it lives for good) and `microsoft-refused.tsv` (the 10 programs Microsoft's tools refuse). It is the pre-push gate's shader set; primase-oracles `producers/slim-set` makes it.
- Manifests: `oracles.d3d-disassembler.riders/shader-pack` is re-made by primase-oracles' committed `tools/wine/d3d_disassemble.py` (e16959b, d3dcompiler_47 10.0.26100.4654, flags 0): 4,886 files, 43,378,516 bytes (was 4,888, 44,321,119). The 2,443 listings are the same text in the stored form of the other disassembler sets, LF with the closing NUL stripped (they had CRLF and the NUL); dx's comparison reads either. Two files no code makes or reads (`feat.pkl`, `summary.tsv`) left the entry.

## 0.1.16 (2026-11-07)

- Manifests: `oracles.d3d-disassembler.general/fxc-sweep-zi` (110,713 files, 320,536,225 bytes) and `oracles.d3d-disassembler.general/fxc-sweep-zod` (110,737 files, 451,833,501 bytes), Microsoft's D3DDisassemble listings of the programs fxc 10.0.26100.4654 builds from four public shader corpora (DXC 2630a187, vkd3d 1e329f31, vkd3d-proton db5628e2, DirectX-Graphics-Samples 68a2b1e9) with `/Zi` (4,613 programs) and with `/Zi /Od` (4,614): each program `NAME.dxbc` beside its listing with no flags (`NAME.dxbc.asm`) and under each of 22 flag words (`NAME.dxbc.flagNN.asm`), and a `MANIFEST.toml` naming the build, the tool and probe hashes and the corpora's commits. They are the only programs with SPDB debug info among the disassembler sets.

## 0.1.15 (2026-11-06)

- Manifests: `installs.stuntboost` and `installs.stuntboost/Content` are STUNTBOOST's Steam build 26429886 (2026-11-04; the entries described build 26237614): STUNTBOOST.dll and STUNTBOOST.exe changed, and `Content` holds 1,898 files, 5,008,787,935 bytes (was 1,892, 5,005,234,552). Steam installed two builds that day; together they add six files (with the first, the board `Models/Resources/Board/102.xnb` and its thumbnail; with the second, `Textures/UI/Crown.xnb`, `Duck.xnb`, `DuckMissing.xnb` and `DuckScreenshots/24.xnb`) and changes two levels: `Level_StairsSlide_1` (its geometry, collision and bakes) and `Level_BoosterLaunching_0`, besides fonts, localisation and audio banks.
- Manifests: `oracles.game-code.stuntboost/levels-build26429886` (318 files, 365,300,235 bytes), the game's own name parser and collision and ledge loader (primase-oracles `stuntboost`, 2753710) over build 26429886's levels. It differs from `levels` (build 26237614, kept as recorded) in two parts: StairsSlide_1, whose invisible `RoomBounds` collision mesh has 80 triangles (was 68) and so one ledge (4,460) and two ledge corners (5,233) more, and BoosterLaunching_0, whose two mandatory checkpoints are named `.Custom(1)` and `.Custom(2)` (were `.Order(1)` and `.Order(2)`).

## 0.1.14 (2026-11-06)

- `ToolCommand::output_within(limit)` runs a configured tool with a time limit: past it the tool is killed and the error (`TimedOut`) names its key and program, so a tool that hangs (a wrapper whose runtime is broken, a program waiting for input) fails the test instead of hanging it. `ToolCommand::stdin_bytes` gives it its standard input; a tool that exits before reading it all is no error.

## 0.1.13 (2026-11-06)

- Manifests: `installs.terraria` (Terraria, Steam build 24893155), identified by Terraria.exe, its content pipeline and ReLogic.Native.dll, with `installs.terraria/Content` (15,997 files, 739,703,531 bytes: 15,994 `.xnb`, 15,108 LZX-compressed); and `oracles.xna.terraria/content.jsonl` (3,128,257 bytes), XNA 4.0's own decompressor (`DecompressStream`) over that folder: each `.xnb`'s header and the SHA-256 of its content (primase-oracles `xna/XnaContent.cs`). monogame-formats' LZX test reads them.

## 0.1.12 (2026-11-06)

- Manifests: `oracles.soulsformats.elden/census-sote` (24 files, 686,382,446 bytes) and `oracles.soulsformats.elden/format-sote` (4 files, 355,204,558 bytes: msb, btl, matbin and metaparam), primase-oracles' Elden Ring harness (6b07350; SoulsFormatsNEXT ee1dd61, UXM 9501be8) on the install with Shadow of the Erdtree (Steam build 25827938). `census` and `format` were made before the DLC was installed: their FLVER, BTL and MSB records lack the DLC's, so tests that walk them leave its files untested. They stay as recorded, and `format` alone holds `havok.jsonl`.

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
