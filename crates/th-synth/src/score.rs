//! The chiptune score as data (plan 002 issue 10a): the [`Score`] the
//! [`MusicDirector`](crate::director::MusicDirector) sequences from.
//!
//! One representation, three surfaces (the plan 10 architecture): the
//! SHIPPED bake ([`SCORE_JSON`] — `assets/score.json`, `include_str!`-ed
//! and the source of `Score::default()`), the LIVE swap
//! (`MusicDirector::set_score` — the wasm `setScore` export parses this
//! type from the page's JSON), and the lab's "Copy JSON" button (it emits
//! [`Score::to_json_pretty`], so the baked file stays diffable against
//! the canonical text).
//!
//! `Score::default()` MUST reproduce the legacy constants' music exactly
//! — `score_default_matches_the_legacy_constants` (this module's tests)
//! compares the parsed default score against the FROZEN pre-refactor
//! (d97d4277) expressions in the private `mod legacy` (verbatim copies
//! of the d97d4277 code) — independent of the `Score::default()`
//! shorthands in `crate::director` (asserting through them would make
//! the test vacuous: both sides the same expression).
//!
//! # The plan 11g phrase (the per-bar figures)
//!
//! Since issue 11g the score carries an OPTIONAL phrase: `bars` — a
//! list of per-bar OVERRIDES of the top-level `bass`/`lead`/`comp`
//! figures. `Score::resolve_bar(bar)` resolves the bar index modulo
//! `bars.len()` and falls back to the top-level figure for each field
//! left `None`; an EMPTY list (the shipped bake — it carries no
//! `bars`) resolves every bar to the top-level figures, which IS
//! today's one-bar behaviour (the backwards-compat contract). The
//! chord cycle is independent: `chord_offset` keeps its own
//! `bars_per_chord` arithmetic over the same absolute bar.

use serde::{Deserialize, Serialize};

use rust_synth::audio::preset::PresetKind;

/// The shipped score: the bake artifact (`assets/score.json`).
pub const SCORE_JSON: &str = include_str!("../assets/score.json");

/// The 15 instrument ids, in `PresetKind`'s ALL_KINDS order (stable JSON
/// ids — the DAW's kind cycler walks this order, so it must match the
/// vendor's `ALL_KINDS` cycle order EXACTLY). The seven FM ids are
/// append-only additions (plan 002 issues 11c/11f — the last seven
/// entries).
pub const ALL_KIND_IDS: [&str; 15] = [
    "pad_zimmer",
    "bass_pulse",
    "heartbeat",
    "drone_sub",
    "shimmer",
    "bell",
    "super_saw",
    "pluck_saw",
    "fm_kick",
    "fm_snare",
    "fm_hat",
    "fm_bass",
    "fm_lead",
    "fm_bell",
    "fm_comp",
];

/// The stable JSON id of a preset kind (the `ALL_KIND_IDS` entry in
/// ALL_KINDS order).
#[must_use]
pub fn kind_id(kind: PresetKind) -> &'static str {
    match kind {
        PresetKind::PadZimmer => "pad_zimmer",
        PresetKind::BassPulse => "bass_pulse",
        PresetKind::Heartbeat => "heartbeat",
        PresetKind::DroneSub => "drone_sub",
        PresetKind::Shimmer => "shimmer",
        PresetKind::Bell => "bell",
        PresetKind::SuperSaw => "super_saw",
        PresetKind::PluckSaw => "pluck_saw",
        PresetKind::FmKick => "fm_kick",
        PresetKind::FmSnare => "fm_snare",
        PresetKind::FmHat => "fm_hat",
        PresetKind::FmBass => "fm_bass",
        PresetKind::FmLead => "fm_lead",
        PresetKind::FmBell => "fm_bell",
        PresetKind::FmComp => "fm_comp",
    }
}

/// The preset kind of a stable JSON id. Case-sensitive; `None` for an
/// unknown id (never panics — `Score::validate` turns this into an `Err`).
#[must_use]
pub fn kind_from_id(id: &str) -> Option<PresetKind> {
    Some(match id {
        "pad_zimmer" => PresetKind::PadZimmer,
        "bass_pulse" => PresetKind::BassPulse,
        "heartbeat" => PresetKind::Heartbeat,
        "drone_sub" => PresetKind::DroneSub,
        "shimmer" => PresetKind::Shimmer,
        "bell" => PresetKind::Bell,
        "super_saw" => PresetKind::SuperSaw,
        "pluck_saw" => PresetKind::PluckSaw,
        "fm_kick" => PresetKind::FmKick,
        "fm_snare" => PresetKind::FmSnare,
        "fm_hat" => PresetKind::FmHat,
        "fm_bass" => PresetKind::FmBass,
        "fm_lead" => PresetKind::FmLead,
        "fm_bell" => PresetKind::FmBell,
        "fm_comp" => PresetKind::FmComp,
        _ => return None,
    })
}

