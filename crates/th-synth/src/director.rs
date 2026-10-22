//! The `MusicDirector` (05-synth.md "Director API"): the app-facing surface
//! over the `rust-synth` engine.
//!
//! The director owns the 8 fixed tracks and, while live, the
//! [`EngineHandle`]. The engine takes its `Vec<Track>` by value; the
//! `TrackParams` we hand it are clones whose `Shared` fields are
//! `Arc<AtomicU32>` — so the live DSP graph reads the *same* atomics the
//! director mutates here: `set_stage`/`set_score`/`bump`/`tick` only write
//! `Shared` params and never touch the graph (no `rebuild_graph`, no
//! clicks on stage changes).
//!
//! # The chiptune score (plan 002 issue 09-taste-2, data'd by 10a)
//!
//! The operator rejected the ambient chord/arp version: the music is a
//! **step-sequenced chiptune drum-and-bass** piece over the same eight
//! `rust-synth` presets:
//!
//! - **drums**: three `Heartbeat` voices — kick (low), snare (mid, clicky),
//!   hat (high) — each with its own stage-tiered 16-step pattern;
//! - **bass**: the `BassPulse` follows a syncopated 16-step line over
//!   the bar's chord root (a step gates the voice with `mute`);
//! - **lead**: the `PluckSaw` plays a 16-step A-minor motif on the key
//!   root (deliberately NOT chord-following);
//! - **harmony**: pad/bell/sub follow the bar chord (major I–vi–IV–V;
//!   minor i–VI–III–VII from stage 8);
//! - **escalation**: BPM 150→174, a denser hat as the stage rises, the
//!   root shift every 3 stages, and the pad/bell layer unlocks.
//!
//! Since 10a the CONTENT of that piece is data — a [`Score`] (`score.rs`),
//! baked in `assets/score.json` (the shipped default, `Score::default()`)
//! and hot-swappable live (`set_score`, the wasm `setScore` export's
//! half): the director's live path reads `self.score` (the gate, the
//! swing, the bpm/brightness ramps, the chord tables, the drum tiers,
//! the bass/lead lines, the slots' gains). The module-level free
//! functions below (`stage_params`, `chord_offset`, `minor_for_stage`,
//! `scale_mode_for_stage`, `kick_bits`/`snare_bits`/`hat_bits`) and the
//! `BPM_*` / `BRIGHTNESS_*` / `ROOT_*` / `GATE_OPEN` / `SWING` consts are
//! the DEFAULT-score shorthands the tests' tick arithmetic asserts
//! against. The bake-fidelity test
//! (`score_default_matches_the_legacy_constants`, in `score.rs`) does
//! NOT: it compares the parsed default score against the FROZEN
//! pre-refactor (d97d4277) expressions in `score.rs`'s private
//! `mod legacy` (verbatim copies) — asserting through these shorthands
//! would make it vacuous (they are `Score::default()` lookups).

use rust_synth::audio::preset::{LFO_CUTOFF, LFO_FREQ};
use rust_synth::audio::{AudioEngine, EngineHandle, PresetKind, Track};
use std::sync::atomic::Ordering;

use crate::beat::{bar_phase, beat_phase, energy, BeatView};
use crate::score::Score;

/// `StageParams` is defined in `score.rs` (it carries the score's
/// `root_hz`/`root_shift`); re-exported here so
/// `th_synth::director::StageParams` keeps resolving.
pub use crate::score::StageParams;

/// One row of the fixed 8-slot track map (the synthesis design: the
/// preset kind, the unlock stage, the ratio to the slot's root, and the
/// chord-following flag. The slot's MIX GAIN is score data —
/// `Score.slots` — since 10a: one source, no drift).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub kind: PresetKind,
    pub unlock: u32,
    pub ratio: f32,
    /// `true`: follows the bar's chord root (pad/bell/sub). The bass/lead
    /// are sequenced per step, and the drums have fixed pitches.
    pub chord: bool,
}

