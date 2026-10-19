# Issue 08a fix — finish the web build/probe

**Worktree:** `.agents/worktrees/slot-2` (base `c9bf8d23`). The 08a code is
present; the host gate is green (567). Main/slot-1 off-limits. **No commit.**

## Current state

- `cargo test -p th-app` 7/7; `bash tools/gate.sh` 4/4, 567 passed.
- The emscripten build is down to one known error:
  `crates/th-app/src/tiny.rs:925` — `.and_then(|w| w.navigator())` where
  `Window::navigator()` returns `Navigator` (not `Option<Navigator>`).
  Fix: `.map(|w| w.navigator())` (or `Some(w.navigator())` inside the
  and_then) and re-run.
- The 08a worker timed out at the build; it had already fixed the
  `AtomicU64`/`AtomicU32` angle static and added the `Navigator`/
  `UserActivation` web-sys features.

## Steps

1. Fix the `navigator()` line; re-run the emsdk build:
   ```sh
   podman run --rm -v "$PWD":/work \
     -v /home/gary/.rustup:/home/gary/.rustup \
     -v /home/gary/.cargo:/home/gary/.cargo \
     -e RUSTUP_HOME=/home/gary/.rustup -e CARGO_HOME=/home/gary/.cargo \
     -w /work docker.io/emscripten/emsdk:latest \
     bash /work/tools/build-wasm-emsdk.sh
   ```
   (or `tools/package-web.sh` inside the container if the staging/probe is
   needed). Fix any further wasm-only errors, but keep the host gate green.
2. Stage the `_WEB` (page + `tinyhexagon.js/.wasm/.data`) and run the 08a
   probe against it (the 03c `web-stub-probe.mjs` was extended; use its
   real-game assertions): canvas, title, `ohStatus`, no console errors,
   input after a gesture, screenshot. No IDBFS/localStorage writes.
3. Host gate again; the 07 desktop tests must stay green.
4. Write `.agents/tasks/plan-002/issue-008a-report.md` (shared entry, shell
   audio unlock, build/probe evidence, screenshot path) and FEED-FORWARD
   **§08a only**. No Android/packaging (08b); no commit.

## Constraints

- Do not re-architect the 08a code; fix only the wasm build/probe.
- No persistence/text in the shell; keep the first-gesture audio unlock.
- The orchestrator will push and run the live gh-pages probe.
