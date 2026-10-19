# Issue 04b report — beat palette/pulse + `DrawOptions` text suppression

Beat-driven palette/pulse (`apply_music_beat`,
`StyleData::apply_beat_hue`/`apply_beat_pulse`, same-frame colours),
`DrawOptions` text/HUD suppression through `th-render`/`th-app`, and the
three 04a review P2s. Base: post-04a `8990a64e`. **No behaviour change for
the default (legacy) options; no 04a mechanics changes; forked review PASS; committed with issue 04b.**

## Beat API + update point

```rust
// th-data::style_data
pub fn apply_beat_hue(&mut self, hue: f32);     // clamp to [hue_min, hue_max], compute_colors
pub fn apply_beat_pulse(&mut self, energy: f32); // clamp to [pulse_min, pulse_max], compute_colors

// th-game::hexagon_game (impl HexagonGame<S>)
pub fn apply_music_beat(&self, beat_phase: f32, bar_phase: f32, energy: f32, base_hue: f32);
```

- `StyleData::apply_beat_hue`: `current_hue = hue` clamped to the style's
  hue range, then `compute_colors()` (same-frame derived colours).
  `apply_beat_pulse`: `pulse_factor = energy` clamped to
  `[pulse_min, pulse_max]`, then `compute_colors()`.
- **Clamp-order safety**: C++ styles may wrap the hue circle
  (`hue_min > hue_max`; the C++ `update` treats them as endpoints, not
  bounds), and `f32::clamp` panics on a reversed range — both applies use
  an unordered-bounds helper (`clamp_range`) instead of a raw clamp.
- `HexagonGame::apply_music_beat`:
  - `EngineState` gains `beat_hue: Option<f32>` (init `None`) +
    `beat_energy: f32` (init `0.0`); the call stores
    `beat_hue = Some(base_hue + shimmer)`, `beat_energy = energy`.
  - The shimmer is a tiny deterministic sine on the beat/bar cycles
    (`±1.5°` total — `BEAT_HUE_SHIMMER_DEG`; phases are cycles in
    `[0, 1)`; phase 0 on both ⇒ shimmer exactly 0). The app's
    `base_hue` stays the dominant term.
  - `status.beat_pulse = energy` immediately, and
    `status.beat_pulse_delay = max(·, levelStatus.beatPulseDelayMax)`
    (re-armed so the engine's own `updateBeatPulse` retrigger — delay ≤ 0
    ⇒ `beat_pulse = beatPulseMax` — does not clobber the synth value on
    the next frame; when the app stops calling, the engine's own beat
    pulse / style animation take over as the fallback).
  - **`status.pulse` is never touched** — it feeds the simulation/RNG
    (`rng.advance(fixup(pulse))` in `update`). Only `beat_pulse` and the
    style colours change.
- **Application point** (`update.rs`, after the existing
  `style_data.update(m_ft, dm.powf(0.8))` block, the non-black-and-white
  style-update half): if `beat_hue` is `Some`, the engine calls
  `apply_beat_hue` + `apply_beat_pulse` on the shared style data in the
  SAME frame, before `update_player_position` / `update_walls`. The style
  animation remains the fallback when no beat is supplied (`beat_hue ==
  None` — inert for every legacy engine).
- The kid composition (07) calls `apply_music_beat` once per frame with
  its own beat/bar phases + synth energy + stage hue.

### C++-parity fix carried in 04b (supervisor ruling A, 2026-10-24)

`StyleData::calculate_color` emulated C++ `std::fmod(hue + shift, 360.f)`
as `total - total / 360.0 * 360.0`, which collapses every hue in
`[0, ~720°)` to ~0 (verified: 90°→0.0, 120°→0.0, 270°→0.0, 451°→0.0,
where C++ fmod gives 0.25 / 0.333 / 0.75 / 0.2527). Dynamic style colours
were therefore frozen near red for normal play — which would have made
the 04b beat palette visually inert. Fixed to the true fmod:
`total - (total / 360.0).trunc() * 360.0` (keeps the dividend's sign,
matching the existing comment). **Intentional parity fix, not a
regression**: dynamic style colours now animate through the hue range as
the C++ intended; the frozen-red behaviour was the bug. Pinned by
`calculate_color_hue_wraps_like_the_cpp_fmod` (90→0.25, 120→1/3, 270→0.75,
451→91/360, 350+shift 30→20/360, −90→−0.25). No test pinned the old
collapsed behaviour.

