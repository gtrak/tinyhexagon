# lib-rust-synth — rust-synth 0.23.0 (Rust)

Pinned version: **rust-synth 0.23.0** (MIT), read from the checked-out
registry source (`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/
rust-synth-0.23.0/src/`); the build graph uses the 05b-vendored
`vendor/rust-synth-0.23.0` (root `[patch.crates-io]` — see "wasm feature
patch (05b)" below; DSP/math/audio are byte-identical to the registry
source).

Companion skill: `.agents/skills/lib-cpal.md` (its audio backend, cpal 0.15.3).
Consumer: `crates/th-synth` (plan 002 issue 05a/05b). Only the `audio` +
`math` modules are used; upstream, the TUI/recording modules compile
unconditionally (their deps — `ratatui`, `crossterm`, `flacenc`,
`vorbis_rs`, `tracing-subscriber` — rode along as lib transitive deps;
that was the 05b cross-target risk, resolved by the vendored feature
gate below).

## Engine (`src/audio/engine.rs`)

- `AudioEngine::start(initial_tracks: Vec<Track>) -> anyhow::Result<EngineHandle>` —
  **asserts `initial_tracks.len() == MAX_TRACKS` (`= 8`)**; builds the cpal
  default-host output stream; `Err` (not panic) when there is no default
  output device (`context("no default output audio device")`) or the stream
  fails to build/play.
- `EngineHandle` fields (all `pub` except `graph`/`_stream`):
  - `tracks: Arc<Mutex<Vec<Track>>>` (parking_lot),
  - `global: GlobalParams` (plain field, see below),
  - `peak_l: Shared`, `peak_r: Shared` — the master envelope followers,
  - `sample_rate: f32`,
  - `phase_clock: Shared` — seconds of audio time, set by the callback
    (`t += dt` per frame, `set_value(t as f32)`);
  - `scope: Arc<Mutex<VecDeque<(f32, f32)>>>` (oscilloscope ring, unused here),
  - `recorder: Arc<RecorderState>` (unused here),
  - private `graph: Arc<Mutex<Net>>`, private `_stream: Stream` (dropping the
    handle stops the stream).
- The audio callback (per buffer): locks the graph once, pulls stereo,
  multiplies by `master.value()` **read per buffer** (`Shared` atomic — a
  live master gain is effective on the next buffer), runs the peak
  followers (`env = (env * 0.9995).max(|sample|)`), fills the scope ring,
  updates `peak_l/peak_r/phase_clock` once per callback.
- `EngineHandle::rebuild_graph()` — rebuilds the master `Net` from the
  current tracks + globals; **discards internal DSP state** (reverb tails,
  oscillator phases → a small audible discontinuity). `th-synth` NEVER calls
  it: stage changes mutate `Shared` params only (click-free by contract).
- `build_master` = sum of the 8 `Preset::build(kind, params, global)` graphs
  piped through `master_bus(brightness)`.

## Presets (`src/audio/preset.rs`)

- `PresetKind` — exactly 8 variants: `PadZimmer, DroneSub, Shimmer,
  Heartbeat, BassPulse, Bell, SuperSaw, PluckSaw` (+ `label()`: "Pad",
  "Drone", "Shimmer", "Heartbeat", "Bass", "Bell", "SuperSaw", "Pluck").
- `GlobalParams` (cloned into the build-time `lfo` closures as `Shared`s):
  | field | default |
  |---|---|
  | `bpm: Shared` | 72.0 |
  | `master_gain: Shared` | 0.7 |
  | `brightness: Shared` | 0.6 |
  | `scale_mode: Shared` | 0.0 (0 = major pent, 1 = minor pent, 2 = bhairavi) |
  All `Shared` → mutable live via `handle.global.<f>.set_value(..)` from the
  UI thread (the closures read them per sample, lock-free).
- `master_bus(brightness)` = per-channel **high-shelf** (3.5 kHz, q 0.7;
  shelf gain `0.2 + 0.8·b`) → **lowpass** (cutoff `3000 · 6^b` Hz) →
  `limiter_stereo(0.001, 0.3)`.
