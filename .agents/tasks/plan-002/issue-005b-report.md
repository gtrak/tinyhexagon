# Issue 05b report — `th-synth` Android/wasm cross-target spike

Cross-target implementation of plan 002 issue 05
(`.agents/plans/002-tinyhexagon/05-synth.md`) per the task spec
`.agents/tasks/plan-002/issue-005b-impl.md`. Worktree `slot-1` (branch
`slot-1`, base `9fbc42f1`). **No commit; no app/07 work.**

## Spike results

### 1. Android — `cargo check -p th-synth --target aarch64-linux-android`

**GREEN on STOCK `rust-synth 0.23.0`** (before any patching), and still
green after the wasm patches. Environment: `. tools/android-env.sh`
(NDK 27.2.12479018, API 28, `aarch64-linux-android`).

- The entire TUI/recording dep set is in the aarch64 graph and checks
  clean: `ratatui 0.29.0`, `crossterm 0.28.1` (+ its `mio 1.2.4` — mio is
  the wasm-only breakage, fine on Android), `flacenc 0.5.1`, `hound
  3.5.1`, `vorbis_rs 0.5.6` (+ `symphonia`, `ogg_next_sys`'s C build
  script ran under the NDK clang), `tracing-subscriber 0.3.23`.
- **oboe-sys 0.6.1: compiled the oboe C++ FROM SOURCE with the NDK.**
  Its `build.rs` default path (`fetch-prebuilt` feature is OFF) drives
  `cc::Build` over 55 bundled `oboe/src/` files + 3 `oboe-ext` wrappers
  (`-std=c++17 -fno-rtti -fno-exceptions`, static, `c++_static`), using
  the NDK `aarch64-linux-android28-clang++` exported by
  `tools/android-env.sh` (build log: `CXX_aarch64_linux_android =
  .../ndk/27.2.12479018/.../aarch64-linux-android28-clang++` + the
  expected `oboe/src/...` C++ warnings). No GitHub prebuilt fetch.
  Link line: `static=oboe-ext` + `log` + `OpenSLES`.
- **AAudio vs OpenSL (the 05a `TODO(05b)` in `lib-cpal.md`, answered
  from source):** cpal 0.15.3's oboe host builds every stream from
  `AudioStreamBuilder::default()` — `AudioApi::Unspecified`, no
  `set_audio_api` call in `src/host/oboe/mod.rs`. oboe 0.6.1
  (`audio_stream_builder.rs:463-466`) then uses AAudio iff
  `is_aaudio_recommended()`, and the bundled C++
  (`oboe-sys-0.6.1/oboe/src/common/AudioStreamBuilder.cpp:64`) defines
  `isAAudioRecommended = (getSdkVersion() >= __ANDROID_API_O_MR1__) &&
  isAAudioSupported()` — AAudio on API ≥ 27, OpenSL ES below. Our
  minSdk is 28 (feed-forward §02a) ⇒ **AAudio on every supported
  device**; OpenSL ES is only the sub-27 fallback. Compile-time/source
  evidence — the on-device confirmation is issue 08's device smoke.
