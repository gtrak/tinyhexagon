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
//!
//! # The score (plan 002 issue 09-taste)
//!
//! The 05 track map was eight sustained voices with static pitches — the
//! "escalating drones" the operator heard. It is now a small generative
//! score over the same eight presets:
//!
//! - a **bar-wise chord progression** (major I–vi–IV–V; minor
//!   i–VI–III–VII from stage 8) that the pad, drone, bass and kick follow;
//! - **pentatonic arpeggios** on the pluck/bell/shimmer/lead, rooted on
//!   the key (not the chord), so every random walk stays in the scale;
//! - an **evolving kick pattern** (2 → 4 → 5 → 7 → 9 hits per 16 steps);
//! - slow **filter/frequency LFOs** and per-stage pulse/gain/character;
//! - the 05 **root shift** every 3 stages and the stage layer unlocks.
//!
//! All of it is just `Shared` writes on the existing DSP graph.

use rust_synth::audio::preset::{LFO_CUTOFF, LFO_FREQ};
use rust_synth::audio::{AudioEngine, EngineHandle, PresetKind, Track};
use rust_synth::math::rhythm::euclidean_bits;
use std::sync::atomic::Ordering;

use crate::beat::{bar_phase, beat_phase, energy, BeatView};

/// One row of the fixed 8-slot track map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// The preset kind fixed to this slot.
    pub kind: PresetKind,
    /// The stage at which this slot un-mutes.
    pub unlock: u32,
    /// The slot's simple ratio to the stage root (root A2 = 110 Hz at
    /// stage 0): 1.0 = root, 0.5 = sub-octave, 1.5 = fifth, 2.0 = octave.
    pub ratio: f32,
    /// `true`: the slot follows the bar's chord root (pad, drone, bass,
    /// kick). `false`: it stays on the key root (the melodic voices).
    pub chord: bool,
    /// Arpeggiator depth: 0 = static pitch, 1 = an in-scale pentatonic
    /// walk every two beats (the rust-synth `arp` knob).
    pub arp: f32,
    /// The slot's mix gain.
    pub gain: f32,
}

/// The 8 slots in engine order (the engine asserts exactly
/// `MAX_TRACKS = 8` tracks; see the "always 8 tracks" test).
pub const SLOTS: [Slot; 8] = [
    Slot {
        kind: PresetKind::PadZimmer,
        unlock: 0,
        ratio: 1.0,
        chord: true,
        arp: 0.0,
        gain: 0.34,
    }, // pad
    Slot {
        kind: PresetKind::DroneSub,
        unlock: 0,
        ratio: 0.5,
        chord: true,
        arp: 0.0,
        gain: 0.30,
    }, // sub drone
    Slot {
        kind: PresetKind::PluckSaw,
        unlock: 0,
        ratio: 2.0,
        chord: false,
        arp: 1.0,
        gain: 0.36,
    }, // arpeggio / lead
    Slot {
        kind: PresetKind::Bell,
        unlock: 2,
        ratio: 4.0,
        chord: false,
        arp: 1.0,
        gain: 0.20,
    }, // bell
    Slot {
        kind: PresetKind::BassPulse,
        unlock: 3,
        ratio: 0.5,
        chord: true,
        arp: 0.0,
        gain: 0.46,
    }, // bass
    Slot {
        kind: PresetKind::Shimmer,
        unlock: 7,
        ratio: 4.0,
        chord: false,
        arp: 1.0,
        gain: 0.14,
    }, // shimmer
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 4,
        ratio: 0.5,
        chord: true,
        arp: 0.0,
        gain: 0.58,
    }, // kick
    Slot {
        kind: PresetKind::SuperSaw,
        unlock: 11,
        ratio: 1.0,
        chord: false,
        arp: 1.0,
        gain: 0.26,
    }, // lead
];