- `TrackParams::default_for(freq)` (all `Shared`): gain 0.45, cutoff 1600.0,
  resonance 0.30, detune 7.0, sweep_k 1.2, sweep_center 1.5, reverb_mix 0.6,
  supermass 0.0, pulse_depth 0.0, mute 0.0, freq, life_mod 1.0,
  `pattern_bits: Arc<AtomicU32>` = `euclidean_bits(4, 0)` (4-on-the-floor),
  pattern_hits 4.0, pattern_rotation 0.0, lfo_rate 0.5, lfo_depth 0.0,
  lfo_target 1.0 (CUT, inaudible at depth 0), character 0.5 (neutral), arp 0.0.
- `TrackParams::dormant(freq)` = default with mute 1.0, gain 0.3.
- `Track::new(id, name, kind, freq)` / `Track::dormant(...)` — a `Track` is
  `{ id: usize, name: String, kind: PresetKind, params: TrackParams }`.
  **`Track` is NOT `Clone`; `TrackParams` IS** — and every `TrackParams`
  field is a `Shared` = `Arc<AtomicU32>` (fundsp `hacker::shared`), so a
  cloned `TrackParams` shares the same atomics the DSP reads. This is how
  `th-synth`'s `MusicDirector` drives the live graph: it hands the engine a
  rebuilt `Vec<Track>` whose `params` are clones sharing its own atomics.
- Drum presets read `pattern_bits` per sample (`rhythm::step_is_active`,
  16-step grid, 4 steps/beat); the Heartbeat envelope shapes a pitch-drop
  kick + sub + click (click amplitude `0.02 + 0.20·character`).

## Math (`src/math/`)

- `pulse.rs` (f64 throughout — long-playback phase stability):
  - `beat_phase(t, bpm) = t.rem_euclid(60.0 / bpm.max(1.0)) / period` → 0..1
    within the beat,
  - `phrase_phase(t, bpm, beats) = t.rem_euclid(beats·60/bpm) / period` →
    0..1 within a `beats`-length phrase (`th-synth` uses `beats = 4`),
  - `pulse_decay(t, bpm, decay) = exp(-beat_phase·decay)`,
  - `pulse_sine(t, bpm) = 0.5 − 0.5·cos(2π·beat_phase)`.
- `rhythm.rs`: `euclidean_bits(hits, rotation)` — 16-step bitmask
  (bit `i` = step `i` active; `euclidean_bits(4,0) == 0b0001_0001_0001_0001`);
  `step_is_active(bits, t, bpm) -> (bool, phi)`.
- `Shared` (fundsp 0.20 `src/shared.rs`): `Arc<AtomicU32>`;
  `set_value/set(&self, f32)`, `value(&self) -> f32` — Relaxed atomics;
  `Clone` shares the cell. `fundsp::hacker::shared(v)` constructs.

## Duck approach (th-synth)

Collision ducking = writing `GlobalParams.master_gain` (`Shared`) from the
UI thread: the per-buffer `master.value()` read makes a `bump()` effective
on the next buffer with zero graph surgery. `th-synth` ducks 0.55 → 0.22
and recovers linearly over 0.6 s (`crates/th-synth/src/director.rs`).

## ALSA coexistence patch (05a)

`rust-synth` → cpal 0.15.3 → alsa 0.9 → `alsa-sys ^0.3.1`; rodio 0.23
(th-render/th-audio) → cpal 0.18.2 → alsa 0.11 → `alsa-sys 0.4.0`. Both
declare `links = "alsa"` → Cargo refuses the graph. Fix (host-verified in
05a; the whole-workspace gate proves coexistence with rodio):
`vendor/alsa-sys-0.3.1/` (registry cache copy, **only** change:
`links = "alsa_0_3"`; the 0.4.0 registry crate keeps `links = "alsa"`)
wired by the root `[patch.crates-io] alsa-sys = { path = "vendor/alsa-sys-0.3.1",
version = "0.3.1" }` (root `Cargo.toml`). Lock: `alsa-sys 0.3.1` (path, no
checksum) + `alsa-sys 0.4.0` (registry) side by side.

## Targets

- **host (Linux x86_64)**: VERIFIED in 05a — `cargo check`/`cargo test -p
  th-synth` + the whole-workspace gate green; cpal 0.15 default host = ALSA
  (build script probes `pkg-config alsa`; no device → `AudioEngine::start`
  returns `Err`, the director falls back silently). Re-verified in 05b
  after the vendor patches (host gate re-run green).
