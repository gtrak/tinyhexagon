//! The `MusicDirector` (05-synth.md "Director API"): the app-facing surface
//! over the `rust-synth` engine.
//!
//! The director owns the 8 fixed tracks and, while live, the
//! [`EngineHandle`]. The engine takes its `Vec<Track>` by value; the
//! `TrackParams` we hand it are clones whose `Shared` fields are
//! `Arc<AtomicU32>` — so the live DSP graph reads the *same* atomics the
//! director mutates here: `set_stage`/`bump`/`tick` only write `Shared`
//! params and never touch the graph (no `rebuild_graph`, no clicks on
//! stage changes). `set_score` is the same for its Shared-only halves
//! (gains, patterns, the baked `muted`/`solo`) — with ONE documented
//! exception (the plan 11 instrument swap): a changed `slots[i].kind`
//! mutates the live engine's track kind and calls `rebuild_graph` ONCE
//! (the vendor's swap path — its reverb-tail discontinuity is accepted
//! for an explicit user action, never on a plain gain/mute edit).
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
//!
//! # The plan 11c role-based timing grid + the FM bake
//!
//! Issue 11c moved `pattern_bits` OUT of `apply_slot`'s Heartbeat arm
//! (where only the drum slots got it) into the ROLE logic: slots 0-2
//! always get their stage-tiered drum patterns (regardless of kind),
//! slots 3/4 (bass/lead) get their OWN note lines (bit i iff the line's
//! step i has a note), and the harmony slots keep the 4-hit Euclidean
//! default. That fixes the melodic slots' `step_is_active`-driven
//! envelopes (PluckSaw's pluck, and now the FM voices') firing on the
//! 4-hit Euclidean default instead of their own notes. The shipped
//! `assets/score.json` re-timbres the roles to the vendor's new FM kit
//! (`fm_kick`/`fm_snare`/`fm_hat`/`fm_bass`/`fm_lead` — the
//! `SLOTS` built-ins stay Heartbeat/BassPulse/PluckSaw, the
//! backwards-compat path for scores without `kind` fields).
//!
//! # The plan 11f `Role` enum (replaces `Slot.chord` + the index hacks)
//!
//! Issue 11f made the role an ENUM — `Role::{Drum, Bass, Lead, Comp,
//! Harmony}` — instead of the `Slot.chord` bool + the `index < 3` drum
//! special-casing (a latent bug source: the drum tier selection and the
//! bass/lead line selection both keyed on the slot's INDEX). The
//! `apply_slot`/`apply_bar` branches now match on `slot.role`, so a
//! slot's driver is data, not arithmetic. The new `Role::Comp` (slot 5
//! — the pad gives way) is the Genesis arrangement's chordal stab:
//! chord-following PITCH (the bar's chord root × the slot's ratio,
//! written in `apply_bar` alongside the harmony slots) plus a hit
//! PATTERN (the score's `comp` step pattern, written to
//! `pattern_bits` — the FM core's `step_is_active` reads it per-sample,
//! so the pattern gates the envelope and no staccato-gate change is
//! needed for the sound; the mute/solo fold covers it like every other
//! slot). The existing roles' observable behaviour is UNCHANGED (the
//! `existing_roles_are_unchanged` + 11c's `the_pattern_bits_follow_the
//! _role_not_the_kind` tests pin it); `PresetKind::FmComp` (the seventh
//! FM voice — an electric-piano stab) is APPENDED to the vendor's
//! enum, so `ALL_KINDS`' existing cycle order and the shipped scores'
//! ids are untouched.
//!
//! # The plan 11g phrase (the per-bar figures — the song form)
//!
//! Issue 11g made the melody more than one bar: `Score.bars` is the
//! phrase — per-bar OVERRIDES of the top-level `bass`/`lead`/`comp`
//! figures, resolved modulo `bars.len()` over the ABSOLUTE bar counter
//! (the chord progression keeps its OWN cycle — `chord_offset`'s
//! `bars_per_chord` + table arithmetic over the same absolute bar — so
//! an 8-bar phrase over a 4-chord progression cycles the chords twice;
//! the two are NOT coupled). A `None` field falls back to the
//! top-level figure; an EMPTY `bars` list is "no phrase": one bar
//! whose figures are the top-level lines, exactly today's behaviour
//! (the backwards-compat contract; the shipped bake is the reference,
//! and it carries no `bars`). The sequencer resolves the bar NOW
//! PLAYING (`Score::resolve_bar`): `apply_step` writes the resolved
//! bass/lead lines' note frequencies, the staccato gate's note-present
//! half reads the resolved lines (a phrase bar that rests where the
//! top-level line plays stays gated — `the_gate_reads_the_resolved_
//! bar` pins it), and `apply_bar` — called exactly when the bar
//! changes — RE-FRESHES the bass/lead/comp `pattern_bits` from the
//! resolved figures: a stale grid (the default bar's bits left on a
//! later bar) is the subtle bug that would make a phrase sound like
//! one bar (`bars_resolve_modulo_and_fall_back` +
//! `the_phrase_actually_changes_what_plays` pin it).
//!
//! # The plan 11 effective-mute fold (mute/solo + the instrument kinds)
//!
//! A slot's role (fixed by index) and its timbre (selectable — the
//! score's `slots[i].kind`, or the built-in `SLOTS[i].kind` when `None`)
//! are split: the role's driver keeps sequencing the slot (the drum
//! tiers, the bass/lead lines, the bar chord); the kind only changes
//! the timbre's parameters (character/LFO/reverb). Mute is the ONE
//! exception to the "Shared writes only" rule above in its USER half:
//! `sequence()` writes EVERY slot's effective mute EVERY tick — the
//! program state (`program_mute`: the stage unlock for drums/harmony,
//! the staccato gate for bass/lead) ORed with the user state (the
//! baked `muted`, a baked `solo` silencing every non-soloed slot). A
//! user mute written once thus survives the gate's every-tick rewrites
//! (`user_mute_survives_the_gate` proves the regression is gone), and a
//! mix change is audible within one tick.

use rust_synth::audio::preset::{LFO_CUTOFF, LFO_FREQ};
use rust_synth::audio::{AudioEngine, EngineHandle, PresetKind, Track};
use std::sync::atomic::Ordering;

use crate::beat::{bar_phase, beat_phase, energy, BeatView};
use crate::score::{kind_from_id, Score};

/// `StageParams` is defined in `score.rs` (it carries the score's
/// `root_hz`/`root_shift`); re-exported here so
/// `th_synth::director::StageParams` keeps resolving.
pub use crate::score::StageParams;

/// The slot's ROLE (plan 002 issue 11f — replaces the `Slot.chord`
/// bool + the `index < 3` drum special-casing): the driver that
/// sequences the slot, INDEPENDENT of the slot's selectable KIND (the
/// timbre — the role/timbre split the plan 11 instrument swap rests
/// on: a swapped kind keeps the role's driver, takes the new preset's
/// parameters).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Fixed pitch (ratio-based), gated by the score's kick/snare/hat
    /// tiers (slots 0-2: the drums' `pattern_bits` come from the
    /// stage's tier, the pitch from the slot's ratio).
    Drum,
    /// Step-sequenced notes from the bass line; follows the bar's chord
    /// root (slot 3 — the note's step gates the voice with `mute`).
    Bass,
    /// Step-sequenced notes from the lead line; on the KEY root
    /// (deliberately NOT chord-following, slot 4).
    Lead,
    /// THE NEW ROLE (issue 11f, slot 5 — the Genesis comp):
    /// chord-following PITCH (the bar's chord root × the slot's ratio,
    /// written in `apply_bar` alongside the harmony slots) plus a hit
    /// PATTERN (the score's `comp` step pattern written to
    /// `pattern_bits` — the FM envelope's gate).
    Comp,
    /// Chord-following and sustained (no step gating): pad/bell/sub
    /// (slots 6-7 — they follow the bar's chord, not the steps).
    Harmony,
}

/// One row of the fixed 8-slot track map (the synthesis design: the
/// preset kind, the unlock stage, the ratio to the slot's root, and the
/// role. The slot's MIX GAIN is score data — `Score.slots` — since 10a:
/// one source, no drift).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub kind: PresetKind,
    pub unlock: u32,
    pub ratio: f32,
    pub role: Role,
}

