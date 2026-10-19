//! The `MusicDirector` (05-synth.md "Director API"): the app-facing surface
//! over the `rust-synth` engine.
//!
//! The director owns the 8 fixed tracks and, while live, the
//! [`EngineHandle`]. The engine takes its `Vec<Track>` by value; the
//! `TrackParams` we hand it are clones whose `Shared` fields are
//! `Arc<AtomicU32>` — so the live DSP graph reads the *same* atomics the
//! director mutates here: `set_stage`/`bump`/`tick` only write `Shared`
//! params and never touch the graph (no `rebuild_graph`, no clicks on
//! stage changes).

use rust_synth::audio::{AudioEngine, EngineHandle, PresetKind, Track};
use rust_synth::math::rhythm::euclidean_bits;
use std::sync::atomic::Ordering;

use crate::beat::{bar_phase, beat_phase, energy, BeatView};

/// One row of the fixed 8-slot track map (05-synth.md "Track map").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// The preset kind fixed to this slot.
    pub kind: PresetKind,
    /// The stage at which this slot un-mutes.
    pub unlock: u32,
    /// The slot's simple ratio to the stage root (root A2 = 110 Hz at
    /// stage 0): 1.0 = root, 0.5 = sub-octave, 1.5 = fifth, 2.0 = octave.
    pub ratio: f32,
}

/// The 8 slots in engine order (the engine asserts exactly
/// `MAX_TRACKS = 8` tracks; see the "always 8 tracks" test).
pub const SLOTS: [Slot; 8] = [
    Slot {
        kind: PresetKind::PadZimmer,
        unlock: 0,
        ratio: 1.0,
    }, // pad
    Slot {
        kind: PresetKind::DroneSub,
        unlock: 0,
        ratio: 0.5,
    }, // sub drone
    Slot {
        kind: PresetKind::PluckSaw,
        unlock: 2,
        ratio: 1.5,
    }, // pluck
    Slot {
        kind: PresetKind::Bell,
        unlock: 5,
        ratio: 2.0,
    }, // bell
    Slot {
        kind: PresetKind::BassPulse,
        unlock: 8,
        ratio: 0.5,
    }, // bass
    Slot {
        kind: PresetKind::Shimmer,
        unlock: 11,
        ratio: 2.0,
    }, // shimmer
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 14,
        ratio: 0.5,
    }, // soft kick
    Slot {
        kind: PresetKind::SuperSaw,
        unlock: 17,
        ratio: 1.0,
    }, // lead/saw
];

/// Stage-0 root: A2 (05-synth.md "Track map").
pub const ROOT_A2_HZ: f32 = 110.0;
/// The root shift per 3-stage block: up a fourth (05-synth.md "shift the
/// root up a fourth every 3 stages").
pub const ROOT_SHIFT: f32 = 4.0 / 3.0;
/// Stage cap of the bpm/brightness/root ramps (05-synth.md "do not shift
/// more than the stage plan" — the app's stage plan caps at 20 here).
pub const STAGE_RAMP_CAP: u32 = 20;
/// `bpm = 70 + 1.5 * min(stage, 20)` → 100 at the cap.
pub const BPM_BASE: f32 = 70.0;
pub const BPM_PER_STAGE: f32 = 1.5;
/// `brightness = 0.45 + 0.02 * min(stage, 20)` → 0.85 at the cap.
pub const BRIGHTNESS_BASE: f32 = 0.45;
pub const BRIGHTNESS_PER_STAGE: f32 = 0.02;
/// The master gain the director drives (05-synth.md "master_gain ≈ 0.55").
pub const MASTER_GAIN: f32 = 0.55;
/// The collision-duck target (05-synth.md "duck to ~0.22 on a bump").
pub const DUCK_GAIN: f32 = 0.22;
/// The duck recovery time (05-synth.md "recover over 0.6 s").
pub const DUCK_RECOVER_S: f32 = 0.6;
/// Arpeggiator scale mode: 0 = major pentatonic (05-synth.md "scale_mode = 0").
const SCALE_MAJOR_PENT: f32 = 0.0;

/// The stage → parameter map (05-synth.md "Track map").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageParams {
    pub bpm: f32,
    pub brightness: f32,
    /// Root shifts applied at this stage (`min(stage, cap) / 3` fourths).
    pub fourths: u32,
}

impl StageParams {
    /// The stage's root frequency: A2 × (4/3)^`fourths`.
    pub fn root(&self) -> f32 {
        ROOT_A2_HZ * ROOT_SHIFT.powi(self.fourths as i32)
    }
}