## Beat colour test results

- `th-data` (4 new, all pass): `apply_beat_hue` re-derives the main +
  player + text + wall colours (distinct hue-shifts/offsets) and clamps to
  `[hue_min, hue_max]`; clamp-safe on a wrapping style (300/30);
  `apply_beat_pulse` moves the derived colours via the pulse components
  and clamps to `[pulse_min, pulse_max]`; the fmod pins above.
- `th-game/tests/music_beat.rs` (4 new, all pass; apeirogon fixture, game
  RNG pinned to a fixed seed):
  - `apply_music_beat_sets_beat_pulse_immediately` — `status.beat_pulse`
    mirrors the energy on the call.
  - `beat_hue_and_pulse_apply_to_the_style_in_the_same_frame` — after one
    `update`: `current_hue == 250.0` exactly (the style animation would
    only have moved 149.0→149.25), `colors[0]` == (90,90,90,255) +
    (75,15,15,0)×1.0, and the STATIC main colour is untouched.
  - `the_beat_shimmer_is_tiny_and_deterministic` — phase 0 ⇒ base hue
    exact; quarter-phase ⇒ +1.5°; three-quarter ⇒ −1.5° (±0.01°).
  - `no_beat_keeps_the_existing_animation_and_the_simulation_is_untouched`
    — twin engines (with/without a beat every frame, 3 updates):
    `status.pulse` bit-identical, game-RNG `engine_state()` bit-identical,
    the no-beat engine's hue follows the pure style animation
    (149.0 + 3×0.25 = 149.75) while the beat engine carries 250.0.

## `DrawOptions` shape/default + skipped passes + recording test

```rust
// th-render::graphics (re-exported at the crate root)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawOptions {
    pub show_text: bool,       // draw_text (timer/status/FPS/message)
    pub show_level_info: bool, // the level-info box + its five lines
    pub show_key_icons: bool,  // the four key icons
    pub show_fps: bool,        // the FPS line within show_text
}
impl Default for DrawOptions { /* all true — the legacy rendering */ }
```

- `FrameInput` gains `pub draw_options: DrawOptions` (all four existing
  `FrameInput { … }` literals updated — 4 in `th-render` tests, 1 in the
  `th-app` bridge).
- `draw_frame` (`th-render::graphics`):
  - `!show_text` ⇒ `text_draw::draw_text` is skipped entirely (its
    per-frame sizing half in the bridge — `update_text_sizes` — is
    skipped too);
  - `!show_key_icons` ⇒ key icons skipped (AND `Config::getShowKeyIcons`);
  - `!show_level_info` ⇒ level info skipped (AND
    `Config::getShowLevelInfo`);
  - `show_fps` is passed into `draw_text` /
    `draw_text_time_and_status` and ANDs with
    `Config::getShowFPS()` — the FPS line is the only line removed; the
    other text keeps drawing when `show_text` is true;
  - background, walls, player, particles and flash are NEVER gated by
    `DrawOptions` (a field only ever removes a pass, never enables one
    over the C++ `Config` gates).
- `th-app::bridge`: `AppDraw` gains a private `draw_options` field
  (init `DrawOptions::default()`), `set_draw_options` + `draw_options()`
  accessors; the update-side `update_key_icons` / `update_level_info`
  calls and the `update_text_sizes` call are AND-gated with it, and it
  flows into `FrameInput`. The future tiny app (07) sets all false.
- Recording tests (all pass):
  - `th-render::graphics` (3): `draw_options_all_false_skips_every_text_pass`
    — zero `Text`/`Rectangle`/`Sprite` events while background tris /
    wall quad / player tri / flash (still the final pass) survive;
    `draw_options_default_keeps_the_text_passes` — 8 text draws (timer +
    status + fps + 5 level-info lines) + 4 key-icon sprites + the box;
    `draw_options_show_fps_false_hides_only_the_fps_line` — exactly one
    fewer text draw, sprites/box unchanged.
  - `th-app/tests/bridge.rs` (1): `draw_options_plumbing_kid_options_draw_zero_text`
    — a real windowed engine (apeirogon, seeded, run to death) through
    the bridge with key icons / level info / fps / flash config-enabled:
    default options ⇒ 8 text / 4+ trail sprites / 1 box; kid options ⇒
    zero text, zero level-info box, key icons gone (particle sprites
    kept), scene + flash survive.

