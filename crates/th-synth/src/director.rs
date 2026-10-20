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
//! # The chiptune score (plan 002 issue 09-taste-2)
//!
//! The operator rejected the ambient chord/arp version: the music is now a
//! **step-sequenced chiptune drum-and-bass** piece over the same eight
//! `rust-synth` presets:
//!
//! - **drums**: three `Heartbeat` voices — kick (low), snare (mid, clicky),
//!   hat (high) — each with its own 16-step bitmask;
//! - **bass**: the `BassPulse` follows a fixed syncopated 16-step line over
//!   the bar's chord root (a step gates the voice with `mute`);
//! - **lead**: the `PluckSaw` plays a fixed 16-step A-minor motif;
//! - **harmony**: pad/bell/sub follow the bar chord (major I–vi–IV–V;
//!   minor i–VI–III–VII from stage 8);
//! - **escalation**: BPM 150→174, a denser hat as the stage rises, the
//!   root shift every 3 stages, and the pad/bell layer unlocks.

use rust_synth::audio::preset::{LFO_CUTOFF, LFO_FREQ};
use rust_synth::audio::{AudioEngine, EngineHandle, PresetKind, Track};
use std::sync::atomic::Ordering;

use crate::beat::{bar_phase, beat_phase, energy, BeatView};

/// One row of the fixed 8-slot track map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub kind: PresetKind,
    pub unlock: u32,
    pub ratio: f32,
    /// `true`: follows the bar's chord root (pad/bell/sub). The bass/lead
    /// are sequenced per step, and the drums have fixed pitches.
    pub chord: bool,
    /// The fixed mix gain.
    pub gain: f32,
}

/// The 8 slots in engine order.
pub const SLOTS: [Slot; 8] = [
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 0.5,
        chord: false,
        gain: 0.9,
    }, // kick
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 1.6,
        chord: false,
        gain: 0.6,
    }, // snare
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 8.0,
        chord: false,
        gain: 0.26,
    }, // hat
    Slot {
        kind: PresetKind::BassPulse,
        unlock: 0,
        ratio: 0.5,
        chord: false,
        gain: 0.8,
    }, // bass
    Slot {
        kind: PresetKind::PluckSaw,
        unlock: 0,
        ratio: 2.0,
        chord: false,
        gain: 0.5,
    }, // lead
    Slot {
        kind: PresetKind::PadZimmer,
        unlock: 4,
        ratio: 1.0,
        chord: true,
        gain: 0.20,
    }, // pad
    Slot {
        kind: PresetKind::Bell,
        unlock: 2,
        ratio: 2.0,
        chord: true,
        gain: 0.20,
    }, // bell
    Slot {
        kind: PresetKind::DroneSub,
        unlock: 0,
        ratio: 0.25,
        chord: true,
        gain: 0.28,
    }, // sub
];

/// Stage-0 root: A2.
pub const ROOT_A2_HZ: f32 = 110.0;
/// The root shift per 3-stage block: up a fourth.
pub const ROOT_SHIFT: f32 = 4.0 / 3.0;
/// Stage cap of the bpm/brightness/root ramps.
pub const STAGE_RAMP_CAP: u32 = 20;
/// `bpm = 150 + 1.2 * min(stage, 20)` → 174 (drum-and-bass tempo).
pub const BPM_BASE: f32 = 150.0;
pub const BPM_PER_STAGE: f32 = 1.2;
/// `brightness = 0.45 + 0.025 * min(stage, 20)` → 0.95.
pub const BRIGHTNESS_BASE: f32 = 0.45;
pub const BRIGHTNESS_PER_STAGE: f32 = 0.025;
/// The master gain.
pub const MASTER_GAIN: f32 = 1.0;
/// The collision-duck target.
pub const DUCK_GAIN: f32 = 0.22;
/// The duck recovery time.
pub const DUCK_RECOVER_S: f32 = 0.6;

/// Beats per bar (4/4), 4 steps per beat → 16 steps.
const STEPS_PER_BAR: u64 = 16;
/// Major progression (semitones): I, vi, IV, V.
const CHORD_MAJOR: [i32; 4] = [0, 9, 5, 7];
/// Minor progression: i, VI, III, VII.
const CHORD_MINOR: [i32; 4] = [0, 8, 3, 10];