/// The pure stage mapping — monotonic in stage, capped at
/// [`STAGE_RAMP_CAP`] (the testable core of `set_stage`).
pub fn stage_params(stage: u32) -> StageParams {
    let s = stage.min(STAGE_RAMP_CAP);
    StageParams {
        bpm: BPM_BASE + BPM_PER_STAGE * s as f32,
        brightness: BRIGHTNESS_BASE + BRIGHTNESS_PER_STAGE * s as f32,
        fourths: s / 3,
    }
}

/// The app-facing music surface (05-synth.md "Director API" + "Web/Android").
///
/// Not `Send`: cpal 0.15's `Stream` carries a
/// `NotSendSyncAcrossAllPlatforms` marker (upstream: the stream API is not
/// thread-safe — AAudio forbids certain calls inside its callback), so the
/// [`EngineHandle`] and this director are main-thread objects. The app
/// keeps the director on its main thread and only hands the pure
/// [`BeatView`] (plain data) elsewhere.
pub struct MusicDirector {
    /// The 8 fixed tracks. While live, the engine's graph reads the same
    /// atomic cells these wrap (its `TrackParams` are `Clone`s sharing the
    /// `Arc<AtomicU32>` cells).
    tracks: Vec<Track>,
    /// `Some` while the cpal stream is live; `None` = fallback clock.
    live: Option<EngineHandle>,
    /// Last stage (echoed in `BeatView::stage`; re-applied on (re)start).
    stage: u32,
    /// Stage-ramp values (re-applied to a fresh engine's globals on start).
    bpm: f32,
    brightness: f32,
    /// Duck recovery progress in seconds (0 = fresh `bump`,
    /// ≥ [`DUCK_RECOVER_S`] = fully recovered).
    duck_recover: f32,
    /// Fallback clock in seconds; advanced only by `tick` while not live.
    fallback_t: f32,
}

impl MusicDirector {
    /// Boot (05-synth.md "Director API" + "Web/Android").
    ///
    /// Desktop/Android: attempts the live engine at boot; on failure (no
    /// device, sink error) it silently stays on the fallback clock —
    /// never panics, and `tick` keeps producing valid `BeatView`s.
    ///
    /// Emscripten: starts on the fallback clock — the autoplay policy
    /// forbids the `AudioContext` before a user gesture; the first canvas
    /// pointer/key event calls `start_on_gesture()` (or `resume()`).
    pub fn start() -> Self {
        // Emscripten: boot stays on the fallback clock (autoplay policy —
        // the engine comes up via `start_on_gesture` / `resume`).
        #[cfg(target_os = "emscripten")]
        {
            return Self::new_fallback();
        }
        #[cfg(not(target_os = "emscripten"))]
        {
            let mut d = Self::new_fallback();
            d.try_start_live();
            d
        }
    }

    /// Web boot hook (emscripten only, 05-synth.md "Web/Android"): call
    /// from the first canvas pointer/key event. Idempotent — a no-op
    /// while the stream is already live; on failure the fallback clock
    /// keeps running (no panic).
    #[cfg(target_os = "emscripten")]
    pub fn start_on_gesture(&mut self) {
        self.try_start_live();
    }

    /// (Re)start the live engine after `stop()`; on web also the
    /// post-suspension path (cpal 0.15's emscripten host owns one
    /// `AudioContext` per stream, so a fresh engine *is* the resume).
    /// Idempotent: a no-op while live; on failure the fallback clock
    /// keeps running (no panic).
    pub fn resume(&mut self) {
        self.try_start_live();
    }

    /// The fallback-only constructor (the web's boot state; host tests).
    fn new_fallback() -> Self {
        let stage = 0u32;
        let p = stage_params(stage);
        Self {
            tracks: Self::build_tracks(stage, &p),
            live: None,
            stage,
            bpm: p.bpm,
            brightness: p.brightness,
            duck_recover: DUCK_RECOVER_S, // unducked at boot
            fallback_t: 0.0,
        }
    }

