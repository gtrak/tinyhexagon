# Issue 11f/11g — a Genesis arrangement: the comp part and the song form

**Status:** IN PROGRESS
**Depends:** 11c (the FM kit), 11e (the dry+wet fix)
**Operator directive (verbatim):** "can I get a real mix of lead and rhythm
included? like I said, we're going for sega genesis" → "let's skip threads and
your phase 1 & 2 sounds right for the moment".

## What "lead and rhythm" requires (and what blocks it today)

Genesis/DX-style arrangements (Sonic, Streets of Rage) are built from, at
minimum: **drums**, **bass**, **comp/rhythm** (the stabbing chordal part) and
**lead** (the melody), arranged in phrases so the parts trade off.

Two structural gaps in the current engine:

1. **There is no comp role.** `Slot.chord: bool` means "follows the bar's chord
   and sustains" (pad/bell/sub); the drums take `pattern_bits` with a fixed
   pitch. A comp needs BOTH: chord-following pitch **and** a hit pattern.
   Nothing expresses that today.
2. **The melody is one bar, forever.** `sequence()` computes
   `bar = step / steps_per_bar` but `apply_step` indexes
   `notes[step % steps_per_bar]` — the same 16 notes every bar. Only the chord
   changes. Lead and rhythm cannot trade off across a phrase.

## The design

### The `Role` enum (replaces `Slot.chord` + the `index < 3` hacks)

```rust
pub enum Role {
    /// Fixed pitch (ratio-based), gated by the score's kick/snare/hat tiers.
    Drum,
    /// Step-sequenced notes from the bass line; follows the bar's chord.
    Bass,
    /// Step-sequenced notes from the lead line; on the KEY root (no chord).
    Lead,
    /// THE NEW ROLE: chord-following pitch + a hit pattern (the Genesis
    /// comp). Pitch per hit comes from the bar's chord; the hits come from
    /// the score's `comp` pattern.
    Comp,
    /// Chord-following and sustained (no step gating): pad/bell/sub.
    Harmony,
}
```

Each `Slot` gains `role: Role` and keeps `ratio`; `unlock`/`kind` stay.
`apply_bar`/`apply_step`/`apply_slot`'s drum-vs-not branches become role
matches — this also removes 11c's `index < 3` special-casing, which is
currently a latent bug source.

### The slot map after this change

| Slot | Role | Kind | Notes |
|---|---|---|---|
| 0-2 | Drum | fm_kick / fm_snare / fm_hat | unchanged |
| 3 | Bass | fm_bass | unchanged |
| 4 | Lead | fm_lead | unchanged |
| **5** | **Comp** | **fm_comp (NEW)** | was PadZimmer (the pad); the comp is rhythm, so `unlock: 0` |
| 6 | Harmony | Bell | unchanged |
| 7 | Harmony | DroneSub (sub) | unchanged |

We keep 8 voices (`MAX_TRACKS`): the pad gives way to the comp, and bell + sub
keep the sustained colour. That is the Genesis texture.

### `fm_comp` — a seventh FM voice

Add `PresetKind::FmComp` (append-only, so `ALL_KINDS`' existing order is
untouched) as another `FmVoice` table row in the shared `fm_voice` core: an FM
electric-piano stab — ratio ≈2.0, a moderate index (~400-500), a quick index
decay, and a SHORT amplitude decay (~0.15-0.25 s, so it stabs rather than
sustains). Every 11c rule holds: `pattern_bits` drives the envelope,
`stereo_gate_voiced` stays, `character`/`resonance` keep their meanings, and
`ALL_KIND_IDS` grows to 15 in the SAME order.

### The score: per-bar figures (the phrase)

`Score` gains a `bars` list (the arrangement), and the single `bass`/`lead`
become the DEFAULT bar (fully backwards compatible — an existing score with no
`bars` behaves exactly as today):

```json
"bars": [
  { "bass": [0,null,null,12,...], "lead": [12,null,15,...], "comp": "x..x..x..x..x..x" },
  { "lead": null }                    // null = "same as the default bar"
]
```