/// The syncopated bass line (16 steps; `None` = a rest/gated step) —
/// a ROLLING line (most 8ths plus the 16th pickups at 11/13) so the
/// groove reads as a walking DnB bass, not sparse blips.
const BASS_LINE: [Option<i32>; 16] = [
    Some(0),
    None,
    None,
    Some(12),
    Some(0),
    None,
    Some(7),
    None,
    Some(0),
    None,
    Some(10),
    Some(12),
    Some(0),
    Some(10),
    Some(3),
    None,
];
/// The staccato gate: bass/lead notes play only this fraction of a step
/// (punchy — the note's tail never rings into the next).
const GATE_OPEN: f32 = 0.7;
/// The swing: odd 16ths attack this fraction of a step LATE (the
/// shuffle that makes the grid a groove).
const SWING: f32 = 0.15;
/// The lead motif (16 steps; A-minor degrees above the key root).
const LEAD_LINE: [Option<i32>; 16] = [
    Some(12),
    None,
    Some(15),
    None,
    Some(19),
    Some(17),
    None,
    Some(15),
    Some(12),
    None,
    Some(10),
    None,
    Some(7),
    Some(10),
    None,
    Some(12),
];

/// The stage → parameter map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageParams {
    pub bpm: f32,
    pub brightness: f32,
    pub fourths: u32,
}

impl StageParams {
    pub fn root(&self) -> f32 {
        ROOT_A2_HZ * ROOT_SHIFT.powi(self.fourths as i32)
    }
}

/// The pure stage mapping — monotonic in stage, capped.
#[must_use]
pub fn stage_params(stage: u32) -> StageParams {
    let s = stage.min(STAGE_RAMP_CAP);
    StageParams {
        bpm: BPM_BASE + BPM_PER_STAGE * s as f32,
        brightness: BRIGHTNESS_BASE + BRIGHTNESS_PER_STAGE * s as f32,
        fourths: s / 3,
    }
}

/// `true` for the darker (minor) half: stages 8..16.
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

/// The chord root offset in semitones for `bar` at `stage`.
#[must_use]
pub fn chord_offset(stage: u32, bar: u64) -> i32 {
    let table = if minor_for_stage(stage) {
        CHORD_MINOR
    } else {
        CHORD_MAJOR
    };
    table[(bar % 4) as usize]
}

/// The kick pattern (16-step bitmask) — the DnB syncopation: 1, the
/// "and" at 7 (off the snare's backbeat), the pickup at 10; ghosts late.
#[must_use]
pub fn kick_bits(stage: u32) -> u32 {
    let mut bits = (1 << 0) | (1 << 7) | (1 << 10);
    if stage >= 10 {
        bits |= 1 << 3;
    }
    if stage >= 16 {
        bits |= 1 << 13;
    }
    bits
}

/// The snare pattern: backbeat on 2 and 4, with a late ghost.
#[must_use]
pub fn snare_bits(stage: u32) -> u32 {
    let mut bits = (1 << 4) | (1 << 12);
    if stage >= 12 {
        bits |= 1 << 14;
    }
    bits
}

/// The hat pattern: 8ths early, 16ths from stage 6, busier late.
#[must_use]
pub fn hat_bits(stage: u32) -> u32 {
    if stage < 6 {
        (1 << 2) | (1 << 6) | (1 << 10) | (1 << 14)
    } else if stage < 14 {
        (1 << 1) | (1 << 3) | (1 << 5) | (1 << 7) | (1 << 9) | (1 << 11) | (1 << 13) | (1 << 15)
    } else {
        0xFFFF & !(1 << 4) & !(1 << 12)
    }
}

/// Semitone offset → frequency ratio.
#[must_use]
pub fn semitone_ratio(semitones: i32) -> f32 {
    2.0_f32.powf(semitones as f32 / 12.0)
}

pub struct MusicDirector {
    tracks: Vec<Track>,
    live: Option<EngineHandle>,
    stage: u32,
    bpm: f32,
    brightness: f32,
    last_bar: Option<u64>,
    last_step: Option<u64>,
    duck_recover: f32,
    fallback_t: f32,
}