    /// The fixed 8 tracks at `stage`: each slot's kind at its simple
    /// ratio to the stage root, muted below its unlock stage.
    fn build_tracks(stage: u32, p: &StageParams) -> Vec<Track> {
        SLOTS
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                let t = Track::new(i, slot.kind.label(), slot.kind, p.root() * slot.ratio);
                if slot.kind == PresetKind::Heartbeat {
                    // "Soft kick" (the table's role column): half-time
                    // (2 hits per 16 steps) instead of the default
                    // 4-on-the-floor pattern.
                    t.params.pattern_hits.set_value(2.0);
                    t.params
                        .pattern_bits
                        .store(euclidean_bits(2, 0), Ordering::Relaxed);
                }
                if stage < slot.unlock {
                    t.params.mute.set_value(1.0);
                }
                t
            })
            .collect()
    }

    /// Map a game stage onto the mix (05-synth.md "Track map").
    ///
    /// Mutates `Shared` params only (bpm / brightness / root-derived
    /// freqs / per-slot mutes) — never the DSP graph — so a stage change
    /// is click-free. Idempotent for the current stage.
    pub fn set_stage(&mut self, stage: u32) {
        if stage == self.stage {
            return;
        }
        self.stage = stage;
        let p = stage_params(stage);
        self.bpm = p.bpm;
        self.brightness = p.brightness;
        for (i, slot) in SLOTS.iter().enumerate() {
            let t = &self.tracks[i];
            t.params
                .mute
                .set_value(if stage >= slot.unlock { 0.0 } else { 1.0 });
            t.params.freq.set_value(p.root() * slot.ratio);
        }
        if let Some(e) = &self.live {
            e.global.bpm.set_value(p.bpm);
            e.global.brightness.set_value(p.brightness);
            e.global.scale_mode.set_value(SCALE_MAJOR_PENT);
        }
    }

    /// Collision duck (05-synth.md "Director API"): the master drops to
    /// [`DUCK_GAIN`] and recovers over [`DUCK_RECOVER_S`] (`tick` advances
    /// the recovery and applies the gain while live).
    pub fn bump(&mut self) {
        self.duck_recover = 0.0;
    }

    /// Advance one frame and return the `BeatView` (05-synth.md "Beat
    /// view").
    ///
    /// Live: `t` is the engine's `phase_clock` (audio-thread time) and
    /// `energy` the master envelope. Fallback: `t` is `fallback_t += dt`
    /// (this call is the only thing that advances it) and `energy` is 0.
    pub fn tick(&mut self, dt: f32) -> BeatView {
        self.duck_recover = (self.duck_recover + dt).min(DUCK_RECOVER_S);
        let (t, energy, playing) = match &self.live {
            Some(e) => {
                e.global.master_gain.set_value(self.master_gain());
                (
                    e.phase_clock.value(),
                    energy(e.peak_l.value(), e.peak_r.value()),
                    true,
                )
            }
            None => {
                self.fallback_t += dt;
                (self.fallback_t, 0.0, false)
            }
        };
        BeatView {
            beat_phase: beat_phase(t, self.bpm),
            bar_phase: bar_phase(t, self.bpm),
            energy,
            stage: self.stage,
            playing,
        }
    }

    /// Stop the stream (drops the engine → the cpal stream /
    /// `AudioContext` goes away). The director stays alive on the
    /// fallback clock with its stage/duck state intact; `resume()` (web:
    /// `start_on_gesture()`) brings the engine back.
    pub fn stop(&mut self) {
        self.live = None;
    }

    /// Attempt the live engine (from the fallback clock or after
    /// `stop()`). Failures never surface: they keep the fallback clock
    /// (05-synth.md: Android "If the stream fails, fall back silently";
    /// Desktop "if no device, fall back").
    fn try_start_live(&mut self) -> bool {
        if self.live.is_some() {
            return true;
        }
        // The engine takes a `Vec<Track>` by value and wraps it in its own
        // `Arc<Mutex<…>>`. `Track` is not `Clone` upstream, but
        // `TrackParams` is — and its `Shared` fields are `Arc<AtomicU32>` —
        // so the rebuilt tracks share the atomic cells `set_stage`/`bump`/
        // `tick` mutate here, and the live graph keeps reading the same
        // values (no drift between this crate's state and the DSP).
        let tracks: Vec<Track> = self
            .tracks
            .iter()
            .map(|t| Track {
                id: t.id,
                name: t.name.clone(),
                kind: t.kind,
                params: t.params.clone(),
            })
            .collect();
        match AudioEngine::start(tracks) {
            Ok(handle) => {
                handle.global.bpm.set_value(self.bpm);
                handle.global.brightness.set_value(self.brightness);
                handle.global.scale_mode.set_value(SCALE_MAJOR_PENT);
                handle.global.master_gain.set_value(self.master_gain());
                self.live = Some(handle);
                true
            }
            Err(err) => {
                eprintln!("th-synth: live audio start failed ({err}); running on the fallback clock (BeatView keeps advancing)");
                false
            }
        }
    }

    /// Current master gain: [`DUCK_GAIN`] right after `bump`, ramping
    /// linearly back to [`MASTER_GAIN`] over [`DUCK_RECOVER_S`].
    fn master_gain(&self) -> f32 {
        let u = (self.duck_recover / DUCK_RECOVER_S).clamp(0.0, 1.0);
        DUCK_GAIN + (MASTER_GAIN - DUCK_GAIN) * u
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_synth::audio::engine::MAX_TRACKS;

    /// 05-synth.md test: "stage map monotonic/capped".
    #[test]
    fn stage_map_monotonic_and_capped() {
        let mut prev = stage_params(0);
        assert_eq!(prev.bpm, BPM_BASE);
        assert_eq!(prev.brightness, BRIGHTNESS_BASE);
        assert_eq!(prev.fourths, 0);
        assert_eq!(prev.root(), ROOT_A2_HZ);
        for stage in 1..=STAGE_RAMP_CAP {
            let p = stage_params(stage);
            assert!(p.bpm > prev.bpm, "bpm must rise up to the cap");
            assert!(p.brightness > prev.brightness, "brightness must rise");
            assert!(p.root() >= prev.root(), "root must not fall");
            // A fourth exactly every 3 stages; the root holds between shifts.
            if stage % 3 == 0 {
                assert!(p.root() > prev.root(), "root must step up at stage {stage}");
            } else {
                assert_eq!(p.root(), prev.root(), "root holds between shifts");
            }
            prev = p;
        }
        // The cap: stage 20 is the maximum; beyond it nothing shifts.
        let cap = stage_params(STAGE_RAMP_CAP);
        assert!((cap.bpm - 100.0).abs() < 1e-4);
        assert!((cap.brightness - 0.85).abs() < 1e-5);
        assert_eq!(cap.fourths, STAGE_RAMP_CAP / 3);
        for beyond in (STAGE_RAMP_CAP + 1)..=64 {
            assert_eq!(
                stage_params(beyond),
                cap,
                "stage {beyond} must not shift past the cap"
            );
        }
        // The shift itself: exactly a fourth (4/3) per 3-stage block.
        assert!((stage_params(3).root() - ROOT_A2_HZ * ROOT_SHIFT).abs() < 1e-1);
        assert!((stage_params(6).root() - ROOT_A2_HZ * ROOT_SHIFT * ROOT_SHIFT).abs() < 1e-1);
    }

    /// 05-synth.md test: the table (slot, kind, unlock) matches the
    /// contract; always 8 tracks (the engine's `MAX_TRACKS` assert).
    #[test]
    fn track_table_matches_contract() {
        assert_eq!(
            SLOTS.iter().map(|s| (s.kind, s.unlock)).collect::<Vec<_>>(),
            vec![
                (PresetKind::PadZimmer, 0),
                (PresetKind::DroneSub, 0),
                (PresetKind::PluckSaw, 2),
                (PresetKind::Bell, 5),
                (PresetKind::BassPulse, 8),
                (PresetKind::Shimmer, 11),
                (PresetKind::Heartbeat, 14),
                (PresetKind::SuperSaw, 17),
            ]
        );
        assert_eq!(SLOTS.len(), MAX_TRACKS);
        let d = MusicDirector::new_fallback();
        assert_eq!(d.tracks.len(), MAX_TRACKS);
    }

    /// A slot is muted exactly below its unlock stage.
    #[test]
    fn slots_unlock_per_stage() {
        for stage in 0..=18 {
            let mut d = MusicDirector::new_fallback();
            d.set_stage(stage);
            for (i, slot) in SLOTS.iter().enumerate() {
                let mute = d.tracks[i].params.mute.value();
                if stage >= slot.unlock {
                    assert_eq!(
                        mute,
                        0.0,
                        "stage {stage}: slot {i} ({} @ {}) must be live",
                        slot.kind.label(),
                        slot.unlock
                    );
                } else {
                    assert_eq!(
                        mute,
                        1.0,
                        "stage {stage}: slot {i} ({} @ {}) must be muted",
                        slot.kind.label(),
                        slot.unlock
                    );
                }
            }
        }
    }

    /// The root shift re-tunes the tracks (freq = root × ratio).
    #[test]
    fn root_shift_retunes_tracks() {
        let mut d = MusicDirector::new_fallback();
        assert_eq!(d.tracks[0].params.freq.value(), ROOT_A2_HZ); // pad @ root
        assert_eq!(d.tracks[1].params.freq.value(), ROOT_A2_HZ * 0.5); // drone @ sub
        d.set_stage(3);
        assert_eq!(d.tracks[0].params.freq.value(), ROOT_A2_HZ * ROOT_SHIFT);
        assert_eq!(
            d.tracks[1].params.freq.value(),
            ROOT_A2_HZ * ROOT_SHIFT * 0.5
        );
        d.set_stage(3); // idempotent
        assert_eq!(d.tracks[0].params.freq.value(), ROOT_A2_HZ * ROOT_SHIFT);
    }

    /// 05-synth.md test: "beat/bar phases at known times" (fallback clock,
    /// stage 0 → bpm 70).
    #[test]
    fn beat_and_bar_phases_at_known_times() {
        let beat = 60.0 / 70.0; // seconds per beat at 70 bpm
        let mut d = MusicDirector::new_fallback();
        let v = d.tick(beat * 0.5); // half a beat in
        assert!((v.beat_phase - 0.5).abs() < 1e-3);
        assert!((v.bar_phase - 0.125).abs() < 1e-3); // half a beat of 8

        // Two beats: the beat wraps, the 4-beat bar is half over.
        let mut d = MusicDirector::new_fallback();
        let v = d.tick(2.0 * beat);
        assert!(v.beat_phase.abs() < 1e-6);
        assert!((v.bar_phase - 0.5).abs() < 1e-6);

        // Four beats: the bar wraps to 0 too.
        let mut d = MusicDirector::new_fallback();
        let v = d.tick(4.0 * beat);
        assert!(v.beat_phase.abs() < 1e-6);
        assert!(v.bar_phase.abs() < 1e-6);

        // Three beats: 3/4 of the bar.
        let mut d = MusicDirector::new_fallback();
        let v = d.tick(3.0 * beat);
        assert!((v.bar_phase - 0.75).abs() < 1e-6);
        // Fallback view: no audio → zero energy, not playing, stage echoed.
        assert_eq!(v.energy, 0.0);
        assert_eq!(v.stage, 0);
        assert!(!v.playing);
    }

    /// 05-synth.md test: "duck lowers and recovers".
    #[test]
    fn duck_lowers_and_recovers_over_0_6s() {
        let mut d = MusicDirector::new_fallback();
        assert!((d.master_gain() - MASTER_GAIN).abs() < 1e-6);
        d.bump();
        assert!((d.master_gain() - DUCK_GAIN).abs() < 1e-6);
        // Half the recovery: the linear midpoint.
        d.tick(DUCK_RECOVER_S * 0.5);
        let expected = DUCK_GAIN + (MASTER_GAIN - DUCK_GAIN) * 0.5;
        assert!((d.master_gain() - expected).abs() < 1e-6);
        // After the full 0.6 s: back at the master; over-ticks don't overshoot.
        d.tick(DUCK_RECOVER_S * 0.5);
        assert!((d.master_gain() - MASTER_GAIN).abs() < 1e-6);
        d.tick(10.0);
        assert!((d.master_gain() - MASTER_GAIN).abs() < 1e-6);
    }

    /// A bump during recovery restarts the ramp.
    #[test]
    fn bump_during_recovery_resets_the_ramp() {
        let mut d = MusicDirector::new_fallback();
        d.bump();
        d.tick(0.3);
        assert!(d.master_gain() > DUCK_GAIN);
        d.bump();
        assert!((d.master_gain() - DUCK_GAIN).abs() < 1e-6);
    }

    /// 05-synth.md test: "start() without a device does not panic; tick
    /// advances the fallback". (No device in CI: the fallback path is
    /// taken; with a device the live path must be equally panic-free.)
    #[test]
    fn start_without_a_device_does_not_panic() {
        let mut d = MusicDirector::start();
        let v1 = d.tick(0.1);
        let v2 = d.tick(0.1);
        for v in [v1, v2] {
            assert!((0.0..=1.0).contains(&v.beat_phase));
            assert!((0.0..=1.0).contains(&v.bar_phase));
            assert!((0.0..=1.0).contains(&v.energy));
        }
        if !v1.playing {
            // No device here: the fallback clock must have advanced.
            assert!(!v2.playing);
            assert!(
                v2.beat_phase != v1.beat_phase || v2.bar_phase != v1.bar_phase,
                "fallback clock did not advance"
            );
        }
    }

    /// `stop` drops to the fallback clock; `resume` is safe to call
    /// either way (it never panics).
    #[test]
    fn stop_drops_to_the_fallback_and_resume_is_safe() {
        let mut d = MusicDirector::start();
        d.stop();
        assert!(d.live.is_none());
        let v = d.tick(0.1);
        assert!(!v.playing);
        d.resume();
        let v = d.tick(0.1);
        assert!((0.0..=1.0).contains(&v.beat_phase));
        // stop again is a no-op (nothing to drop).
        d.stop();
        d.tick(0.1);
    }
}