/// The comp's internal step pattern default (plan 002 issue 11f —
/// the plain off-beat Genesis stab: a hit on each beat's "and", the
/// 8th-note off-beats at steps 2/6/10/14 of the 16-step grid). Played
/// when a score leaves `comp` absent (`None`) — the backwards-compat
/// path. A 16-char string to match the default `steps_per_bar`; the
/// shipped bake carries its own `comp` pattern (score data the
/// operator edits), this constant is only the fallback.
pub const COMP_DEFAULT_PATTERN: &str = "..x...x...x...x.";

/// The step pattern's `pattern_bits` (the char-index parse every
/// pattern writer shares — `drum_pattern`'s documented discipline:
/// `x`/`X` = a hit, step 0 first, never byte arithmetic; indices >=
/// 32 skipped because they do not fit the u32 grid). Issue 11g:
/// the per-bar comp patterns' bits come through here too.
#[must_use]
pub fn pattern_bits(pattern: &str) -> u32 {
    let mut bits = 0u32;
    for (step, char) in pattern.chars().enumerate() {
        if (char == 'x' || char == 'X') && step < 32 {
            bits |= 1 << step;
        }
    }
    bits
}

/// The full chiptune score (the plan 10 schema, snake_case field names).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Score {
    /// Steps per bar: the drum grid + the melodic lines' length.
    pub steps_per_bar: u32,
    /// The staccato gate: a note plays only this fraction of its step.
    pub gate_open: f32,
    /// Odd 16ths attack this fraction of a step late (the shuffle that
    /// turns the grid into a groove).
    pub swing: f32,
    /// `bpm = base + per_stage * min(stage, cap)`.
    pub bpm: BpmRamp,
    /// `brightness = base + per_stage * min(stage, cap)` (the cap is the
    /// bpm ramp's — today's behaviour).
    pub brightness: BrightnessRamp,
    /// The stage-0 root (A2 in the default score).
    pub root_hz: f32,
    /// The root shift per 3-stage block (a fourth in the default score).
    pub root_shift: f32,
    /// How many bars one chord table entry lasts.
    pub bars_per_chord: u32,
    /// The minor chord table's stage window: `minor_from <= stage <
    /// minor_until`.
    pub minor_from: u32,
    pub minor_until: u32,
    /// The bar's chord root offsets (semitones).
    pub chords: Chords,
    /// The kick's stage tiers (pattern + optional excluded steps).
    pub kick: DrumTiered,
    /// The snare's stage tiers.
    pub snare: DrumTiered,
    /// The hat's stage tiers.
    pub hat: DrumTiered,
    /// The bass line (one note per step, `None` = rest).
    pub bass: NoteLine,
    /// The lead line (one note per step, `None` = rest).
    pub lead: NoteLine,
    /// The comp's step pattern (plan 002 issue 11f — the Genesis
    /// arrangement's chordal stab): a step pattern like the drum tiers
    /// (`x` = a hit, `X` accepted, anything else a rest; parsed by CHAR
    /// index, exactly `steps_per_bar` chars — validated). The pattern
    /// drives the comp's `pattern_bits` (the FM envelope's gate); the
    /// PITCH is the bar's chord root × the comp slot's ratio (the
    /// comp follows the chord, unlike the lead). `None` (a score with
    /// no `comp` field — the backwards-compat path) plays the internal
    /// off-beat default ([`COMP_DEFAULT_PATTERN`]).
    #[serde(default)]
    pub comp: Option<String>,
    /// The phrase (plan 002 issue 11g — the song form): the per-bar
    /// figures, resolved modulo `len` over the ABSOLUTE bar counter —
    /// INDEPENDENT of the chord cycle (`chord_offset` keeps its own
    /// `bars_per_chord` + table arithmetic over the same absolute bar,
    /// so an 8-bar phrase over a 4-chord progression cycles the chords
    /// twice). A `None` field falls back to the top-level figure; an
    /// EMPTY list means "no phrase": one bar whose figures are the
    /// top-level `bass`/`lead`/`comp` — exactly today's behaviour
    /// (a score written before this issue parses and plays unchanged
    /// — the backwards-compat contract; the shipped bake is the
    /// reference). `len` is sanity-bounded to 64 (`validate`).
    #[serde(default)]
    pub bars: Vec<Bar>,
    /// The 8 slots' mix gains, engine order (`SLOTS`' order).
    pub slots: Vec<SlotGain>,
    /// The baked solo (an FL-project track solo, plan 11): the
    /// effective-mute fold silences every slot except this one
    /// (`None` = no solo — the shipped bake; a stray baked solo would
    /// silence the game for a child, so the DAW panel gets a
    /// "clear mix" button).
    #[serde(default)]
    pub solo: Option<u32>,
}

/// The bpm ramp: `base + per_stage * min(stage, cap)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BpmRamp {
    /// The stage-0 tempo.
    pub base: f32,
    /// The per-stage step.
    pub per_stage: f32,
    /// The ramp's stage cap (flat after it).
    pub cap: u32,
}

/// The brightness ramp: `base + per_stage * min(stage, the bpm cap)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BrightnessRamp {
    /// The stage-0 brightness.
    pub base: f32,
    /// The per-stage step.
    pub per_stage: f32,
}

/// A drum's stage-tiered pattern list (the director resolves the last
/// tier whose `stage <= stage`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DrumTiered {
    /// The tiers, ascending by stage (the first must be stage 0).
    pub tiers: Vec<DrumTier>,
}

