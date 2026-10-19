# Issue 06 — tiny content pack (report)

**Worktree:** `slot-2` (branch `slot-2`, base `9130c969`). **Committed by the orchestrator after the forked `VERDICT: PASS`.**
**Mode:** write-first — the pack was authored from the 06 contract before
any engine reading; only `th-app/tests/common` + `bridge.rs` (composition
shape), the `th-data`/`th-assets` public id/count APIs, and the shipped
pack JSONs were consulted.

## Pack files created (all under `_RELEASE/Packs/tiny/`)

| File | Notes |
|---|---|
| `pack.json` | disambiguator `tinyhexagon`, name `tiny`, author `tinyhexagon`, version 1, priority `0` (contract: any value loading after `base`; `base` is the min-priority pack), dependency `base` (`ohvrvanilla`, `vittorio romeo`, min_version 1) |
| `Levels/meadow.json` | `unscored: true`, `styleId: "meadow"`, `musicId: "nullMusicId"`, `luaFile: "Scripts/Levels/meadow.lua"` |
| `Levels/river.json` | same shape, `styleId: "river"` |
| `Levels/stars.json` | same shape, `styleId: "stars"` |
| `Scripts/Levels/meadow.lua` | contract skeleton; no tutorial text; `t_wait(180)` in `onLoad` |
| `Scripts/Levels/river.lua` | same, per-level values below |
| `Scripts/Levels/stars.lua` | same, per-level values below |
| `Styles/meadow.json` | contract shape; hue 100–160 (green); 3D off |
| `Styles/river.json` | hue 190–250 (blue); 3D off |
| `Styles/stars.json` | hue 270–330 (violet); 3D off |

No `difficultyMults` in the level JSONs (the loader appends `1.0`; the app
passes its own multiplier). No OGG music, no `Shaders/`, no
replay/score/Steam references.

## ids (as loaded by `Assets`)

- Pack id: `tinyhexagon_tinyhexagon_tiny_1`
  (`th_data::build_pack_id("tinyhexagon", "tinyhexagon", "tiny", 1)`).
- Level ids: `tinyhexagon_tinyhexagon_tiny_1_meadow`,
  `tinyhexagon_tinyhexagon_tiny_1_river`,
  `tinyhexagon_tinyhexagon_tiny_1_stars`.
- Style ids: `meadow`, `river`, `stars` (loaded via each level's `styleId`).

## Authored per-level values (land in `LevelStatus`)

| level | `l_setSpeedMult` | `l_setDelayMult` |
|---|---|---|
| meadow | 0.55 | 1.75 |
| river | 0.62 | 1.70 |
| stars | 0.70 | 1.65 |

Common to all: sides 6 (min/max 6), `*_inc` zero (speed/rotation/delay),
`l_setSpeedMax(1.0)`, `l_setRotationSpeed(0.04)` (max 0.25),
`l_setFastSpin(0.0)`, `l_setIncTime(999999)` (no in-level increment — the
app's stage arc owns escalation), `l_setTutorialMode(false)`, beat pulse
max 10 / delay max `60.0/70.0*60.0` / speed mult 0.4.

**First-pattern delay:** `t_wait(180)` as the first timeline entry in
`onLoad` (180 frames ≈ 3 s at 60 fps — the contract's value; the test's
update rate is the port's `TIME_STEP`).

## Hues

meadow 100–160 (green family), river 190–250 (blue family), stars 270–330
(violet family); `hue_increment 0.5`, `hue_ping_pong false`,
`pulse_min 0.0` / `pulse_max 1.2` / `pulse_increment 0.02`, `3D_depth/skew/
spacing` all zero, white `main`, `cap_color` legacy — no shader references.

## Test

`crates/th-app/tests/tiny_pack.rs::tiny_pack_loads_and_runs` (composition
mirrors `tests/bridge.rs` + `common`):

- asserts the pack id and all three level ids are valid in the loaded
  `Assets` (styles load with their levels);
- per level: headless compose, `set_soft_collisions(true)`,
  `new_game(PACK, level, true, 1.0)`, `set_must_start(true)`, up to
  `30 * 240` updates at `TIME_STEP` (30 s budget);
- asserts: no `has_died`, `get_sides() == 6`, and the authored
  `speed_mult` / `delay_mult` in `get_level_status()` (a Lua error in the
  script — missing `base` helper, bad pattern call — would leave the 1.0
  defaults and fail these).

**Result:** `test tiny_pack_loads_and_runs ... ok` (first run, no
iteration needed).

## No-oped Lua globals

**None.** Every helper used exists: `u_execDependencyScript`, `shuffle`,
`u_rndInt`, `t_wait`, `pBarrageSpiral`, `pInverseBarrage`, `pAltBarrage`,
`pSpiral` (base `utils`/`common`/`commonpatterns`), all `l_set*` hooks,
and the `onInit`/`onLoad`/`onStep`/`onIncrement`/`onUnload` entry points.
No replay/score/Steam globals were referenced or restored.

## Gate

First `bash tools/gate.sh` run failed only on the hardcoded `_RELEASE` tree
counts (expected — the new pack shifts them); updated:

- `crates/th-assets/tests/release_tree.rs`: pack count 8→9 (+ tiny id to
  the shipped-id list), headless assets 57→60 / full 61→64 (3 style JSONs),
  levels 37→40, selectable packs 7→8, discovery list gains `Packs/tiny/`.
- `crates/th-data/tests/load_release.rs`: levels 37→40, styles 33→36
  (music 24 unchanged).

Final gate: **fmt / build / clippy / `cargo test --workspace` all
`GATE_EXIT=0`** (one `cargo fmt` reflow of the new test's `load_release`).

## Deviations / open notes

- `pack.json` priority set to `0` (contract: exact value not important;
  keeps `tiny` in the same tier as `workshopexample`, after `base`).
- Pattern arguments are the contract's starting points
  (`pBarrageSpiral(u_rndInt(1,2),1,1)`, `pInverseBarrage(0)`,
  `pAltBarrage(u_rndInt(2,3),2)`, `pSpiral(10,0)`) — content-loading
  verified; feel-tuning is Issue 09's.
