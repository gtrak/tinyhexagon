# Issue 09 — Tuning, docs, gate and playtests

**Status:** IN PROGRESS (09-taste feel pass landed; final tuning/docs/playtests pending)
**Depends:** 08

## Objective

Take the game from "works" to "a child can play it": tune the 10-minute
escalation, verify the no-reading/intuitive bar, finish the docs, run the
full gate on all targets, and close the plan. This is the only issue where
feel is the deliverable.

## Tuning (one table)

All tunables live in `th-app/src/journey.rs` (and the tiny styles/levels):

- Stage duration 30 s; 20 stages; plateau after stage 20.
- `difficulty = 0.45 + 0.085 * t`, `t = stage + elapsed/30` (0.45..2.15;
  written every frame). **09-taste landed.**
- `bpm = 70 + 2.5 * t` (cap 120).
- Music layers: pluck 0, bell 2, bass 3, kick 4, shimmer 7, saw 11
  (bar-wise chord progression + arpeggios + an evolving kick).
- One continuous `journey` level (no mid-run `new_game`); the shape tour
  `[6,6,4,4,3,3,5,5,6,6,7,7,4,4,8,8,5,5,6,6,6]`.
- Base hue `(100 + 28*t) mod 360`, dynamic hues/background, 6° beat
  shimmer, a synthesized beat pulse on the fallback clock.
- Soft-collision `timescale`/duration, duck depth/recovery, flash decay,
  particle counts, icon size/placement, SFX volumes.
- Helping-hand idle delay (2 s) and steering speed.
- The first wall wait (~1 s after spawn: `l_setWallSpawnDistance(1050)`, `t_wait(30)`).

Playtest target: a 4–6 year old who cannot read can start, rotate, understand
the first collision without distress, and still be playing after 5 minutes.
A 10-minute run should feel like a journey, not a difficulty spike; the
stage transitions should be legible from the palette/music alone.

Process:

1. Run the game on desktop and at least one of Android/web.
2. Watch/ask the child; record what they did and where they hesitated.
3. Change one tunable family at a time; keep the table as the single source.
4. Re-run the 10-minute headless/accelerated check and the manual run.

Do not add new mechanics to fix confusion; fix the pacing, the gap width, the
intro, the assist or the feedback.

## Docs

- `README.md` (final):
  - What tinyhexagon is (kid-friendly Open Hexagon; no reading, no menus,
    no fail).
  - Controls table (desktop + touch).
  - The 10-minute stage arc and soft collision.
  - Build/run: `cargo run`, `tools/package.sh`, Android/web packaging.
  - Generated music (`rust-synth`) and SFX.
  - No local storage/profiles.
  - Upstream relationship + the skill/changelog pointer.
  - Credits (Open Hexagon / Super Hexagon / music note) and license
    (AFL 3.0 inherited; `rust-synth` MIT third-party notice).
- `.agents/HISTORY.md`: final read-through; ensure it links the archive, the
  active plan, the upstream skill, and records the pivot.
- `.agents/plans/002-tinyhexagon/PLAN.md`: mark issues LANDED/closed; update
  the status table and feed-forward.
- `.agents/skills/upstream/changelog.md`: top SHA current (or a recorded
  pending delta with a triage note).
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md`: final lessons.

## Full verification

- `bash tools/gate.sh` green on Linux and Windows (fmt/build/clippy/test).
- Desktop: `xvfb-run -a cargo run -p th-app` (or the packaged binary) opens,
  plays, resets, toggles fullscreen, exits; a 10-minute run reaches the
  plateau; a screenshot is captured.
- Android: `check-android.sh`, packaged APK, device smoke (touch, audio, no
  text, no persistence).
- Web: emscripten build, browser smoke (first-gesture audio, canvas, touch/
  mouse, fullscreen, no IDBFS/localStorage, no text).
- Packaging: all four `tinyhexagon-*` artifacts produced; `actionlint` clean.
- No `oh-*` packages; no C++ tree; no parity tooling; `.agents` summary +
  archive + upstream skill/changelog present.
- No local storage: assert no `config.json`/`Profiles/`/`Replays/` writes in
  the desktop temp root; no IDBFS/localStorage in the web probe.
- Text audit: the kid render path has no `draw_text`/`Text`/font calls.
- A headless 10-minute accelerated run reaches stage 20, plateaus, never dies.
- The forked reviewer PASSes the final issue; the orchestrator re-runs the
  gate; the commit cites both.

## Manual checklist (the ship bar)

- The game starts without a menu or a click.
- Holding left/right visibly rotates; releasing stops.
- The first wall is understandable within one attempt.
- A collision feels like a bump, not a punishment; play continues.
- The next stage feels like progress, not a new game.
- F11/F, R, Esc work; touch halves and corner icons work.
- No text appears anywhere.
- Music is audible, calm, and changes at stage-ups and hits; SFX are gentle.
- A child who cannot read can play without instruction.
- The 10-minute arc escalates slowly and plateaus without a wall.

## Steps

1. Tune on desktop with a child/playtest; record the values.
2. Verify the same values on Android and web (frame rate, touch, audio).
3. Write the final README/HISTORY/plan status.
4. Run the full gate and package all targets.
5. Dispatch the forked reviewer; fix any blockers; re-gate.
6. Commit with the final evidence and mark the plan complete.

## Verification evidence to keep

- Gate log (`target/gate.log`) tail for the final run.
- Desktop screenshot.
- Android device/browser notes.
- Web probe output.
- The tuning table values and the playtest notes (in the report, not in the
  README).

## Risks

- Subjective tuning can loop; timebox it and prioritize the first-contact
  experience and the 10-minute pacing.
- Target audio/perf differences may require per-target degradation; record
  it in `FEED-FORWARD.md` and the README.
- Do not reintroduce menus/text to solve a discoverability problem; use the
  assist, colour, motion and pacing.

## Feed-forward

Close the plan: archive `002-tinyhexagon` per the plan-process skill when the
operator agrees, leaving the durable record. Record the final tunables and
any per-target deviations.