- `Bar { bass: Option<NoteLine>, lead: Option<NoteLine>, comp: Option<String> }`
- The sequencer resolves the bar index modulo `bars.len()` (the phrase), and a
  field that is `None` falls back to the top-level `bass`/`lead`/`comp`.
- The chord progression keeps its own cycle (`bars_per_chord` + the chord
  table), so the phrase length and the chord cycle are independent — e.g. an
  8-bar phrase over a 4-chord progression repeats the chords twice.
- Validation: every `bars[i].bass/lead.notes` length must equal
  `steps_per_bar`; every `comp` pattern string likewise; `bars` may be empty
  (meaning "one default bar"). All → `Err`, never a panic.

### The `comp` pattern

`Score.comp: String` (a step pattern like the drum tiers, `x` = a hit; the
pitch comes from the bar's chord root × the slot's ratio). Default: an
off-beat Genesis stab.

## Two issues, so Phase 1 is audible fast

| # | Issue | Deliverable |
|---|-------|-------------|
| 11f | the `Role` enum refactor + `fm_comp` + the `comp` pattern, one bar | **hear lead + comp + drums together** |
| 11g | the `bars` phrase (per-bar figures) | **hear an 8-bar arrangement where lead and comp trade** |

11g starts only after 11f lands.

## THE COMPOSITION BOUNDARY (operator directive, 2026-10-23)

**Workers build the MECHANISM. The orchestrator (me) writes the MUSIC.**
The operator was explicit: "when it's time to write the actual song and
composition, you do it yourself."

So, for 11f and 11g, a worker must NOT attempt to author a good-sounding part,
"improve" the melody, re-tune the arrangement by taste, or chase a vibe. Its
job is that the machinery works and is AUDIBLE, with the most obvious correct
placeholder:

- 11f: `fm_comp` exists and stabs; the `comp` pattern is a plain off-beat
  default; the slot map is rewired. The ACTUAL comp part, the balance and the
  melody come from me afterwards, by editing `assets/score.json` (it is data)
  and re-rendering.
- 11g: the `bars` phrase mechanism works and renders; the ARRANGEMENT — which
  bar has what, where lead and comp trade — is mine.

A worker report that says "I tuned the comp to sit better in the mix" is out of
scope, even when it sounds reasonable. Numbers (peak/RMS/atk-tail/headroom) are
the worker's business; musical judgement is not.

## The audition loop (this is how the operator judges it)

`audition_render` currently renders 4 bars as ONE static setup. It must render
**the whole phrase** (e.g. 8 bars) so the arrangement is audible, and it should
print which bar is which (the section markers) plus the existing per-slot table
and the atk/tail + headroom lines. The operator listens to the WAV; the numbers
guard the mix.

## Non-negotiables

- **Backwards compatibility**: a score JSON with no `bars`/`comp` must parse and
  play EXACTLY as today (the shipped bake is the reference). `mod legacy`
  fidelity tests must keep passing for the STRUCTURE they pin (the note lines
  and drum tiers); the new fields are additive.
- The mute/solo fold (11a) keeps working for every role, including `Comp`.
- The dry+wet reverb topology (11e) is untouched.
- `MAX_TRACKS` stays 8.
- No new dependencies.
- Every voice stays genuinely FM where it is FM (the index-decay envelope and
  the index-dependent spectrum).

## Verification (the orchestrator's, not the worker's)

1. `bash tools/gate.sh > /tmp/g.log 2>&1; echo $?` + grep `GATE_EXIT` — never
   `| tail`.
2. `cargo check -p th-synth --target wasm32-unknown-emscripten`.
3. Audition render: the comp is AUDIBLE and distinct (its own peak/RMS/atk/tail
   row), the mix does not clip (headroom ≈ 0 dB), and the phrase actually
   varies bar to bar (11g).
4. Default probe PASS + `OH_SCORELAB=1` PASS.
5. The operator listens.