impl MusicDirector {
    pub fn start() -> Self {
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

    #[cfg(target_os = "emscripten")]
    pub fn start_on_gesture(&mut self) {
        self.try_start_live();
    }

    pub fn resume(&mut self) {
        self.try_start_live();
    }

    #[must_use]
    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    fn new_fallback() -> Self {
        let stage = 0u32;
        let p = stage_params(stage);
        Self {
            tracks: Self::build_tracks(stage),
            live: None,
            stage,
            bpm: p.bpm,
            brightness: p.brightness,
            last_bar: None,
            last_step: None,
            duck_recover: DUCK_RECOVER_S,
            fallback_t: 0.0,
        }
    }

    fn build_tracks(stage: u32) -> Vec<Track> {
        let mut tracks: Vec<Track> = SLOTS
            .iter()
            .enumerate()
            .map(|(i, slot)| Track::new(i, slot.kind.label(), slot.kind, 0.0))
            .collect();
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(&mut tracks[i], slot, i, stage);
        }
        tracks
    }

    fn apply_slot(track: &mut Track, slot: &Slot, index: usize, stage: u32) {
        let s = stage.min(STAGE_RAMP_CAP) as f32;
        let p = stage_params(stage);
        let pms = track.params.clone();
        pms.mute
            .set_value(if stage >= slot.unlock { 0.0 } else { 1.0 });
        pms.gain.set_value(slot.gain);
        pms.arp.set_value(0.0);
        pms.life_mod.set_value(0.85 + 0.15 * (s / 20.0));
        pms.freq.set_value(p.root() * slot.ratio);

        match slot.kind {
            PresetKind::Heartbeat => {
                let bits = match index {
                    0 => kick_bits(stage),
                    1 => snare_bits(stage),
                    _ => hat_bits(stage),
                };
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(match index {
                    0 => 0.30,
                    1 => 0.85,
                    _ => 0.95,
                });
                pms.reverb_mix.set_value(0.05);
                pms.pattern_hits.set_value(bits.count_ones() as f32);
                pms.pattern_rotation.set_value(0.0);
                pms.pattern_bits.store(bits, Ordering::Relaxed);
            }
            PresetKind::BassPulse => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.45);
                pms.reverb_mix.set_value(0.08);
            }
            PresetKind::PluckSaw => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.05);
                pms.lfo_rate.set_value(0.2);
                pms.lfo_target.set_value(LFO_FREQ as f32);
                pms.character.set_value(0.62);
                pms.reverb_mix.set_value(0.20);
            }
            PresetKind::PadZimmer => {
                pms.pulse_depth.set_value(0.12);
                pms.lfo_depth.set_value(0.10);
                pms.lfo_rate.set_value(0.07);
                pms.lfo_target.set_value(LFO_CUTOFF as f32);
                pms.character.set_value(0.40);
                pms.reverb_mix.set_value(0.28);
            }
            PresetKind::Bell => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.06);
                pms.lfo_rate.set_value(0.15);
                pms.lfo_target.set_value(LFO_FREQ as f32);
                pms.character.set_value(0.50);
                pms.reverb_mix.set_value(0.28);
            }
            PresetKind::DroneSub => {
                pms.pulse_depth.set_value(0.18);
                pms.lfo_depth.set_value(0.08);
                pms.lfo_rate.set_value(0.05);
                pms.lfo_target.set_value(LFO_CUTOFF as f32);
                pms.character.set_value(0.35);
                pms.reverb_mix.set_value(0.15);
            }
            PresetKind::Shimmer | PresetKind::SuperSaw => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.5);
                pms.reverb_mix.set_value(0.5);
            }
        }
    }

    pub fn set_stage(&mut self, stage: u32) {
        if stage == self.stage {
            return;
        }
        self.stage = stage;
        let p = stage_params(stage);
        self.bpm = p.bpm;
        self.brightness = p.brightness;
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(&mut self.tracks[i], slot, i, stage);
        }
        // Re-apply the current bar/step on the next tick.
        self.last_bar = None;
        self.last_step = None;
        if let Some(e) = &self.live {
            e.global.bpm.set_value(p.bpm);
            e.global.brightness.set_value(p.brightness);
            e.global.scale_mode.set_value(scale_mode_for_stage(stage));
        }
    }

    pub fn bump(&mut self) {
        self.duck_recover = 0.0;
    }

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
        self.sequence(t);
        BeatView {
            beat_phase: beat_phase(t, self.bpm),
            bar_phase: bar_phase(t, self.bpm),
            energy,
            stage: self.stage,
            playing,
        }
    }

    /// Drive the 16-step sequencer from the mono clock `t` (seconds):
    /// the step/bar writes on change, then the GATE every tick — the
    /// staccato window (a note plays only `GATE_OPEN` of its step) with
    /// the odd 16ths swung late (`SWING` of a step), which is what turns
    /// the grid into a groove.
    fn sequence(&mut self, t: f32) {
        let step_len = 60.0 / self.bpm / 4.0;
        let step_f = t.max(0.0) / step_len;
        let step = step_f as u64;
        let in_step = step_f - step as f32;
        let bar = step / STEPS_PER_BAR;
        if self.last_bar != Some(bar) {
            self.apply_bar(bar);
            self.last_bar = Some(bar);
        }
        if self.last_step != Some(step) {
            self.apply_step(step % STEPS_PER_BAR, bar);
            self.last_step = Some(step);
        }

        let idx = (step % STEPS_PER_BAR) as usize;
        let window = if step % 2 == 1 {
            // the swung 16th: the attack lands late; the window shifts
            // with it (and may run to the next step's edge)
            SWING..(SWING + GATE_OPEN).min(1.0)
        } else {
            0.0..GATE_OPEN
        };
        let on = window.contains(&in_step);
        self.gate_voice(3, BASS_LINE[idx].is_some() && on);
        self.gate_voice(4, LEAD_LINE[idx].is_some() && on);
    }

    /// Write one sequenced voice's mute (`on` = audible).
    fn gate_voice(&mut self, slot: usize, on: bool) {
        self.tracks[slot]
            .params
            .mute
            .set_value(if on { 0.0 } else { 1.0 });
    }

    /// Write the bar's chord onto the harmonic slots (pad/bell/sub).
    fn apply_bar(&mut self, bar: u64) {
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

    /// Write the 16th step's bass/lead note (rests gate with `mute`).
    fn apply_step(&mut self, step: u64, bar: u64) {
        let p = stage_params(self.stage);
        let offset = chord_offset(self.stage, bar);
        let idx = (step % STEPS_PER_BAR) as usize;

        // Bass (slot 3): the chord root/fifth/octave line, gated per step.
        let bass = &self.tracks[3].params;
        if let Some(note) = BASS_LINE[idx] {
            bass.freq
                .set_value(p.root() * 0.5 * semitone_ratio(offset + note));
            bass.mute.set_value(0.0);
        } else {
            bass.mute.set_value(1.0);
        }

        // Lead (slot 4): the fixed A-minor motif on the key root.
        let lead = &self.tracks[4].params;
        if let Some(note) = LEAD_LINE[idx] {
            lead.freq.set_value(p.root() * 2.0 * semitone_ratio(note));
            lead.mute.set_value(0.0);
        } else {
            lead.mute.set_value(1.0);
        }
    }

    pub fn stop(&mut self) {
        self.live = None;
    }

    fn try_start_live(&mut self) -> bool {
        if self.live.is_some() {
            return true;
        }
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
                self.last_bar = None;
                self.last_step = None;
                true
            }
            Err(err) => {
                eprintln!("th-synth: live audio start failed ({err}); running on the fallback clock (BeatView keeps advancing)");
                false
            }
        }
    }

    fn master_gain(&self) -> f32 {
        let u = (self.duck_recover / DUCK_RECOVER_S).clamp(0.0, 1.0);
        DUCK_GAIN + (MASTER_GAIN - DUCK_GAIN) * u
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_synth::audio::engine::MAX_TRACKS;

    #[test]
    fn stage_map_monotonic_and_capped() {
        let mut prev = stage_params(0);
        assert_eq!(prev.bpm, BPM_BASE);
        assert_eq!(prev.brightness, BRIGHTNESS_BASE);
        assert_eq!(prev.fourths, 0);
        assert_eq!(prev.root(), ROOT_A2_HZ);
        for stage in 1..=STAGE_RAMP_CAP {
            let p = stage_params(stage);
            assert!(p.bpm > prev.bpm);
            assert!(p.brightness > prev.brightness);
            assert!(p.root() >= prev.root());
            if stage % 3 == 0 {
                assert!(p.root() > prev.root());
            } else {
                assert_eq!(p.root(), prev.root());
            }
            prev = p;
        }
        let cap = stage_params(STAGE_RAMP_CAP);
        assert!((cap.bpm - 174.0).abs() < 1e-4);
        assert!((cap.brightness - 0.95).abs() < 1e-5);
        assert_eq!(cap.fourths, STAGE_RAMP_CAP / 3);
        for beyond in (STAGE_RAMP_CAP + 1)..=64 {
            assert_eq!(stage_params(beyond), cap);
        }
    }

    #[test]
    fn track_table_matches_contract() {
        assert_eq!(
            SLOTS.iter().map(|s| (s.kind, s.unlock)).collect::<Vec<_>>(),
            vec![
                (PresetKind::Heartbeat, 0),
                (PresetKind::Heartbeat, 0),
                (PresetKind::Heartbeat, 0),
                (PresetKind::BassPulse, 0),
                (PresetKind::PluckSaw, 0),
                (PresetKind::PadZimmer, 4),
                (PresetKind::Bell, 2),
                (PresetKind::DroneSub, 0),
            ]
        );
        assert_eq!(SLOTS.len(), MAX_TRACKS);
        assert_eq!(MusicDirector::new_fallback().tracks.len(), MAX_TRACKS);
    }

    #[test]
    fn slots_unlock_per_stage() {
        for stage in 0..=18 {
            let mut d = MusicDirector::new_fallback();
            d.set_stage(stage);
            for (i, slot) in SLOTS.iter().enumerate() {
                let mute = d.tracks[i].params.mute.value();
                if stage >= slot.unlock {
                    assert_eq!(mute, 0.0, "stage {stage}: slot {i} must be live");
                } else {
                    assert_eq!(mute, 1.0, "stage {stage}: slot {i} must be muted");
                }
            }
        }
    }

    #[test]
    fn root_shift_retunes_the_harmony() {
        let mut d = MusicDirector::new_fallback();
        d.set_stage(3);
        assert_eq!(d.tracks[5].params.freq.value(), ROOT_A2_HZ * ROOT_SHIFT);
        d.set_stage(3);
        assert_eq!(d.tracks[5].params.freq.value(), ROOT_A2_HZ * ROOT_SHIFT);
    }

    #[test]
    fn the_drum_patterns_are_distinct_and_evolve() {
        assert_ne!(kick_bits(0), snare_bits(0));
        assert_ne!(kick_bits(0), hat_bits(0));
        // 8th hats early, 16th hats late.
        assert_eq!(hat_bits(0).count_ones(), 4);
        assert_eq!(hat_bits(6).count_ones(), 8);
        assert!(hat_bits(16).count_ones() > 8);
        // The kick gains ghosts, the snare a late ghost.
        assert_eq!(kick_bits(0), (1 << 0) | (1 << 7) | (1 << 10));
        assert!(kick_bits(16).count_ones() > kick_bits(0).count_ones());
        assert!(snare_bits(12).count_ones() > snare_bits(11).count_ones());
        let mut d = MusicDirector::new_fallback();
        d.set_stage(6);
        assert_eq!(
            d.tracks[2].params.pattern_bits.load(Ordering::Relaxed),
            hat_bits(6)
        );
    }

    #[test]
    fn the_bass_and_lead_are_sequenced_per_step() {
        let mut d = MusicDirector::new_fallback();
        d.tick(0.01); // step 0
        let bass0 = d.tracks[3].params.freq.value();
        let lead0 = d.tracks[4].params.freq.value();
        assert_eq!(d.tracks[3].params.mute.value(), 0.0);
        assert_eq!(d.tracks[4].params.mute.value(), 0.0);
        // Step 1 is a bass rest AND a lead rest (and the swung window
        // opens late — no note either way).
        d.tick(60.0 / 150.0 / 4.0);
        assert_eq!(d.tracks[3].params.mute.value(), 1.0);
        assert_eq!(d.tracks[4].params.mute.value(), 1.0);
        // Step 3 is a bass octave (an odd, swung step: audible only
        // once its window opens at SWING).
        d.tick(60.0 / 150.0 / 4.0); // step 2
        d.tick(60.0 / 150.0 / 4.0); // step 3
        let swung_mute = d.tracks[3].params.mute.value();
        assert_eq!(swung_mute, 1.0, "a swung note rests until its window");
        d.tick(60.0 / 150.0 / 4.0 * (SWING + 0.05));
        assert_eq!(d.tracks[3].params.mute.value(), 0.0, "the window opens");
        assert!(
            (d.tracks[3].params.freq.value() / bass0 - 2.0).abs() < 1e-3,
            "bass octave"
        );
        // The lead returns on step 2 with a higher note.
        assert!(d.tracks[4].params.freq.value() > lead0);
    }

    #[test]
    fn the_gate_releases_between_notes() {
        // The staccato: within one even step the note mutes at GATE_OPEN.
        let step_len = 60.0 / BPM_BASE / 4.0;
        let mut d = MusicDirector::new_fallback();
        d.tick(0.01); // step 0: note on
        assert_eq!(d.tracks[3].params.mute.value(), 0.0);
        d.tick(step_len * GATE_OPEN);
        assert_eq!(
            d.tracks[3].params.mute.value(),
            1.0,
            "the note's tail is cut at GATE_OPEN"
        );
    }

    #[test]
    fn the_bar_chord_moves_the_harmony_only() {
        let bar = 4.0 * 60.0 / BPM_BASE;
        let mut d = MusicDirector::new_fallback();
        let pad0 = d.tracks[5].params.freq.value();
        let lead0 = d.tracks[4].params.freq.value();
        d.tick(0.01);
        assert!((d.tracks[5].params.freq.value() - pad0).abs() < 1e-4);
        d.tick(bar * 1.05);
        let expected = pad0 * semitone_ratio(9);
        assert!(
            (d.tracks[5].params.freq.value() - expected).abs() < 0.5,
            "pad follows vi"
        );
        // The lead melody is on the key root, not the chord.
        assert_ne!(d.tracks[4].params.freq.value(), lead0 * semitone_ratio(9));
    }

    #[test]
    fn beat_and_bar_phases_at_known_times() {
        let beat = 60.0 / BPM_BASE;
        let mut d = MusicDirector::new_fallback();
        let v = d.tick(beat * 0.5);
        assert!((v.beat_phase - 0.5).abs() < 1e-3);
        assert!((v.bar_phase - 0.125).abs() < 1e-3);
        let mut d = MusicDirector::new_fallback();
        let v = d.tick(4.0 * beat);
        assert!(v.bar_phase.abs() < 1e-6);
        assert!(!v.playing);
    }

    #[test]
    fn duck_lowers_and_recovers_over_0_6s() {
        let mut d = MusicDirector::new_fallback();
        assert!((d.master_gain() - MASTER_GAIN).abs() < 1e-6);
        d.bump();
        assert!((d.master_gain() - DUCK_GAIN).abs() < 1e-6);
        d.tick(DUCK_RECOVER_S * 0.5);
        let expected = DUCK_GAIN + (MASTER_GAIN - DUCK_GAIN) * 0.5;
        assert!((d.master_gain() - expected).abs() < 1e-6);
        d.tick(10.0);
        assert!((d.master_gain() - MASTER_GAIN).abs() < 1e-6);
    }

    #[test]
    fn start_without_a_device_does_not_panic() {
        let mut d = MusicDirector::start();
        let v1 = d.tick(0.1);
        let v2 = d.tick(0.1);
        for v in [v1, v2] {
            assert!((0.0..=1.0).contains(&v.beat_phase));
            assert!((0.0..=1.0).contains(&v.bar_phase));
        }
        if !v1.playing {
            assert!(!v2.playing);
        }
    }

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