- **aarch64-linux-android (oboe)**: VERIFIED in 05b — `cargo check -p
  th-synth --target aarch64-linux-android` GREEN, first on STOCK registry
  rust-synth (the whole TUI/recording dep set — ratatui 0.29, crossterm
  0.28.1, flacenc, hound, vorbis_rs 0.5.6 + symphonia, tracing-subscriber
  — is in the android graph and checks clean; the wasm-only breakage is in
  crossterm's mio + vorbis's C build, not in the crates themselves). The
  patched (feature-gated) graph is green too. `oboe-sys 0.6.1` compiles
  the bundled oboe C++ from source with the NDK clang++ from
  `tools/android-env.sh` (no prebuilt fetch — `fetch-prebuilt` is off);
  AAudio is the runtime API on our minSdk-28 devices (the oboe 0.6.1
  source logic — see `lib-cpal.md` "Oboe host").
- **wasm32-unknown-emscripten**: VERIFIED in 05b — STOCK registry
  rust-synth FAILS (`cargo check` breaks on `mio 1.2.4`, pulled by
  crossterm 0.28.1: "This wasm target is unsupported by mio"; and on
  `ogg_next_sys 0.1.5`'s build script via vorbis_rs 0.5.6: it needs a C
  compiler, emcc, which `check` does not provide). The 05b vendor patch
  (below) + the cpal 0.15.3 emscripten ABI patch (`lib-cpal.md` "05b
  emscripten ABI patch") make `cargo check -p th-synth --target
  wasm32-unknown-emscripten` GREEN. Boot stays pre-gesture —
  `th-synth`'s `start_on_gesture` is cfg-gated to emscripten for exactly
  this (cpal's emscripten `Stream::play()` = `AudioContext.resume()` whose
  rejected promise panics).

## wasm feature patch (05b)

`vendor/rust-synth-0.23.0/` — registry cache copy (upstream git
12b25e0d881381ec7e5db60db239f98a4997894e; LICENSE in; registry-extraction
artifacts + the author's `Cargo.toml.orig`/`Cargo.lock` dropped). Changes,
feature-wiring only — `src/audio` (DSP/presets/engine) is byte-identical
except the 5 `#[cfg(feature = "recording")]` touch points on the recorder
plumbing, `src/math` untouched:

- `Cargo.toml`: `[features]` — `default = []`; `recording =
  ["dep:flacenc", "dep:vorbis_rs", "dep:hound"]`; `tui = ["dep:ratatui",
  "dep:crossterm", "dep:tracing-subscriber", "recording"]` (the TUI's
  record controls reference `crate::recording`, so `tui` implies
  `recording`); those six deps are now `optional`; both `[[bin]]`s
  (`rust-synth` → `required-features = ["tui"]`, `rust-synth-render` →
  `["recording"]`).
- `src/lib.rs`: `pub mod recording` / `pub mod tui` are
  `#[cfg(feature = …)]`-gated; `audio`/`math`/`persistence` stay
  unconditional (`persistence` = pure-Rust TOML preset I/O — builds on
  every target).
- `src/audio/engine.rs`: the 5 `RecorderState` touch points are
  `#[cfg(feature = "recording")]`-gated — the import, the
  `EngineHandle.recorder` field, the `RecorderState::new` local, the
  `start_stream` parameter + call argument, and the per-frame
  `recorder.push_frame(l, r)` in the callback. With the feature off the
  audio path is byte-identical in behaviour (the recorder is a no-op
  sink anyway — `th-synth` never records); with it on, everything is as
  upstream.

`th-synth` depends on `rust-synth = "0.23.0"` with the default (no)
features, so every target builds the audio-only graph. Music is NOT
substituted or dropped on any target (the 05 contract). `diff -rq` against
the registry source shows exactly: `Cargo.toml`, `src/lib.rs`,
`src/audio/engine.rs` (+ the dropped artifact files).

## Pitfalls

- The `MAX_TRACKS == 8` assert panics (not `Err`) on the wrong track count.
- `Stream` is `!Send + !Sync` (cpal 0.15.3 marker) → `EngineHandle` and
  `MusicDirector` are single-thread objects (keep on the app's main thread).
- `phase_clock` is f32 seconds: exact enough for the beat view (the f64
  phase math happens after the `as f64` cast).
- A suspended `AudioContext` (web, hidden tab) stops the callback:
  `phase_clock`/peaks freeze — `th-synth` reports `playing` as "live" (cpal
  0.15.3 exposes no suspend query on `Stream`); issue 08's gesture handler
  must call `resume()`/`start_on_gesture()`.
