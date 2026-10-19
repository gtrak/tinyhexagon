# Issue 05b — `th-synth` Android/wasm cross-target spike

**Worktree:** `.agents/worktrees/slot-1` (branch slot-1, base `9fbc42f1`).
Main and slot-2 are OFF-LIMITS. **Do not commit.**

**Read:** `.agents/plans/002-tinyhexagon/05-synth.md` (contract),
`.agents/tasks/plan-002/issue-005a-report.md` (host spike/patch),
`.agents/plans/002-tinyhexagon/FEED-FORWARD.md` §05a/§05b.
This is the 05b target spike only: no app/07 work.

## Steps

1. **Android** (the host has the NDK at `~/Android/Sdk/ndk/27.2.12479018`):
   ```sh
   . tools/android-env.sh     # PATH + CC/CXX/AR/RANLIB for aarch64-linux-android
   cargo check -p th-synth --target aarch64-linux-android
   ```
   `oboe-sys 0.6.1` may fetch its prebuilt from GitHub or compile the oboe C++
   with the NDK; record which. Also confirm the `oboe` host (AAudio vs OpenSL)
   and fill the `TODO(05b)` markers in `.agents/skills/lib-cpal.md`.
2. **wasm/emscripten**:
   ```sh
   cargo check -p th-synth --target wasm32-unknown-emscripten
   ```
   No emcc is needed for `check` (no link), but the `.cargo/config.toml`
   target flags apply. The risk is `rust-synth`'s UNCONDITIONAL TUI/recording
   deps (`ratatui`, `crossterm`, `flacenc`, `vorbis_rs`,
   `tracing-subscriber`) on wasm. If they fail, vendor/patch `rust-synth`
   behind features per the 05 contract (DSP/presets unchanged), wire it via
   path/[patch], and re-run. Do NOT substitute another synth or drop music.
   Record the exact failure + patch in the report and
   `.agents/skills/lib-rust-synth.md` (fill its `TODO(05b)`).
3. Re-run the host `cargo check -p th-synth` and `cargo test -p th-synth`
   after any vendor patch, plus `bash tools/gate.sh`.
4. Do NOT touch the app/runtime (07), Android/web entries (08), or the 05a
   host API.

## Verification

The two target `cargo check`s green (stock or patched) + the host gate.
If a target genuinely cannot build, report the exact blocker and the patch
attempt; do not claim green.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-005b-report.md`: both target results,
stock vs patched, the oboe prebuilt/compile, the wasm dependency outcome,
skill `TODO(05b)` fills, gate. Update FEED-FORWARD **§05b only**; no commit.
