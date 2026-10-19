# Issue 04a report — engine kid mechanics

Soft collisions (no death), live difficulty setter, bump serial, wall-gap
access + nearest-gap angle. Base: post-rename `b751d36a` (+ stub-bin
`5740e9bc`). **No behaviour change on the non-kid path. Committed by the orchestrator after the forked `VERDICT: PASS`.**

## Final API

```rust
// th-game::hexagon_game (impl HexagonGame<S>)
pub fn set_soft_collisions(&self, enabled: bool);
pub fn soft_collisions(&self) -> bool;
pub fn set_difficulty_mult(&self, mult: f32);   // writes shared difficultyMult, clamp 0.1..=4.0
pub fn bump_serial(&self) -> u64;
pub fn wall_gaps(&self) -> Vec<WallGap>;
pub fn nearest_gap_angle(&self, player_angle: f32) -> Option<f32>;

// th-game (re-exported; defined in th-game::wall)
pub struct WallGap {
    pub center_angle: f32,      // in [0, TAU)
    pub half_width_angle: f32,
    pub radius: f32,            // min of the two bounding walls' radii
}
pub fn compute_wall_gaps(walls: &[CWall]) -> Vec<WallGap>;  // pure

// th-game::wall (impl CWall)
pub fn kill(&mut self);                          // sets the existing `killed` flag
pub fn angle_span(&self) -> Option<(f32, f32, f32)>;  // (start, end, radius), angles in [0, TAU)

// th-game::custom_wall_manager (impl CCustomWallManager)
pub fn soft_kill_overlapping(&mut self, p_pos: &Vec2) -> usize;  // kills alive+canCollide overlaps
```

Storage: `EngineState.soft_collisions: bool` (init `false`) and
`EngineState.bump_serial: u64` (init `0`), in the `hexagon_game::new`
constructor (and the `lua_console.rs` test stand-in). The kid composition
(07) calls `set_soft_collisions(true)` once; default `false` keeps the
classic lethal path.

## Soft-collision branch + non-soft unchanged proof

`update_walls` (`update.rs`):

- `let soft = self.est().soft_collisions;` read once at the top.
- First round, on overlap: if `soft` → `g.walls[i].kill()`,
  `status.flash_effect = f32::max(status.flash_effect, 120.0)`,
  `bump_serial += 1`, `collided = true`, `continue` — **no `push_cwall`,
  no `perform_player_kill`**.
- After the first round: `if soft { return; }` — the second "always
  deadly" round never runs in soft mode.
- Killed walls are removed by the existing `g.walls.retain(|w|
  !w.is_dead())` that already follows `update_walls` in `update()`
  (unchanged position).

`update_custom_walls` (`update.rs`): `handle_collision` unchanged. On
`hit`: soft → `kill_soft_custom_walls()` (kills the live collidable
custom walls overlapping the post-push player position; flash + one bump
per killed wall), non-soft → `perform_player_kill()` (unchanged call).

**Proof the non-soft path is byte-identical**: the `update.rs` diff is
purely additive — 3 hunks in `update_walls` (the `soft` read; the
`if soft { … }` branch; the `if soft { return; }` early return) plus the
`hit`-branch split in `update_custom_walls` (`if soft { … } else
{ self.perform_player_kill(); }` — the `else` arm is the original
statement verbatim). No existing line was modified or deleted, and every
added line is gated on `soft == true`. The update order (update pass →
first round → second round → retain → custom walls) is untouched.

## Bump semantics

