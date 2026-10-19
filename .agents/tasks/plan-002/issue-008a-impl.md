# Issue 08a — real web (emscripten) tinyhexagon entry

**Worktree:** `.agents/worktrees/slot-2` (branch slot-2, base `c9bf8d23`).
Main/slot-1 off-limits. **Do not commit.** One squashed commit after review.

**Read:** `.agents/plans/002-tinyhexagon/08-android-web-packaging.md` (the web
half), `.agents/tasks/plan-002/issue-007-report.md`, FEED-FORWARD §07/§08,
the 05b report/skills, and the 03c web page/probe.

## Goal

Run the **real** desktop `tinyhexagon` loop in the browser, reusing the 07
runtime and the 05a/05b synth, with the 03c build/probe pipeline. Android and
the packaging/CI renames are 08b.

## Steps

1. **Make the 07 runtime target-agnostic.**
   - Extract the shared journey/loop/composition into a `th_app::tiny` entry
     (`pub fn run(...)` / `tiny_web()`) that takes the platform window; the
     desktop `run_windowed` becomes a thin caller. No desktop-only calls
     (filesystem paths, `xvfb`, desktop mode probe) in the shared loop.
   - Reuse the 07 `TinyRuntime`/journey/touch/icons/DrawOptions/SFX/MusicDirector;
     do not duplicate the game logic in `th-web`.
2. **`th-web` entry.**
   - Target-emscripten deps: `th-app`, `th-game`, `wasm-bindgen`, `js-sys`,
     `web-sys` (the canvas/window; the existing `th-render` emscripten layer
     owns the canvas/GL/input).
   - `src/main.rs`: `#[cfg(target_os="emscripten")]` boot the shared tiny
     runtime (the 07 composition over `_RELEASE`); host stays a no-op.
   - Export `#[wasm_bindgen] pub fn resume_audio()` (or `resumeAudio` naming
     consistent with the shell) that calls the `MusicDirector`'s
     `start_on_gesture`/`resume` and the `th-audio` resume; idempotent.
     Keep the 03c `status()` probe (now the real game's status hook/log).
3. **Shell (`crates/th-web/site/index.html`).**
   - Keep the 03c minimal page (canvas, `#status`, module import) and the
     fullscreen button; add the **first-gesture audio unlock**: the first
     `pointerdown`/`keydown` calls the exported resume once (idempotent).
   - No IME/`#ime`, no text entry, no IDBFS/localStorage/persistence.
   - Keep a minimal `ohStatus`/log hook for the probes; the 05b
     `BeatView`/audio fallback runs until the gesture.
4. **Build/probe.**
   - `tools/package-web.sh` builds `th-web` and stages `tinyhexagon.js/.wasm/
     .data` + the page; keep the emscripten flags/preload.
   - Update the 03c `tools/web-stub-probe.mjs` (or add the full probe) to
     assert the real game: canvas, title, `ohStatus`, no console errors, and
     gameplay after a key/mouse/touch input; capture a screenshot. The
     `pages` deploy must be gated on it.
   - Build via the pinned emsdk podman recipe (as 03c did) if emcc is not on
     the host; run the probe against the staged `_WEB`; the orchestrator will
     push and run the live probe.
5. **Tests:** the 07 headless tests stay green; no host regression. If the
   shared-loop refactor needs a small test, add it.

## Constraints

- Desktop behavior unchanged (the 07 tests still pass).
- No Android entry, no packaging script/CI artifact renames (08b).
- No persistence/text; the 08 web page must not mount IDBFS.
- Do not substitute/drop the synth; the 05b vendored patches are the truth.

## Verification

```sh
cargo test -p th-app
bash tools/gate.sh
bash tools/package-web.sh          # or the emsdk container recipe
node tools/web-probe.mjs http://127.0.0.1:8080/ 30   # or the 03c probe
```

The staged page must show the real game canvas and respond to input; the
first gesture must start/resume audio; no persistence writes; the desktop
suite stays green.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-008a-report.md` (shared entry, shell
audio-unlock export, probe/build evidence, screenshot, no commit). Update
FEED-FORWARD **§08a only**; hand the Android/packaging notes to 08b.
