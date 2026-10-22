# Issue 11c/11d — a Sonic-style FM instrument kit + the audition editor

**Status:** IN PROGRESS
**Depends:** 11a/11b (the DAW landed: role/timbre split, the kind picker)
**Operator directive (verbatim):** "these instruments suck, give me better ones
from rust-synth. How did we come up with them?" → "I want an instrument kit
that is similar to sonic the hedgehog fm synthesis, audition editor sounds
fine, but give me a good start." → "also I want to mute the game sfx from the
beat editor."

## Why the current kit is bad (the honest provenance)

Recorded here so it is not repeated. The 8 slots were NOT chosen for sound:

1. Plan 05 (`9fbc42f1`) picked 8 presets for a **rising-unlock ambient** piece
   (PadZimmer stage 0 … Heartbeat stage **14**, SuperSaw **17**).
2. The chiptune rewrite (`6780a935`) discarded that score and kept the same 8
   presets, **re-purposing** them: slots 0-2 became kick/snare/hat, 3 bass,
   4 lead. The `Slot` numbers were then tuned until the pattern was audible —
   not auditioned as instruments.

Consequence: **two of the three drum slots are the same preset** (`Heartbeat`
at `ratio` 0.5 / 1.6 / 8.0), and all three are a 3-layer ambient kick, not a
drum kit. The presets are ambient/trance voices (PadZimmer, Shimmer, SuperSaw,
DroneSub) that have no business in a drum-and-bass score.

## The target: YM2612-style FM (Sonic)

The Genesis's YM2612 is **FM**: a sine carrier whose phase is modulated by
another sine, with a **decaying modulation index** — that envelope, not the
ratio, is what makes FM sound like FM. The vendor already proves the idiom is
reachable: `bell_preset` builds `(carrier_freq + modulator*scale) >> sine()`
with a `character`-driven ratio.

So the kit is **6 new `PresetKind`s**, one shared core, each voice a table row:

| Kind | Design (YM2612 idiom) |
|---|---|
| `fm_kick` | Sine carrier + fast downward pitch sweep (≈×4 → ×1 over 60 ms); a short high-ratio modulator gives the click transient. |
| `fm_snare` | Noise burst + a short pitched FM body; `character` = noise/tone balance. |
| `fm_hat` | Inharmonic high ratio (≈7-11) with a 20-40 ms decay; `character` = ratio/brightness. |
| `fm_bass` | Sine carrier, modulator ratio ≈1:1-1:2, **decaying index** → the FM bass thump. |
| `fm_lead` | 2-op, ratio ≈1:2-1:3, moderate index, slight detune; the melodic voice. |
| `fm_bell` | Short percussive FM (ratio ≈3.5-4) — the Sonic marimba/celesta. |

One `fm_voice(p, g, cfg: FmVoice)` core, with `FmVoice` a struct of the numbers
above; `Preset::build` dispatches the 6 kinds to it. `character` morphs the
ratio (via the existing `lerp3` idiom: 0.5 reproduces the tuned default),
`resonance` scales the modulation index, `cutoff`/`detune`/`reverb_mix` behave
as elsewhere.

### Why inside the vendor (and the provenance discipline)

We already vendor `rust-synth` legitimately (the TUI/recording modules do not
build on wasm/Android — see the Cargo.toml provenance block). That patch's
rule was "keep the DSP/presets unchanged". This issue **deliberately relaxes
that rule for the first time**, and the relaxation must be documented in the
same place, in the same style:

- `vendor/rust-synth-0.23.0/Cargo.toml`'s provenance block gains a bullet:
  the FM kit is OUR addition (plan 002 issue 11c), append-only to `PresetKind`.
- `PresetKind` is **append-only** (the 6 new variants go last) so `ALL_KINDS`'s
  existing cycle order and `default_track_set` are untouched in meaning.
- The six new `PresetKind`s must be handled by every exhaustive match: the enum
  itself, `label()`, `ALL_KINDS`, `Preset::build`. `default_track_set` may add
  them as dormant slots (nice for the TUI) or leave them — if `ALL_KINDS` grows
  its length, `crates/th-synth/src/score.rs`'s `ALL_KIND_IDS: [&str; 8]` must
  grow with it (a `[&str; 14]`), and the page's picker follows automatically.

## The kit must be AUDIBLE without a browser — the audition renderer

The operator cannot judge FM from a diff, and the previous kit's failure was
exactly a failure of auditioning. So this issue ships an **offline WAV
renderer**: `cargo run -p th-synth --example audition_render -- out.wav` renders
the REAL director setup (the actual slots, kinds and the shipped drum tier
patterns) for a few bars at 48 kHz stereo, so the operator can listen on their
own machine before any game build. It is both the design instrument and the
deliverable.

## The bake (no schema change in 11c)

The shipped `assets/score.json` gains the re-timbre: `slots[i].kind` set to the
FM ids for the roles that should use them (kick/snare/hat/bass/lead). This is
exactly the mechanism 11a added — no new schema, and "Copy JSON" keeps working.

`apply_slot`'s per-preset blocks gain the FM arms with **auditioned** values
(the renderer is how they are auditioned; the chosen numbers are recorded in
the report).

### A real fix this issue must make: melodic slots have no `pattern_bits`

Today `apply_slot` sets `pattern_bits` **only** for the Heartbeat arm, i.e. only
the drum slots. But `pluck_saw`/`heartbeat` read `pattern_bits` for their
AMPLITUDE ENVELOPE — so the lead (slot 4, PluckSaw) currently fires its pluck
envelope on `TrackParams::default_for`'s 4-hit Euclidean default, unrelated to
its own notes. The FM melodic voices need a per-note envelope, so:

- **Per plan 11's role/timbre split, the drum timing is a ROLE property**: slots
  0-2 get their `pattern_bits` from the score's kick/snare/hat tiers,
  **regardless of kind** (not only under the Heartbeat arm).
- **Slots 3 and 4 (bass, lead) get `pattern_bits` from their own note lines**:
  bit `i` set iff `bass.notes[i].is_some()` / `lead.notes[i].is_some()`. This
  makes `step_is_active`-driven envelopes line up with the notes, for the FM
  voices AND for PluckSaw if the operator picks it.

## Verification (orchestrator's, not the worker's)

1. `bash tools/gate.sh > /tmp/g.log 2>&1; echo $?` + grep `GATE_EXIT` — never
   `| tail` (it reports tail's status and hid a red clippy stage once).
2. `cargo check -p th-synth --target wasm32-unknown-emscripten`.
3. The audition renderer actually produces a WAV; **the orchestrator listens**
   and/or checks the render's peaks/RMS per slot are sane (no silent slot, no
   clipping) via the renderer's own printed table.
4. The bake fidelity: the default score still parses and the legacy tests pass
   (the re-timbre changes the SHIPPED defaults deliberately — the
   `mod legacy` fidelity tests are about the SCORE STRUCTURE and the sequencer,
   which must not change; if a kind change breaks them, that is a finding).
5. Probe on a fresh port: default PASS + `OH_SCORELAB=1` PASS.

## Issues

| # | Issue | Owner | Slot |
|---|-------|-------|------|
| 11c | the FM kit (vendor), the role-based pattern_bits fix, the re-timbre bake, the audition renderer | worker | slot-1 |
| 11d | the audition editor UI (per-slot timbre sliders) + the SFX mute | worker | slot-2 |

11d starts only after 11c lands (it needs the FM kinds in the wasm build).
