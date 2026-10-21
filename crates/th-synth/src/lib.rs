//! The tiny runtime's live soundtrack (plan 002 issue 05a).
//!
//! Wraps `rust-synth 0.23` (MIT modular ambient synth; only its `audio` +
//! `math` modules are used) with a fixed set of 8 pre-allocated tracks
//! (the engine asserts `MAX_TRACKS = 8`), driven by [`MusicDirector`]:
//!
//! * **stage map** — stage → bpm / brightness / root-fifths / per-slot
//!   unlocks (05-synth.md "Track map": 8 slots, fixed `PresetKind` each);
//! * **[`BeatView`]** — the beat/bar/energy frame the visuals consume;
//! * **collision ducking** — `bump()` ducks the master to ~0.22, recovers
//!   over 0.6 s;
//! * **no-device fallback** — `start()` never panics without a device;
//!   `tick` advances a fallback clock so `BeatView` keeps advancing;
//! * **web boot (the one-mixer plan, 09)** — on emscripten the engine
//!   starts lazily on the first user gesture (`start_on_gesture`,
//!   cfg-gated to emscripten) as the IN-MEMORY engine
//!   (`AudioEngine::start_in_memory` — no cpal stream, no private WebAudio
//!   context): the frame timer's [`MusicDirector::render_stereo`] pass
//!   feeds the SFX device's OpenAL context as a streaming source, so the
//!   SFX and the music share ONE context and ONE gesture unlock. The
//!   phase clock the `BeatView` consumes is driven by that render pass.
//!   Desktop/Android start at boot on the cpal device stream.
//!
//! Stage changes mutate `Shared` params only — never `rebuild_graph` — so
//! a stage change is click-free. SFX stay in `th-audio`; this crate never
//! plays one-shots.
//!
//! # ALSA coexistence (the 05a blocker)
//!
//! `rust-synth` pulls cpal 0.15 → alsa 0.9 → `alsa-sys 0.3.1`, while
//! rodio 0.23 (th-render/th-audio) pulls cpal 0.18 → alsa 0.11 →
//! `alsa-sys 0.4.0`; both declared `links = "alsa"`. The vendored
//! `vendor/alsa-sys-0.3.1` (registry copy with `links = "alsa_0_3"`),
//! patched in by the root `[patch.crates-io]`, lets both versions
//! coexist. The Android/wasm target checks are issue 05b.
//!
//! Skills: `.agents/skills/lib-rust-synth.md` (crate API/presets/clock)
//! and `.agents/skills/lib-cpal.md` (cpal 0.15.3 hosts + autoplay).

pub mod beat;
pub mod director;

pub use beat::BeatView;
pub use director::MusicDirector;
/// The in-memory engine's fixed render rate (the one-mixer plan's web
/// half — the OpenAL streaming buffers are labeled at this rate; the
/// re-export keeps the th-app web frame timer from naming the engine
/// crate directly).
pub use rust_synth::audio::engine::IN_MEMORY_SAMPLE_RATE;
