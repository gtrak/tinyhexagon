# Issue 07 — tiny runtime, loop and renderer integration (report)

**Worktree:** `slot-2` (branch `slot-2`, base `9fbc42f1`). **NOT committed**
(per the 07 contract; the orchestrator commits after review).
**Mode:** the 07 runtime code was already in place and compiling; this pass
finished the three failing integration tests, aligned the reset-click mouse
source, and made the gate green.

## Deliverables in place (from the 07 impl, verified by this pass)

- `th-app/src/tiny.rs` — `TinyRuntime` (the window-agnostic per-frame half
  `frame(dt, &FrameEvents) -> FrameActions` + the windowed draw) and the
  desktop `run_windowed` loop; `th-app/src/bin/tinyhexagon.rs` — the real
  `tinyhexagon` bin.
- `th-app/src/journey.rs` — the 20-stage arc + table (below).
- `th-app/src/touch.rs` — the two-hold-halves + two-icon touch state (below).

## Composition (07's "Composition")

- `TinyRuntime::new(assets: Arc<Assets>, root: PathBuf, window: bool,
  config: Config, audio: Option<Arc<TinyAudio>>, music: MusicDirector)`:
  `compose_with_config(assets, root, window, Arc<RwLock<Config>>,
  renderer: None, engine_audio: Option<Arc<dyn Audio>>,
  Arc<NoJoystick>)` — the 02 de-scope (no profiles / replay / Steam /
  Discord / menu) — then ONCE `game.set_soft_collisions(true)` (04a), then
  stage 0: `new_game(PACK, journey.level_id(PACK), true,
  journey.difficulty())` + `start()`. `audio: None` = the engine's
  null-audio seam (the headless tests); `Some` = the real `th_audio::Audio`.
- The bin's config: `Config::default()` → `load_and_reapply` with
  `DesktopModeSource::Lazy` (the C++ `mainClient` load + reapply) →
  `kid_config` (`no_music=true` — `th-synth` owns the soundtrack, 3D off,
  1024×768; the beat pulse stays on). In-memory only: the kid config never
  writes back.
- `DrawOptions { show_text: false, show_level_info: false,
  show_key_icons: false, show_fps: false }` (04b — zero text / HUD; the
  scene + flash + the two icons keep drawing).

## Journey table (`journey.rs`)

- Stage duration 30 s; stages 0..=20 (20 transitions = 10 min); after 20
  the plateau holds (no further difficulty / tempo / hue / layer / level
  increase).