/// One drum pattern tier: from `stage` on, `pattern` (with `exclude`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DrumTier {
    /// The first stage this pattern covers.
    pub stage: u32,
    /// The step pattern: exactly `steps_per_bar` chars, `x`/`X` = hit,
    /// anything else = rest (step 0 first). Parsed by CHAR index, never
    /// byte arithmetic (the strings are ASCII today; the parse must not
    /// rely on it).
    pub pattern: String,
    /// Step indices forced OFF in this tier (the late-hat case: all 16
    /// minus the snares' steps). Omitted from the JSON when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<u32>,
}

/// A melodic line: one semitone offset per step, `None` = a rest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NoteLine {
    /// One entry per step: the semitone offset above the line's root
    /// (the bass: the bar's chord root; the lead: the key root — the
    /// lead deliberately ignores the chord's offset).
    pub notes: Vec<Option<i32>>,
    /// The line's octave: `2^octave_shift` multiplies the root before
    /// the note's semitones (the default score: bass -1 — the legacy
    /// `root * 0.5`; lead +1 — the legacy `root * 2.0`).
    pub octave_shift: i32,
}

/// One bar of the phrase (plan 002 issue 11g — the song form): the
/// per-bar OVERRIDES of the top-level figures. A `None` field falls
/// back to the top-level value (`Score.bass`/`lead`/`comp`) — so
/// `Bar { lead: None, .. }` means "this bar uses the default lead
/// line". An empty `bars` list is "no phrase": every bar resolves to
/// the top-level figures — exactly today's behaviour (the
/// backwards-compat contract; the shipped bake carries no `bars`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Bar {
    /// This bar's bass line (a FULL line — `notes` AND
    /// `octave_shift`). `None` = the top-level `bass`.
    #[serde(default)]
    pub bass: Option<NoteLine>,
    /// This bar's lead line. `None` = the top-level `lead`.
    #[serde(default)]
    pub lead: Option<NoteLine>,
    /// This bar's comp step pattern. `None` = the top-level `comp`
    /// (which itself falls back to the internal off-beat default,
    /// [`COMP_DEFAULT_PATTERN`], when absent).
    #[serde(default)]
    pub comp: Option<String>,
}

/// One bar's RESOLVED figures (plan 002 issue 11g — `Score::
/// resolve_bar`'s result): the top-level figures with the phrase's
/// per-bar overrides applied. An empty `bars` list resolves every bar
/// to exactly the top-level figures — today's behaviour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedBar<'a> {
    /// The bass line the bar plays (the top-level `bass` unless the
    /// phrase bar overrides it — `notes` AND `octave_shift` are the
    /// override's when it is present).
    pub bass: &'a NoteLine,
    /// The lead line the bar plays (the top-level `lead` unless the
    /// phrase bar overrides it).
    pub lead: &'a NoteLine,
    /// The comp's step pattern the bar plays (always present: the
    /// phrase bar's `comp`, else the top-level `comp`, else
    /// [`COMP_DEFAULT_PATTERN`]).
    pub comp: &'a str,
}

/// One slot's mix gain (the `slots` array; engine/SLOTS order).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SlotGain {
    /// The slot's fixed mix gain.
    pub gain: f32,
    /// The slot's instrument (the plan 11 role/timbre split): the stable
    /// kind id ([`ALL_KIND_IDS`]). `None` = the slot's built-in kind
    /// (`director::SLOTS[i].kind`) — which keeps an issue-10 score file
    /// (no `kind` fields) valid and keeps the shipped bake diff-free.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The slot's PITCH INTERVAL as a frequency multiplier of the bar's
    /// chord root — this is what makes CHORDS possible: the comp plus
    /// the two harmony voices at, say, 2.0 (root), 2.52 (a major third
    /// up) and 3.0 (a fifth up) sound a triad. `None` = the slot's
    /// built-in ratio (`director::SLOTS[i].ratio`), so every existing
    /// score file is unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ratio: Option<f32>,
    /// The baked mute (an FL-project track mute, plan 11): the
    /// effective-mute fold ORs it with the program state EVERY tick, so
    /// a `true` written once survives the gate's every-tick rewrites.
    #[serde(default)]
    pub muted: bool,
}

/// The bar's chord root offset tables (semitones).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Chords {
    /// The major progression (I, vi, IV, V in the default score).
    pub major: Vec<i32>,
    /// The minor progression (i, VI, III, VII in the default score).
    pub minor: Vec<i32>,
}

/// A stage's mapped parameters (the score-driven `stage_params`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageParams {
    /// The stage's tempo.
    pub bpm: f32,
    /// The stage's brightness.
    pub brightness: f32,
    /// Root-shift blocks elapsed (a `root_shift` up per block).
    pub fourths: u32,
    root_hz: f32,
    root_shift: f32,
}

impl StageParams {
    /// The stage's root frequency: `root_hz * root_shift^fourths`.
    #[must_use]
    pub fn root(&self) -> f32 {
        self.root_hz * self.root_shift.powi(self.fourths as i32)
    }
}

