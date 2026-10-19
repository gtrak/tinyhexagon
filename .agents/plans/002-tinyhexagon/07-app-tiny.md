# Issue 07 — `th-app` tiny runtime, loop and renderer integration (desktop)

**Status:** LANDED
**Depends:** 04, 05, 06

## Objective

Make the playable `tinyhexagon` on desktop, using the same `th-render`/
`bridge`/`GlDrawTarget` path that Android and web will use. Implement the
journey (20-stage arc), soft-collision handling, music/visual coupling, SFX,
input, touch mapping, and the two corner icons. No text, no menus, no
persistence.

## Key decisions

- Binary: `tinyhexagon` from `th-app`.
- Engine composition: no profiles, no replay, no Steam/Discord, no menu.
- Audio: `th-audio::Audio` for SFX; `th-synth` for music. Set the engine's
  `no_music=true` so it never tries OGG level music.
- Config is in-memory (`Config::default()` + kid settings); never write
  `config.json`, `Profiles/` or `Replays/`. Every run starts at stage 0.
- Rendering: `bridge::draw` / `th-render::graphics::draw_frame` with
  `DrawOptions` (text/level-info/key-icons/FPS off) and a `GlDrawTarget`.
- Touch: `th-render::Window`'s touch/pointer feed mapped to left/right halves
  and the two icons. The adult `th-additions` overlay is gone.
- Fullscreen: desktop `Window::set_fullscreen`; web browser Fullscreen API via
  the salvaged `web_fullscreen.rs` (wired in Issue 08).
- The `tinyhexagon` binary can be a stub until this issue; here it becomes
  the real game.

## Journey / stage plan

`th-app/src/journey.rs` owns the table (from plan 002):

```rust
pub struct Journey {
    pub stage: u32,          // 0..=20
    pub elapsed: f32,        // seconds in the current stage
}
```

- Stage duration 30 s; stage 0..=20 (20 transitions = 10 min); after stage 20
  hold the plateau (no further difficulty/tempo/layer increase).
- `difficulty = 0.55 + 0.03 * min(stage, 20)`.
- `bpm = 70 + 1.5 * min(stage, 20)`.
- Level sequence every 2 stages: `meadow`, `river`, `stars`, cycle.
- Base hue: `style.hue_min + 47° * (stage / 6)` plus bar drift (computed by
  the app and passed to the engine).
- Music layers: 0 pad+drone; 2 pluck; 5 bell; 8 bass; 11 shimmer;
  14 heartbeat; 17 supersaw (all 8). Call `MusicDirector::set_stage`.
- On stage advance: `new_game` when the level changes (every 2 stages) with
  the new multiplier; otherwise `set_difficulty_mult`. Reset `elapsed`,
  emit a burst, play the stage-up SFX, update the music stage.
- Manual reset: restart the current stage (`new_game` with the current level
  and multiplier), reset `elapsed`, play `restart.ogg`, no stage change.
- A run never ends; the plateau is the end of escalation.

## Composition

- Root/assets: find `_RELEASE` (the port's resolver logic; keep it in
  `lifecycle.rs` or a small helper). `Assets::new(root, headless, false)`.
- `Config::default()` then set kid values: `no_music=true`, beat pulse on,
  3D off, `show_text` handled by `DrawOptions`, resolution 1024×768.
- `compose_with_config` (de-scoped in Issue 02): no profiles/replay store/
  Steam/Discord. Pass `Some(Arc<th_audio::Audio>)` for SFX and `None` for the
  engine renderer (the bridge draws).
- `HexagonGame::new_game("tiny", level, first_play, multiplier, ...)`;
  `start()`.
- The engine's `new_game`/`start` signature comes from the Issue 02
  feed-forward; do not guess.

## Loop

- `th_render::Window::create(1024, 768, "tinyhexagon", 4)`.
- Fixed 240 Hz engine ticks (`TIME_STEP = 0.25`) accumulated from the frame
  `dt`; clamp `dt` to ~0.1 s.
- Per frame:
  1. Drain input edges (keyboard/mouse/touch).
  2. Set `set_input_impl_ccw/cw` from the held left/right.
  3. Run the engine ticks with `timescale` (0.35 for ~0.35 s after a bump,
     1.0 otherwise); `post_update` after each.
  4. Consume `take_bump()`: play the bump SFX, duck the synth, add particles,
     set the flash.
  5. Update the journey timer; advance/reset as above.
  6. Compute the beat view, `apply_music_beat`, set `status.pulse` if needed.
  7. Build `DrawOptions { show_text:false, show_level_info:false,
     show_key_icons:false, show_fps:false }`; `bridge::draw` into the
     `GlDrawTarget`.
  8. Draw the two icons (reset top-left, fullscreen top-right) in screen
     space via the `DrawTarget` (rectangles/triangles; no fonts).
  9. `window.swap_buffers()`.
