# Issue 06 — tiny content pack (write-first)

**Worktree:** `/home/gary/dev/tinyhexagon/.agents/worktrees/slot-2` (branch
`slot-2`, base `9130c969`). Main/slot-1 off-limits. **Do not commit.**

**Contract:** `.agents/plans/002-tinyhexagon/06-tiny-pack.md` (read it once).
This spec is deliberately prescriptive: the previous attempt spent the whole
time reading engine internals and wrote nothing. **Write the pack files
FIRST**; do not read `th-game` internals or re-derive FT/timeline math.

## 1. Create the pack (all files, immediately)

Create exactly the files in the contract under `_RELEASE/Packs/tiny/`:

- `pack.json` (disambiguator `tinyhexagon`, name `tiny`, dependency `base`
  `ohvrvanilla`/`vittorio romeo`).
- `Levels/{meadow,river,stars}.json` (the contract's shape; `unscored: true`;
  styleId matching; musicId `nullMusicId`; luaFile paths).
- `Scripts/Levels/{meadow,river,stars}.lua` (the contract's skeleton; no
  tutorial text; first pattern delayed with `t_wait(180)`; six sides;
  speed 0.55/0.62/0.70; delay 1.75/1.70/1.65; `*_inc` zero; rotation 0.04).
- `Styles/{meadow,river,stars}.json` (the contract's shape; hue families
  100–160, 190–250, 270–330; 3D off; no shaders).
- Do not add `difficultyMults`, OGG music, shaders, replay/score/Steam.

## 2. Test (simple; do not analyze the engine)

Add `th-app/tests/tiny_pack.rs` (or the existing pack-test home) using the
existing `th-app/tests/common` helpers. Read only `common/mod.rs` and the
existing `bridge.rs` test for the composition shape. Skeleton:

```rust
use th_config::TIME_STEP;

#[test]
fn tiny_pack_loads_and_runs() {
    let assets = /* common's load_release or the bridge helper */;
    let root = /* common's release_dir */;
    for (level, expected_speed) in [("meadow", 0.55_f32), ("river", 0.62), ("stars", 0.70)] {
        let c = /* common::compose(assets.clone(), root.clone(), false) */;
        c.game.set_soft_collisions(true);
        c.game.new_game(/* the tiny pack id, e.g. "tinyhexagon_tiny" */, level, true, 1.0);
        c.game.set_must_start(true);
        for _ in 0..(30 * 240) {
            if c.game.get_status().has_died { break; }
            c.game.update(TIME_STEP, 1.0);
            c.game.post_update();
        }
        assert!(!c.game.get_status().has_died, "{level} died");
        assert_eq!(c.game.get_sides(), 6, "{level} sides");
        // Use the actual level-status accessor; assert expected_speed.
    }
}
```

- Discover the exact pack/level ids from the `Assets` data or the pack's
  `pack.json` (`tinyhexagon_tiny`, levels `meadow`/`river`/`stars`) with one
  short check; do not read the engine.
- Run the test; if the level script fails, the authored speed/sides won't
  land and the test fails — that is the signal.
- If a `base` helper is missing, no-op it in the script and record it; do
  not restore replay/Steam/score.
- Run `bash tools/gate.sh` in the slot.

## 3. Report / feed-forward

Write `.agents/tasks/plan-002/issue-006-report.md` (files, ids, speed/delay,
first-pattern delay, hues, test result, no-oped globals, gate, no commit).
Update `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` **§06 only**.

## Risks

- Do not over-research the engine; write the content first.
- The test's 30 s is 30*240 updates at `TIME_STEP`; a hard loop cap is fine.
- The test may need the `th-app` common module's `load_release`; if absent,
  use the helper from `tests/bridge.rs`.
