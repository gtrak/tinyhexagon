# Issue 04b — beat palette/pulse + `DrawOptions` text suppression

**Umbrella:** `.agents/plans/002-tinyhexagon/04-engine-kid.md` (04b is the
visual/render half). **Base:** post-04a `8990a64e`.
No behaviour change for the default (legacy) options; no 04a mechanics
changes except the review P2s below.

## Goal

- Make the engine palette/pulse respond to the synth beat
  (`apply_music_beat`, `StyleData::apply_beat_hue`/pulse, same-frame colours).
- Add `DrawOptions` and suppress text/FPS/level-info/key-icons in the
  renderer while keeping walls/player/particles/flash.
- Carry the 04a review P2s: assert the difficulty DELAY consumer; add the
  soft-collision wall-burst visual if the existing particle helper makes it
  clean, otherwise record it for 07; do not change the non-soft path.

## Beat API

```rust
// th-data::style_data
pub fn apply_beat_hue(&mut self, hue: f32);   // clamp to hue_min..hue_max, compute_colors
pub fn apply_beat_pulse(&mut self, energy: f32); // clamp to pulse_min..pulse_max, compute_colors

// th-game::hexagon_game
pub fn apply_music_beat(
    &self,
    beat_phase: f32,
    bar_phase: f32,
    energy: f32,
    base_hue: f32,
);
```

- `EngineState` stores the latest beat (`beat_hue: Option<f32>`,
  `beat_energy: f32`); `apply_music_beat` sets them and
  `status.beat_pulse = energy`, `status.beat_pulse_delay` as appropriate.
- In `update.rs`, after the existing `style_data.update(...)` call (the
  non-black-and-white branch), if a beat is set apply the hue/pulse so the
  derived colours reflect it in the SAME frame. The style's own animation
  remains the fallback when no beat is supplied. The app computes the stage
  hue; the engine may add a tiny `sin(beat_phase/bar_phase)` shimmer, but the
  base hue must be the dominant term. Do not change `status.pulse` (it feeds
  the simulation/RNG) — only `beat_pulse`/style colours.
- Tests: `apply_beat_hue` changes the derived main/player/text/wall colours
  and clamps; `apply_music_beat` changes them via the engine; no beat keeps
  the existing animation.

## `DrawOptions`

Add to `th-render::graphics`:

```rust
#[derive(Clone, Copy, Debug)]
pub struct DrawOptions {
    pub show_text: bool,
    pub show_level_info: bool,
    pub show_key_icons: bool,
    pub show_fps: bool,
}
impl Default for DrawOptions { /* all true */ }
```

- `FrameInput` gains `pub draw_options: DrawOptions`.
- `draw_frame`:
  - skip `text_draw::draw_text` (and its update/sizing) when
    `!show_text`;
  - skip key icons when `!show_key_icons` (AND the config gate);
  - skip level info when `!show_level_info` (AND the config gate);
  - pass `show_fps` into the text draw so the FPS line is skipped when
    false (keep the other text when `show_text` is true);
  - keep background, walls, player, particles and flash.
- `th-app::bridge`: `AppDraw` gains a `draw_options` field defaulting to
  `DrawOptions::default()`, a `set_draw_options` setter, and passes it into
  `FrameInput`. The future tiny app sets all false.
- Update every `FrameInput { ... }` literal (tests/bridge).
- Tests: a recording `DrawTarget` with all-false options sees zero text
  draws; the default still sees the text passes; `show_fps=false` hides only
  the FPS line.

## 04a review P2s

1. `soft_collisions.rs::set_difficulty_mult_writes_the_shared_mult`: add the
   DELAY consumer assertion (`get_delay_mult_dm` or the public equivalent),
   in addition to speed/music.
2. Soft wall burst: inspect the death/particle path. If a reusable
   wall-burst/flash particle spawn exists, call it from the soft branch (no
   death, same flash 120/bump semantics). If the death path has no reusable
   burst, record that the engine visual is the flash and the app's SFX/slow
   motion lands in 07; do not invent a new particle system.
3. Soft custom-wall hits still run `handle_collision`'s push before the kill;
   the tiny pack should not use custom walls. Record it; fix only if the
   manager exposes a clean kill-before-push path.

## Steps

1. `StyleData` beat hue/pulse methods + tests.
2. `EngineState` beat state + `apply_music_beat`; apply after the style
   update; tests.
3. `DrawOptions` type/field; skip passes; `AppDraw` field/setter; tests.
4. 04a P2 delay assertion + burst/record.
5. Gate and forked review.

## Verification

```sh
cargo test -p th-data style
cargo test -p th-render draw_options
cargo test -p th-game soft_collisions
bash tools/gate.sh; echo "gate_rc=$?"
rg -n 'DrawOptions|apply_music_beat|apply_beat_hue|apply_beat_pulse' crates/th-render/src crates/th-game/src crates/th-data/src crates/th-app/src
```

The non-soft collision path, update order and game RNG must be unchanged.
The default `DrawOptions` must keep the legacy rendering.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-004b-report.md` with:

- The beat API/update point and colour test results.
- The `DrawOptions` shape/default, the skipped passes and the recording test.
- The 04a P2 outcomes (delay assertion, burst decision, custom-wall note).
- Any RNG/order note, tests, gate result/tail, "No commit made."

Update `FEED-FORWARD.md` §04b: API names, `DrawOptions` shape, the beat
application point, the 05/07 handoff, and the P2 outcomes. Do not mark 04b
LANDED in `PLAN.md`; the orchestrator does that at commit.

## Risks

- Applying the beat after the style update must not lag a frame; call
  `compute_colors` after the hue/pulse set.
- `status.pulse` feeds simulation/RNG — do not change it; only
  `beat_pulse`/style colours.
- Text suppression must not hide walls/player/particles/flash.
- The default options must preserve every existing test's visual state.