- The helping hand: in stage 0 only, after 2 s without left/right input,
  steer toward `nearest_gap_angle`; any input disables it for the session.
- Fullscreen edge: `window.set_fullscreen(!window.get_fullscreen())`, play
  `beep.ogg`.
- Reset edge: restart the stage, play `restart.ogg`.
- Esc: close the window.
- Icons: hit-test the top-left/top-right ~64×64 px squares; mouse click and
  touch tap. On desktop, draw a small cursor visible state; no text.

## Touch and fullscreen

- `Window::get_input_state()` gives fingers and `finger_down_positions()`.
  For each new finger, map `x < width/2` to left, else right; track finger ids
  so multi-touch holds both sides; a finger on an icon is consumed by the
  icon (no rotation).
- The adult overlay is removed. Implement a small `touch.rs` in `th-app`
  (~50–100 lines).
- Web fullscreen: the salvaged `web_fullscreen.rs` (from `th-additions`)
  installs the browser Fullscreen API sinks; wire it in Issue 08. On desktop,
  `Window::set_fullscreen`; on Android the activity is already fullscreen, so
  the icon can be hidden or a no-op.

## SFX

Use `th_audio::Audio` via the `Audio` trait:

| Event | Sound |
|---|---|
| Start | `go.ogg` (engine `start`) |
| Collision | `beep.ogg` (soft volume; music duck) |
| Stage-up | `increment.ogg` or `levelUp.ogg` |
| Reset | `restart.ogg` |
| Icon tap | `select.ogg` |
| Fullscreen | `beep.ogg` |

Set a gentle sound volume (`set_sound_volume`); keep the synth master gain
independent. On web, start/resume both audio paths on the first gesture
(Issue 08).

## Headless tests

- A 60 s simulated run: no panic, stage advances at 30 s, no `has_died`,
  frames contain wall/player batches.
- A forced collision: bump recorded, no death, stage/time preserved,
  timescale applied.
- Reset: stage unchanged, elapsed reset, `new_game` called with the same
  level/multiplier.
- Input: left/right changes the player angle.
- Text: with `DrawOptions` all false, a recording target has zero text draw
  calls.
- No persistence: after a run, no `config.json`/`Profiles/`/`Replays/` writes
  (use a temp root and assert directory contents).

## Files

| File | Change |
|---|---|
| `th-app/src/main.rs` | `tinyhexagon` bin entry. |
| `th-app/src/tiny.rs` | Window loop + input + stage orchestration. |
| `th-app/src/journey.rs` | Stage table/timers. |
| `th-app/src/touch.rs` | Two-zone touch + icon hit tests. |
| `th-app/src/web_fullscreen.rs` | Salvaged install. |
| `th-app/src/compose.rs` | De-scoped composition. |
| `th-app/src/bridge.rs` | `DrawOptions`. |
| `th-app/src/draw_target.rs` | `GlDrawTarget` (extracted). |
| `th-app/src/lifecycle.rs` | Root/assets only; no config save. |
| `th-app/Cargo.toml` | `th-synth`, `th-audio`; binary `tinyhexagon`. |
| tests | Headless session/collision/reset/input/text/no-storage. |

## Steps

1. Wire the bin and composition; verify a headless 1 s run.
2. Add the journey/stage loop and input.
3. Add `DrawOptions` + `GlDrawTarget` + icons; render a frame under Xvfb.
4. Add soft collision/bump, timescale, SFX, synth duck.
5. Add the beat palette and music stage updates.
6. Add touch/icon mapping and fullscreen/reset edges.
7. Add the helping hand.
8. Add the headless tests; run the gate and the forked reviewer.
9. Manual desktop: `xvfb-run -a cargo run`, screenshot, 5–10 min run.

## Verification

- `xvfb-run -a cargo run -p th-app` (the `tinyhexagon` bin) opens, renders,
  responds to left/right, resets, toggles fullscreen, exits on Esc.
- 10-minute accelerated/headless run reaches stage 20 and plateaus.
- Collisions never set `has_died`; bump/flash/duck/slow happen.
- No text draw calls; no config/profile/replay writes.
- `cargo test -p th-app` headless green.
- A screenshot shows the arena, walls, player and both icons.

## Risks

- The engine's beat hue override and the renderer's style colours must agree;
  verify the palette changes with the beat.
- Text suppression must keep the scene rendering; test with the recording
  target.
- Touch icon hit tests vs. rotation zones: consume the icon touch first.
- The 3D/background passes may need config tuning (3D off, background on).

## Feed-forward

Record: the final composition signature, `DrawOptions`, journey table, touch
mapping, SFX ids, and the beat-hue call. Issue 08 reuses the loop for
Android/web; Issue 09 tunes the table.
