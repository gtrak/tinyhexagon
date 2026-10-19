# Issue 05 — implementation spec (`th-synth`: cross-target spike + music director)

**Worktree:** `/home/gary/dev/tinyhexagon/.agents/worktrees/slot-1` (branch
`slot-1`, base `9130c969`). The main worktree and slot-2 are OFF-LIMITS.
**Contract:** `.agents/plans/002-tinyhexagon/05-synth.md` — read it fully; it
is the detailed spec. This file adds the slot working agreement.
**Do not commit.** The orchestrator reviews/gates and lands ONE squashed
commit after PASS.

## Deliverables

- New `crates/th-synth` crate (`rust-synth 0.23.0`; only `audio` + `math`)
  and the workspace member.
- The cross-target spike FIRST:
  `cargo check -p th-synth` host, `--target aarch64-linux-android`,
  `--target wasm32-unknown-emscripten`. If the stock crate fails on the
  TUI/recording deps, vendor/patch it (features or path/[patch]) as the
  contract says; record the exact failure + patch in the report/skill.
- `MusicDirector`/`BeatView` per the contract: 8 tracks, stage map, `tick`,
  `bump` duck/recover, fallback clock with no device, web
  `start_on_gesture`, desktop/Android boot start, no graph rebuild on stage
  changes.
- `.agents/skills/lib-rust-synth.md` and `lib-cpal.md` written from the real
  crate source (API, presets, clock, duck, cpal 0.15.3 emscripten/oboe hosts,
  autoplay, patch notes), indexed in `.agents/skills/README.md`.
- Tests per the contract: stage map monotonic/capped, beat/bar phases,
  duck/recover, no-device fallback, always 8 tracks.

## Constraints

- Do not wire the app/runtime (07) or the web gesture handling (08); expose
  `start_on_gesture` only.
- Do not add SFX (the app's `th-audio` owns them).
- Do not rebuild the graph on stage changes; mutate `Shared` params only.
- If a target check fails and the vendored patch is not viable, stop and
  report the exact failure — do not silently drop music on a target.

## Verification

```sh
cargo check -p th-synth
cargo check -p th-synth --target aarch64-linux-android
cargo check -p th-synth --target wasm32-unknown-emscripten
cargo test -p th-synth
bash tools/gate.sh
```

Manual 20 s desktop listen if an audio device exists; otherwise record the
fallback-clock evidence. No device must not panic.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-005-report.md` (spike result stock vs
patched, patch location, director API, track/layer table, target caveats,
tests/gate, no commit). Update `.agents/plans/002-tinyhexagon/FEED-FORWARD.md`
**§05 only**; do not edit the main worktree or mark the plan LANDED.