- `difficulty = 0.55 + 0.03 * min(stage, 20)` (0.55..=1.15).
- `bpm = 70 + 1.5 * min(stage, 20)` (the synth's `stage_params` owns the
  live mix; this is the journey's mirror).
- Level: one level per 2 stages, cycling every 6 — `meadow`, `river`,
  `stars` (06's pack ids `{PACK}_{level}`).
- Base hue: `style.hue_min + 47° * (stage / 6)` (meadow 100, river 190,
  stars 270 at their levels; capped at the plateau).
- Music layers (05a): 0 pad+drone, 2 pluck, 5 bell, 8 bass, 11 shimmer,
  14 heartbeat, 17 supersaw — `MusicDirector::set_stage(stage)`.
- Stage-up work (`TinyRuntime::stage_up`): even stages (the level change)
  → `new_game` + `start()` with the new level + multiplier; odd stages →
  live `set_difficulty_mult`; the music stage updates; the stage-up burst
  flash (engine flash, 90); the stage-up SFX.
- Manual reset (`reset_stage`): `new_game` on the SAME level + multiplier
  + `start()` (no stage change), `journey.reset_elapsed()`, `restart.ogg`.

## Loop (the plan's steps 1–9)

`TinyRuntime::frame`: (1) drain the input edges (the touch feed, the
keyboard A/LEFT / D/RIGHT holds, the Esc edge, the icon tap edges); (2)
the icon actions (reset / fullscreen + the SFX); the helping hand (stage 0
only: after 2 s without left/right input it steers toward
`nearest_gap_angle`; ANY input disables it for the session); (3) the fixed
240 Hz engine ticks (`th_config::TIME_STEP` = 0.25 frame-units per tick,
frame-dt clamped to 0.1 s, the accumulator leftover kept); (4) the held
left/right into the engine impl flags (the helping hand re-steers per
tick); (5) the bumps — `bump_serial` delta → `beep.ogg` + `music.bump()`
(the 05a duck) + the slow-motion re-arm (0.35× for 0.35 s — the slice that
STARTS while the bump is live gets the slow timescale); (6) the journey in
REAL time (`journey.advance(dt)` — the slow-motion does not delay the
stages); (7) the beat palette EVERY frame (04b P2: `beat_hue` is never
cleared — `apply_music_beat(beat.beat_phase, beat.bar_phase,
beat.energy, journey.base_hue())`). `FrameActions { close,
toggle_fullscreen }` leave the window layer alone (the bin toggles
`set_fullscreen`, closes on Esc). `draw` runs the bridge (all-false
`DrawOptions`) and then the two corner icons in screen space (a 1:1 view;
`~64×64` backing squares + the reset triangle / the fullscreen expand
bars — no fonts).

## Touch mapping (`touch.rs`)

- Two hold halves: each NEW finger is bound to the half it came down on
  (`x < width/2` → left/CCW, else right/CW) and keeps it for the whole
  touch (finger-captured halves — multi-touch holds both sides).
- The two icons (~64×64, margin 8): top-left = reset, top-right =
  fullscreen; a finger that comes down on an icon is CONSUMED by the icon
  (one tap on the down-edge, no rotation, and a held icon finger keeps no
  tap — the icon-re-fire bug's `Bound` state, kept).
- `TouchState::feed(&[(id, pos)], width) -> (left_held, right_held,
  icon_tap)`; the geometry is shared with the icon draw
  (`icon_at` / `icon_position`).
- **Mouse source (this pass's alignment):** the icon's mouse click source
  is `FrameEvents.mouse_left` — the window feed's field, the ONE place
  `frame` reads it. `run_windowed` mirrors it from the drained
  `InputState`'s left button, so the windowed and the test-fed paths
  cannot diverge. (Before: `frame` read
  `ev.input.is_button_pressed(Button::LEFT)` while the test fed
  `mouse_left` — the two paths diverged and the reset click never fired.)

## SFX ids / volumes

| Event | Sound |
|---|---|
| Start | `go.ogg` (the engine's `start`) |
| Collision (bump) | `beep.ogg` + the 05a synth duck (0.55 → 0.22 over 0.6 s) + the 0.35×/0.35 s slow-motion |
| Stage-up | `levelUp.ogg` |
| Reset | `restart.ogg` |
| Icon tap | `select.ogg` |
| Fullscreen | `beep.ogg` |

Gentle sound volume: the bin sets `audio.set_sound_volume(60.0)` (SFML
units 0..=100; the synth master gain stays independent).

## Headless tests (`tests/tiny_runtime.rs` — 7/7 green)

1. `the_sixty_second_run_advances_the_stage_and_never_dies` — a 65 s
   simulated run: the stage advances at 30 s (`set_difficulty_mult`) and
   60 s (`new_game` on the river level), no `has_died` (soft collisions),
   the frame carries the wall / player / background / flash batches.
2. `a_forced_collision_bumps_without_dying_and_applies_the_timescale` —
   `set_always_spin_right(true)` (the engine overrides the impl flags per
   tick, so the helping hand cannot rescue the player) drives the player
   into the first wall: the bump (the `bump_serial` delta) fires, no
   death, the bump slow-motion is active, the stage + the stage clock are
   preserved (the journey is real time).
3. `the_reset_restarts_the_stage_without_changing_it` — the reset icon's
   mouse click (`FrameEvents.mouse_left` at (32, 32)): no stage change,
   the stage clock restarted, `new_game` + `start` on the SAME
   level/multiplier (the meadow @ 0.55×), the player angle re-zeroed.
4. `left_and_right_input_change_the_player_angle` — the keyboard A/LEFT
   hold steers CCW (the angle falls), the D/RIGHT hold CW (it re-crosses
   zero), and the touch halves drive the same impl flags (a left-half
   finger turns CCW).
5. `the_kid_options_draw_zero_text_with_the_scene_intact` — with the
   all-false `DrawOptions`, a recording target sees ZERO text draws while
   the scene (background / walls / player / flash) + the two icons'
   rectangles + the reset triangle keep drawing.
6. `the_run_writes_no_config_profiles_or_replays` — the no-storage proof
   (below).
7. `the_ten_minute_run_reaches_stage_20_and_plateaus` — 600 s of real
   time reaches the stage-20 plateau and holds it (the capped table).

### The three failures this pass fixed (root causes)

- **Stage-id expectation:** the engine's `get_level_data().id` is the
  level's SHORT id (the JSON's `"id"` — `river` / `meadow` / `stars`);
  the registry's `{pack}_{level}` key is what `new_game` takes. The tests
  now assert `LevelData.pack_id == PACK` + the short `id` (the 65 s run's
  stage-2 river and the reset's meadow).
- **Reset-click mouse source:** aligned to `FrameEvents.mouse_left` as
  the single icon-click source (above) — the test's down frame now fires
  the reset.
- **Forced-collision budget:** the first wall spawns ~3 s in (the
  meadow script's `t_wait(180)`), but at meadow's 0.55× speed multiplier
  the walls then take ~12 s more to travel the default 1600 px spawn
  distance in (~108 px/s — `speed × 5 × mFT` per tick at 240 Hz), so the
  first soft bump lands ~16 s in (seed-0 stream, instrumented). The
  budget went 15 s → 20 s (the `set_always_spin_right` drive was already
  in place).

## Gate

`bash tools/gate.sh` — **fmt / build / clippy `-D warnings` /
`cargo test --workspace` all `GATE_EXIT=0`** (the full `th-app` suite:
lib 25/25, bridge 8/8, pack_shaders 9/9, tiny_pack 1/1, tiny_runtime
7/7).

Getting there (the 07 code pre-dated the last clippy/fmt passes): the
gate's clippy stage flagged 9 lints in the 07 files — 8
`doc_lazy_continuation` (doc lines that START WITH `+ ` parse as markdown
list items: the `kid_config` doc's `+ the kid overrides...` line and the
lib module-doc's `+ the beat palette...` line — reworded, no indent
hacks) + 1 `manual_is_multiple_of` (`stage % 2 == 0` →
`stage.is_multiple_of(2)` in `stage_up`); the test's two
`field_reassign_with_default` instances were moved into struct-update
initializers; `cargo fmt --all` reflowed the 07 files (journey / tiny /
lib / the test).

## xvfb run + screenshot (this box HAS Xvfb)

`xvfb-run -a cargo run -p th-app --bin tinyhexagon -- --tiny-shot=14`
opens the 1024×768 window, runs, saves `screenshot.png` (the CWD) and
closes. The 14 s capture shows the arena (the six meadow-sector
background, rotated by the level's 0.04 rotation), the white wall bars
in flight, the player hexagon + its dot at the gap, and the player trail.
No death by 14 s (the soft collisions + the helping hand).

**Screenshot/icon note:** the bridge's `mustTakeScreenshot` save fires at
`draw_frame` step 12 (the C++ `HGGraphics.cpp:305-313` slot — inside the
scene draw, BEFORE the screen-space icon pass that `TinyRuntime::draw`
appends after the bridge returns). The bridge PNG therefore captures the
scene WITHOUT the two corner icons; the icons' geometry is pinned instead
by the headless `the_kid_options_draw_zero_text_with_the_scene_intact`
(6 icon rectangles: 2 backing + 4 expand bars, + the reset triangle, +
the scene batches, 0 text). A post-icon framebuffer capture would need a
new `DrawTarget` hook — out of scope for this pass (and 08's).

## No-storage proof

`the_run_writes_no_config_profiles_or_replays`: a run (composition +
10 s of frames + a rendered frame) against a TEMP root (the `_RELEASE`
content reached through `Assets` / `Packs` symlinks) leaves the root free
of `config.json` / `Profiles/` / `Replays/` (nothing beside the two
planted symlinks), and the `_RELEASE/config.json` stays byte-identical.

## Deviations / open notes

- No commit (per the contract). FEED-FORWARD §07 filled (the section's
  status stays "pending" until the commit).
- The 07 plan's ideal screenshot ("arena, walls, player and both icons")
  is met except the icons (the bridge save-ordering above) — the icons
  are headless-verified instead; 08 can add the post-icon capture if the
  evidence must be one PNG.
- The 05a handoff notes in FEED-FORWARD §07/§08 still stand (the live
  audio start falls back to the clock on this audio-less box — the run
  log shows the fallback; the beat keeps advancing).