- `bump_serial` is incremented **exactly once per killed wall**: each
  overlapping CWall kills itself once in the first round (one `+= 1`),
  and the custom-wall path adds `killed` (the manager's kill count) in a
  single statement.
- A killed wall is retained out after the update, so repeated updates
  on the same (now-removed) wall do not double-bump (pinned by test).
- Flash: `status.flash_effect = max(status.flash_effect, 120.0)` per
  bump event (distinct from the death flash 255).
- Consumers (05 synth duck / 07 slow-motion + SFX) compare the serial
  against their last-seen value.

## Difficulty clamp

`set_difficulty_mult(mult)` writes the shared `difficulty_mult` (the
Lua-visible `difficultyMult`) with `mult.clamp(0.1, 4.0)`. No new
consumer: the existing `difficulty_mult()` getter, `getSpeedMultDM` /
`getDelayMultDM` (SharedState), `getMusicDMSyncFactor` (`dm^0.12`), and
the style-update `dm^0.8` path all read it. The app (07) owns the
0.55..1.15 arc; the engine only clamps the wide kid range.

## Gap algorithm

- `CWall::angle_span`: from the two distance vertices
  (`vertex_positions[0]`/`[1]` — the C++ ctor's `angleN`/`angleP` orbit
  points, both at the wall's current radius): `(start, end, radius)`
  with `atan2` angles normalized to `[0, TAU)` (`rem_euclid`); the span
  runs `start → end` in the increasing-angle direction and may wrap the
  `0`/`TAU` seam (width `(end-start) mod TAU` is positive). `None` for
  degenerate vertices (coincident or co-angular — zero-width/ambiguous).
  `radius = |v0|` (world origin — the engine's `CENTER_POS` is always
  `(0,0)`). No new CWall fields.
- `compute_wall_gaps` (pure): collect spans → split wrapping spans into
  `[start, TAU)` + `[0, …)` → sort (stable; start, then end) → merge
  overlapping/touching intervals (EPS = 1e-5 rad), tracking the radius
  of the span owning the merged interval's min-start edge and of the
  span owning its max-end edge → complement: every interior gap plus the
  seam gap (last end → first start + TAU), dropped when ≤ EPS (bounds
  touch). Gap `radius` = min of the two bounding walls' radii (the gap
  closes once the nearer bound reaches the player). `center_angle =
  (g0 + width/2) mod TAU`, `half_width_angle = width/2`; gaps ordered by
  start angle. Empty input or a fully covered ring → empty vec.
- `wall_gaps()` delegates to `compute_wall_gaps(&st().walls)`.
- `nearest_gap_angle(player_angle)`: signed difference normalized to
  `[-PI, PI]` (`rem_euclid(TAU)` then `− TAU` when > `PI`); the gap with
  the smallest |distance|; exact ties keep the first gap in the returned
  (start-angle-ordered) list; `None` when there is no gap. The app may
  instead read `wall_gaps()` and pick the widest/nearest itself.

## Custom-wall limitation / RNG-order note

- **Custom walls**: `handle_collision` returns a single `bool` (no
  per-wall info), so the soft path re-derives the hit set as
  "alive + `canCollide` + overlapping the post-push player position"
  (`soft_kill_overlapping`). This set equals the walls that could have
  caused the `hit` (a hit means some such wall overlaps the final
  position — phase 3 of `handle_collision` checks exactly that). The
  pushes in `handle_collision` still run in soft mode (spec: only the
  post-hit outcome changes). A soft hit with a `deadly` custom wall
  still kills the wall (soft = no death, by design). The tiny pack
  (06) should not rely on custom walls.
- **RNG order**: unchanged. The soft branch draws nothing from the game
  RNG (wall kill/removal consumes no draws), and the non-soft path is
  byte-identical; input/update order untouched.

## Tests (18 new, all discriminating)

`crates/th-game/tests/soft_collisions.rs` (9):

- `soft_collision_kills_the_wall_not_the_player` — static overlap,
  soft: `!has_died`, `bump_serial == 1`, `flash_effect == 120`, wall
  retained out.
- `non_soft_collision_kills_the_player_not_the_wall` — same collision,
  soft off: `has_died`, `bump_serial == 0`, `flash_effect == 255` (death
  flash), wall survives alive.
- `soft_run_driven_into_a_wall_bumps_never_dies` — moving wall, soft:
  no pre-existing overlap at spawn, bump ≥ 1, never dies.
- `non_soft_run_driven_into_the_same_wall_dies_on_the_same_tick` — the
  classic death tick equals the soft bump tick for the same injected
  wall (same collision, same tick).
- `bump_serial_advances_once_per_killed_wall` — 1 → (repeated updates,
  no double-bump) 1 → 2 → two overlapping walls in one frame → 4;
  `!has_died` throughout.
- `soft_custom_wall_collision_kills_the_wall_not_the_player` /
  `non_soft_custom_wall_collision_kills_the_player` — custom-wall twin
  (killed handle returns to the free list; no bump on the death path).
- `set_difficulty_mult_writes_the_shared_mult` /
  `set_difficulty_mult_clamps_out_of_range` — 3.0 flows into
  `getMusicDMSyncFactor` (`dm^0.12`) and `getSpeedMultDM` (with the
  level's `speedMax` cap); 99/1e9 → 4.0, 0.001/0.0 → 0.1, in-range
  passes through.

`crates/th-game/tests/wall_gaps.rs` (9):

- `angle_span_of_a_plain_side` / `angle_span_wraps_the_seam` — known
  6-sided wall spans (30°..90°; 330°..30° across the seam) + radius.
- `five_walls_leave_one_gap_centered_on_the_missing_side` — center 180°,
  half-width 30°, radius 1600.
- `the_gap_across_the_seam_is_centered_on_zero` — 0/TAU-seam gap
  centred on 0 (mod TAU, seam-aware assertion).
- `two_gaps_are_reported_for_a_sparse_ring` — 120° + 300° centres,
  start-angle order.
- `a_full_ring_leaves_no_gaps` / `no_walls_no_gaps`.
- `engine_wall_gaps_and_nearest_gap_angle` — engine delegation over
  `st().walls` (== `compute_wall_gaps`) + nearest-gap picks (wrap and
  non-wrap sides).
- `nearest_gap_angle_is_none_without_gaps` — no walls and full ring.

## Gate

```
$ bash tools/gate.sh
== cargo fmt --all -- --check        GATE_EXIT=0
== cargo build --workspace          GATE_EXIT=0
== cargo clippy … -D warnings       GATE_EXIT=0
== cargo test --workspace           GATE_EXIT=0
gate_rc=0
```

`cargo test --workspace`: **62 suites, 527 passed / 0 failed / 1
ignored** (the pre-existing th-lua sandbox doc-test ignore). Baseline at
HEAD (`5740e9bc`, measured in a temp worktree): 60 suites, 509 passed /
0 failed / 1 ignored — the delta is exactly the 2 new test files
(18 tests); no pre-existing test regressed.

**Committed by the orchestrator after the forked `VERDICT: PASS`.**