/// The 8 slots in engine order.
pub const SLOTS: [Slot; 8] = [
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 0.5,
        chord: false,
    }, // kick
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 1.6,
        chord: false,
    }, // snare
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 8.0,
        chord: false,
    }, // hat
    Slot {
        kind: PresetKind::BassPulse,
        unlock: 0,
        ratio: 0.5,
        chord: false,
    }, // bass
    Slot {
        kind: PresetKind::PluckSaw,
        unlock: 0,
        ratio: 2.0,
        chord: false,
    }, // lead
    Slot {
        kind: PresetKind::PadZimmer,
        unlock: 4,
        ratio: 1.0,
        chord: true,
    }, // pad
    Slot {
        kind: PresetKind::Bell,
        unlock: 2,
        ratio: 2.0,
        chord: true,
    }, // bell
    Slot {
        kind: PresetKind::DroneSub,
        unlock: 0,
        ratio: 0.25,
        chord: true,
    }, // sub
];

/// Stage-0 root: A2. The DEFAULT score's `root_hz`; the director's live
/// path reads `self.score`.
pub const ROOT_A2_HZ: f32 = 110.0;
/// The root shift per 3-stage block: up a fourth. The DEFAULT score's
/// `root_shift`; the live path reads `self.score`.
pub const ROOT_SHIFT: f32 = 4.0 / 3.0;
/// Stage cap of the bpm/brightness/root ramps. The DEFAULT score's
/// `bpm.cap`; the live path reads `self.score`.
pub const STAGE_RAMP_CAP: u32 = 20;
/// `bpm = 150 + 1.2 * min(stage, 20)` → 174 (drum-and-bass tempo). The
/// DEFAULT score's `bpm.base`/`bpm.per_stage`; the live path reads
/// `self.score`.
pub const BPM_BASE: f32 = 150.0;
pub const BPM_PER_STAGE: f32 = 1.2;
/// `brightness = 0.45 + 0.025 * min(stage, 20)` → 0.95. The DEFAULT
/// score's `brightness.*`; the live path reads `self.score`.
pub const BRIGHTNESS_BASE: f32 = 0.45;
pub const BRIGHTNESS_PER_STAGE: f32 = 0.025;
/// The master gain.
pub const MASTER_GAIN: f32 = 1.0;
/// The collision-duck target.
pub const DUCK_GAIN: f32 = 0.22;
/// The duck recovery time.
pub const DUCK_RECOVER_S: f32 = 0.6;
/// The staccato gate: a note plays only this fraction of a step
/// (punchy — the note's tail never rings into the next). The DEFAULT
/// score's `gate_open`; the director's live path reads
/// `self.score.gate_open` (the const keeps the tests' tick arithmetic
/// honest against the baked default).
pub const GATE_OPEN: f32 = 0.7;
/// The swing: odd 16ths attack this fraction of a step LATE (the
/// shuffle that makes the grid a groove). The DEFAULT score's `swing`;
/// the live path reads `self.score.swing`.
pub const SWING: f32 = 0.15;

/// The pure stage mapping — monotonic in stage, capped. The
/// DEFAULT-score shorthand (`Score::default().stage_params`): the
/// director's live path reads `self.score`.
#[must_use]
pub fn stage_params(stage: u32) -> StageParams {
    Score::default().stage_params(stage)
}