/// The 8 slots in engine order (plan 002 issue 11f's map: slot 5 is
/// now the new `Comp` role on the new `FmComp` voice — the
/// pad's kind/role moves out, the comp is rhythm so it unlocks at 0;
/// slots 6/7 keep `Harmony`, and the eight-voice `MAX_TRACKS` contract
/// stands).
pub const SLOTS: [Slot; 8] = [
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 0.5,
        role: Role::Drum,
    }, // kick
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 1.6,
        role: Role::Drum,
    }, // snare
    Slot {
        kind: PresetKind::Heartbeat,
        unlock: 0,
        ratio: 8.0,
        role: Role::Drum,
    }, // hat
    Slot {
        kind: PresetKind::BassPulse,
        unlock: 0,
        ratio: 0.5,
        role: Role::Bass,
    }, // bass
    Slot {
        kind: PresetKind::PluckSaw,
        unlock: 0,
        ratio: 2.0,
        role: Role::Lead,
    }, // lead
    Slot {
        kind: PresetKind::FmComp,
        unlock: 0,
        ratio: 1.0,
        role: Role::Comp,
    }, // the comp (was the PadZimmer pad)
    Slot {
        kind: PresetKind::Bell,
        unlock: 2,
        ratio: 2.0,
        role: Role::Harmony,
    }, // bell
    Slot {
        kind: PresetKind::DroneSub,
        unlock: 0,
        ratio: 0.25,
        role: Role::Harmony,
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
    /// The per-slot PROGRAM mute state (the plan 11 fold's program half):
    /// the stage-unlock state for drums/harmony (recorded by
    /// `apply_slot`), the staccato gate for bass/lead (recorded by
    /// `sequence`'s `gate_voice`). `fold_mutes` is the ONLY `mute`
    /// writer: program OR the user state (baked `muted`/`solo`), every
    /// tick.
    program_mute: [bool; 8],
    /// The count of graph rebuilds caused by kind swaps (test/
    /// instrumentation surface — mirroring how `tracks`/`score()` are
    /// exposed for tests): incremented by `apply_kinds` exactly where it
    /// calls the live engine's `rebuild_graph()`, i.e. only when a
    /// slot's kind actually changed while live. A plain gain/mute/solo
    /// edit never advances it, and construction never does (a fresh
    /// engine is built with the resolved kinds in `build_tracks`, no
    /// `rebuild_graph` call).
    kind_rebuilds: u32,
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
        let (tracks, program_mute) = Self::build_tracks(stage, &score);
        let mut d = Self {
            tracks,
            live: None,
            stage,
            score,
            program_mute,
            kind_rebuilds: 0,
            bpm: p.bpm,
            brightness: p.brightness,
            last_bar: None,
            last_step: None,
            duck_recover: DUCK_RECOVER_S,
            fallback_t: 0.0,
        };
        // The initial mute state (the old `apply_slot`'s mute write —
        // now the fold's job: the locked pad/bell start muted).
        d.fold_mutes();
        d
    }

    fn build_tracks(stage: u32, score: &Score) -> (Vec<Track>, [bool; 8]) {
        // The tracks are CONSTRUCTED with the score's RESOLVED kind/label
        // (the plan 11 fresh-start path: a swapped instrument is in the
        // track at construction — the live-swap path mutates the
        // engine's EXISTING tracks instead, `apply_kinds`).
        let mut program_mute = [false; 8];
        let mut tracks: Vec<Track> = SLOTS
            .iter()
            .enumerate()
            .map(|(i, _slot)| {
                let kind = Self::resolved_kind(score, i);
                Track::new(i, kind.label(), kind, 0.0)
            })
            .collect();
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(&mut tracks[i], slot, i, stage, score, &mut program_mute);
        }
        (tracks, program_mute)
    }

    fn apply_slot(
        track: &mut Track,
        slot: &Slot,
        index: usize,
        stage: u32,
        score: &Score,
        program_mute: &mut [bool; 8],
    ) {
        let s = stage.min(score.bpm.cap) as f32;
        let p = score.stage_params(stage);
        let pms = track.params.clone();
        // The stage-unlock PROGRAM state (the fold's program half —
        // recorded, NOT written: `fold_mutes` is the only `mute` writer
        // now; the bass/lead re-record theirs every tick in `sequence`
        // instead of this unlock state).
        program_mute[index] = stage < slot.unlock;
        pms.gain.set_value(score.slots[index].gain);
        pms.arp.set_value(0.0);
        pms.life_mod
            .set_value(0.85 + 0.15 * (s / (score.bpm.cap as f32)));
        pms.freq.set_value(p.root() * slot.ratio);

        // The timing grid (`pattern_bits`) is a ROLE property, not a
        // kind's (issue 11c's fix — the old code set it only inside the
        // Heartbeat arm, so a melodic slot's `step_is_active`-driven
        // envelope fired on `default_for`'s 4-hit Euclidean pattern,
        // unrelated to that line's notes); issue 11f made the ROLE the
        // enum that selects it (the `index < 3` drum special-casing is
        // gone): the drum slots ALWAYS get their stage-tiered pattern
        // (any preset on a drum slot keeps the role's grid), the
        // bass/lead get their OWN note lines (bit i iff the line's
        // step i has a note), the comp gets the score's `comp` step
        // pattern (issue 11f — the pattern gates the FM envelope; the
        // chord-following pitch is `apply_bar`'s job), and the
        // harmony slots keep the 4-hit Euclidean default (they follow
        // the chord, not the steps). Note lines longer than 32 steps:
        // indices >= 32 do not fit the u32 grid (`Score::
        // drum_pattern`'s documented convention).
        match slot.role {
            Role::Drum => {
                let bits = match index {
                    0 => score.drum_pattern(&score.kick, stage),
                    1 => score.drum_pattern(&score.snare, stage),
                    _ => score.drum_pattern(&score.hat, stage),
                };
                pms.pattern_hits.set_value(bits.count_ones() as f32);
                pms.pattern_rotation.set_value(0.0);
                pms.pattern_bits.store(bits, Ordering::Relaxed);
            }
            Role::Bass | Role::Lead => {
                let notes = match slot.role {
                    Role::Bass => &score.bass.notes,
                    _ => &score.lead.notes,
                };
                let bits = Self::note_line_bits(notes);
                pms.pattern_hits.set_value(bits.count_ones() as f32);
                pms.pattern_rotation.set_value(0.0);
                pms.pattern_bits.store(bits, Ordering::Relaxed);
            }
            Role::Comp => {
                // The comp's pattern (the score's `comp`, or the
                // off-beat default when the field is absent —
                // `comp_pattern_bits`' backwards-compat fallback).
                let bits = score.comp_pattern_bits();
                pms.pattern_hits.set_value(bits.count_ones() as f32);
                pms.pattern_rotation.set_value(0.0);
                pms.pattern_bits.store(bits, Ordering::Relaxed);
            }
            Role::Harmony => {} // the `default_for` Euclidean default stands
        }

        // The per-preset parameters key on the slot's RESOLVED kind
        // (the plan 11 role/timbre split: a swapped instrument keeps the
        // role's driver — the tiered drum pattern / the step line / the
        // bar chord — but takes the NEW preset's character/LFO/reverb
        // values).
        match Self::resolved_kind(score, index) {
            PresetKind::Heartbeat => {
                // The drum-role slots' character is the slot's tuning
                // (kick soft, snare clicky, hat snappy); a Heartbeat
                // swapped INTO another slot takes the neutral value. Its
                // `pattern_bits` come from the ROLE block above — never
                // from this arm (the issue 11c fix).
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(match index {
                    0 => 0.30,
                    1 => 0.85,
                    2 => 0.95,
                    _ => 0.5, // the swapped-in Heartbeat: the neutral character
                });
                pms.reverb_mix.set_value(0.05);
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
            // Plan 002 issue 11c — the FM kit (the auditioned values;
            // the `audition_render` example's table is how they were
            // chosen). `resonance` is the FM depth (the modulation
            // index's scale); `character` 0.5 = each voice's tuned
            // ratio. Issue 11e: `reverb_mix` is now a true WET AMOUNT
            // in [0, 1] (0 = bone dry) — it scales the voice's reverb
            // (wet) branch ONLY; the dry signal is re-added by the
            // preset. The pre-11e "level trim" meaning was the bug
            // (0.8–0.9 of wet, NO dry at all); re-auditioned after the
            // fix: drums 0.15–0.30 of wet (the attack must dominate —
            // the atk/tail column), the bass dry-ish, lead/bell more
            // air. Each of these slots' `pattern_bits` comes from the
            // ROLE block above (the tiered drum patterns, the
            // bass/lead note lines) — the step-gated FM envelope needs
            // it.
            PresetKind::FmKick => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.50);
                pms.resonance.set_value(0.50);
                pms.reverb_mix.set_value(0.25); // 11e re-audit: a roomy low tail, attack first
            }
            PresetKind::FmSnare => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.60); // a touch more noise in the balance
                pms.resonance.set_value(0.50);
                pms.reverb_mix.set_value(0.20); // 11e re-audit: punch over room
            }
            PresetKind::FmHat => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.50);
                pms.resonance.set_value(0.35);
                pms.reverb_mix.set_value(0.15); // 11e re-audit: tight, almost dry
            }
            PresetKind::FmBass => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.50);
                pms.resonance.set_value(0.50);
                pms.cutoff.set_value(500.0);
                pms.reverb_mix.set_value(0.15); // 11e re-audit: dry bass weight
            }
            PresetKind::FmLead => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.50);
                pms.resonance.set_value(0.35);
                pms.detune.set_value(14.0);
                pms.reverb_mix.set_value(0.30); // 11e re-audit: melodic air
            }
            PresetKind::FmBell => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.06);
                pms.lfo_rate.set_value(0.15);
                pms.lfo_target.set_value(LFO_FREQ as f32);
                pms.character.set_value(0.50);
                pms.resonance.set_value(0.40);
                pms.reverb_mix.set_value(0.35); // 11e re-audit: marimba room
            }
            // Plan 002 issue 11f — the Genesis comp (the arrangement's
            // chordal stab): the placeholder row's parameters (the
            // composition boundary: the mechanism + its audibility are
            // the deliverable, the actual part/balance is the
            // operator's). `character` 0.5 = the voice's tuned ratio
            // (2.0); `resonance` 0.50 = a moderate index (0.5 × 450 Hz
            // at the hit, decaying fast); the comp's `pattern_bits`
            // come from the ROLE block above (the score's `comp`
            // pattern) — the step-gated FM envelope needs it, exactly
            // like the other FM voices.
            PresetKind::FmComp => {
                pms.pulse_depth.set_value(0.0);
                pms.lfo_depth.set_value(0.0);
                pms.character.set_value(0.50);
                pms.resonance.set_value(0.50);
                pms.reverb_mix.set_value(0.25); // between the bass (0.15) and the lead (0.30) — a moderate air
            }
        }
    }

    /// The note line's `pattern_bits`: bit i set iff the line's step i
    /// has a note (issue 11c's role-based grid for the bass/lead — the
    /// `step_is_active`-driven envelopes now fire on the line's actual
    /// notes). Indices >= 32 do not fit the u32 grid (`Score::
    /// drum_pattern`'s documented convention).
    fn note_line_bits(notes: &[Option<i32>]) -> u32 {
        notes.iter().enumerate().fold(0u32, |bits, (i, note)| {
            if note.is_some() && i < 32 {
                bits | (1 << i)
            } else {
                bits
            }
        })
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
            Self::apply_slot(
                &mut self.tracks[i],
                slot,
                i,
                stage,
                &self.score,
                &mut self.program_mute,
            );
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
        // The stage-unlock fold (the old `apply_slot`'s mute write —
        // now the fold's job): a layer unlock/lock lands immediately.
        self.fold_mutes();
    }

    /// Hot-swap the chiptune score in place (the score lab's LIVE path —
    /// the wasm `setScore` export's director half): re-apply the 8 slots'
    /// gains + the drum patterns from the new score, re-derive the
    /// stage's bpm/brightness, re-arm the bar/step writes (the next
    /// tick rewrites the chord/notes from the new score), and re-push
    /// the live globals. The Shared-only halves (gains, patterns, the
    /// baked `muted`/`solo`) take NO graph rebuild — click-free (the
    /// one-mixer contract) and effective within one frame. The one
    /// exception (plan 11): a changed `slots[i].kind` mutates the live
    /// engine's track kind and calls `rebuild_graph` ONCE (the vendor's
    /// documented swap path — its reverb-tail discontinuity is accepted
    /// for an explicit instrument swap, never on a plain gain/mute
    /// edit).
    pub fn set_score(&mut self, score: Score) {
        self.score = score;
        let p = self.score.stage_params(self.stage);
        self.bpm = p.bpm;
        self.brightness = p.brightness;
        for (i, slot) in SLOTS.iter().enumerate() {
            Self::apply_slot(
                &mut self.tracks[i],
                slot,
                i,
                self.stage,
                &self.score,
                &mut self.program_mute,
            );
        }
        // The instrument swap (plan 11): the resolved kinds reach the
        // tracks — and the live engine (guarded: only on an actual
        // kind change, one rebuild for all changed slots).
        self.apply_kinds();
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
        // The mute/solo fold (the baked `muted`/`solo` + the program
        // state) — effective now, and re-written every tick from
        // `sequence`.
        self.fold_mutes();
    }

    /// The slot's RESOLVED instrument kind (the plan 11 role/timbre
    /// split): the score's `kind` (validated by `Score::from_json`)
    /// or the slot's built-in kind (`SLOTS[i].kind`) when the score
    /// leaves it `None` (the issue-10 backwards-compat path).
    fn resolved_kind(score: &Score, i: usize) -> PresetKind {
        score.slots[i]
            .kind
            .as_deref()
            .map_or(SLOTS[i].kind, |id| {
                kind_from_id(id).expect(
                    "Score::from_json validates the kind ids (a hand-built Score with an unknown id must not reach the director)",
                )
            })
    }

    /// The plan 11 instrument swap: apply the new score's RESOLVED
    /// kinds to the director's tracks (which `try_start_live` clones
    /// into a FRESH engine — the construction path) and, when a kind
    /// actually CHANGED, to the live engine's track kinds + ONE
    /// `rebuild_graph` (the vendor's documented swap path — its
    /// reverb-tail/oscillator-phase discontinuity is the cost the guard
    /// exists to avoid on every other edit). `kind_rebuilds` counts
    /// exactly these calls.
    fn apply_kinds(&mut self) {
        let kinds: [PresetKind; 8] = core::array::from_fn(|i| Self::resolved_kind(&self.score, i));
        let changed = kinds
            .iter()
            .enumerate()
            .any(|(i, &kind)| self.tracks[i].kind != kind);
        if !changed {
            return; // the guard: no kind change → no mutation, no rebuild
        }
        for (i, &kind) in kinds.iter().enumerate() {
            self.tracks[i].kind = kind;
            self.tracks[i].name = kind.label().to_string();
        }
        if let Some(e) = &self.live {
            let mut live_tracks = e.tracks.lock();
            for (t, kind) in live_tracks.iter_mut().zip(kinds.iter()) {
                t.kind = *kind;
                t.name = kind.label().to_string();
            }
            drop(live_tracks);
            e.rebuild_graph();
            // The ONE real rebuild — the counter advances here and
            // nowhere else (the guard's no-change path never reaches
            // this line).
            self.kind_rebuilds += 1;
        }
    }

    /// The effective-mute fold (the plan 11 mute/solo core): write
    /// ALL 8 slots' `mute` — the PROGRAM state (the stage unlock for
    /// drums/harmony, the staccato gate for bass/lead —
    /// `program_mute`) ORed with the USER state (the baked `muted`, a
    /// baked `solo` silencing every non-soloed slot). Called from
    /// `sequence` EVERY tick — which is what makes a user mute written
    /// once survive the gate's every-tick rewrites (the regression
    /// this restructure exists to prevent) — and from the stage/score
    /// writers so a mix change is current before the next tick too.
    fn fold_mutes(&mut self) {
        for i in 0..8 {
            let user = self.score.slots[i].muted || self.score.solo.is_some_and(|s| s != i as u32);
            let effective = self.program_mute[i] || user;
            self.tracks[i]
                .params
                .mute
                .set_value(if effective { 1.0 } else { 0.0 });
        }
    }

    /// The director's current score (the lab's read half; `score()`'s
    /// JSON is the `getScore` export's return value).
    #[must_use]
    pub fn score(&self) -> &Score {
        &self.score
    }

    /// The director's 8 `Track`s (the exact resolved kinds + the
    /// `apply_slot`/role params): the offline audition renderer builds
    /// the engine's graph from THESE instead of mirroring `apply_slot`
    ///'s values by hand (`TrackParams`' fields are `Shared` atomics —
    /// the values read here are the ones the live graph reads).
    #[must_use]
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// The count of `rebuild_graph()` calls `apply_kinds` actually ran
    /// (the test/instrumentation surface that pins the swap guard): only
    /// a score edit that changed a slot's instrument kind advances this
    /// — one per rebuild, never on a plain gain/mute/solo edit, never on
    /// construction (fresh engines are built with the resolved kinds, no
    /// rebuild).
    #[must_use]
    pub fn kind_rebuilds(&self) -> u32 {
        self.kind_rebuilds
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
        // The gate's program half: the bass/lead are audible only while
        // their step has a note AND the window is open (the staccato).
        // The note-present half reads the bar NOW PLAYING's RESOLVED
        // lines (issue 11g — a phrase bar that rests where the
        // top-level line plays must stay gated; the `ResolvedBar`
        // temporaries drop before the `&mut self` gate writes, which
        // is why the booleans are pulled out first). The user state
        // (baked `muted`/`solo`) folds on top in `fold_mutes` — a user
        // mute can no longer be clobbered by the gate (the plan 11
        // regression).
        let bass_some = self.score.resolve_bar(bar).bass.notes[idx].is_some();
        let lead_some = self.score.resolve_bar(bar).lead.notes[idx].is_some();
        self.gate_voice(3, bass_some && on);
        self.gate_voice(4, lead_some && on);
        // The effective-mute fold: ALL 8 slots, EVERY tick.
        self.fold_mutes();
    }

    /// Record one sequenced voice's PROGRAM state (`on` = the program
    /// wants it audible): the staccato gate's half of the fold. The
    /// user state (`muted`/`solo`) is folded on top in `fold_mutes` —
    /// so a user mute can no longer be clobbered by the gate (plan 11).
    fn gate_voice(&mut self, slot: usize, on: bool) {
        self.program_mute[slot] = !on;
    }

    /// Write the bar onto the tracks (called from `sequence` EXACTLY
    /// when the bar changes): the chord-following slots' PITCH (the
    /// comp — issue 11f's chord root × the slot's ratio — and the
    /// harmony slots) AND the per-bar FIGURE grids (issue 11g — the
    /// phrase): the bass/lead `pattern_bits` from the bar NOW
    /// PLAYING's resolved note lines, the comp's from the bar's
    /// resolved comp pattern. The refresh lives HERE (not in
    /// `apply_slot`, which writes the top-level/default figures at
    /// construction and set_stage/set_score time) because a stale
    /// grid — the default bar's bits left on a later bar — is the
    /// subtle bug that would make a phrase sound like one bar
    /// (`bars_resolve_modulo_and_fall_back` +
    /// `the_phrase_actually_changes_what_plays` pin it).
    fn apply_bar(&mut self, bar: u64) {
        let p = self.score.stage_params(self.stage);
        let offset = self.score.chord_offset(self.stage, bar);
        let resolved = self.score.resolve_bar(bar);
        for (i, slot) in SLOTS.iter().enumerate() {
            match slot.role {
                Role::Comp => {
                    // The comp's PITCH: the chord root × ratio (the
                    // 11f contract)...
                    self.tracks[i]
                        .params
                        .freq
                        .set_value(p.root() * slot.ratio * semitone_ratio(offset));
                    // ...and its HIT PATTERN: the bar's resolved comp
                    // pattern's bits (the top-level `comp`, or the
                    // internal off-beat default — `resolve_bar`'s
                    // fallback).
                    Self::write_slot_grid(
                        &mut self.tracks[..],
                        i,
                        crate::score::pattern_bits(resolved.comp),
                    );
                }
                Role::Harmony => {
                    self.tracks[i]
                        .params
                        .freq
                        .set_value(p.root() * slot.ratio * semitone_ratio(offset));
                }
                // The per-bar note lines (the phrase's resolved
                // figures — the top-level lines when the bar leaves
                // the field `None` / the list empty).
                Role::Bass => Self::write_slot_grid(
                    &mut self.tracks[..],
                    i,
                    Self::note_line_bits(&resolved.bass.notes),
                ),
                Role::Lead => Self::write_slot_grid(
                    &mut self.tracks[..],
                    i,
                    Self::note_line_bits(&resolved.lead.notes),
                ),
                Role::Drum => {} // the stage tiers (written at stage/score changes)
            }
        }
    }

    /// One slot's per-bar grid write (issue 11g): `pattern_bits` +
    /// the hit count + a zero rotation — the same triple
    /// `apply_slot` writes at construction. Takes the tracks (not the
    /// whole director) so `apply_bar` can call it while the
    /// `ResolvedBar`'s borrow of `self.score` is still live (disjoint
    /// fields).
    fn write_slot_grid(tracks: &mut [Track], i: usize, bits: u32) {
        tracks[i]
            .params
            .pattern_hits
            .set_value(bits.count_ones() as f32);
        tracks[i].params.pattern_rotation.set_value(0.0);
        tracks[i].params.pattern_bits.store(bits, Ordering::Relaxed);
    }

    /// Write the step's bass/lead note (rests gate the voice in
    /// `sequence` — the note-present half of the fold's program state;
    /// this pass writes freqs only, never `mute`). The lines come from
    /// the bar NOW PLAYING's resolved figures (issue 11g — the
    /// phrase's per-bar lines, with the top-level lines as the
    /// fallback; empty `bars` = the top-level lines, exactly today).
    fn apply_step(&mut self, step: u64, bar: u64) {
        let p = self.score.stage_params(self.stage);
        let offset = self.score.chord_offset(self.stage, bar);
        let idx = (step % self.score.steps_per_bar as u64) as usize;
        let resolved = self.score.resolve_bar(bar);

        // Bass (slot 3): the chord root/fifth/octave line, gated per
        // step. The octave is data (`bass.octave_shift`: -1 in the
        // default score — the legacy `root * 0.5`): `2^octave_shift`.
        let bass = &self.tracks[3].params;
        if let Some(note) = resolved.bass.notes[idx] {
            bass.freq.set_value(
                p.root()
                    * 2.0_f32.powf(resolved.bass.octave_shift as f32)
                    * semitone_ratio(offset + note),
            );
        }

        // Lead (slot 4): the fixed motif on the key root, one octave up
        // (`lead.octave_shift`: +1 — the legacy `root * 2.0`),
        // deliberately NOT chord-following.
        let lead = &self.tracks[4].params;
        if let Some(note) = resolved.lead.notes[idx] {
            lead.freq.set_value(
                p.root() * 2.0_f32.powf(resolved.lead.octave_shift as f32) * semitone_ratio(note),
            );
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
                (PresetKind::FmComp, 0),
                (PresetKind::Bell, 2),
                (PresetKind::DroneSub, 0),
            ]
        );
        // The role map (plan 002 issue 11f — the `Slot.chord` bool is
        // gone: the comp is the new slot 5, the harmony slots are
        // 6/7).
        assert_eq!(
            SLOTS.iter().map(|s| s.role).collect::<Vec<_>>(),
            vec![
                Role::Drum,
                Role::Drum,
                Role::Drum,
                Role::Bass,
                Role::Lead,
                Role::Comp,
                Role::Harmony,
                Role::Harmony,
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
        // Slot 5 (the comp — issue 11f replaced the pad here) follows
        // the root shift exactly like the harmony slots do.
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

    /// Issue 11c's role-based grid: with the shipped score, slot 4
    /// (lead)'s `pattern_bits` equals the LEAD line's set steps — not
    /// the 4-hit Euclidean default the old Heartbeat-only writer left
    /// melodic slots with — slot 3's equals the bass line's, slot 0's
    /// equals the stage-0 kick tier, and the grid is a ROLE property:
    /// swapping a slot's KIND does not move its grid (the mutation
    /// check — reverting to the old only-in-Heartbeat-arm behaviour
    /// fails slot 4's first assert: the lead line's bits vs the
    /// Euclidean default). Issue 11f: slot 5 is now the COMP (its grid
    /// is the score's `comp` pattern — `the_comp_role_follows_the_chord
    /// _and_takes_the_pattern` pins it), so the untouched-Euclidean
    /// assertion covers the harmony slots 6-7 only.
    #[test]
    fn the_pattern_bits_follow_the_role_not_the_kind() {
        use rust_synth::math::rhythm::euclidean_bits;
        let score = Score::default();
        // The lines' set steps, computed INDEPENDENTLY (not through
        // `note_line_bits` — the test must fail if that helper drifts).
        let line_bits = |notes: &[Option<i32>]| -> u32 {
            notes.iter().enumerate().fold(
                0u32,
                |b, (i, n)| {
                    if n.is_some() {
                        b | (1 << i)
                    } else {
                        b
                    }
                },
            )
        };
        let lead_bits = line_bits(&score.lead.notes);
        let bass_bits = line_bits(&score.bass.notes);
        // The shipped lead line has notes on steps 0,2,4,5,7,8,10,12,
        // 13,15 — unmistakably not the 4-hit Euclidean default.
        assert_eq!(lead_bits, 0b1011_0101_1011_0101);
        assert_ne!(
            lead_bits,
            euclidean_bits(4, 0),
            "the lead line is NOT the Euclidean default"
        );

        let mut d = MusicDirector::new_fallback(); // stage 0, the shipped score
        assert_eq!(
            d.tracks[4].params.pattern_bits.load(Ordering::Relaxed),
            lead_bits,
            "slot 4 (lead) gets the lead line's set steps"
        );
        assert_eq!(
            d.tracks[3].params.pattern_bits.load(Ordering::Relaxed),
            bass_bits,
            "slot 3 (bass) gets the bass line's set steps"
        );
        assert_eq!(
            d.tracks[0].params.pattern_bits.load(Ordering::Relaxed),
            (1 << 0) | (1 << 7) | (1 << 10),
            "slot 0 gets the stage-0 kick tier"
        );
        // The harmony slots (6-7 since issue 11f — slot 5 is the
        // comp's) keep the Euclidean default (untouched).
        for i in 6..8 {
            assert_eq!(
                d.tracks[i].params.pattern_bits.load(Ordering::Relaxed),
                euclidean_bits(4, 0),
                "slot {i} (harmony) keeps the 4-hit Euclidean default"
            );
        }

        // ROLE, not kind: a Heartbeat swapped into the lead slot keeps
        // the lead line's grid (the old code's Heartbeat arm only wrote
        // slots 0-2, so a swapped-in Heartbeat got the Euclidean
        // default — the bug this test pins).
        let mut swapped = Score::default();
        swapped.slots[4].kind = Some("heartbeat".to_string());
        d.set_score(swapped);
        assert_eq!(
            d.tracks[4].params.pattern_bits.load(Ordering::Relaxed),
            lead_bits,
            "a Heartbeat swapped into the lead slot keeps the lead line's grid"
        );
        // And a PluckSaw swapped into the kick slot keeps the kick tier.
        let mut swapped = Score::default();
        swapped.slots[0].kind = Some("pluck_saw".to_string());
        d.set_score(swapped);
        assert_eq!(
            d.tracks[0].params.pattern_bits.load(Ordering::Relaxed),
            (1 << 0) | (1 << 7) | (1 << 10),
            "a PluckSaw swapped into the kick slot keeps the kick tier's grid"
        );
    }

    /// Plan 002 issue 11f — the comp role (slot 5): its `pattern_bits`
    /// equal the score's `comp` pattern's set steps (computed
    /// INDEPENDENTLY from the chars — not through `comp_pattern_bits`,
    /// so a drift in that helper is caught), AND its frequency
    /// follows the bar's chord (the comp is chord-following — unlike
    /// the lead, which stays on the key root). Mutation check:
    /// dropping the comp from `apply_bar`'s chord write fails the
    /// bar-1 assertion; leaving the comp on the Euclidean default (no
    /// `Role::Comp` grid arm) fails the first assertion (the off-beat
    /// pattern's bits vs the 4-hit Euclidean default).
    #[test]
    fn the_comp_role_follows_the_chord_and_takes_the_pattern() {
        use rust_synth::math::rhythm::euclidean_bits;
        let score = Score::default();
        // The comp pattern's set steps, computed INDEPENDENTLY from
        // the shipped score's `comp` chars (never through
        // `comp_pattern_bits` — the test must fail if that helper
        // drifts).
        let comp_bits = score
            .comp
            .as_deref()
            .expect("the shipped score carries a comp pattern")
            .chars()
            .enumerate()
            .fold(0u32, |bits, (i, c)| {
                if c == 'x' || c == 'X' {
                    bits | (1 << i)
                } else {
                    bits
                }
            });
        // The off-beat 8ths: hits on 2/6/10/14 — unmistakably not
        // the 4-hit Euclidean default (which has no hit at step 2).
        assert_eq!(comp_bits, 0b0100_0100_0100_0100);
        assert_ne!(
            comp_bits,
            euclidean_bits(4, 0),
            "the comp pattern is NOT the Euclidean default"
        );

        let mut d = MusicDirector::new_fallback(); // stage 0, the shipped score
        assert_eq!(
            d.tracks[5].params.pattern_bits.load(Ordering::Relaxed),
            comp_bits,
            "slot 5 (comp) gets the comp pattern's set steps"
        );

        // The PITCH: the bar's chord root × the slot's ratio (1.0),
        // written by `apply_bar` — bar 0 is I (offset 0).
        let root = stage_params(0).root();
        d.tick(0.01); // bar 0, step 0
        let comp0 = d.tracks[5].params.freq.value();
        assert!(
            (comp0 - root).abs() < 1e-4,
            "the comp is on the bar-0 chord root (I): got {comp0}, want {root}"
        );
        let lead0 = d.tracks[4].params.freq.value();

        // Bar 1 (vi, +9 semitones): the COMP retunes with the chord...
        let p = stage_params(0);
        let step_len = 60.0 / p.bpm / 4.0;
        d.tick(16.0 * step_len + 0.3 * step_len - 0.01); // t lands at bar 1, in-step 0.3
        let comp1 = d.tracks[5].params.freq.value();
        assert!(
            (comp1 - comp0 * semitone_ratio(9)).abs() < 0.5,
            "the comp follows the bar's chord (vi): got {comp1}, want {}",
            comp0 * semitone_ratio(9)
        );
        // ...while the lead stays on the key root (the contrast that
        // makes the comp chord-following, unlike the lead).
        assert!(
            (d.tracks[4].params.freq.value() - lead0).abs() < 1e-4,
            "the lead ignores the chord (still {lead0})"
        );
    }

    /// Plan 002 issue 11f — the Role refactor's contract: the
    /// EXISTING roles' observable behaviour is unchanged. For the
    /// default score: the drum slots' `pattern_bits` are the stage-0
    /// tier patterns, the bass/lead bits are their note lines, and
    /// the harmony slots (6-7) keep the 4-hit Euclidean default — the
    /// same assertions 11c's role test made, so a refactor slip (e.g.
    /// the role selection reordering the bass/lead lines onto the
    /// wrong slots) is caught: the lines have different set steps.
    #[test]
    fn existing_roles_are_unchanged() {
        use rust_synth::math::rhythm::euclidean_bits;
        let score = Score::default();
        // The tier/line bits, computed INDEPENDENTLY (the char-level
        // parse and the note-line fold — not through the director's
        // helpers, so a drift in them is caught).
        let tier_bits = |pattern: &str| -> u32 {
            pattern.chars().enumerate().fold(0u32, |bits, (i, c)| {
                if c == 'x' || c == 'X' {
                    bits | (1 << i)
                } else {
                    bits
                }
            })
        };
        let line_bits =
            |notes: &[Option<i32>]| -> u32 {
                notes.iter().enumerate().fold(0u32, |bits, (i, n)| {
                    if n.is_some() {
                        bits | (1 << i)
                    } else {
                        bits
                    }
                })
            };
        let kick = tier_bits(&score.kick.tiers[0].pattern);
        let snare = tier_bits(&score.snare.tiers[0].pattern);
        let hat = tier_bits(&score.hat.tiers[0].pattern);
        let bass = line_bits(&score.bass.notes);
        let lead = line_bits(&score.lead.notes);
        // The drivers are DISTINCT numbers (so a "wrong role assigned
        // to the slot" is an inequality, not an equality accident):
        // the tiers differ from each other, the lines from each other,
        // and neither line is the 4-hit Euclidean default.
        assert_ne!(kick, snare, "kick and snare tiers differ");
        assert_ne!(snare, hat, "snare and hat tiers differ");
        assert_ne!(
            bass, lead,
            "the bass and lead lines have different set steps"
        );
        assert_ne!(
            bass,
            euclidean_bits(4, 0),
            "the bass line is NOT the Euclidean default"
        );
        assert_ne!(
            lead,
            euclidean_bits(4, 0),
            "the lead line is NOT the Euclidean default"
        );

        let d = MusicDirector::new_fallback(); // stage 0, the shipped score
        assert_eq!(
            d.tracks[0].params.pattern_bits.load(Ordering::Relaxed),
            kick,
            "slot 0 (drum) keeps the stage-0 kick tier"
        );
        assert_eq!(
            d.tracks[1].params.pattern_bits.load(Ordering::Relaxed),
            snare,
            "slot 1 (drum) keeps the stage-0 snare tier"
        );
        assert_eq!(
            d.tracks[2].params.pattern_bits.load(Ordering::Relaxed),
            hat,
            "slot 2 (drum) keeps the stage-0 hat tier"
        );
        assert_eq!(
            d.tracks[3].params.pattern_bits.load(Ordering::Relaxed),
            bass,
            "slot 3 (bass) keeps the bass line's set steps"
        );
        assert_eq!(
            d.tracks[4].params.pattern_bits.load(Ordering::Relaxed),
            lead,
            "slot 4 (lead) keeps the lead line's set steps"
        );
        for i in 6..8 {
            assert_eq!(
                d.tracks[i].params.pattern_bits.load(Ordering::Relaxed),
                euclidean_bits(4, 0),
                "slot {i} (harmony) keeps the 4-hit Euclidean default"
            );
        }
    }

    /// Plan 002 issue 11f — the backwards-compat contract: a score
    /// JSON with no `comp` field (an 11e-era file) still parses
    /// (serde default → `None`) and PLAYS: the comp slot resolves to
    /// the internal off-beat default pattern (never silent — the
    /// fallback pattern's bits land in `pattern_bits`) and its
    /// chord-following pitch is written like every other chord
    /// follower's.
    #[test]
    fn a_score_without_comp_still_parses_and_plays() {
        use crate::score::COMP_DEFAULT_PATTERN;
        // The pre-11f shape: the canonical JSON minus the `comp`
        // field (what the 11e bake emitted).
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        value
            .as_object_mut()
            .expect("the score is an object")
            .remove("comp");
        let score = Score::from_json(&value.to_string()).expect("a score without `comp` parses");
        assert_eq!(score.comp, None, "`comp` defaults to absent");

        // It PLAYS: the director's comp slot gets the off-beat
        // default's bits (independently computed) and a live,
        // chord-rooted pitch at stage 0 (the comp unlocks at 0).
        let default_bits = COMP_DEFAULT_PATTERN
            .chars()
            .enumerate()
            .fold(0u32, |bits, (i, c)| {
                if c == 'x' || c == 'X' {
                    bits | (1 << i)
                } else {
                    bits
                }
            });
        let mut d = MusicDirector::new_fallback();
        d.set_score(score);
        assert_eq!(
            d.tracks[5].params.pattern_bits.load(Ordering::Relaxed),
            default_bits,
            "the absent `comp` plays the off-beat default pattern"
        );
        d.tick(0.01); // bar 0 (I, offset 0)
        let root = stage_params(0).root();
        assert!(
            (d.tracks[5].params.freq.value() - root).abs() < 1e-4,
            "the comp is on the bar-0 chord root: got {}, want {root}",
            d.tracks[5].params.freq.value()
        );
        assert_eq!(
            d.tracks[5].params.mute.value(),
            0.0,
            "the comp (unlock 0) is live, not stage-muted"
        );
    }

    /// Plan 002 issue 11f — the fold covers the COMP role: a
    /// user-muted comp stays muted across a whole bar of ticks
    /// (INCLUDING its pattern's hit steps — the comp's envelope is
    /// pattern-driven, but the MUTE is the fold's), and soloing the
    /// comp silences every other slot (the fold's user half) while
    /// the soloed comp follows its own program (live — unlock 0).
    #[test]
    fn the_mute_solo_fold_silences_the_comp() {
        let step_len = 60.0 / BPM_BASE / 4.0;
        // The mute half: the user-muted comp stays muted on every
        // step of the bar — its pattern's hit steps (the off-beat
        // 8ths: 2/6/10/14) included.
        let mut d = MusicDirector::new_fallback();
        let mut score = Score::default();
        score.slots[5].muted = true; // the comp
        d.set_score(score);
        assert_eq!(
            d.tracks[5].params.mute.value(),
            1.0,
            "the user mute lands immediately (the fold — not a stale write)"
        );
        for step in 0..16u64 {
            d.tick(step_len); // one full step per tick: steps 0..15 of the bar
            assert_eq!(
                d.tracks[5].params.mute.value(),
                1.0,
                "the user-muted comp stays muted on step {step} (its pattern's hits included)"
            );
        }
        // The fold is per-slot: the kick is not touched by the comp's
        // mute (unlock 0 → live).
        assert_eq!(
            d.tracks[0].params.mute.value(),
            0.0,
            "the kick is not touched by the comp's mute"
        );

        // The solo half: soloing the comp silences the other seven;
        // the comp itself follows its program (unlock 0 → live on
        // every step).
        let mut d = MusicDirector::new_fallback();
        let score = Score {
            solo: Some(5),
            ..Default::default()
        };
        d.set_score(score);
        for k in 0..8u64 {
            d.tick(step_len);
            assert_eq!(
                d.tracks[5].params.mute.value(),
                0.0,
                "the soloed comp follows its own program (unlock 0 → live, tick {k})"
            );
            for i in [0usize, 1, 2, 3, 4, 6, 7] {
                assert_eq!(
                    d.tracks[i].params.mute.value(),
                    1.0,
                    "soloing the comp silences slot {i} (tick {k})"
                );
            }
        }
        // Clear the solo: the other slots come back to their program
        // state. The comp's program half is the unlock state, not the
        // pattern — it is live on a cleared solo (the bell stays
        // stage-locked, as before 11f).
        d.set_score(Score::default());
        d.tick(step_len);
        assert_eq!(
            d.tracks[5].params.mute.value(),
            0.0,
            "clearing the solo un-mutes the comp (unlock 0 → live)"
        );
        for i in [0usize, 1, 2] {
            assert_eq!(
                d.tracks[i].params.mute.value(),
                0.0,
                "clearing the solo un-mutes the unlocked drum {i}"
            );
        }
        assert_eq!(
            d.tracks[6].params.mute.value(),
            1.0,
            "the bell is STILL stage-locked at stage 0 (program, not solo)"
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
        let comp0 = d.tracks[5].params.freq.value();
        let lead0 = d.tracks[4].params.freq.value();
        d.tick(0.01);
        assert!((d.tracks[5].params.freq.value() - comp0).abs() < 1e-4);
        d.tick(bar * 1.05);
        let expected = comp0 * semitone_ratio(9);
        assert!(
            (d.tracks[5].params.freq.value() - expected).abs() < 0.5,
            "the comp follows vi"
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
        score.slots[0] = SlotGain {
            gain: 0.3,
            kind: None,
            muted: false,
        };
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

    /// The regression the plan 11 per-tick fold exists to prevent: before
    /// the fold, `mute` was written by the score/stage writers
    /// (`apply_slot`'s unlock write, the gate's every-tick rewrite), so
    /// a user mute written once was clobbered. Here the kick (slot 0 —
    /// its tier has hits at steps 0/7/10) is user-muted and must STAY
    /// muted across a whole bar of ticks, INCLUDING those hit steps;
    /// and the bass (slot 3 — the GATED voice whose gate rewrote `mute`
    /// every tick) must stay muted while its window is OPEN on a note
    /// step. With the fold removed, the first half fails at the first
    /// assert (the `set_score` clobber) and the second at every in-window
    /// tick (the gate's clobber).
    #[test]
    fn user_mute_survives_the_gate() {
        // The drum half: the user-muted kick stays muted across a full
        // bar of ticks (its tier's hit steps 0/7/10 included).
        let mut d = MusicDirector::new_fallback();
        let mut score = Score::default();
        score.slots[0].muted = true; // the kick
        d.set_score(score);
        assert_eq!(
            d.tracks[0].params.mute.value(),
            1.0,
            "the user mute lands immediately (the fold — not a stale write)"
        );
        let step_len = 60.0 / BPM_BASE / 4.0;
        for step in 0..16u64 {
            d.tick(step_len * 0.5);
            assert_eq!(
                d.tracks[0].params.mute.value(),
                1.0,
                "the user-muted kick stays muted on step {step} (its tier's hits at 0/7/10 included)"
            );
        }
        // The fold is per-slot: the snare is not touched by the kick's
        // mute (unlock 0 → live).
        assert_eq!(
            d.tracks[1].params.mute.value(),
            0.0,
            "the snare is not touched by the kick's mute"
        );

        // The gate half: the user-muted BASS (the gated voice) stays
        // muted while its window is OPEN on a note step — pre-fold,
        // `sequence`'s gate wrote mute 0.0 on exactly these ticks.
        let mut d = MusicDirector::new_fallback();
        let mut score = Score::default();
        score.slots[3].muted = true; // the bass
        d.set_score(score);
        // Step 0 carries the default bass's note 0; the even-step
        // window is 0..0.7, and in-steps 0.125..0.625 sit well inside
        // it — the pre-fold gate (note present, window open) would have
        // clobbered the mute on every one of these ticks.
        for k in 1..=5 {
            d.tick(step_len * 0.125);
            assert_eq!(
                d.tracks[3].params.mute.value(),
                1.0,
                "the user-muted bass stays muted inside its open window (in-step ≈{}*0.125)",
                k
            );
        }
    }

    /// The plan 11 solo contract: `solo = Some(0)` silences every slot
    /// except the soloed one (the fold's user half), the soloed slot
    /// follows its own PROGRAM (the kick's unlock state), and clearing
    /// the solo brings the other slots back to their program state —
    /// the stage-locked pad/bell STAY muted at stage 0 (the fold is
    /// program OR solo, not just the solo's absence).
    #[test]
    fn solo_isolates_one_slot() {
        let mut d = MusicDirector::new_fallback();
        let step_len = 60.0 / BPM_BASE / 4.0;
        // Solo the kick (slot 0).
        let score = Score {
            solo: Some(0),
            ..Default::default()
        };
        d.set_score(score);
        for k in 0..8u64 {
            d.tick(step_len * 0.5);
            assert_eq!(
                d.tracks[0].params.mute.value(),
                0.0,
                "the soloed kick follows its own program (unlock 0 → live, tick {k})"
            );
            for i in 1..8 {
                assert_eq!(
                    d.tracks[i].params.mute.value(),
                    1.0,
                    "soloing the kick silences slot {i} (tick {k})"
                );
            }
        }
        // Clear the solo: the other slots come back to their PROGRAM
        // state. The tick lands on step 4, in-step ≈0.5 (the even
        // window 0..0.7 open; the default bass AND lead both carry a
        // note on step 4) — the gated voices come back through their
        // gate; the unlocked drums/harmony/comp are live; the bell is
        // still stage-locked (program, not solo).
        d.set_score(Score::default());
        d.tick(step_len * 0.5);
        for i in [1usize, 2, 5, 7] {
            assert_eq!(
                d.tracks[i].params.mute.value(),
                0.0,
                "clearing the solo un-mutes the unlocked slot {i}"
            );
        }
        assert_eq!(
            d.tracks[3].params.mute.value(),
            0.0,
            "the bass comes back through its gate (step 4's note, window open)"
        );
        assert_eq!(
            d.tracks[4].params.mute.value(),
            0.0,
            "the lead comes back through its gate (step 4's note, window open)"
        );
        assert_eq!(
            d.tracks[6].params.mute.value(),
            1.0,
            "the bell is STILL stage-locked at stage 0 (program, not solo)"
        );
    }

    /// The plan 11 backwards-compat contract: a score JSON saved by
    /// issue 10 (NO `kind`, NO `muted`, NO `solo` fields) still parses
    /// (serde defaults) and every slot resolves to its BUILT-IN kind
    /// (`SLOTS[i].kind`) — the operator's saved score files keep
    /// working.
    #[test]
    fn slots_without_kind_use_the_builtin() {
        // Simulate an issue-10 file: strip the 11a fields from the
        // canonical JSON (what issue-10's `to_json_pretty` emitted).
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        for slot in value["slots"].as_array_mut().expect("slots is an array") {
            let slot = slot.as_object_mut().expect("a slot is an object");
            slot.remove("kind");
            slot.remove("muted");
        }
        value
            .as_object_mut()
            .expect("the score is an object")
            .remove("solo");
        let score = Score::from_json(&value.to_string()).expect("an issue-10 score file parses");
        for (i, slot) in SLOTS.iter().enumerate() {
            assert!(score.slots[i].kind.is_none(), "slot {i} keeps no kind");
            assert!(!score.slots[i].muted, "slot {i} defaults to unmuted");
            assert_eq!(
                MusicDirector::resolved_kind(&score, i),
                slot.kind,
                "slot {i} resolves to its built-in kind"
            );
        }
        assert_eq!(score.solo, None, "solo defaults to null");
    }

    /// The `set_score` re-arm pair (the review's uncovered lines): the
    /// cleared `last_bar`/`last_step` are what make the next tick
    /// re-write the current bar's chord frequencies (`apply_bar`) and
    /// the current step's note frequencies (`apply_step`). The mutes
    /// cannot pin this — `sequence()`'s effective-mute fold rewrites
    /// them every tick, masking a missed re-apply. Driven through a
    /// real fallback director: tick to step 0 (the default bass's note
    /// 0 is on), swap in a score that moved that note AND the bar's
    /// chord, and within the SAME step (a tiny dt — the step index does
    /// not advance) both freqs must already be the new values; without
    /// the re-arm the already-visited bar/step are skipped and both
    /// stay stale.
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
            "the pre-swap comp is the bar-0 chord (I, offset 0)"
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
    /// Plan 002 issue 11g — the backwards-compat contract: an EMPTY
    /// `bars` list (the shipped bake — no `bars` field at all) resolves
    /// every bar to the top-level figures: bar after bar, the bass/
    /// lead/comp `pattern_bits` are the top-level lines' set steps and
    /// the step-0 frequencies are the top-level lines over the
    /// ABSOLUTE bar's chord (the phrase machinery is transparent when
    /// there is no phrase). Driven through real fallback directors
    /// (one per bar — the tick lands exactly on bar {bar}, step 0,
    /// in-step ≈0.15 — the even-step window open, and well inside the
    /// step, so float noise cannot land on the wrong step).
    #[test]
    fn an_empty_bars_list_is_todays_behaviour() {
        let score = Score::default();
        assert!(score.bars.is_empty(), "the shipped bake carries no `bars`");
        // The top-level figures' grids, computed INDEPENDENTLY (the
        // note-line fold + the char parse — not through
        // `resolve_bar`/`pattern_bits`/`note_line_bits`, so a drift in
        // any of them is caught).
        let line_bits = |notes: &[Option<i32>]| -> u32 {
            notes.iter().enumerate().fold(
                0u32,
                |b, (i, n)| {
                    if n.is_some() {
                        b | (1 << i)
                    } else {
                        b
                    }
                },
            )
        };
        let char_bits = |pattern: &str| -> u32 {
            pattern.chars().enumerate().fold(0u32, |b, (i, c)| {
                if c == 'x' || c == 'X' {
                    b | (1 << i)
                } else {
                    b
                }
            })
        };
        let bass_bits = line_bits(&score.bass.notes);
        let lead_bits = line_bits(&score.lead.notes);
        let comp_bits = char_bits(
            score
                .comp
                .as_deref()
                .unwrap_or(crate::score::COMP_DEFAULT_PATTERN),
        );

        let stage = 3u32; // a shifted root (fourths = 1) — the legacy `root * ROOT_SHIFT`
        let p = stage_params(stage);
        let root = ROOT_A2_HZ * ROOT_SHIFT.powi(p.fourths as i32);
        let step_len = 60.0 / p.bpm / 4.0;
        // A full chord cycle, bar after bar: the figures are the
        // top-level ones EVERY time (the empty-phrase resolution).
        for bar in 0..4u64 {
            let mut d = MusicDirector::new_fallback();
            d.set_stage(stage);
            d.tick((bar as f32 * 16.0 + 0.25) * step_len - 0.01); // bar {bar}, step 0, in-step ≈0.15
            assert_eq!(
                d.tracks[3].params.pattern_bits.load(Ordering::Relaxed),
                bass_bits,
                "bar {bar}: the bass grid is the top-level line's (no phrase)"
            );
            assert_eq!(
                d.tracks[4].params.pattern_bits.load(Ordering::Relaxed),
                lead_bits,
                "bar {bar}: the lead grid is the top-level line's (no phrase)"
            );
            assert_eq!(
                d.tracks[5].params.pattern_bits.load(Ordering::Relaxed),
                comp_bits,
                "bar {bar}: the comp grid is the top-level pattern's (no phrase)"
            );
            let offset = score.chord_offset(stage, bar);
            let bass0 = root
                * 2.0_f32.powf(score.bass.octave_shift as f32)
                * semitone_ratio(
                    offset + score.bass.notes[0].expect("the default bass's step 0 is a note"),
                );
            assert!(
                (d.tracks[3].params.freq.value() - bass0).abs() < 1e-4,
                "bar {bar}: the bass step-0 freq is the top-level line over the bar's chord (got {}, want {bass0})",
                d.tracks[3].params.freq.value()
            );
            let lead0 = root
                * 2.0_f32.powf(score.lead.octave_shift as f32)
                * semitone_ratio(score.lead.notes[0].expect("the default lead's step 0 is a note"));
            assert!(
                (d.tracks[4].params.freq.value() - lead0).abs() < 1e-4,
                "bar {bar}: the lead step-0 freq is the top-level line on the key root (got {}, want {lead0})",
                d.tracks[4].params.freq.value()
            );
            let comp0 = root * semitone_ratio(offset);
            assert!(
                (d.tracks[5].params.freq.value() - comp0).abs() < 1e-4,
                "bar {bar}: the comp pitch is the bar's chord root (got {}, want {comp0})",
                d.tracks[5].params.freq.value()
            );
        }
    }

    /// Plan 002 issue 11g — the resolution contract: the bar index
    /// resolves modulo `bars.len()`, and a `None` field falls back to
    /// the top-level figure. A 2-bar phrase over 4 bars resolves
    /// 0,1,0,1 — asserted BOTH on the pure `resolve_bar` resolution
    /// and on the DIRECTOR's per-bar grids (the `pattern_bits` refresh
    /// at the bar change is exactly what keeps bar 1's grid from
    /// being bar 0's stale bits — a resolution that always returned
    /// bar 0 fails the bar-1/bar-3 asserts).
    #[test]
    fn bars_resolve_modulo_and_fall_back() {
        use crate::score::{Bar, NoteLine};
        // A 16-entry line with ONE note (at the given step) — distinct
        // per override, so a wrong figure is an inequality.
        let one_note = |step: usize, note: i32| {
            let mut notes = vec![None; 16];
            notes[step] = Some(note);
            NoteLine {
                notes,
                octave_shift: 0,
            }
        };
        // The top-level figures' grids, computed INDEPENDENTLY (the
        // note-line fold + the char parse).
        let line_bits = |notes: &[Option<i32>]| -> u32 {
            notes.iter().enumerate().fold(
                0u32,
                |b, (i, n)| {
                    if n.is_some() {
                        b | (1 << i)
                    } else {
                        b
                    }
                },
            )
        };
        let char_bits = |pattern: &str| -> u32 {
            pattern.chars().enumerate().fold(0u32, |b, (i, c)| {
                if c == 'x' || c == 'X' {
                    b | (1 << i)
                } else {
                    b
                }
            })
        };
        let mut score = Score::default();
        let top_bass_bits = line_bits(&score.bass.notes);
        let top_lead_bits = line_bits(&score.lead.notes);
        let top_comp_bits = char_bits(
            score
                .comp
                .as_deref()
                .unwrap_or(crate::score::COMP_DEFAULT_PATTERN),
        );
        // The TEST-ONLY 2-bar phrase: bar 0 overrides the bass (step
        // 0, +5) and the comp (16ths); bar 1 overrides the lead (step
        // 0, +15); every `None` field falls back to the top-level
        // figure.
        let bar0_comp = "x.x.x.x.x.x.x.x.";
        score.bars = vec![
            Bar {
                bass: Some(one_note(0, 5)),
                lead: None,
                comp: Some(bar0_comp.to_string()),
            },
            Bar {
                bass: None,
                lead: Some(one_note(0, 15)),
                comp: None,
            },
        ];
        // The overrides are distinct figures (so the grid asserts are
        // inequalities, not equality accidents).
        assert_ne!(
            1u32, top_bass_bits,
            "bar 0's bass override is a distinct grid"
        );
        assert_ne!(
            1u32, top_lead_bits,
            "bar 1's lead override is a distinct grid"
        );
        assert_ne!(
            char_bits(bar0_comp),
            top_comp_bits,
            "bar 0's comp override is a distinct pattern"
        );

        // The pure resolution: 0,1,0,1 + the per-field fallbacks.
        let bar0 = score.resolve_bar(0);
        assert_eq!(bar0.bass.notes[0], Some(5), "bar 0's override bass");
        assert!(
            bar0.bass.notes.iter().skip(1).all(Option::is_none),
            "bar 0's override bass rests elsewhere"
        );
        assert_eq!(
            bar0.lead, &score.lead,
            "bar 0's lead falls back to the top-level line"
        );
        assert_eq!(bar0.comp, bar0_comp, "bar 0's override comp");
        let bar1 = score.resolve_bar(1);
        assert_eq!(
            bar1.bass, &score.bass,
            "bar 1's bass falls back to the top-level line"
        );
        assert_eq!(bar1.lead.notes[0], Some(15), "bar 1's override lead");
        assert_eq!(
            bar1.comp,
            score
                .comp
                .as_deref()
                .expect("the shipped score's comp pattern"),
            "bar 1's comp falls back to the top-level pattern"
        );
        // The modulo: 2 == 0, 3 == 1.
        assert_eq!(score.resolve_bar(2), bar0, "bar 2 resolves to phrase bar 0");
        assert_eq!(score.resolve_bar(3), bar1, "bar 3 resolves to phrase bar 1");
        assert_eq!(score.phrase_len(), 2);

        // The DIRECTOR half: the per-bar grids follow the resolution
        // (the `apply_bar` refresh — a stale grid is the subtle bug
        // that would make the phrase sound like one bar).
        let step_len = 60.0 / BPM_BASE / 4.0;
        let mut d = MusicDirector::new_fallback();
        d.set_score(score);
        let expected = [
            (1u32, top_lead_bits, char_bits(bar0_comp)),
            (top_bass_bits, 1, top_comp_bits),
        ];
        for bar in 0..4u64 {
            // The first tick lands 0.01 s INSIDE step 0 of bar 0 (in-
            // step ≈0.15 — the even-step window open); each later tick
            // advances exactly 16 steps (a full bar), keeping in-step
            // ≈0.15 of the bar's step 0.
            d.tick(if bar == 0 {
                0.25 * step_len - 0.01
            } else {
                16.0 * step_len
            });
            let (e_bass, e_lead, e_comp) = expected[(bar % 2) as usize];
            assert_eq!(
                d.tracks[3].params.pattern_bits.load(Ordering::Relaxed),
                e_bass,
                "bar {bar}: the bass grid (the bar-change refresh)"
            );
            assert_eq!(
                d.tracks[4].params.pattern_bits.load(Ordering::Relaxed),
                e_lead,
                "bar {bar}: the lead grid (the bar-change refresh)"
            );
            assert_eq!(
                d.tracks[5].params.pattern_bits.load(Ordering::Relaxed),
                e_comp,
                "bar {bar}: the comp grid (the bar-change refresh)"
            );
        }
    }

    /// Plan 002 issue 11g — THE mutation-sensitive contract: two bars
    /// with DIFFERENT bass lines actually change what plays — the
    /// bass's `pattern_bits` AND its step frequencies differ between
    /// the bars (independently computed expectations), and the grid
    /// RETURNS to bar 0's on bar 2 (the modulo). If the resolution
    /// always returned bar 0 (or the top-level line), the bar-1
    /// asserts fail.
    #[test]
    fn the_phrase_actually_changes_what_plays() {
        use crate::score::{Bar, NoteLine};
        let mut score = Score::default();
        // bar 0: a bass line with a note ONLY at step 0 (+0, the bar's
        // chord root); bar 1: a bass line with a note ONLY at step 4
        // (+7, a fifth up). Different grid, different pitch.
        let mut b0 = vec![None; 16];
        b0[0] = Some(0);
        let mut b1 = vec![None; 16];
        b1[4] = Some(7);
        let oct = score.bass.octave_shift;
        score.bars = vec![
            Bar {
                bass: Some(NoteLine {
                    notes: b0,
                    octave_shift: oct,
                }),
                lead: None,
                comp: None,
            },
            Bar {
                bass: Some(NoteLine {
                    notes: b1,
                    octave_shift: oct,
                }),
                lead: None,
                comp: None,
            },
        ];
        let root = stage_params(0).root();
        let step_len = 60.0 / BPM_BASE / 4.0;
        // One fresh fallback director per observation (the tick lands
        // exactly on the target bar/step, in-step ≈0.15 — the even-step
        // window open, well inside the step).
        let state_at = |bar: u64, step: u64| -> (u32, f32) {
            let mut d = MusicDirector::new_fallback();
            d.set_score(score.clone());
            d.tick((bar as f32 * 16.0 + step as f32 + 0.25) * step_len - 0.01);
            (
                d.tracks[3].params.pattern_bits.load(Ordering::Relaxed),
                d.tracks[3].params.freq.value(),
            )
        };
        // bar 0, step 0: grid = bit 0, freq = root * 2^oct * semitone(0 + 0)
        // (the bar's chord I's offset 0 + the line's note 0).
        let (bits0, freq0) = state_at(0, 0);
        assert_eq!(bits0, 0b1, "bar 0's bass grid (its line's set steps)");
        let want0 = root * 2.0_f32.powf(oct as f32) * semitone_ratio(0);
        assert!(
            (freq0 - want0).abs() < 1e-4,
            "bar 0's bass step-0 pitch: got {freq0}, want {want0}"
        );
        // bar 1, step 4: grid = bit 4, freq = root * 2^oct *
        // semitone(9 + 7) (vi's offset 9 + the line's fifth).
        let (bits1, freq1) = state_at(1, 4);
        assert_eq!(bits1, 1 << 4, "bar 1's bass grid (a DIFFERENT figure)");
        let want1 = root * 2.0_f32.powf(oct as f32) * semitone_ratio(9 + 7);
        assert!(
            (freq1 - want1).abs() < 1e-4,
            "bar 1's bass step-4 pitch: got {freq1}, want {want1}"
        );
        assert_ne!(bits0, bits1, "the phrase actually changes the grid");
        assert_ne!(freq0, freq1, "the phrase actually changes the pitch");
        // bar 2 = phrase bar 0 again (the modulo): the grid RETURNS to
        // bit 0; bar 3 = phrase bar 1: bit 4.
        let (bits2, _) = state_at(2, 0);
        assert_eq!(
            bits2, bits0,
            "bar 2 resolves to phrase bar 0 (the grid returns)"
        );
        let (bits3, _) = state_at(3, 4);
        assert_eq!(
            bits3, bits1,
            "bar 3 resolves to phrase bar 1 (the grid returns)"
        );
    }

    /// Plan 002 issue 11g — the gate reads the RESOLVED bar's figure,
    /// not the top-level default: the default bass has a note at step
    /// 0, but the phrase's bar 1 overrides the bass with a line that
    /// RESTS at step 0 — on bar 1's step 0 the bass must be GATED
    /// (muted) even though the top-level line plays there (and
    /// un-muted on bar 0's step 0, where the top-level line applies —
    /// the fallback keeps today's behaviour). A gate that reads
    /// `score.bass.notes` (the pre-11g expression) fails the bar-1
    /// half.
    #[test]
    fn the_gate_reads_the_resolved_bar() {
        use crate::score::{Bar, NoteLine};
        let mut score = Score::default();
        // bar 1's bass: the default line with step 0 RESTED (and step
        // 1 given a note — so the override's grid is not the default's
        // either).
        let mut b1 = score.bass.notes.clone();
        b1[0] = None;
        b1[1] = Some(3);
        let oct = score.bass.octave_shift;
        score.bars = vec![
            Bar {
                bass: None, // bar 0: the top-level line
                lead: None,
                comp: None,
            },
            Bar {
                bass: Some(NoteLine {
                    notes: b1,
                    octave_shift: oct,
                }),
                lead: None,
                comp: None,
            },
        ];
        let step_len = 60.0 / BPM_BASE / 4.0;
        let mut d = MusicDirector::new_fallback();
        d.set_score(score);
        // bar 0, step 0 (in-step ≈0.25 — the even window 0..0.7 open,
        // and clear of BOTH window edges so float noise cannot land
        // outside): the top-level line plays → the gate OPEN.
        d.tick(0.35 * step_len - 0.01);
        assert_eq!(
            d.tracks[3].params.mute.value(),
            0.0,
            "bar 0 step 0: the top-level line's note keeps the bass live"
        );
        // bar 1, step 0 (in-step ≈0.25): the override RESTS → the gate
        // CLOSED, even though the top-level line has a note there.
        d.tick(16.0 * step_len);
        assert_eq!(
            d.tracks[3].params.mute.value(),
            1.0,
            "bar 1 step 0: the resolved rest gates the bass (the top-level line plays here — the gate must not read it)"
        );
        // ...and the override's OWN note (step 1 — ODD, swung: the
        // window is SWING..SWING+GATE_OPEN = 0.15..0.85; in-step
        // ≈0.25 sits well inside) is live there.
        d.tick(step_len);
        assert_eq!(
            d.tracks[3].params.mute.value(),
            0.0,
            "bar 1 step 1: the override's note is live"
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

    /// The plan 11 instrument swap reaches the LIVE engine (CI-safe:
    /// the in-memory engine, no device — same path as
    /// `in_memory_engine_renders_the_score`): a changed `slots[3].kind`
    /// actually mutates the engine's track kind (the graph rebuilds
    /// from it — the vendor's documented swap path), a second swap
    /// diffs against the engine's CURRENT kind (the tracking, not the
    /// built-in), and a plain gain edit triggers NO kind mutation AND
    /// NO graph rebuild — the guard. The rebuild is pinned by the
    /// `kind_rebuilds` COUNTER (the kind vector alone is vacuous: it
    /// is unchanged whether the guard skips the work or the work is
    /// redone identically — the report's mutation check proves it).
    #[test]
    fn kind_change_reaches_the_engine() {
        let mut d = MusicDirector::new_fallback();
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
        d.live = Some(handle);
        // The construction snapshot (the resolved kinds — since 11c the
        // shipped bake's FM kinds for the drum/bass/lead roles, not the
        // `SLOTS` built-ins): a gain-only edit must leave these UNTOUCHED.
        let construction: Vec<PresetKind> = d.tracks.iter().map(|t| t.kind).collect();
        let engine_kinds = |d: &MusicDirector| -> Vec<PresetKind> {
            d.live
                .as_ref()
                .expect("live")
                .tracks
                .lock()
                .iter()
                .map(|t| t.kind)
                .collect()
        };

        // The snapshot AFTER construction: a fresh director + a fresh
        // in-memory engine did NO `rebuild_graph` (construction builds
        // the tracks with the resolved kinds — `build_tracks`), so the
        // counter is 0 here and every later assert is an absolute one.
        let before = d.kind_rebuilds();
        assert_eq!(before, 0, "construction does a fresh engine, not a rebuild");

        // 1. The guard: a plain gain edit changes NO kind — so nothing
        //    is mutated and NO rebuild runs. The counter is the
        //    observable (the kind vector unchanged is kept as the
        //    secondary assert; the gain itself DID apply).
        let mut score = Score::default();
        score.slots[3].gain = 0.3;
        d.set_score(score);
        assert_eq!(
            d.kind_rebuilds(),
            before,
            "a gain-only edit triggers NO graph rebuild (the guard)"
        );
        assert_eq!(
            engine_kinds(&d),
            construction,
            "a gain-only edit touches no kind (no rebuild)"
        );
        assert_eq!(
            d.tracks[3].params.gain.value(),
            0.3,
            "the gain itself applied (Shared write, no rebuild)"
        );

        // 2. The swap: the bass (slot 3 — the shipped bake's FmBass)
        //    → SuperSaw. The
        //    engine's track kind AND label actually change (the graph
        //    rebuilds from the mutated tracks — the vendor's path) —
        //    and the rebuild is counted exactly once.
        let mut score = Score::default();
        score.slots[3].kind = Some("super_saw".to_string());
        d.set_score(score);
        {
            let live_tracks = d.live.as_ref().expect("live").tracks.lock();
            assert_eq!(
                live_tracks[3].kind,
                PresetKind::SuperSaw,
                "the swapped kind reaches the engine's tracks"
            );
            assert_eq!(
                live_tracks[3].name, "SuperSaw",
                "the track's label follows the kind"
            );
        }
        assert_eq!(
            d.tracks[3].kind,
            PresetKind::SuperSaw,
            "the director's tracks mirror the swap (the next fresh engine builds it at construction)"
        );
        assert_eq!(
            d.kind_rebuilds(),
            before + 1,
            "the real swap rebuilt the graph exactly once"
        );

        // 3. The diff tracks the engine's CURRENT kind (not the
        //    built-in): a second swap (SuperSaw → PluckSaw) still
        //    diffs — one more rebuild.
        let mut score = Score::default();
        score.slots[3].kind = Some("pluck_saw".to_string());
        d.set_score(score);
        let swapped = vec![
            PresetKind::FmKick,
            PresetKind::FmSnare,
            PresetKind::FmHat,
            PresetKind::PluckSaw,
            PresetKind::FmLead,
            PresetKind::FmComp,
            PresetKind::Bell,
            PresetKind::DroneSub,
        ];
        assert_eq!(
            engine_kinds(&d),
            swapped,
            "the second swap diffs against the engine's current kind"
        );
        assert_eq!(
            d.kind_rebuilds(),
            before + 2,
            "the second real swap rebuilt exactly once more"
        );

        // 4. Re-applying the SAME kind is a no-op diff: nothing mutates
        //    AND the counter does not advance (no redundant rebuild).
        let mut same = Score::default();
        same.slots[3].kind = Some("pluck_saw".to_string());
        d.set_score(same);
        assert_eq!(
            d.kind_rebuilds(),
            before + 2,
            "re-applying the same kind is a no-op diff (no redundant rebuild)"
        );
        assert_eq!(
            engine_kinds(&d),
            swapped,
            "re-applying the same kind is a no-op diff"
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
