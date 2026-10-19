# Issue 05 — `th-synth`: cross-target spike + music director

**Status:** PARTIAL — 05a LANDED; 05b pending
**Depends:** 03
**Unblocks:** 07 (music integration)

## Objective

Generate the soundtrack with `rust-synth` on Linux, Windows, Android and
emscripten, with a stage mapping, a beat view for the visuals, collision
ducking, and a no-device fallback. The cross-target spike comes first: do not
build the game on the synth until it compiles on Android and wasm.

## Key decisions

- Crate: `rust-synth = "0.23.0"` (MIT). It is a lib + TUI app; we use only its
  `audio` and `math` modules.
- Exactly 8 tracks (the crate asserts `MAX_TRACKS = 8`).
- Fixed preset kinds per slot; stage changes only mutate `Shared` parameters
  (no `rebuild_graph` on every stage — no clicks).
- The beat clock is `EngineHandle.phase_clock` when the stream is live;
  otherwise a fallback clock advanced by `tick(dt)`.
- Web starts the synth lazily on the first user gesture (autoplay policy);
  desktop/Android start at boot. The fallback clock runs until then.
- SFX are handled by `th-audio` in the app; `th-synth` does not play SFX.

## Cross-target spike (gate for the issue)

Before implementing the director:

```sh
cargo check -p th-synth                         # host
cargo check -p th-synth --target aarch64-linux-android
cargo check -p th-synth --target wasm32-unknown-emscripten
```

Verified support: `cpal 0.15.3` has `src/host/emscripten` and
`src/host/oboe` (Android). `rust-synth` also compiles its TUI/recording
modules (`ratatui`, `crossterm`, `flacenc`, `vorbis_rs`, `tracing-subscriber`)
unconditionally; these are the risk.

If the spike fails:

- Vendor a patched `rust-synth` (e.g. `vendor/rust-synth` or
  `th-synth/rust-synth`) with the `tui`/`recording` modules gated behind
  features (or removed) and only `audio`/`math` built. Keep the DSP/presets
  unchanged.
- Use `[patch.crates-io]` or a path dependency; document the patch in
  `.agents/skills/lib-rust-synth.md` and the feed-forward.
- Do **not** silently substitute a different synth or skip music on a target.

## Director API

```rust
pub struct BeatView {
    pub beat_phase: f32,   // 0..1 within the beat
    pub bar_phase: f32,    // 0..1 within 4 beats
    pub energy: f32,       // 0..1 from peak_l/peak_r
    pub stage: u32,
    pub playing: bool,
}

pub struct MusicDirector { /* EngineHandle or fallback */ }
impl MusicDirector {
    pub fn start() -> Self;              // attempts audio; Err => fallback
    pub fn set_stage(&mut self, stage: u32);
    pub fn bump(&mut self);              // collision duck
    pub fn tick(&mut self, dt: f32) -> BeatView;
    pub fn stop(&mut self);
}
```

## Track map

| Slot | PresetKind | Role | Unlock |
|---|---|---|---|
| 0 | PadZimmer | pad | stage 0 |
| 1 | DroneSub | sub drone | stage 0 |
| 2 | PluckSaw | pluck | stage 2 |
| 3 | Bell | bell | stage 5 |
| 4 | BassPulse | bass | stage 8 |
| 5 | Shimmer | shimmer | stage 11 |
| 6 | Heartbeat | soft kick | stage 14 |
| 7 | SuperSaw | lead/saw | stage 17 |

- `scale_mode = 0` (major pentatonic); root A2 = 110 Hz; simple ratios.
- `bpm = 70 + 1.5 * min(stage, 20)` (cap 100).
- `brightness = 0.45 + 0.02 * min(stage, 20)` (cap ~0.85).
- `master_gain ≈ 0.55`; duck to ~0.22 on a bump and recover over 0.6 s.
- Every 3 stages shift the root up a fourth (or another simple interval); do
  not shift more than the stage plan.
- The app owns the stage plan; the director maps stage → parameters.

## Beat view

- `beat_phase = rust_synth::math::pulse::beat_phase(t, bpm)`.
- `bar_phase = rust_synth::math::pulse::phrase_phase(t, bpm, 4)`.
- `energy = clamp(max(peak_l, peak_r) * 3.0, 0, 1)` (smooth if needed).
- `t = phase_clock.value()` when live; otherwise `fallback_t += dt`.
- `playing` is true when the stream is live and not suspended.

## Web/Android

- `cfg(target_os = "emscripten")`: do not start at boot; expose
  `start_on_gesture()` / `resume()` and call it from the first canvas
  pointer/key event. The fallback clock runs until then.
- Android: start at boot (oboe). If the stream fails, fall back silently.
- Desktop: start at boot; if no device, fall back.

## Files

| File | Change |
|---|---|
| `crates/th-synth/Cargo.toml` | New crate; `rust-synth`, no game deps. |
| `crates/th-synth/src/{lib,director,beat}.rs` | API + mapping + tests. |
| workspace `Cargo.toml` | Add member. |
| `.agents/skills/lib-rust-synth.md` | Source-verified skill (API, presets, clock, duck, targets, patch notes). |
| `.agents/skills/lib-cpal.md` | 0.15.3 emscripten/oboe hosts, autoplay, target flags. |
| `.agents/skills/README.md` | Index the new skills. |

## Steps

1. Add the crate and dependency.
2. Run the cross-target spike; if it fails, vendor the patched crate first and
   re-run. Record the decision.
3. Implement `MusicDirector`, 8 tracks, stage map, duck/recover, fallback.
4. Add `start_on_gesture` for web.
5. Tests:
   - stage map monotonic/capped and matches the table;
   - beat/bar phases at known times;
   - duck lowers and recovers;
   - `start()` without a device does not panic; `tick` advances the fallback;
   - 8 tracks always.
6. Write the two skills from the real source.
7. Manual: a 20 s listen on desktop; Android/web after Issue 08.
8. Gate + forked reviewer.

## Verification

- All three `cargo check` targets green (or the vendored patch green).
- `cargo test -p th-synth` green with no audio device.
- Manual desktop listen: sound is audible, calm, and changes at stage
  boundaries; bump ducks.
- No graph rebuild on stage changes (instrument or code review); no clicks.
- The fallback clock keeps `BeatView` advancing with no device.

## Risks

- TUI/recording deps on wasm/Android; the spike is the gate.
- Web autoplay: a suspended AudioContext produces no callbacks; the fallback
  clock and gesture start are required.
- Two cpal versions (0.15 for rust-synth, 0.18 via rodio in `th-render`) —
  they can coexist, but watch build size/link errors.
- Stage changes that rebuild the graph click; only mutate `Shared` params.

## Feed-forward

Record: the spike result (stock vs patched), the exact director API, the
stage/layer table, the web gesture hook, and any target caveats. Issue 07
consumes `BeatView` and `bump`; Issue 08 wires the gesture.