## 04a P2 outcomes

1. **Difficulty DELAY consumer (fixed)**:
   `soft_collisions.rs::set_difficulty_mult_writes_the_shared_mult` now
   also asserts `get_delay_mult_dm()` (the public `SharedState` method,
   `delayMult / dm^0.10`, delayMax-capped when `hasDelayMaxLimit`) with
   the new mult — alongside the existing speed/music consumers.
2. **Soft wall burst (recorded — no helper exists)**: the death path
   carries no reusable wall-burst particle spawn (death = flash 255 +
   camera shake + the player death effect; the only burst spawners in the
   engine are the swap-specific `SwapParticleSpawnInfo` bursts, and the
   PB-star path is a dead surface since 02c1). **Decision: the engine's
   soft-collision visual IS the flash (120) + bump serial; the app's
   SFX / slow-motion lands in 07** — no new particle system invented.
   Recorded at the soft branch in `update.rs`.
3. **Soft custom-wall push (recorded)**: `handle_collision`'s push still
   runs before the kill — it is a single 3-phase push-then-report
   algorithm and the manager exposes NO kill-before-push path (the soft
   half re-derives the hit set via `soft_kill_overlapping` AFTER the
   pushes). The tiny pack (06) should not use custom walls. Recorded on
   `kill_soft_custom_walls`.

## RNG / order note

- The beat application block in `update.rs` is purely additive, sits
  AFTER the style update (unchanged position relative to
  `update_player_position` / `update_walls` / the RNG-advance block), and
  is inert when no beat is set (legacy engines: `beat_hue == None` → no
  `apply_*` calls, no extra lock traffic of consequence).
- `apply_music_beat` / `apply_beat_hue` / `apply_beat_pulse` draw NOTHING
  from the game RNG, and `status.pulse` (the RNG feed) is untouched —
  pinned bit-identically by
  `no_beat_keeps_the_existing_animation_and_the_simulation_is_untouched`
  (twin engines: same `status.pulse` + same `rng.engine_state()` after 3
  frames with/without the beat).
- The non-soft collision path is byte-identical (04b touched only the
  soft branch's doc comment). The 04a mechanics are otherwise untouched.
- The only legacy-observable change is the fmod parity fix (dynamic style
  colours now animate as C++ intended — see above).

## Tests + gate

- New: `th-data` +4 (beat hue/pulse/clamp/wrap + fmod pins),
  `th-game/tests/music_beat.rs` +4, `th-render` +3 (draw_options),
  `th-app` +1 (bridge plumbing). Modified:
  `th-game/tests/soft_collisions.rs` (delay assertion inside an existing
  test). No pre-existing test weakened; every existing `FrameInput`
  literal got `draw_options: DrawOptions::default()` (the legacy visual
  state is preserved — the full-frame / determinism bridge tests pass
  unchanged).
- `cargo test -p th-game soft_collisions` (the spec's verification filter)
  matches zero test NAMES (the 04a tests are named `soft_collision_*`,
  file `soft_collisions.rs`) — it ran the same 0 tests pre-04b; the file
  itself: `cargo test -p th-game --test soft_collisions` ⇒ 9/9 pass.
- Gate:

```
$ bash tools/gate.sh
== cargo fmt --all -- --check        GATE_EXIT=0
== cargo build --workspace          GATE_EXIT=0
== cargo clippy … -D warnings       GATE_EXIT=0
== cargo test --workspace           GATE_EXIT=0
gate_rc=0
```

`cargo test --workspace`: **539 passed / 0 failed / 1 ignored** (the
pre-existing th-lua sandbox doc-test ignore). 04a baseline was 527
passed — the delta is exactly the 12 new tests.

Committed by the orchestrator after the forked `VERDICT: PASS`.

(Note: an unrelated untracked file
`.agents/tasks/plan-002/issue-003c-impl.md` appeared in the worktree
during the run — orchestrator-side, not part of this change.)