/// `true` for the darker (minor) half: stages 8..16 (the default score's
/// window). The DEFAULT-score shorthand: the live path reads
/// `self.score.minor_for_stage`.
#[must_use]
pub fn minor_for_stage(stage: u32) -> bool {
    Score::default().minor_for_stage(stage)
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

/// The chord root offset in semitones for `bar` at `stage`. The
/// DEFAULT-score shorthand: the live path reads `self.score.chord_offset`.
#[must_use]
pub fn chord_offset(stage: u32, bar: u64) -> i32 {
    Score::default().chord_offset(stage, bar)
}

/// The kick pattern (16-step bitmask) — the DnB syncopation: 1, the
/// "and" at 7 (off the snare's backbeat), the pickup at 10; ghosts late.
/// The DEFAULT-score shorthand: the live path reads
/// `self.score.drum_pattern(&self.score.kick, stage)`.
#[must_use]
pub fn kick_bits(stage: u32) -> u32 {
    let score = Score::default();
    score.drum_pattern(&score.kick, stage)
}

/// The snare pattern: backbeat on 2 and 4, with a late ghost. The
/// DEFAULT-score shorthand: the live path reads
/// `self.score.drum_pattern(&self.score.snare, stage)`.
#[must_use]
pub fn snare_bits(stage: u32) -> u32 {
    let score = Score::default();
    score.drum_pattern(&score.snare, stage)
}

/// The hat pattern: 8ths early, 16ths from stage 6, busier late. The
/// DEFAULT-score shorthand: the live path reads
/// `self.score.drum_pattern(&self.score.hat, stage)`.
#[must_use]
pub fn hat_bits(stage: u32) -> u32 {
    let score = Score::default();
    score.drum_pattern(&score.hat, stage)
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
    /// The chiptune score being sequenced (the plan 10 single source of
    /// truth; `Score::default()` = the baked `assets/score.json`).
    score: Score,
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
        if !self.try_start_live() {
            // A failed start leaves the fallback; nothing more to do —
            // the next gesture retries.
            return;
        }
        // A previously-started engine whose context is still suspended
        // (a resume rejected earlier) gets a fresh resume per gesture.
        self.replay_context();
    }

    pub fn resume(&mut self) {
        self.try_start_live();
    }

    #[must_use]
    pub fn is_live(&self) -> bool {
        self.live.is_some()
    }

    /// 09 instrumentation — the live engine's WebAudio context state
    /// ("" on the fallback clock / non-emscripten hosts, and on the
    /// in-memory engine — its shared OpenAL context is the SFX device's,
    /// so the status's `ctx=` field reads that device's recorded state).
    #[must_use]
    pub fn audio_context_state(&self) -> String {
        match &self.live {
            Some(e) => e.audio_context_state(),
            None => String::new(),
        }
    }

    /// 09 fix — re-resume the live context from a user gesture (the
    /// rejection case: a context built once outside an activation stays
    /// suspended; each new gesture gets a resume() attempt). No-op on
    /// the fallback clock / other hosts.
    pub fn replay_context(&mut self) {
        if let Some(e) = &self.live {
            e.replay_context();
        }
    }

    /// The deterministic no-device director (the fallback clock only) —
    /// the tests' entry: parallel test threads opening live audio devices
    /// concurrently crash the Windows WASAPI host (the flaky
    /// STATUS_ACCESS_VIOLATION in the CI windows gate); the fallback
    /// director never touches a device on any platform.
    #[must_use]
    pub fn new_fallback() -> Self {
        let score = Score::default();
        let stage = 0u32;
        let p = score.stage_params(stage);
        Self {
            tracks: Self::build_tracks(stage, &score),
            live: None,
            stage,
            score,
            bpm: p.bpm,
            brightness: p.brightness,
            last_bar: None,
            last_step: None,
            duck_recover: DUCK_RECOVER_S,
            fallback_t: 0.0,
        }
    }

    fn build_tracks(stage: u32, score: &Score) -> Vec<Track> {
        let mut tracks: Vec<Track> = SLOTS
            .iter()
            .enumerate()
            .map(|(i, slot)| Track::new(i, slot.kind.label(), slot.kind, 0.0))
            .collect();
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(&mut tracks[i], slot, i, stage, score);
        }
        tracks
    }

    fn apply_slot(track: &mut Track, slot: &Slot, index: usize, stage: u32, score: &Score) {
        let s = stage.min(score.bpm.cap) as f32;
        let p = score.stage_params(stage);
        let pms = track.params.clone();
        pms.mute
            .set_value(if stage >= slot.unlock { 0.0 } else { 1.0 });
        pms.gain.set_value(score.slots[index].gain);
        pms.arp.set_value(0.0);
        pms.life_mod
            .set_value(0.85 + 0.15 * (s / (score.bpm.cap as f32)));
        pms.freq.set_value(p.root() * slot.ratio);

        match slot.kind {
            PresetKind::Heartbeat => {
                let bits = match index {
                    0 => score.drum_pattern(&score.kick, stage),
                    1 => score.drum_pattern(&score.snare, stage),
                    _ => score.drum_pattern(&score.hat, stage),
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
        let p = self.score.stage_params(stage);
        self.bpm = p.bpm;
        self.brightness = p.brightness;
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(&mut self.tracks[i], slot, i, stage, &self.score);
        }
        // Re-apply the current bar/step on the next tick.
        self.last_bar = None;
        self.last_step = None;
        if let Some(e) = &self.live {
            e.global.bpm.set_value(p.bpm);
            e.global.brightness.set_value(p.brightness);
            e.global
                .scale_mode
                .set_value(if self.score.minor_for_stage(stage) {
                    1.0
                } else {
                    0.0
                });
        }
    }

    /// Hot-swap the chiptune score in place (the score lab's LIVE path —
    /// the wasm `setScore` export's director half): re-apply the 8 slots'
    /// gains + the drum patterns from the new score, re-derive the
    /// stage's bpm/brightness, re-arm the bar/step writes (the next
    /// tick rewrites the chord/notes from the new score), and re-push
    /// the live globals. NO graph rebuild — Shared writes only, so the
    /// swap is click-free (the one-mixer contract) and effective within
    /// one frame.
    pub fn set_score(&mut self, score: Score) {
        self.score = score;
        let p = self.score.stage_params(self.stage);
        self.bpm = p.bpm;
        self.brightness = p.brightness;
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(&mut self.tracks[i], slot, i, self.stage, &self.score);
        }
        // Re-apply the current bar/step on the next tick (from the
        // new score).
        self.last_bar = None;
        self.last_step = None;
        if let Some(e) = &self.live {
            e.global.bpm.set_value(p.bpm);
            e.global.brightness.set_value(p.brightness);
            e.global
                .scale_mode
                .set_value(if self.score.minor_for_stage(self.stage) {
                    1.0
                } else {
                    0.0
                });
        }
    }

    /// The director's current score (the lab's read half; `score()`'s
    /// JSON is the `getScore` export's return value).
    #[must_use]
    pub fn score(&self) -> &Score {
        &self.score
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

    /// Drive the step sequencer from the mono clock `t` (seconds):
    /// the step/bar writes on change, then the GATE every tick — the
    /// staccato window (a note plays only `gate_open` of its step) with
    /// the odd 16ths swung late (`swing` of a step), which is what turns
    /// the grid into a groove. The gate/swing/steps come from
    /// `self.score` (a non-default score moves the window —
    /// `set_score_takes_effect_live` proves it).
    fn sequence(&mut self, t: f32) {
        let step_len = 60.0 / self.bpm / 4.0;
        let step_f = t.max(0.0) / step_len;
        let step = step_f as u64;
        let in_step = step_f - step as f32;
        let steps = self.score.steps_per_bar as u64;
        let bar = step / steps;
        if self.last_bar != Some(bar) {
            self.apply_bar(bar);
            self.last_bar = Some(bar);
        }
        if self.last_step != Some(step) {
            self.apply_step(step % steps, bar);
            self.last_step = Some(step);
        }

        let idx = (step % steps) as usize;
        let window = if step % 2 == 1 {
            // the swung 16th: the attack lands late; the window shifts
            // with it (and may run to the next step's edge)
            self.score.swing..(self.score.swing + self.score.gate_open).min(1.0)
        } else {
            0.0..self.score.gate_open
        };
        let on = window.contains(&in_step);
        self.gate_voice(3, self.score.bass.notes[idx].is_some() && on);
        self.gate_voice(4, self.score.lead.notes[idx].is_some() && on);
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
        let p = self.score.stage_params(self.stage);
        let offset = self.score.chord_offset(self.stage, bar);
        for (i, slot) in SLOTS.iter().enumerate() {
            if slot.chord {
                self.tracks[i]
                    .params
                    .freq
                    .set_value(p.root() * slot.ratio * semitone_ratio(offset));
            }
        }
    }

    /// Write the step's bass/lead note (rests gate with `mute`).
    fn apply_step(&mut self, step: u64, bar: u64) {
        let p = self.score.stage_params(self.stage);
        let offset = self.score.chord_offset(self.stage, bar);
        let idx = (step % self.score.steps_per_bar as u64) as usize;

        // Bass (slot 3): the chord root/fifth/octave line, gated per
        // step. The octave is data (`bass.octave_shift`: -1 in the
        // default score — the legacy `root * 0.5`): `2^octave_shift`.
        let bass = &self.tracks[3].params;
        if let Some(note) = self.score.bass.notes[idx] {
            bass.freq.set_value(
                p.root()
                    * 2.0_f32.powf(self.score.bass.octave_shift as f32)
                    * semitone_ratio(offset + note),
            );
            bass.mute.set_value(0.0);
        } else {
            bass.mute.set_value(1.0);
        }

        // Lead (slot 4): the fixed motif on the key root, one octave up
        // (`lead.octave_shift`: +1 — the legacy `root * 2.0`),
        // deliberately NOT chord-following.
        let lead = &self.tracks[4].params;
        if let Some(note) = self.score.lead.notes[idx] {
            lead.freq.set_value(
                p.root() * 2.0_f32.powf(self.score.lead.octave_shift as f32) * semitone_ratio(note),
            );
            lead.mute.set_value(0.0);
        } else {
            lead.mute.set_value(1.0);
        }
    }

    pub fn stop(&mut self) {
        self.live = None;
    }

    /// The one-mixer plan's web render half (09): render `out`
    /// (interleaved stereo `f32`, an even length) from the live
    /// in-memory engine (the emscripten `start_on_gesture` engine —
    /// desktop/Android live engines are cpal streams and never render
    /// through this). The engine's phase clock (the `BeatView`'s time
    /// source) advances with the rendered frames — the render pass
    /// drives the clock, same as today's audio callback; the web's
    /// frame timer queues the frames into the SFX OpenAL context's
    /// streaming source (ONE context for the SFX and the music). `false`
    /// on the fallback clock (nothing to render).
    pub fn render_stereo(&self, out: &mut [f32]) -> bool {
        if let Some(e) = &self.live {
            e.render_stereo(out);
            true
        } else {
            false
        }
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
        // The one-mixer plan (09): on emscripten the live engine is the
        // IN-MEMORY half (no cpal stream, no private WebAudio context) —
        // its frames feed the SFX device's OpenAL streaming source
        // (th-app's web frame timer queues them; the phase clock the
        // `BeatView` consumes advances in `render_stereo`).
        // Desktop/Android keep the cpal device stream (the rodio-`Source`
        // half — the synth as a rodio source on ONE device stream — is the
        // plan's deferred item).
        #[cfg(target_os = "emscripten")]
        let started = AudioEngine::start_in_memory(tracks);
        #[cfg(not(target_os = "emscripten"))]
        let started = AudioEngine::start(tracks);
        match started {
            Ok(handle) => {
                handle.global.bpm.set_value(self.bpm);
                handle.global.brightness.set_value(self.brightness);
                handle
                    .global
                    .scale_mode
                    .set_value(if self.score.minor_for_stage(self.stage) {
                        1.0
                    } else {
                        0.0
                    });
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
    use rust_synth::audio::engine::IN_MEMORY_SAMPLE_RATE;

    use super::*;
    use crate::score::{Score, SlotGain};
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

    /// The 10a fix-pass OCTAVE contract (issue 10a report, FIX PASS 1):
    /// the director's bass/lead step writes keep the legacy octaves —
    /// bass `root * 2^(-1)` over the bar's chord offset (the legacy
    /// `root * 0.5 * semitone(offset + note)`), lead `root * 2^(1)` on
    /// the key root with NO chord offset (the legacy `root * 2.0 *
    /// semitone(note)`) — driven through a real fallback director:
    /// set the stage, tick to a known bar/step, read `tracks[3]`/
    /// `tracks[4]`'s `params.freq`/`mute`.
    #[test]
    fn the_bass_and_lead_write_the_legacy_octaves() {
        let score = Score::default();
        assert_eq!(score.bass.octave_shift, -1, "the baked bass octave");
        assert_eq!(score.lead.octave_shift, 1, "the baked lead octave");

        let stage = 3u32; // fourths = 1: the root a fourth above A2
        let p = stage_params(stage);
        let root = ROOT_A2_HZ * ROOT_SHIFT.powi(p.fourths as i32);
        let step0_bass_note = 0i32; // BASS_LINE[0]
        let step0_lead_note = 12i32; // LEAD_LINE[0]
        let bar0_offset = 0i32; // CHORD_MAJOR[0] (I)
        let bar1_offset = 9i32; // CHORD_MAJOR[1] (vi)

        let mut d = MusicDirector::new_fallback();
        d.set_stage(stage);
        d.tick(0.01); // bar 0, step 0 — an even step, in-step < GATE_OPEN
        let bass0 = root * 2.0_f32.powi(-1) * semitone_ratio(bar0_offset + step0_bass_note);
        let lead0 = root * 2.0_f32.powi(1) * semitone_ratio(step0_lead_note);
        assert_eq!(d.tracks[3].params.mute.value(), 0.0, "bass step 0 is on");
        assert!(
            (d.tracks[3].params.freq.value() - bass0).abs() < 1e-4,
            "bass writes root * 2^(-1) * semitone(offset + note): got {}, want {bass0}",
            d.tracks[3].params.freq.value()
        );
        assert_eq!(d.tracks[4].params.mute.value(), 0.0, "lead step 0 is on");
        assert!(
            (d.tracks[4].params.freq.value() - lead0).abs() < 1e-4,
            "lead writes root * 2^(1) * semitone(note) (no chord offset): got {}, want {lead0}",
            d.tracks[4].params.freq.value()
        );

        // Bar 1, step 0: the bass follows the chord (vi: +9 semitones),
        // the lead stays on the key root (the same freq as bar 0).
        let step_len = 60.0 / p.bpm / 4.0;
        d.tick(16.0 * step_len + 0.3 * step_len - 0.01); // t lands at bar 1, in-step 0.3
        let bass1 = root * 2.0_f32.powi(-1) * semitone_ratio(bar1_offset + step0_bass_note);
        assert_eq!(
            d.tracks[3].params.mute.value(),
            0.0,
            "bass step 0 of bar 1 is on"
        );
        assert!(
            (d.tracks[3].params.freq.value() - bass1).abs() < 1e-4,
            "the bass follows the bar's chord: got {}, want {bass1}",
            d.tracks[3].params.freq.value()
        );
        assert!(
            (d.tracks[4].params.freq.value() - lead0).abs() < 1e-4,
            "the lead ignores the chord (still {lead0}, got {})",
            d.tracks[4].params.freq.value()
        );
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

    /// The score lab's LIVE path (10a): a `set_score` swap takes effect
    /// on the next tick with no graph rebuild — a rest swapped into the
    /// bass's step 0 gates the bass, and a moved `swing` moves the
    /// swung-note window's timing.
    #[test]
    fn set_score_takes_effect_live() {
        // Rest half: the default score's step-0 bass note plays...
        let mut d = MusicDirector::new_fallback();
        d.tick(0.01); // step 0: the default bass's note 0 is on
        assert_eq!(d.tracks[3].params.mute.value(), 0.0);
        // ...until the swapped-in score makes step 0 a rest (and swings
        // the odd 16ths much later — the window-timing half below).
        let mut score = Score::default();
        score.bass.notes[0] = None;
        d.set_score(Score {
            swing: 0.5,
            ..score
        });
        d.tick(0.001); // still step 0, re-applied from the new score
        assert_eq!(
            d.tracks[3].params.mute.value(),
            1.0,
            "the swapped rest gates the bass on the next tick"
        );

        // Swing half: a fresh default director against a fresh
        // swapped-score (swing 0.5) director, identical ticks — both
        // reach t ≈ 0.325: step 3 (odd, swung; the default bass's note
        // 12) at in-step ≈ 0.25. The default swing's window
        // (0.15..0.85) is open there; the 0.5 swing's (0.5..1.0) is
        // still closed.
        let mut def = MusicDirector::new_fallback();
        let mut sw = MusicDirector::new_fallback();
        let swapped = Score {
            swing: 0.5,
            ..Score::default()
        };
        sw.set_score(swapped);
        def.tick(0.30);
        sw.tick(0.30);
        def.tick(0.025);
        sw.tick(0.025);
        assert_eq!(
            def.tracks[3].params.mute.value(),
            0.0,
            "the default swing's window (0.15..0.85) is open at in-step 0.25"
        );
        assert_eq!(
            sw.tracks[3].params.mute.value(),
            1.0,
            "the swapped 0.5 swing's window (0.5..1.0) is still closed"
        );
    }

    /// A non-default score's SLOTS gains take effect through
    /// `set_score` (the 8-slot mix is data, not just the lines).
    #[test]
    fn set_score_rewrites_the_slot_gains() {
        let mut d = MusicDirector::new_fallback();
        let before = d.tracks[0].params.gain.value();
        let mut score = Score::default();
        score.slots[0] = SlotGain { gain: 0.3 };
        d.set_score(score);
        assert_eq!(d.tracks[0].params.gain.value(), 0.3);
        assert!(before > 0.3);
        // The drum patterns re-apply too: an all-off kick tier.
        score = Score::default();
        score.kick.tiers[0].pattern = ".".repeat(score.steps_per_bar as usize);
        d.set_score(score);
        assert_eq!(
            d.tracks[0].params.pattern_bits.load(Ordering::Relaxed),
            0,
            "the swapped-in silent kick pattern lands in the atomics"
        );
    }

    /// The `set_score` re-arm pair (the review's uncovered lines): the
    /// cleared `last_bar`/`last_step` are what make the next tick
    /// re-write the current bar's chord frequencies (`apply_bar`) and
    /// the current step's note frequencies (`apply_step`). The mutes
    /// cannot pin this — `sequence()`'s gate_voice rewrites them every
    /// tick, masking a missed re-apply. Driven through a real fallback
    /// director: tick to step 0 (the default bass's note 0 is on),
    /// swap in a score that moved that note AND the bar's chord, and
    /// within the SAME step (a tiny dt — the step index does not
    /// advance) both freqs must already be the new values; without the
    /// re-arm the already-visited bar/step are skipped and both stay
    /// stale.
    #[test]
    fn set_score_reapplies_the_current_step_and_bar() {
        let mut d = MusicDirector::new_fallback();
        d.tick(0.01); // bar 0, step 0: the default bass's note 0, chord I
        let root = stage_params(0).root();
        assert!(
            (d.tracks[3].params.freq.value() - root * 2.0_f32.powi(-1)).abs() < 1e-4,
            "the pre-swap bass is the default score's step-0 note"
        );
        assert!(
            (d.tracks[5].params.freq.value() - root).abs() < 1e-4,
            "the pre-swap pad is the bar-0 chord (I, offset 0)"
        );

        // A moved step note (0 → +5) and a moved bar-0 chord (0 → +7):
        // the same score otherwise.
        let mut score = Score::default();
        score.bass.notes[0] = Some(5);
        score.chords.major[0] = 7;
        d.set_score(score);

        d.tick(0.001); // still step 0 — the step index does not advance
        let new_bass = root * 2.0_f32.powi(-1) * semitone_ratio(7 + 5);
        assert!(
            (d.tracks[3].params.freq.value() - new_bass).abs() < 1e-4,
            "the step's note freq re-applies within the same step (got {}, want {new_bass})",
            d.tracks[3].params.freq.value()
        );
        let new_pad = root * semitone_ratio(7);
        assert!(
            (d.tracks[5].params.freq.value() - new_pad).abs() < 1e-4,
            "the bar's chord freq re-applies within the same bar (got {}, want {new_pad})",
            d.tracks[5].params.freq.value()
        );
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

    /// The one-mixer plan's web half (09): the in-memory engine (no
    /// device — CI-safe, unlike `AudioEngine::start`'s cpal open) renders
    /// the director's stage-2 score through `render_stereo`: nonzero
    /// output, and the peaks / phase clock driven by the render pass
    /// (the `BeatView`'s time source).
    #[test]
    fn in_memory_engine_renders_the_score() {
        let mut d = MusicDirector::new_fallback();
        d.set_stage(2); // unlocks the bell — more voices than stage 0
        let tracks: Vec<Track> = d
            .tracks
            .iter()
            .map(|t| Track {
                id: t.id,
                name: t.name.clone(),
                kind: t.kind,
                params: t.params.clone(),
            })
            .collect();
        let handle = AudioEngine::start_in_memory(tracks).expect("in-memory start");
        handle.global.bpm.set_value(d.bpm);
        handle.global.brightness.set_value(d.brightness);
        handle
            .global
            .scale_mode
            .set_value(scale_mode_for_stage(d.stage));
        handle.global.master_gain.set_value(MASTER_GAIN);
        d.live = Some(handle);

        let mut out = vec![0.0f32; (IN_MEMORY_SAMPLE_RATE as usize) * 2]; // one second
        assert!(
            d.render_stereo(&mut out),
            "the live in-memory engine renders"
        );
        let peak = out.iter().fold(0.0f32, |p, &s| p.max(s.abs()));
        assert!(
            peak > 0.0,
            "the stage-2 score must render audible output, peak {peak}"
        );
        // The render pass drove the clock: ~1 s of frames → ~1 s of time
        // (the `BeatView`'s time source, the web frame timer's contract).
        let clock = d.live.as_ref().expect("live").phase_clock.value();
        assert!(
            (clock - 1.0).abs() < 1e-3,
            "phase clock advanced by the render: {clock}"
        );
    }

    #[test]
    fn render_stereo_is_false_on_the_fallback_clock() {
        let d = MusicDirector::new_fallback();
        let mut out = vec![0.0f32; 512];
        assert!(
            !d.render_stereo(&mut out),
            "the fallback clock has no engine to render"
        );
    }
}