impl Default for Score {
    /// The shipped score: parse [`SCORE_JSON`] (the bake). A corrupt
    /// baked file is a build-time asset bug, not user input — `expect`,
    /// not a fallible default.
    fn default() -> Self {
        Score::from_json(SCORE_JSON).expect(
            "the shipped assets/score.json failed validation (a build-time asset, not user input)",
        )
    }
}

impl Score {
    /// The canonical JSON text (pretty — the BAKE artifact: the lab's
    /// "Copy JSON" button emits exactly this, so `assets/score.json`
    /// stays diffable against the parsed score).
    #[must_use]
    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("a valid Score serializes")
    }

    /// The comp's step pattern as `pattern_bits` (plan 002 issue 11f):
    /// the same char-index parse discipline as [`drum_pattern`] —
    /// `x`/`X` = a hit, step 0 first, never byte arithmetic — with
    /// step indices >= 32 skipped (they do not fit the u32 grid;
    /// `drum_pattern`'s documented convention). `None` (a score
    /// without a `comp` field) resolves to the internal off-beat
    /// default so an old score still plays the comp (the backwards-
    /// compat rule — the pattern is additive data, not a requirement).
    #[must_use]
    pub fn comp_pattern_bits(&self) -> u32 {
        pattern_bits(self.comp.as_deref().unwrap_or(COMP_DEFAULT_PATTERN))
    }

    /// The phrase's length in bars (plan 002 issue 11g — the
    /// resolution modulus the sequencer's bar index wraps on): the
    /// `bars` count when the score carries a phrase, `1` (the single
    /// default bar) when `bars` is empty — today's one-bar behaviour.
    #[must_use]
    pub fn phrase_len(&self) -> usize {
        if self.bars.is_empty() {
            1
        } else {
            self.bars.len()
        }
    }

    /// The bar's RESOLVED figures (plan 002 issue 11g — the phrase's
    /// resolution the sequencer uses): the bar index is resolved
    /// modulo `bars.len()` (the phrase cycles), and each figure falls
    /// back to the top-level value when the phrase bar leaves it
    /// `None` (for `comp`, the top-level `comp` field, then the
    /// internal off-beat default). An EMPTY `bars` list resolves
    /// EVERY bar to the top-level figures — exactly today's behaviour
    /// (the backwards-compat contract).
    #[must_use]
    pub fn resolve_bar(&self, bar: u64) -> ResolvedBar<'_> {
        let entry = if self.bars.is_empty() {
            None
        } else {
            Some(&self.bars[(bar % self.bars.len() as u64) as usize])
        };
        ResolvedBar {
            bass: entry.and_then(|b| b.bass.as_ref()).unwrap_or(&self.bass),
            lead: entry.and_then(|b| b.lead.as_ref()).unwrap_or(&self.lead),
            comp: entry
                .and_then(|b| b.comp.as_deref())
                .or(self.comp.as_deref())
                .unwrap_or(COMP_DEFAULT_PATTERN),
        }
    }

    /// Parse + validate a score. `Err(message)` is human-facing
    /// (surfaced in the lab's status line): serde's own error text — it
    /// carries the field path and line/column when serde gives one — for
    /// parse failures, and the failing field's path prefixed for the
    /// validation failures. A failure is an `Err`, never a panic.
    pub fn from_json(text: &str) -> Result<Self, String> {
        let score: Score =
            serde_json::from_str(text).map_err(|err| format!("score json: {err}"))?;
        score.validate()?;
        Ok(score)
    }

    /// The tier resolution the director uses: the last tier whose
    /// `stage <= stage` (validated tiers start at 0, so this always
    /// resolves), the pattern's `x`/`X` steps on, the tier's `exclude`
    /// steps forced off.
    ///
    /// The engine's `pattern_bits` is a `u32` (a 16-step grid), so step
    /// indices beyond 32 do not fit the grid and are skipped (a valid
    /// score's patterns never reach them — the default is 16).
    #[must_use]
    pub fn drum_pattern(&self, tiered: &DrumTiered, stage: u32) -> u32 {
        let tier = tiered
            .tiers
            .iter()
            .rev()
            .find(|t| t.stage <= stage)
            .expect(
                "drum tiers must be non-empty with tiers[0].stage == 0 (Score::from_json validates this; a hand-built Score must too)",
            );
        let mut bits = pattern_bits(&tier.pattern);
        for &excluded in &tier.exclude {
            if excluded < 32 {
                bits &= !(1 << excluded);
            }
        }
        bits
    }

    /// A stage's bpm/brightness/root — the score-driven `stage_params`.
    #[must_use]
    pub fn stage_params(&self, stage: u32) -> StageParams {
        let s = stage.min(self.bpm.cap);
        StageParams {
            bpm: self.bpm.base + self.bpm.per_stage * s as f32,
            brightness: self.brightness.base + self.brightness.per_stage * s as f32,
            fourths: s / 3,
            root_hz: self.root_hz,
            root_shift: self.root_shift,
        }
    }

    /// The bar's chord root offset (semitones): the major/minor table
    /// selected by the stage window, one entry per `bars_per_chord`
    /// bars (1 in the default score — today's one-chord-per-bar; a
    /// trivial index divide, so the field is honoured).
    #[must_use]
    pub fn chord_offset(&self, stage: u32, bar: u64) -> i32 {
        let table = if self.minor_for_stage(stage) {
            &self.chords.minor
        } else {
            &self.chords.major
        };
        let entry = (bar / u64::from(self.bars_per_chord)) % (table.len() as u64);
        table[entry as usize]
    }

    /// `true` inside the minor chord table's stage window.
    #[must_use]
    pub fn minor_for_stage(&self, stage: u32) -> bool {
        self.minor_from <= stage && stage < self.minor_until
    }

    /// The soundness checks `from_json` runs after the serde pass: a
    /// score that would make the director index out of bounds (or
    /// resolve no tier for some stage) is an `Err`, never a panic.
    fn validate(&self) -> Result<(), String> {
        let steps = self.steps_per_bar;
        if !(1..=64).contains(&steps) {
            return Err(format!("steps_per_bar: must be 1..=64 (got {steps})"));
        }
        for (name, line) in [("bass", &self.bass), ("lead", &self.lead)] {
            if line.notes.len() != steps as usize {
                return Err(format!(
                    "{name}.notes: must have {steps} entries, one per step (got {})",
                    line.notes.len()
                ));
            }
        }
        if let Some(comp) = &self.comp {
            let len = comp.chars().count();
            if len != steps as usize {
                return Err(format!(
                    "comp: must be {steps} chars, one per step (got {len})"
                ));
            }
        }
        // The phrase's per-bar figures (issue 11g): the sanity bound
        // first (an unbounded phrase is a score bug, not a feature),
        // then the same length discipline as the top-level figures —
        // an override, not a definition, still has to fit the grid.
        if self.bars.len() > 64 {
            return Err(format!(
                "bars: must have at most 64 entries (a phrase, not an unbounded song) (got {})",
                self.bars.len()
            ));
        }
        for (i, bar) in self.bars.iter().enumerate() {
            if let Some(line) = &bar.bass {
                if line.notes.len() != steps as usize {
                    return Err(format!(
                        "bars[{i}].bass.notes: must have {steps} entries, one per step (got {})",
                        line.notes.len()
                    ));
                }
            }
            if let Some(line) = &bar.lead {
                if line.notes.len() != steps as usize {
                    return Err(format!(
                        "bars[{i}].lead.notes: must have {steps} entries, one per step (got {})",
                        line.notes.len()
                    ));
                }
            }
            if let Some(comp) = &bar.comp {
                let len = comp.chars().count();
                if len != steps as usize {
                    return Err(format!(
                        "bars[{i}].comp: must be {steps} chars, one per step (got {len})"
                    ));
                }
            }
        }
        for (name, table) in [
            ("chords.major", &self.chords.major),
            ("chords.minor", &self.chords.minor),
        ] {
            if !(1..=16).contains(&table.len()) {
                return Err(format!(
                    "{name}: must have 1..=16 entries (got {})",
                    table.len()
                ));
            }
        }
        for (i, slot) in self.slots.iter().enumerate() {
            if let Some(r) = slot.ratio {
                if !r.is_finite() || r <= 0.0 {
                    return Err(format!(
                        "slots[{i}].ratio: must be a positive frequency multiplier (got {r})"
                    ));
                }
            }
        }
        if self.slots.len() != 8 {
            return Err(format!(
                "slots: must have exactly 8 entries (got {})",
                self.slots.len()
            ));
        }
        for (i, slot) in self.slots.iter().enumerate() {
            if let Some(kind) = &slot.kind {
                if kind_from_id(kind).is_none() {
                    return Err(format!(
                        "slots[{i}].kind: \"{kind}\" is not a known kind id (expected one of {})",
                        ALL_KIND_IDS.join(", ")
                    ));
                }
            }
        }
        if let Some(solo) = self.solo {
            if !(solo < self.slots.len() as u32) {
                return Err(format!(
                    "solo: {solo} must index one of the {} slots (0..{}), or be absent",
                    self.slots.len(),
                    self.slots.len()
                ));
            }
        }
        if self.bars_per_chord == 0 {
            return Err(
                "bars_per_chord: must be >= 1 (one chord entry needs at least one bar)".to_string(),
            );
        }
        if !(self.minor_from < self.minor_until) {
            return Err(format!(
                "minor window: minor_from ({}) must be < minor_until ({})",
                self.minor_from, self.minor_until
            ));
        }
        for (name, tiered) in [
            ("kick", &self.kick),
            ("snare", &self.snare),
            ("hat", &self.hat),
        ] {
            let tiers = &tiered.tiers;
            if tiers.is_empty() {
                return Err(format!("{name}.tiers: must not be empty"));
            }
            if tiers[0].stage != 0 {
                return Err(format!(
                    "{name}.tiers[0].stage: must be 0 so every stage resolves a tier (got {})",
                    tiers[0].stage
                ));
            }
            for (i, tier) in tiers.iter().enumerate() {
                let len = tier.pattern.chars().count();
                if len != steps as usize {
                    return Err(format!(
                        "{name}.tiers[{i}].pattern: must be {steps} chars, one per step (got {len})"
                    ));
                }
                for &excluded in &tier.exclude {
                    if excluded >= steps {
                        return Err(format!(
                            "{name}.tiers[{i}].exclude: step {excluded} is outside 0..{steps}"
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frozen copies of the pre-refactor (d97d4277) music expressions —
    /// the bake-fidelity contract. `score_default_matches_the_legacy_constants`
    /// compares `Score::default()` against THESE, not against
    /// `crate::director`'s free functions: since 10a those are
    /// `Score::default()` shorthands, and asserting the default score
    /// through them would be circular (both sides the same expression,
    /// so the test could not fail on a wrong bake).
    mod legacy {
        /// Stage-0 root: A2 (d97d4277 `ROOT_A2_HZ`).
        const ROOT_A2_HZ: f32 = 110.0;
        /// The root shift per 3-stage block: up a fourth
        /// (d97d4277 `ROOT_SHIFT`).
        const ROOT_SHIFT: f32 = 4.0 / 3.0;
        /// Stage cap of the bpm/brightness/root ramps
        /// (d97d4277 `STAGE_RAMP_CAP`).
        const STAGE_RAMP_CAP: u32 = 20;
        /// `bpm = 150 + 1.2 * min(stage, 20)` → 174 (d97d4277 `BPM_BASE`).
        const BPM_BASE: f32 = 150.0;
        const BPM_PER_STAGE: f32 = 1.2;
        /// `brightness = 0.45 + 0.025 * min(stage, 20)` → 0.95
        /// (d97d4277 `BRIGHTNESS_BASE`).
        const BRIGHTNESS_BASE: f32 = 0.45;
        const BRIGHTNESS_PER_STAGE: f32 = 0.025;
        /// Major progression (semitones): I, vi, IV, V
        /// (d97d4277 `CHORD_MAJOR`).
        const CHORD_MAJOR: [i32; 4] = [0, 9, 5, 7];
        /// Minor progression: i, VI, III, VII (d97d4277 `CHORD_MINOR`).
        const CHORD_MINOR: [i32; 4] = [0, 8, 3, 10];
        /// The syncopated bass line (d97d4277 `BASS_LINE`).
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
        /// The lead motif (d97d4277 `LEAD_LINE`).
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

        /// The pre-refactor stage → (bpm, brightness, fourths, root) map:
        /// d97d4277's `stage_params` + `StageParams::root` frozen into a
        /// tuple (the original `root()` = `ROOT_A2_HZ * ROOT_SHIFT.powi(fourths)`).
        ///
        /// `pub` within the private `legacy` module: the `tests` module
        /// (the parent) names these, which Rust's privacy allows only for
        /// `pub` items of a child module.
        pub fn legacy_stage_params(stage: u32) -> (f32, f32, u32, f32) {
            let s = stage.min(STAGE_RAMP_CAP);
            let fourths = s / 3;
            (
                BPM_BASE + BPM_PER_STAGE * s as f32,
                BRIGHTNESS_BASE + BRIGHTNESS_PER_STAGE * s as f32,
                fourths,
                ROOT_A2_HZ * ROOT_SHIFT.powi(fourths as i32),
            )
        }

        /// The pre-refactor kick pattern (d97d4277 `kick_bits`, verbatim
        /// literal bitmask expression).
        pub fn legacy_kick_bits(stage: u32) -> u32 {
            let mut bits = (1 << 0) | (1 << 7) | (1 << 10);
            if stage >= 10 {
                bits |= 1 << 3;
            }
            if stage >= 16 {
                bits |= 1 << 13;
            }
            bits
        }

        /// The pre-refactor snare pattern (d97d4277 `snare_bits`, verbatim
        /// literal bitmask expression).
        pub fn legacy_snare_bits(stage: u32) -> u32 {
            let mut bits = (1 << 4) | (1 << 12);
            if stage >= 12 {
                bits |= 1 << 14;
            }
            bits
        }

        /// The pre-refactor hat pattern (d97d4277 `hat_bits`, verbatim —
        /// the three literal branches).
        pub fn legacy_hat_bits(stage: u32) -> u32 {
            if stage < 6 {
                (1 << 2) | (1 << 6) | (1 << 10) | (1 << 14)
            } else if stage < 14 {
                (1 << 1)
                    | (1 << 3)
                    | (1 << 5)
                    | (1 << 7)
                    | (1 << 9)
                    | (1 << 11)
                    | (1 << 13)
                    | (1 << 15)
            } else {
                0xFFFF & !(1 << 4) & !(1 << 12)
            }
        }

        /// The pre-refactor chord offset (d97d4277 `chord_offset`,
        /// verbatim — the original `table[(bar % 4) as usize]` indexing).
        pub fn legacy_chord_offset(stage: u32, bar: u64) -> i32 {
            let table = if legacy_minor_for_stage(stage) {
                CHORD_MINOR
            } else {
                CHORD_MAJOR
            };
            table[(bar % 4) as usize]
        }

        /// The pre-refactor minor half (d97d4277 `minor_for_stage`,
        /// verbatim).
        pub fn legacy_minor_for_stage(stage: u32) -> bool {
            (8..16).contains(&stage)
        }

        /// The pre-refactor bass line (d97d4277 `BASS_LINE`, verbatim).
        pub fn legacy_bass_line() -> [Option<i32>; 16] {
            BASS_LINE
        }

        /// The pre-refactor lead line (d97d4277 `LEAD_LINE`, verbatim).
        pub fn legacy_lead_line() -> [Option<i32>; 16] {
            LEAD_LINE
        }
    }

    /// The canonical pretty text re-parses to the same score (the lab's
    /// "Copy JSON" → `setScore` round-trip contract).
    #[test]
    fn score_default_round_trips() {
        let score = Score::default();
        let parsed =
            Score::from_json(&score.to_json_pretty()).expect("the canonical pretty text re-parses");
        assert_eq!(parsed, score);
    }

    /// The 11a kind-id mapping pins the vendor's ALL_KINDS cycle order
    /// (a UI cycling kinds walks a STABLE order — a drift between the
    /// ids and the vendor's enum breaks the cycle, so this asserts the
    /// pairing against `ALL_KINDS` itself, plus the case-sensitivity and
    /// unknown-id contracts of `kind_from_id`).
    #[test]
    fn kind_ids_follow_the_vendor_cycle_order() {
        use rust_synth::audio::preset::ALL_KINDS;
        for (id, kind) in ALL_KIND_IDS.iter().zip(ALL_KINDS.iter()) {
            assert_eq!(kind_id(*kind), *id, "the stable id of {kind:?}");
            assert_eq!(kind_from_id(id), Some(*kind), "the round-trip of {id}");
        }
        assert_eq!(
            kind_from_id("PAD_ZIMMER"),
            None,
            "kind_from_id is case-sensitive"
        );
        assert_eq!(kind_from_id("nope"), None, "an unknown id is None");
    }

    /// A bad kind id or out-of-range solo is an `Err` naming the culprit
    /// (the lab's status line must be able to surface both), not a panic
    /// or a silently-ignored field.
    #[test]
    fn unknown_kind_is_an_error() {
        // A kind id the mapping does not know: the Err names the slot
        // index and the bad id.
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        value["slots"][3]["kind"] = serde_json::json!("nope");
        let err = Score::from_json(&value.to_string()).unwrap_err();
        assert!(err.contains("slots[3]"), "names the slot index: {err}");
        assert!(err.contains("nope"), "names the bad id: {err}");

        // A solo that indexes no slot.
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        value["solo"] = serde_json::json!(99);
        let err = Score::from_json(&value.to_string()).unwrap_err();
        assert!(err.contains("solo"), "names the solo field: {err}");
    }

    /// The bake IS the canonical text: `assets/score.json` is
    /// `Score::default().to_json_pretty()` + the conventional trailing
    /// newline — so the file stays diffable against the parsed default
    /// (a drift in the file's values OR formatting breaks this, not
    /// just the round-trip).
    #[test]
    fn the_baked_score_is_the_canonical_pretty_text() {
        let canonical = Score::default().to_json_pretty();
        assert_eq!(
            SCORE_JSON,
            format!("{canonical}\n"),
            "assets/score.json must be the canonical pretty text + trailing newline"
        );
    }

    /// THE bake-fidelity test (the 10a fix pass): for stages 0..=20 the
    /// default score's kick/snare/hat bitmasks equal the FROZEN legacy
    /// bitmasks (`mod legacy`, the d97d4277 expressions verbatim — NOT
    /// `crate::director`'s shorthands, which are `Score::default()`
    /// rewrites and would make this circular), its stage ramp equals the
    /// frozen legacy `stage_params` (bpm/brightness/root within 1e-4 —
    /// the JSON stores 1.3333334 for `4.0/3.0` — fourths exactly), its
    /// minor window and chord offsets equal the frozen legacy
    /// `minor_for_stage`/`chord_offset` (both chord tables, a full
    /// cycle of bars), and its bass/lead lines are the frozen legacy
    /// arrays. A failure here is a REAL finding: the baked JSON does not
    /// reproduce the pre-refactor music.
    #[test]
    fn score_default_matches_the_legacy_constants() {
        use self::legacy::{
            legacy_bass_line, legacy_chord_offset, legacy_hat_bits, legacy_kick_bits,
            legacy_lead_line, legacy_minor_for_stage, legacy_snare_bits, legacy_stage_params,
        };
        // The bake is the operator's music now; the frozen constants
        // are pinned by the LEGACY reference score.
        let score = crate::director::legacy_score().clone();
        assert_eq!(score.bass.notes, legacy_bass_line(), "bass line");
        assert_eq!(score.lead.notes, legacy_lead_line(), "lead line");
        for stage in 0..=20 {
            assert_eq!(
                score.drum_pattern(&score.kick, stage),
                legacy_kick_bits(stage),
                "kick stage {stage}"
            );
            assert_eq!(
                score.drum_pattern(&score.snare, stage),
                legacy_snare_bits(stage),
                "snare stage {stage}"
            );
            assert_eq!(
                score.drum_pattern(&score.hat, stage),
                legacy_hat_bits(stage),
                "hat stage {stage}"
            );
            let p = score.stage_params(stage);
            let (l_bpm, l_brightness, l_fourths, l_root) = legacy_stage_params(stage);
            assert!(
                (p.bpm - l_bpm).abs() < 1e-4,
                "bpm stage {stage} ({} vs {l_bpm})",
                p.bpm
            );
            assert!(
                (p.brightness - l_brightness).abs() < 1e-4,
                "brightness stage {stage} ({} vs {l_brightness})",
                p.brightness
            );
            assert_eq!(p.fourths, l_fourths, "fourths stage {stage}");
            assert!(
                (p.root() - l_root).abs() < 1e-4,
                "root stage {stage} ({} vs {l_root})",
                p.root()
            );
            assert_eq!(
                score.minor_for_stage(stage),
                legacy_minor_for_stage(stage),
                "minor window stage {stage}"
            );
            for bar in 0..8u64 {
                assert_eq!(
                    score.chord_offset(stage, bar),
                    legacy_chord_offset(stage, bar),
                    "chord stage {stage} bar {bar}"
                );
            }
        }
    }

    /// A bad score is an `Err` (human-readable, no panic) — the four
    /// shapes the lab's status line must be able to surface.
    #[test]
    fn a_bad_score_is_an_error_not_a_panic() {
        // The wrong root: not even an object.
        let err = Score::from_json("{}").unwrap_err();
        assert!(err.contains("steps_per_bar"), "missing field: {err}");

        // Right root, missing everything else (serde's missing-field
        // text carries the field name).
        let err = Score::from_json("{\"steps_per_bar\":16}").unwrap_err();
        assert!(err.contains("gate_open"), "missing field: {err}");

        // A pattern string of the wrong length (validated, not parsed
        // into a panic by the director).
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        value["kick"]["tiers"][0]["pattern"] = serde_json::json!("x");
        let err = Score::from_json(&value.to_string()).unwrap_err();
        assert!(
            err.contains("kick.tiers[0].pattern"),
            "pattern length: {err}"
        );

        // A `comp` pattern of the wrong length (issue 11f — the same
        // discipline as the drum tiers: exactly `steps_per_bar` chars).
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        value["comp"] = serde_json::json!("x");
        let err = Score::from_json(&value.to_string()).unwrap_err();
        assert!(err.contains("comp"), "comp pattern length: {err}");

        // An absent `comp` is the backwards-compat path (serde default
        // → `None` — the off-beat internal default, not an error).
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        value
            .as_object_mut()
            .expect("the score is an object")
            .remove("comp");
        assert!(
            Score::from_json(&value.to_string()).is_ok(),
            "no `comp` parses"
        );

        // A `slots` array of 7 (the 8-slot contract).
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        value["slots"]
            .as_array_mut()
            .expect("slots is an array")
            .pop();
        let err = Score::from_json(&value.to_string()).unwrap_err();
        assert!(err.contains("slots"), "slots count: {err}");
    }

    /// Plan 002 issue 11g — a bad BAR is an `Err` NAMING THE BAR
    /// INDEX (the lab's status line must be able to surface it),
    /// never a panic: a wrong-length per-bar note line, a
    /// wrong-length per-bar comp pattern (on bar 1 — the index, not
    /// just the field), and an over-long `bars` list (past the 64-bar
    /// sanity bound). The VALID degenerate case: a bar that overrides
    /// nothing (every field `None`) falls back to the top-level
    /// figures, so it parses and resolves to the defaults.
    #[test]
    fn a_bad_bar_is_an_error() {
        // A per-bar bass line of the wrong length.
        let mut value = serde_json::to_value(Score::default()).expect("default serializes");
        value["bars"] = serde_json::json!([{ "bass": { "notes": [0, null], "octave_shift": -1 } }]);
        let err = Score::from_json(&value.to_string()).unwrap_err();
        assert!(err.contains("bars[0].bass"), "names the bar index: {err}");

        // A per-bar comp pattern of the wrong length (bar 1 — the
        // index, not just the field).
        value = serde_json::to_value(Score::default()).expect("default serializes");
        value["bars"] = serde_json::json!([{}, { "comp": "x" }]);
        let err = Score::from_json(&value.to_string()).unwrap_err();
        assert!(err.contains("bars[1].comp"), "names the bar index: {err}");

        // An over-long bars list (more than the 64-bar sanity bound).
        value = serde_json::to_value(Score::default()).expect("default serializes");
        value["bars"] = serde_json::json!(vec![serde_json::json!({}); 65]);
        let err = Score::from_json(&value.to_string()).unwrap_err();
        assert!(err.contains("bars"), "names the field: {err}");

        // The valid degenerate case: a bar that overrides NOTHING
        // (all fields `None`) parses and resolves to the top-level
        // figures (the fallback contract).
        value = serde_json::to_value(Score::default()).expect("default serializes");
        value["bars"] = serde_json::json!([{}, {}]);
        let score = Score::from_json(&value.to_string()).expect("all-null bars parse");
        assert_eq!(score.phrase_len(), 2);
        assert_eq!(score.resolve_bar(0), score.resolve_bar(1));
        assert_eq!(score.resolve_bar(0).bass, &score.bass);
        assert_eq!(score.resolve_bar(0).lead, &score.lead);
    }
}
