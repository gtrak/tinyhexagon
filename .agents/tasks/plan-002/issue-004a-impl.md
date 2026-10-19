# Issue 04a — engine kid mechanics (soft collision, difficulty, bump, gaps)

**Umbrella:** `.agents/plans/002-tinyhexagon/04-engine-kid.md` (04a is the
engine half; 04b is the beat/text half). **Base:** post-rename `b751d36a`.
No behaviour change on the non-kid path.

## Goal

Add the engine mechanics the tiny runtime needs:

- no-death soft collisions (kill the wall, flash, bump, no player kill);
- a live difficulty setter;
- a bump serial for the app's slow-motion/SFX/duck;
- wall-gap access / nearest-gap angle for the helping hand.

## API decisions

```rust
// th-game::hexagon_game
pub fn set_soft_collisions(&self, enabled: bool);
pub fn soft_collisions(&self) -> bool;
pub fn set_difficulty_mult(&self, mult: f32); // clamp 0.1..=4.0
pub fn bump_serial(&self) -> u64;
pub fn wall_gaps(&self) -> Vec<WallGap>;
pub fn nearest_gap_angle(&self, player_angle: f32) -> Option<f32>;

// th-game (re-exported)
pub struct WallGap {
    pub center_angle: f32,
    pub half_width_angle: f32,
    pub radius: f32,
}
pub fn compute_wall_gaps(walls: &[CWall]) -> Vec<WallGap>; // pure, testable
```

- Store `soft_collisions: bool` and `bump_serial: u64` in `EngineState`
  (init false/0); the composition calls `set_soft_collisions(true)` once.
- `CWall` gets `pub fn kill(&mut self)` (sets the existing `killed` flag);
  the existing `is_dead`/retain removes it after the update.
- The bump sets `status.flash_effect = f32::max(status.flash_effect, 120.0)`
  and increments `bump_serial` exactly once per killed wall.

## Steps

### 1. Soft collision

- `EngineState`: add `soft_collisions` + `bump_serial`; init in the
  constructor. `HexagonGame::set_soft_collisions`/`soft_collisions` read/write.
- `update_walls`:
  - read `soft` once at the top;
  - first round: on overlap, if soft → `g.walls[i].kill()`, set the flash,
    increment the bump, `collided = true`, and `continue` (no `push_cwall`,
    no `perform_player_kill`);
  - after the first round, if soft, return (the second "always deadly" round
    must not run; the killed walls are removed by the existing retain);
  - non-soft path byte-identical.
- `update_custom_walls`: if `hit` and soft, set flash/bump and do not call
  `perform_player_kill`; kill the custom wall if the manager exposes it
  cleanly (otherwise record the custom-wall limitation). The tiny pack should
  not rely on custom walls.
- Do not reorder the update; only the collision branch changes.
- Tests:
  - a run with `soft = true` driven into walls reaches a bump and never sets
    `has_died`;
  - a run with `soft = false` still dies on the same kind of collision (the
    old path);
  - the bump serial advances once per collision (assert it increases, and
    that a repeated update on the same killed wall does not double-count).

### 2. Difficulty setter

- `pub fn set_difficulty_mult(&self, mult: f32)` writes
  `self.est().difficulty_mult = mult.clamp(0.1, 4.0)`.
- The existing `difficulty_mult()` getter and speed/delay paths use it.
- Test: setting 3.0 changes the speed multiplier (`difficulty_mult().powf(
  0.12)` or the public getter) and clamps out-of-range values.

### 3. Wall gaps

- `CWall`: add `angle_span(&self) -> Option<(f32, f32, f32)>` computed from
  the two distance vertices (`vertex_positions[0]`/`[1]`): normalized angles
  in `[0, TAU)`, positive span, radius. Do not add new fields if the vertices
  suffice.
- `compute_wall_gaps(walls: &[CWall]) -> Vec<WallGap>`: collect the spans,
  sort/merge, and return the complement arcs with center, half-width and the
  adjacent radius. Empty walls → empty vec.
- `HexagonGame::wall_gaps()` delegates over `st().walls`.
- `nearest_gap_angle(player_angle)`: the gap center with the smallest angular
  distance to the player (normalize the difference to `[-PI, PI]`); `None`
  when empty. The app can also use `wall_gaps()` to pick the widest/nearest.
- Test: synthetic `CWall`s at known sides produce the expected gap centers /
  widths; `nearest_gap_angle` picks the expected side.

### 4. Gate

```sh
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tools/gate.sh
```

## Verification

```sh
rg -n 'soft_collisions|bump_serial|set_difficulty_mult|wall_gaps|nearest_gap_angle' crates/th-game/src
bash tools/gate.sh; echo "gate_rc=$?"
```

New tests must discriminate (soft vs non-soft; clamp; known gap). Existing
engine/renderer tests stay green.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-004a-report.md` with:

- The final API signatures and `WallGap` shape.
- The soft-collision branch and the non-soft unchanged proof.
- The bump semantics, the difficulty clamp, the gap algorithm.
- Any custom-wall limitation / RNG-order note.
- Tests and gate result/tail; "No commit made."

Update `FEED-FORWARD.md` §04a: API signatures, bump semantics, gap helper,
and the 04b/07 handoff. Do not mark 04a LANDED in `PLAN.md`; the orchestrator
does that at commit.

## Risks

- The collision branch is delicate: keep the non-soft path byte-identical
  and do not reorder the update.
- Killing a wall mid-round must not double-bump or leave a live collider.
- The gap math must normalize wrap-around; test the 0/TAU seam.
- Custom walls are a separate collision path; if the manager cannot expose a
  kill cleanly, document the limitation instead of forcing a push.
