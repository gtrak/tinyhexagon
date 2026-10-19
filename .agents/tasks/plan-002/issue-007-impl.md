# Issue 07 — implementation spec (tiny runtime, desktop)

**Worktree:** `.agents/worktrees/slot-2` (branch slot-2, base `9fbc42f1`).
Main and slot-1 are OFF-LIMITS. **Do not commit.** One squashed commit after
review.

**Read:** `.agents/plans/002-tinyhexagon/07-app-tiny.md` in full (the
contract), plus the 04a/04b/05a/06 feed-forward sections. Do not re-derive
the engine/renderer APIs; use the feed-forward signatures.

## Exact APIs (from the feed-forward)

```text
HexagonGame::new(config, assets, audio, script_host, joystick, cosmetic_rng,
                  window, renderer)                 // 8 args
compose_with_config(assets, root, window, config, renderer, audio, joystick)
compose_with(assets, root, window)
new_game(pack_id, level_id, first_play, difficulty_mult)
set_soft_collisions(true), bump_serial(), set_difficulty_mult(f32),
nearest_gap_angle(player_angle), wall_gaps()
DrawOptions { show_text, show_level_info, show_key_icons, show_fps } // all false
MusicDirector::{start, set_stage, bump, tick, stop, resume, start_on_gesture}
StyleData beat via HexagonGame::apply_music_beat(beat_phase, bar_phase, energy, hue)
```

Tiny pack ids: `tinyhexagon_tinyhexagon_tiny_1_{meadow,river,stars}` (the 06
test), style ids meadow/river/stars, hue families 100–160/190–250/270–330.
Engine `no_music=true`; SFX via `th_audio::Audio`; music via `th-synth`.

## Deliverables (per the 07 plan)

- Real `th-app/src/main.rs` + `tiny.rs` window loop; `journey.rs` (20×30 s,
  difficulty/BPM/level/hue/layer table); `touch.rs` (two hold halves + two
  icons); the real `tinyhexagon` bin (replace the stub).
- Composition: in-memory `Config::default()` + kid settings; no profiles/
  replay/Steam/Discord/menu; `Assets`/`_RELEASE`; audio Some, renderer None;
  the `_RELEASE` root resolver.
- Fixed 240 Hz ticks, dt clamp, bump timescale 0.35/0.35 s, SFX ids
  (go/beep/increment/restart/select), music `set_stage`/`bump`, beat hue,
  `DrawOptions` all false, the two corner icons drawn with `DrawTarget`
  (no fonts), reset/fullscreen/Esc, stage-0 helping hand.
- Tests: 60 s simulated run (stage advance, no death, wall/player batches),
  forced collision (bump, no death, timescale), reset, left/right input,
  zero text draws with all-false options, no config/profile/replay writes
  (temp root). Use the existing `th-app/tests/common` composition.
- `xvfb-run -a cargo run -p th-app` (or the bin) opens/renders if Xvfb is
  available; capture a screenshot as evidence if possible.

## Constraints

- No text/menus/persistence; every run starts stage 0.
- Do not wire Android/web (08) except keeping the shared loop reusable.
- Do not change the engine/renderer/synth semantics; only compose them.
- Write the loop/journey/tests first; do not over-research. If a `th-render`
  input/window API is unclear, read only the relevant window/input file.

## Verification

```sh
cargo test -p th-app
bash tools/gate.sh
xvfb-run -a cargo run -p th-app   # if Xvfb exists; otherwise record the check
```

A 10-minute accelerated headless run must reach stage 20 and plateau; a
forced collision must never set `has_died`; the recording target must see
zero text draws; the temp root must stay free of `config.json`/Profiles/
Replays.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-007-report.md` (composition signature,
journey table, touch mapping, SFX ids, beat call, test/gate, screenshot/no-
storage proof, no commit). Update FEED-FORWARD **§07 only**; hand off the
08 reuse notes. Do not mark the plan LANDED; no commit.
