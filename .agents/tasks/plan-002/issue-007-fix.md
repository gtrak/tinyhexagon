# Issue 07 fix — finish the tiny_runtime integration tests

**Worktree:** `.agents/worktrees/slot-2` (base `9fbc42f1`). The 07 code is
already present and compiles; the previous worker timed out at test-finish.
Main/slot-1 off-limits. **Do not commit.**

**Read:** `.agents/tasks/plan-002/issue-007-impl.md`,
`.agents/plans/002-tinyhexagon/07-app-tiny.md`, then only the failing test
file + the specific `th-app/src/{tiny,journey,touch}.rs` paths involved.

## Current state

- `cargo build -p th-app` / the lib tests are green (25/25).
- `cargo test -p th-app --test tiny_runtime`: 4/7, with these failures:
  1. `the_sixty_second_run_advances_the_stage_and_never_dies` (line ~83):
     expected `"tinyhexagon_tinyhexagon_tiny_1_river"`, actual `"river"`.
     `get_level_data().id` is the short level id; fix the test expectation
     (or compare the short ids).
  2. `the_reset_restarts_the_stage_without_changing_it` (line ~188): the
     icon click never fires because `tiny::frame` reads
     `ev.input.is_button_pressed(Button::LEFT)`, while the test sets
     `FrameEvents.mouse_left = true`. Make them agree: prefer the
     `FrameEvents.mouse_left` edge as the icon-click source (the window
     feed's field; document it) or set the InputState button in the test
     (`set_button_pressed`); do not leave both paths divergent.
  3. `a_forced_collision_bumps_without_dying_and_applies_the_timescale`
     (line ~140): no bump within the 15 s budget. The test does not drive
     the player into a wall; set `set_always_spin_right(true)` /
     `set_must_start(true)` (or the equivalent) and allow enough ticks for
     the first wall (~3 s) before asserting the bump, no death, and the
     timescale.
- The `touch.rs` icon-re-fire bug is already fixed (the `Bound` state); do
  not undo it.

## Constraints

- Fix only the failing tests / the mouse-source alignment; do not rewrite the
  runtime, journey, or touch state machine.
- Keep the 07 behaviors: no death, bump/timescale, reset, input, zero text,
  no config/profile/replay writes.
- No new features; no 08 wiring.

## Verification

```sh
cargo test -p th-app
bash tools/gate.sh
xvfb-run -a cargo run -p th-app   # if Xvfb exists; screenshot if possible
```

The full `th-app` suite must be green (the three failures fixed); the gate
4/4; the no-storage/temp-root proof must remain. Update the 07 report /
FEED-FORWARD §07 only if they exist; otherwise write the 07 report with the
composition/journey/touch/SFX/test evidence. No commit.