/// Stage-0 root: A2 (05-synth.md "Track map").
pub const ROOT_A2_HZ: f32 = 110.0;
/// The root shift per 3-stage block: up a fourth (05-synth.md "shift the
/// root up a fourth every 3 stages").
pub const ROOT_SHIFT: f32 = 4.0 / 3.0;
/// Stage cap of the bpm/brightness/root ramps (the app's stage plan caps at
/// 20).
pub const STAGE_RAMP_CAP: u32 = 20;
/// `bpm = 70 + 2.5 * min(stage, 20)` → 120 at the cap (the 09-taste arc:
/// the old 100 BPM plateau was too tame).
pub const BPM_BASE: f32 = 70.0;
pub const BPM_PER_STAGE: f32 = 2.5;
/// `brightness = 0.45 + 0.025 * min(stage, 20)` → 0.95 at the cap.
pub const BRIGHTNESS_BASE: f32 = 0.45;
pub const BRIGHTNESS_PER_STAGE: f32 = 0.025;
/// The master gain the director drives (05-synth.md "master_gain ≈ 0.55").
pub const MASTER_GAIN: f32 = 0.55;
/// The collision-duck target (05-synth.md "duck to ~0.22 on a bump").
pub const DUCK_GAIN: f32 = 0.22;
/// The duck recovery time (05-synth.md "recover over 0.6 s").
pub const DUCK_RECOVER_S: f32 = 0.6;

/// Beats per bar (4/4).
const BEATS_PER_BAR: f32 = 4.0;
/// Bars per chord (one chord per bar).
const CHORD_BARS: u64 = 1;
/// Major progression (semitones above the stage root): I, vi, IV, V.
const CHORD_MAJOR: [i32; 4] = [0, 9, 5, 7];
/// Minor progression: i, VI, III, VII.
const CHORD_MINOR: [i32; 4] = [0, 8, 3, 10];

/// The stage → parameter map.
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

/// `true` for the darker (minor) half of the arc: stages 8..16.
#[must_use]
pub fn minor_for_stage(stage: u32) -> bool {
    (8..16).contains(&stage)
}

/// The engine's `scale_mode` (0 major pentatonic, 1 minor pentatonic).
#[must_use]
pub fn scale_mode_for_stage(stage: u32) -> f32 {
    if minor_for_stage(stage) {
        1.0
    } else {
        0.0
    }
}

/// The chord's root offset in semitones for `bar` at `stage` (the
/// progression loops once per four bars).
#[must_use]
pub fn chord_offset(stage: u32, bar: u64) -> i32 {
    let table = if minor_for_stage(stage) {
        CHORD_MINOR
    } else {
        CHORD_MAJOR
    };
    table[((bar / CHORD_BARS) % 4) as usize]
}

/// The kick's 16-step pattern `(hits, rotation)` — busier with the stage.
#[must_use]
pub fn kick_pattern(stage: u32) -> (u32, u32) {
    match stage {
        0..=5 => (2, 0),
        6..=9 => (4, 0),
        10..=13 => (5, 3),
        14..=17 => (7, 2),
        _ => (9, 5),
    }
}

