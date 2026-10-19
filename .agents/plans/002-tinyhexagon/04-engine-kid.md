# Issue 04 — Engine kid alterations

**Status:** PARTIAL — 04a LANDED; 04b pending
**Depends:** 03
**Unblocks:** 05, 07

**Execution split (2026-10-19):** **04a** = soft collision, difficulty
setter, bump accessor and wall gaps (engine mechanics); **04b** = beat
palette/pulse and `DrawOptions` text suppression (visual/render). The
contract below is the umbrella; the sub-issue task specs carry the detail.

## Objective

Alter the engine and renderer seams needed by the kid game: no-death soft
collisions, live difficulty control, a music-driven palette/pulse, text
suppression, and wall-gap access for the helping hand. Keep the non-kid paths
behaviourally unchanged; no reordering of the simulation.

## Key decisions

- **No death.** Soft collisions only.
- The stage timer, 20-stage arc, level selection and 10-minute plateau live in
  `th-app` (Issue 07); the engine provides the setters/events.
- The existing renderer keeps drawing the scene; the kid path suppresses text
  via `DrawOptions`.
- Any RNG-order change is avoided; if unavoidable, record it in
  `FEED-FORWARD.md`.

## Changes

### 4.1 Soft collisions

- Add `soft_collisions: bool` (or `kid_mode`) to `HexagonGame`, set by the
  composition; default false.
- In `update_walls` (the first/second collision rounds) and
  `update_custom_walls`, when soft and an overlap is found:
  - kill the wall (so it is retained out) and spawn the existing burst/flash
    visuals;
  - set `status.flash_effect` to a bump value;
  - record a `bump` counter/timestamp in `EngineState` (public getter), for
    the app's slow-motion and the synth duck;
  - **do not** call `perform_player_kill`/`death`.
- The non-soft path stays exactly as it is.
- The player is not pushed out by `push_cwall` in the soft branch; the wall
  dies, so there is no repeated collision.

### 4.2 Difficulty setter

- Add `HexagonGame::set_difficulty_mult(f32)` writing the shared
  `difficulty_mult` (the update already uses it for speed/delay/style).
- Clamp to a safe range (e.g. 0.1..=4.0); the app owns the 0.55..1.15 arc.
- A test asserts the engine's speed/delay change with the multiplier.

### 4.3 Beat-driven palette and pulse

- Add `HexagonGame::apply_music_beat(beat_phase, bar_phase, energy,
  base_hue)` (or an equivalent struct).
- Add `StyleData::apply_beat_hue(hue)` (or a `hue_override` used by `update`)
  that sets `current_hue`, clamps to the style range, and calls the existing
  `compute_colors()`. The derived main/player/text/wall/background colours
  must reflect the new hue in the same frame.
- In the engine's update, after the style's own `update`, apply the beat
  hue/pulse override when set. The existing style animation remains the
  fallback when no beat is supplied.
- Set `status.pulse`/`status.beat_pulse` from the music where the renderer
  uses them (the background pulse).
- The app computes `hue = style.hue_min + stage_offset + drift`, and passes
  the result; the engine does not know the stage plan.

### 4.4 Text suppression

- Add `DrawOptions { show_text, show_level_info, show_key_icons, show_fps }`
  to the `th-app` bridge and/or `th-render` `FrameInput`.
- When false, skip the text/FPS/level-info/key-icon passes, the `update_text`
  call, and the `drawText_*` layout; keep background, walls, player,
  particles and flash.
- The default is `true` for any legacy caller; the kid path sets all false.
- A recording `DrawTarget` test asserts zero text draw calls with the kid
  options.

### 4.5 Wall gaps

- Add `HexagonGame::wall_gaps() -> Vec<Gap>` (angle center, half-width,
  radius/scale) or expose the wall vertex positions. The helping hand chooses
  the widest gap nearest the player.
- If the data cannot be exposed cleanly, provide
  `HexagonGame::nearest_gap_angle(player_angle) -> Option<f32>`; the app uses
  it directly.
- Tests: a synthetic wall set returns the expected gap; the assist steers
  toward it.

### 4.6 Bump event

- Expose `HexagonGame::take_bump()` or `bump_serial()`; the app uses it once
  per collision to trigger `timescale`, SFX and the synth duck.
- Ensure one bump per physical collision (a wall killed once).

## Files

| File | Change |
|---|---|
| `th-game/src/update.rs` | Soft collision branch, bump, no-death path. |
| `th-game/src/hexagon_game.rs` | `soft_collisions`, `set_difficulty_mult`, `apply_music_beat`, `wall_gaps`, bump accessor. |
| `th-game/src/status.rs` | Bump/flash fields if needed. |
| `th-data/src/style_data.rs` | `apply_beat_hue` / colour refresh. |
| `th-app/src/bridge.rs` | `DrawOptions` plumbing. |
| `th-render/src/graphics.rs` | Skip text passes under the options. |
| tests | Soft collision, difficulty, hue/colour, text suppression, gaps. |

## Steps

1. Add the soft-collision flag and branch; test the non-kid path is unchanged.
2. Add the difficulty setter; test speed/delay.
3. Add the style hue override + `apply_music_beat`; test derived colours.
4. Add `DrawOptions` and skip the text passes; test with a recording target.
5. Add wall-gap/nearest-gap access; test.
6. Add the bump accessor; test one bump per collision.
7. Run the gate and the forked reviewer.

## Verification

- New tests pass and discriminate: disabling the soft flag kills (old path);
  the beat hue changes the derived colours; the text recording target sees no
  text calls; the gap test returns the expected angle.
- Existing engine/renderer tests pass (except tests explicitly retired with
  replay).
- A headless 10 s run with soft collisions never sets `has_died` and produces
  at least one bump when driven into a wall.
- `cargo build --workspace`, clippy, tests green on host.

## Risks

- The collision path is delicate; keep the non-soft branch byte-identical.
- `compute_colors()` must run after the hue change or the palette lags a
  frame.
- Text suppression must not remove the whole frame when `show_text=false`.

## Feed-forward

Record the final method names/signatures, the `DrawOptions` shape, whether the
gap getter or nearest-angle helper landed, and any RNG-order note. Issue 07
uses these APIs; Issue 05 consumes the bump signal.