- "No device" on Android still surfaces as a stream build/play error
  (cpal oboe's `default_output_device()` is always `Some`), so the
  05a-verified silent fallback path applies unchanged.

### 2. wasm — `cargo check -p th-synth --target wasm32-unknown-emscripten`

**RED on stock → GREEN patched.** Two successive blockers:

**Blocker 1 (the plan-05 risk, exactly as predicted): rust-synth's
unconditional TUI/recording deps.**

```
error: This wasm target is unsupported by mio. If using Tokio, disable the net feature.
error[E0583]: file not found for module `selector` / `waker`   (mio 1.2.4)
error: failed to run custom build command for `ogg_next_sys v0.1.5`
  (cc: ToolNotFound: failed to find tool "emcc")
```

`mio 1.2.4` comes from `crossterm 0.28.1`; `ogg_next_sys 0.1.5` from
`vorbis_rs 0.5.6` (its build script compiles vendored ogg C and needs a
C compiler — emcc is absent for `check`). `ratatui`/`flacenc`/`hound`/
`tracing-subscriber` themselves would have checked; the failure is in
the two platform-hostile transitive crates.

**Fix (the 05 contract's spike-failure path): vendor a feature-gated
rust-synth.** `vendor/rust-synth-0.23.0/` — registry cache copy
(upstream git `12b25e0d881381ec7e5db60db239f98a4997894e`; LICENSE in;
`.cargo-ok`/`.cargo_vcs_info.json` + the author's `Cargo.toml.orig`/
`Cargo.lock` dropped). Changes, feature-wiring only — DSP/presets
(`src/audio`'s math) byte-identical, `src/math` untouched:

- `Cargo.toml`: `[features]` — `default = []`; `recording =
  ["dep:flacenc", "dep:vorbis_rs", "dep:hound"]`; `tui = ["dep:ratatui",
  "dep:crossterm", "dep:tracing-subscriber", "recording"]` (the TUI's
  record controls reference `crate::recording`, so `tui` implies
  `recording`); those six deps become `optional`; `[[bin]] rust-synth`
  gains `required-features = ["tui"]`, `[[bin]] rust-synth-render`
  gains `required-features = ["recording"]`.
- `src/lib.rs`: `pub mod recording` / `pub mod tui` cfg-gated;
  `audio`/`math`/`persistence` stay unconditional (`persistence` is
  pure-Rust TOML preset I/O — builds on every target; part of the
  preset surface the contract keeps).
- `src/audio/engine.rs`: the 5 `RecorderState` touch points are
  `#[cfg(feature = "recording")]`-gated — the import, the
  `EngineHandle.recorder` field, the `RecorderState::new` local, the
  `start_stream` parameter + call argument, and the per-frame
  `recorder.push_frame(l, r)` in the callback. Feature off ⇒ audio path
  byte-identical in behaviour (the recorder is a no-op sink; `th-synth`
  never records). Feature on ⇒ upstream behaviour.

Wired by the root `Cargo.toml`:
`[patch.crates-io] rust-synth = { path = "vendor/rust-synth-0.23.0",
version = "0.23.0" }`. `crates/th-synth/Cargo.toml` keeps
`rust-synth = "0.23.0"` (default features ⇒ audio-only graph on every
target). **No substitution, no dropped music** — the same engine builds
on all four targets.

**Blocker 2 (NEW — not in plan 05's risk list): cpal 0.15.3's own
emscripten host vs wasm-bindgen 0.2.129.** With blocker 1 cleared,
`cargo check` then failed inside `cpal 0.15.3` itself (3 errors, all in
`cpal/src/lib.rs` around `BufferSize`):

```
error[E0277]: the trait bound `BufferSize: FromWasmAbi` is not satisfied
  (the `#[wasm_bindgen]` derive on `StreamConfig`, field `buffer_size`)
error[E0308]: mismatched types — the hand-written
  `impl IntoWasmAbi for BufferSize`'s `match { … }.into_abi()` no longer
  type-checks (expected `Option<u32>`, found `f64`)
```

cpal 0.15.3 hand-wrote only `WasmDescribe` + `IntoWasmAbi` for
`BufferSize`; the newer wasm-bindgen derive on `StreamConfig` requires
BOTH Abi directions on every pub field, and the `Option<u32>::into_abi()`
routing died with the API drift. Constraints checked before deciding:
**cpal 0.15.3 is the LAST 0.15.x release** (crates.io index: 0.15.0–
0.15.3 only, none yanked — no registry bump exists), and downgrading
wasm-bindgen to the cpal-era 0.2.8x would break `th-web`'s documented
`wasm-bindgen = "0.2.129"` pin that matches the installed
`wasm-bindgen` CLI 0.2.129 (the 03c web recipe calls version drift
build-breaking). **Escalated to the supervisor (need_decision); ruling
A:** vendor/patch cpal surgically, emscripten ABI layer only.

**Fix (ruling A): `vendor/cpal-0.15.3/`** — registry cache copy
(`.cargo-ok`/`.cargo_vcs_info.json`/author `Cargo.lock` dropped;
LICENSE in) with exactly two changes, both
`#[cfg(target_os = "emscripten")]`-gated in `src/lib.rs`:

1. the missing `impl FromWasmAbi for BufferSize` — `type Abi =
   Option<u32>`, `None ⇔ Default`, `Some(fc) ⇔ Fixed(fc)` (the exact
   mirror of the pre-existing `IntoWasmAbi` encoding);
2. `IntoWasmAbi::into_abi` drops the trailing `.into_abi()` — the match
   already yields `Option<u32>` (identical encoding).

No non-emscripten host, no API/DSP surface touched. `diff -r` against
the registry source shows only the `Cargo.toml` provenance header +
those two `lib.rs` blocks. Wired by the root `Cargo.toml`:
`[patch.crates-io] cpal = { path = "vendor/cpal-0.15.3", version =
"0.15.3" }` — the version pin limits the patch to the 0.15.3 line; the
**0.18.2 registry cpal rodio pulls stays stock** (same scheme as the
05a alsa-sys 0.3.x line patch). Residual, per the surgical ruling: two
UPSTREAM dead-code warnings from the vendored copy's emscripten host
(`StreamId` never constructed; an unused `#[macro_use]` import) — left
in place (harmless; path-dep warnings don't reach the workspace gate,
which lints only workspace members on the host target).

**Post-patch result: `cargo check -p th-synth --target
wasm32-unknown-emscripten` GREEN** (cpal emscripten host + the
audio-only rust-synth graph compile clean; the only warnings are the
two upstream cpal ones above).

**One in-crate follow-up (th-synth, not the vendor layer):**
`MusicDirector::start()` (05a code) had a cfg-emscripten bare `return`
making the rest of the fn unreachable — a warning that only appears now
the emscripten target actually compiles. Re-cfg'd into two
mutually-exclusive blocks (`crates/th-synth/src/director.rs`) — same
calls, same signature, same behaviour on every target; the warning is
gone and the 05a API is untouched.

## Lock / graph

`Cargo.lock`: `rust-synth 0.23.0` and `cpal 0.15.3` now resolve to the
two path entries (no checksums, like the 05a alsa-sys path line); the
TUI/recording subtree (`ratatui`, `crossterm` + `mio`, `flacenc`,
`vorbis_rs` + `symphonia`/`ogg_next_sys`, `hound`,
`tracing-subscriber`, and their exclusive transitives) dropped out of
the graph. `cpal 0.18.2` / `rodio` / the two alsa lines are unchanged.

## Verification (all re-run after the vendor patches)

```
cargo check -p th-synth --target aarch64-linux-android   → GREEN (rc 0; also GREEN pre-patch on stock)
cargo check -p th-synth --target wasm32-unknown-emscripten → GREEN (rc 0; RED on stock, both blockers documented above)
cargo check -p th-synth                                  → GREEN
cargo test  -p th-synth                                  → 12 passed / 0 failed (no audio device → fallback path, as 05a)
bash tools/gate.sh                                       → 4/4 GATE_EXIT=0
```

Gate detail: `cargo fmt --all -- --check` 0 / `cargo build
--workspace` 0 / `cargo clippy --workspace --all-targets -- -D
warnings` 0 / `cargo test --workspace` 0 — **552 passed / 0 failed**
(551 at 05a + 06's `tiny_pack` test; the device-less host's expected
`th-synth: live audio start failed …` fallback log line is not a
failure).

## Skills / feed-forward

- `.agents/skills/lib-rust-synth.md` — both `TODO(05b)` target markers
  filled (android: stock-green + oboe C++ source-compile + AAudio;
  wasm: stock-red + patched green) + new "wasm feature patch (05b)"
  section (exact vendor changes, wiring, the `diff -rq` surface).
- `.agents/skills/lib-cpal.md` — the oboe `TODO(05b)` replaced with the
  AAudio-vs-OpenSL source answer + the oboe-sys prebuilt/compile answer;
  new "05b emscripten ABI patch" section (blocker, the two-impl patch,
  wiring, the residual warnings); the emscripten-host section notes the
  05b green.
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` — §05b filled
  (stock-vs-patched, both patch locations, director-API-unchanged note,
  target caveats for 08); the plan-002 §07/§08 hand-off bullets about
  the pending listen remain 07/08's.
- Plan file `05-synth.md` status line left for the orchestrator (this
  issue does not commit).

## Changed files

- `vendor/rust-synth-0.23.0/` (new; registry copy + the feature gate:
  `Cargo.toml`, `src/lib.rs`, `src/audio/engine.rs`)
- `vendor/cpal-0.15.3/` (new; registry copy + the 2-line-class
  emscripten ABI patch in `src/lib.rs` + `Cargo.toml` provenance)
- `Cargo.toml` (root: two new `[patch.crates-io]` lines + comments)
- `Cargo.lock` (both crates → path; TUI/recording subtree dropped)
- `crates/th-synth/Cargo.toml` (dep comment — the "ride along" wording
  replaced with the patch reference; no version/feature change)
- `crates/th-synth/src/director.rs` (`start()` cfg restructure —
  behaviour-identical, kills the emscripten-only warning)
- `.agents/skills/lib-rust-synth.md`, `.agents/skills/lib-cpal.md`
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` (§05b)
- `.agents/tasks/plan-002/issue-005b-report.md` (this file)

## Residual risks / open items

- **No runtime audio on any target was exercised** (no device / no
  browser here) — 05b is a compile-spike; AAudio selection, the
  emscripten `AudioContext` gesture flow, and the 20 s listen are 07/08
  items. `BeatView.playing` still means "live", not "un-suspended"
  (08 must re-fire the gesture hook — unchanged from 05a).
- The vendored cpal's 2 upstream emscripten-host warnings are expected
  noise (documented in both skills + feed-forward).
- The `rust-synth-render` / `rust-synth` bins stay feature-gated in the
  vendor copy — irrelevant to `th-synth` (lib consumer), but any future
  use of the TUI/recording API needs `rust-synth = { version, features =
  ["tui"/"recording"] }` + awareness of the 05b cfg seams.
- Windows cross target was NOT part of 05b's mandate (the contract
  names host/Android/wasm); the audio-only graph is a strict subset of
  the host graph, so windows-gnu is lower-risk, not re-verified.