/// Semitone offset → frequency ratio.
#[must_use]
pub fn semitone_ratio(semitones: i32) -> f32 {
    2.0_f32.powf(semitones as f32 / 12.0)
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
    /// The bar whose chord was last written to the tracks.
    last_bar: u64,
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

    /// The live-engine state (05a's boot contract; plan 002 08a: the web
    /// `status()` line's `audio=` half): `true` while the live
    /// `MusicEngine` runs, `false` while the fallback clock runs.
    #[must_use]
    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    /// The fallback-only constructor (the web's boot state; host tests).
    fn new_fallback() -> Self {
        let stage = 0u32;
        let p = stage_params(stage);
        Self {
            tracks: Self::build_tracks(stage),
            live: None,
            stage,
            bpm: p.bpm,
            brightness: p.brightness,
            last_bar: u64::MAX,
            duck_recover: DUCK_RECOVER_S, // unducked at boot
            fallback_t: 0.0,
        }
    }

    /// The 8 tracks at `stage` (unlock/mix/arp/LFO/pattern applied; the
    /// harmonic slots on the bar-0 chord).
    fn build_tracks(stage: u32) -> Vec<Track> {
        let mut tracks: Vec<Track> = SLOTS
            .iter()
            .enumerate()
            .map(|(i, slot)| Track::new(i, slot.kind.label(), slot.kind, 0.0))
            .collect();
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(&mut tracks[i], slot, stage);
        }
        tracks
    }

    /// Write one slot's stage-static parameters (everything except the
    /// per-bar chord frequency) onto `track`.
    fn apply_slot(track: &mut Track, slot: &Slot, stage: u32) {
        let s = stage.min(STAGE_RAMP_CAP) as f32;
        let p = stage_params(stage);
        let pms = track.params.clone();
        pms.mute
            .set_value(if stage >= slot.unlock { 0.0 } else { 1.0 });
        pms.gain.set_value(slot.gain);
        pms.arp.set_value(slot.arp);
        pms.life_mod.set_value(0.85 + 0.15 * (s / 20.0));
        // Static fallback pitch (the chord pass overwrites the harmonic
        // slots on the first tick; the melodic slots keep this pitch).
        let base = p.root() * slot.ratio;
        let offset = if slot.chord {
            chord_offset(stage, 0)
        } else {
            0
        };
        pms.freq.set_value(base * semitone_ratio(offset));

        match slot.kind {
            PresetKind::PadZimmer => {
                pms.pulse_depth.set_value(0.10 + 0.005 * s);
                pms.lfo_depth.set_value(0.12);
                pms.lfo_rate.set_value(0.06);
                pms.lfo_target.set_value(LFO_CUTOFF as f32);
                pms.character.set_value(0.40);
                pms.reverb_mix.set_value(0.70);
                pms.supermass.set_value(0.10 * (s / 20.0));
            }
            PresetKind::DroneSub => {
                pms.pulse_depth.set_value(0.20);
                pms.lfo_depth.set_value(0.08);
                pms.lfo_rate.set_value(0.05);
                pms.lfo_target.set_value(LFO_CUTOFF as f32);
                pms.character.set_value(0.35);
                pms.reverb_mix.set_value(0.50);
                pms.supermass.set_value(0.0);
            }
            PresetKind::PluckSaw => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.60);
                pms.reverb_mix.set_value(0.55);
            }
            PresetKind::Bell => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.08);
                pms.lfo_rate.set_value(0.13);
                pms.lfo_target.set_value(LFO_FREQ as f32);
                pms.character.set_value(0.55);
                pms.reverb_mix.set_value(0.70);
                pms.supermass.set_value(0.05 * (s / 20.0));
            }
            PresetKind::BassPulse => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.50);
                pms.reverb_mix.set_value(0.25);
            }
            PresetKind::Shimmer => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.10);
                pms.lfo_rate.set_value(0.17);
                pms.lfo_target.set_value(LFO_FREQ as f32);
                pms.character.set_value(0.60);
                pms.reverb_mix.set_value(0.80);
                pms.supermass.set_value(0.08 * (s / 20.0));
            }
            PresetKind::Heartbeat => {
                let (hits, rotation) = kick_pattern(stage);
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.55 + 0.01 * s);
                pms.reverb_mix.set_value(0.15);
                pms.pattern_hits.set_value(hits as f32);
                pms.pattern_rotation.set_value(rotation as f32);
                pms.pattern_bits
                    .store(euclidean_bits(hits, rotation), Ordering::Relaxed);
            }
            PresetKind::SuperSaw => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.12);
                pms.lfo_rate.set_value(0.09);
                pms.lfo_target.set_value(LFO_CUTOFF as f32);
                pms.character.set_value(0.50);
                pms.reverb_mix.set_value(0.60);
            }
        }
    }

    /// Map a game stage onto the mix. Mutates `Shared` params only
    /// (bpm / brightness / freq / mute / arp / mix) — never the DSP graph
    /// — so a stage change is click-free. Idempotent for the current
    /// stage.
    pub fn set_stage(&mut self, stage: u32) {
        if stage == self.stage {
            return;
        }
        self.stage = stage;
        let p = stage_params(stage);
        self.bpm = p.bpm;
        self.brightness = p.brightness;
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(&mut self.tracks[i], slot, stage);
        }
        // Force the next tick to re-apply the chord (the bar clock did not
        // stop across the stage change).
        self.last_bar = u64::MAX;
        if let Some(e) = &self.live {
            e.global.bpm.set_value(p.bpm);
            e.global.brightness.set_value(p.brightness);
            e.global.scale_mode.set_value(scale_mode_for_stage(stage));
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
    /// The bar's chord is written to the pad/drone/bass/kick on the bar
    /// boundary.
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
        self.apply_bar(t);
        BeatView {
            beat_phase: beat_phase(t, self.bpm),
            bar_phase: bar_phase(t, self.bpm),
            energy,
            stage: self.stage,
            playing,
        }
    }

    /// Write the current bar's chord onto the harmonic slots. `t` is the
    /// mono clock in seconds; the bar length is `4 * 60 / bpm`.
    fn apply_bar(&mut self, t: f32) {
        let bar_len = BEATS_PER_BAR * 60.0 / self.bpm.max(1.0);
        let bar = ((t.max(0.0) / bar_len) as u64) / CHORD_BARS;
        if bar == self.last_bar {
            return;
        }
        self.last_bar = bar;
        let p = stage_params(self.stage);
        let offset = chord_offset(self.stage, bar);
        for (i, slot) in SLOTS.iter().enumerate() {
            if slot.chord {
                self.tracks[i]
                    .params
                    .freq
                    .set_value(p.root() * slot.ratio * semitone_ratio(offset));
            }
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
                handle
                    .global
                    .scale_mode
                    .set_value(scale_mode_for_stage(self.stage));
                handle.global.master_gain.set_value(self.master_gain());
                self.live = Some(handle);
                self.last_bar = u64::MAX;
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
            if stage % 3 == 0 {
                assert!(p.root() > prev.root(), "root must step up at stage {stage}");
            } else {
                assert_eq!(p.root(), prev.root(), "root holds between shifts");
            }
            prev = p;
        }
        let cap = stage_params(STAGE_RAMP_CAP);
        assert!((cap.bpm - 120.0).abs() < 1e-4);
        assert!((cap.brightness - 0.95).abs() < 1e-5);
        assert_eq!(cap.fourths, STAGE_RAMP_CAP / 3);
        for beyond in (STAGE_RAMP_CAP + 1)..=64 {
            assert_eq!(
                stage_params(beyond),
                cap,
                "stage {beyond} must not shift past the cap"
            );
        }
        assert!((stage_params(3).root() - ROOT_A2_HZ * ROOT_SHIFT).abs() < 1e-1);
        assert!((stage_params(6).root() - ROOT_A2_HZ * ROOT_SHIFT * ROOT_SHIFT).abs() < 1e-1);
    }

    /// The table (slot, kind, unlock) matches the 09-taste contract; always
    /// 8 tracks (the engine's `MAX_TRACKS` assert).
    #[test]
    fn track_table_matches_contract() {
        assert_eq!(
            SLOTS.iter().map(|s| (s.kind, s.unlock)).collect::<Vec<_>>(),
            vec![
                (PresetKind::PadZimmer, 0),
                (PresetKind::DroneSub, 0),
                (PresetKind::PluckSaw, 0),
                (PresetKind::Bell, 2),
                (PresetKind::BassPulse, 3),
                (PresetKind::Shimmer, 7),
                (PresetKind::Heartbeat, 4),
                (PresetKind::SuperSaw, 11),
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

    /// The root shift re-tunes the tracks (freq = root × ratio at the
    /// progression's first chord).
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

    /// The chord progression: the harmonic slots move on the bar boundary,
    /// the melodic slots stay on the key root, and the progression is the
    /// major table at stage 0 / the minor table from stage 8.
    #[test]
    fn the_bar_chord_moves_harmony_and_keeps_melody_on_the_key() {
        let bar = 4.0 * 60.0 / BPM_BASE; // 70 bpm → 3.4286 s
        let mut d = MusicDirector::new_fallback();
        let key_pad = d.tracks[0].params.freq.value();
        let key_pluck = d.tracks[2].params.freq.value();
        d.tick(0.05); // bar 0
        assert!((d.tracks[0].params.freq.value() - key_pad).abs() < 1e-4);
        d.tick(bar * 1.05); // bar 1 → vi (9 semitones)
        let expected = key_pad * semitone_ratio(9);
        assert!(
            (d.tracks[0].params.freq.value() - expected).abs() < 0.5,
            "pad follows the vi chord"
        );
        // The kick follows too; the melody does not.
        assert!((d.tracks[6].params.freq.value() - key_pad * 0.5 * semitone_ratio(9)).abs() < 0.5);
        assert_eq!(d.tracks[2].params.freq.value(), key_pluck);

        // Stage 8 switches to minor; bar 0 is still the root, bar 1 is VI.
        let mut d = MusicDirector::new_fallback();
        d.set_stage(8);
        let root = stage_params(8).root();
        d.tick(0.05);
        assert!((d.tracks[0].params.freq.value() - root).abs() < 0.5);
        d.tick(bar * 1.05);
        assert!(
            (d.tracks[0].params.freq.value() - root * semitone_ratio(8)).abs() < 0.5,
            "stage 8 bar 1 is the minor VI"
        );
    }

    /// The melodic slots carry the arpeggiator; the harmonic ones do not.
    #[test]
    fn arpeggios_are_on_the_melody_only() {
        let d = MusicDirector::new_fallback();
        for (i, slot) in SLOTS.iter().enumerate() {
            if slot.chord {
                assert_eq!(d.tracks[i].params.arp.value(), 0.0, "slot {i}");
            } else {
                assert_eq!(d.tracks[i].params.arp.value(), 1.0, "slot {i}");
            }
        }
    }

    /// The kick pattern grows with the stage (2 → 4 → 5 → 7 → 9 hits).
    #[test]
    fn the_kick_pattern_evolves() {
        assert_eq!(kick_pattern(0), (2, 0));
        assert_eq!(kick_pattern(8), (4, 0));
        assert_eq!(kick_pattern(12), (5, 3));
        assert_eq!(kick_pattern(16), (7, 2));
        assert_eq!(kick_pattern(20), (9, 5));
        let mut d = MusicDirector::new_fallback();
        d.set_stage(12);
        let bits = d.tracks[6].params.pattern_bits.load(Ordering::Relaxed);
        assert_eq!(bits, euclidean_bits(5, 3));
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

        let mut d = MusicDirector::new_fallback();
        let v = d.tick(2.0 * beat);
        assert!(v.beat_phase.abs() < 1e-6);
        assert!((v.bar_phase - 0.5).abs() < 1e-6);

        let mut d = MusicDirector::new_fallback();
        let v = d.tick(4.0 * beat);
        assert!(v.beat_phase.abs() < 1e-6);
        assert!(v.bar_phase.abs() < 1e-6);

        let mut d = MusicDirector::new_fallback();
        let v = d.tick(3.0 * beat);
        assert!((v.bar_phase - 0.75).abs() < 1e-6);
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
        d.tick(DUCK_RECOVER_S * 0.5);
        let expected = DUCK_GAIN + (MASTER_GAIN - DUCK_GAIN) * 0.5;
        assert!((d.master_gain() - expected).abs() < 1e-6);
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
    /// advances the fallback".
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
        d.stop();
        d.tick(0.1);
    }
}
