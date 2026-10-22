# Issue 11c — FM kit, role-based `pattern_bits`, re-timbre bake, audition renderer

**Worktree:** `.agents/worktrees/slot-1` (branch `slot-1`) — nothing committed.
**Result:** all gates green (fmt / build / clippy `-D warnings` / test /
wasm32-unknown-emscripten check / the required
`cargo run -q -p th-synth --example audition_render -- /tmp/audition.wav`).

## 1. The six FM voices (vendor `src/audio/preset.rs`)

Genuine FM in the `bell_preset` idiom — a sine CARRIER whose phase is
modulated by a second sine with a **decaying modulation index** (not a
saw/square-with-filter):

```rust
// shape (per voice, via the shared core `fm_voice(p, g, cfg)`):
let mod_freq = lfo(|t| carrier_hz * ratio(t)) >> follow(0.08) >> sine();
let index    = lfo(|t| depth(t));                 // depth decays: the FM envelope
let sig      = (carrier_freq + mod_freq * index) >> sine();
```

`struct FmVoice` (the per-voice cfg) fields: `ratio` (at `character` 0.5),
`ratio_lo`/`ratio_hi` (the `lerp3` extremes — `character` morphs the ratio,
0.5 = the tuned value), `index` (Hz, at `resonance` 0.65 — `resonance` is
clamped ≤ 0.65 and scales the index, mirroring `bell_preset`), `index_decay`
(Φ-driven `exp(-phi·k)` FM envelope — the hit's click), `amp_decay` (the
amplitude envelope, step-gated from `p.pattern_bits` via
`rhythm::step_is_active`, exactly like `pluck_saw`/`heartbeat`), `sweep` /
`sweep_decay` (the kick's 4×→1× pitch drop, 10.0 ≈ 60 ms at 150 bpm),
`noise_hp` / `noise_decay` (snare's high-passed white burst), `filter_cap`
(bass' moog-idiom low-pass cap), `detune_carrier` (lead's +7-cent second
carrier at ×0.5, the `pluck_saw` detune idiom), `haas` (stereo width),
`reverb` (the fixed in-voice tail, every voice keeps `stereo_reverb_mix` +
`supermass_send` + the `stereo_gate_voiced` tail like its siblings). All
inputs clamped — no value panics.

Final cfg values (as shipped):

| voice | ratio (lo/hi) | index Hz | index decay | amp decay | sweep (decay) | noise hp (decay) | filter cap | detune | haas | reverb (room, s, damp) |
|---|---|---|---|---|---|---|---|---|---|---|
| `FmKick` | 6.0 (4.0/10.0) | 900 | 28 | 3.0 | 4.0× (10) | — | — | — | 8 ms | (10, 1.5, 0.88) |
| `FmSnare` | 2.0 (1.5/2.8) | 600 | 12 | 8.0 | — | 1500 (14) | — | — | 6 ms | (12, 2.0, 0.88) |
| `FmHat` | 9.0 (7.0/11.0) | 800 | 20 | 14.0 (≈29 ms) | — | — | — | — | — | (10, 1.5, 0.88) |
| `FmBass` | 1.5 (1.0/2.0) | 600 | 6 | 4.0 | — | — | 900 | — | 14 ms | (14, 2.5, 0.88) |
| `FmLead` | 2.0 (1.5/3.0) | 350 | 3 | 4.5 | — | — | — | yes | — | (18, 3.5, 0.88) |
| `FmBell` | 3.5 (3.0/4.2) | 450 | 8 | 5.0 | — | — | — | — | — | (25, 8.0, 0.85) |

Verified offline (probe render, full-rate analysis of the kick): the carrier
dominant frequency drops from ≈220 Hz (the 4× attack) to ≈55 Hz within ~60 ms
on each hit, the n=1 FM sideband (550 Hz at ratio 6) is strong at the attack
and decays with the index — i.e. a real decaying-index 2-op FM with the
pitch sweep, not a filtered waveform.

`PresetKind` grew by six **appended** variants (`ALL_KINDS` is now
`[PresetKind; 14]`); `label()` adds "FM Kick"/"FM Snare"/"FM Hat"/"FM
Bass"/"FM Lead"/"FM Bell". `default_track_set` (engine.rs) is left at its 8
tracks (MAX_TRACKS 8) and does **not** use the new kinds there — per the
spec, the new kinds ship only via the score bake / DAW picker. `vibe.rs`
needed no changes (explicit kinds only; compiles).

## 2. `pattern_bits` role fix (crates/th-synth/src/director.rs)

`apply_slot` used to set `pattern_bits` ONLY in the `Heartbeat` arm, so
melodic slots (and now the FM voices' step-gated envelopes) fired on
`default_for`'s 4-hit Euclidean default. Moved to **role** logic in
`build_tracks` (applied after the per-kind `apply_slot` arms, so it works
regardless of kind):

- slots 0–2 (drums): the score's kick/snare/hat tier pattern for the stage —
  always.
- slot 3 (bass) / slot 4 (lead): `bit i set iff score.<role>.notes[i].is_some()`
  (helper `note_line_bits`; indices ≥ 32 don't fit, per
  `Score::drum_pattern`'s documented convention).
- slots 5–7 (harmony): untouched — the 4-hit Euclidean default stands
  (they follow the chord, not steps).
- `pattern_hits` kept coherent with what is stored (set to the stored
  bits' `count_ones()`, rotation 0.0 — for the drums and the note-line
  slots). The presets that read `pattern_bits` (heartbeat, pluck_saw, and
  now the FM voices) all read the bitmask directly; `pattern_hits` is the
  TUI's recompute input (vendor comment), so it stays in sync.

Added `MusicDirector::tracks()` (a `&[Track]` getter) so the audition
renderer reads the director's REAL resolved tracks instead of mirroring any
number.

**Test:** `the_pattern_bits_follow_the_role_not_the_kind` — with the shipped
score: slot 4's `pattern_bits` == the lead line's set steps
(`0b1011_0101_1011_0101` = 46517, independently recomputed in the test from
`score.lead.notes`), NOT the Euclidean default (4369); slot 0's == the
stage-0 kick tier (`0b0000010010000001`); slot 3's == the bass line's set
steps; a Heartbeat swapped into the lead slot STILL gets the lead line's
bits (role, not kind).
**Mutation check:** removed the role block (old behavior) → the test failed
with `4369 != 46517` on the lead slot; restored, green.
Also updated `kind_change_reaches_the_engine` (its construction snapshot now
expects the bake's FM kinds for slots 0–4 — the resolved kinds, not the
`SLOTS` built-ins).

## 3. The re-timbre bake (crates/th-synth/assets/score.json)

`slots[i].kind` baked (auditioned first — the table below):
kick→`fm_kick`, snare→`fm_snare`, hat→`fm_hat`, bass→`fm_bass`,
lead→`fm_lead`. Final gains: kick 0.90, snare 0.90, hat 0.40, bass 0.95,
lead 0.60 (up from 0.90/0.60/0.26/0.80/0.50 — the audition raised snare
punch, hat presence, bass weight, lead forwardness).

**Bell slot decision: keep `Bell` (not `fm_bell`).** Auditioned both:
`fm_bell` solo peak 0.0609 / rms 0.0148 vs `Bell`'s 0.0147 / 0.0058 — the
FM bell is louder and more present, but it is a **step-gated** marimba stab
that fires on the harmony slots' Euclidean 4-hit grid (steps 0/4/8/12) —
i.e. directly on top of the kick (step 0) and both snare hits (steps 4/12),
at the root pitch. That is a rhythm-section instrument wearing a harmony
slot's hat: it collides with the backbeat and stops following the chord.
The existing `Bell` (sustained, chord-following, wobbled metallic pad) is
audible and distinct in the mix (not silent/clipped: peak 0.0147, rms
0.0058, brightness 0.0053 vs pad 0.0029, sub 0.0000), and `fm_bell` remains
fully usable via the DAW picker for any slot where a marimba stab belongs.
Documented call; the spec leaves it to the audition.

## 4. The audition renderer (crates/th-synth/examples/audition_render.rs, new)

- Reads the shipped `Score::default()`, builds the director with
  `MusicDirector::new_fallback()`, and renders the **director's REAL
  tracks** (`tracks()` getter — no hand-mirrored numbers; the values-USED
  line per slot is printed anyway). Sums the per-slot `Preset::build` nets
  (`+`), pipes `master_bus(brightness)` at stage 0.
- 4 bars @ the shipped bpm (6.4 s @ 48 kHz), 16-bit stereo WAV with a
  hand-written 44-byte header (no `hound`, no new deps).
- Per-slot table: peak + RMS + a 4 kHz-highpass "brightness" fraction with
  each slot soloed (muted via the same `Shared` mutes the engine folds),
  plus the mix's peak/RMS.
- Optional output path argument; default `audition.wav`.

Two harness notes (documented in the code):
- **Spin-up flush:** the spec-mandated `follow(0.08)` after the frequency
  closures means a FRESH graph's carriers take ~0.3 s to spin up from 0 Hz
  (live, the graph runs continuously so this never happens). The renderer
  renders +0.3 s and measures/writes the settled tail only.
- **Dev-profile DSP optimization (deviation, see §7):** the unoptimized
  dev build runs the DSP ~20× slower than realtime; a 4-bar × 9-pass render
  would be a ~20-minute gate. Root `Cargo.toml` gained
  `[profile.dev.package.fundsp] opt-level = 3` and
  `[profile.dev.package.rust-synth] opt-level = 3` — dev profile only,
  release untouched. The gate then finishes in ~13 s.

### The printed table (final gate run, `/tmp/audition.wav`)

```
== per-slot (soloed, 6.40 s) + mix ==
slot             peak        rms bright (4k+)
FM Kick        0.2723     0.0521       0.0015
FM Snare       0.1505     0.0302       0.0068
FM Hat         0.0568     0.0120       0.0759
FM Bass        0.2179     0.0415       0.0029
FM Lead        0.1993     0.0441       0.0055
Pad            0.0078     0.0019       0.0029
Bell           0.0147     0.0058       0.0053
Drone          0.0313     0.0141       0.0000
mix            0.3955     0.0875       0.0048

wrote /tmp/audition.wav (1228844 bytes, 16-bit stereo 48 kHz)
```

Read-out: every slot non-silent and well off the clip floor (WAV max sample
12958/32767, 0 clipped samples; the limiter ceiling is 1.0). Balance: kick
leads, bass/lead strong, snare a punchy backbeat (2 hits/bar, hits in
isolation), hat light, pad/bell/sub background. Spectrally distinct
(brightness): hat 0.076 (the bright one), snare/lead mid, kick/bass dark,
sub the darkest — no two slots are indistinguishable. The mix runs at peak
0.40 (−4 dBFS), healthy headroom for the live master gain.

## 5. Kind-count changes touched outside the vendor

- `crates/th-synth/src/score.rs`: `ALL_KIND_IDS` `[&str; 8]` → `[&str; 14]`
  (the six ids appended in enum order), `kind_id`/`kind_from_id` arms added.
  The id↔kind round-trip test walks `ALL_KIND_IDS`/`ALL_KINDS` zipped, so it
  now covers all 14; no len==8 assertion existed to fix.
- `crates/th-web/site/index.html`: `KIND_IDS` 8 → 14 (same order as
  `ALL_KIND_IDS`); the DAW doc comment "the 8 kind ids" → "the 14".
  `SLOTS_META.builtin` left unchanged on purpose: it mirrors the director's
  `SLOTS[i].kind` BUILT-INS (unchanged by 11c — the engine's default when a
  score leaves `kind` absent), while the baked kind now drives the display.
- `tools/web-tiny-probe.mjs`: the track-header check now asserts
  `k.options.length === 14` (was 8) and its comment says "the 14 kinds".
  The kind round-trip check still uses `bell`/`shimmer` (both valid ids).

## 6. Provenance / skill edits

- `vendor/rust-synth-0.23.0/Cargo.toml` provenance block: new bullet naming
  the 11c FM kit as the **first deliberate change to the DSP/presets**
  (append-only `PresetKind`, the `fm_voice` core + six voices, step-gated
  envelopes; everything else in `src/audio`/`src/math` untouched; the
  `persistence.rs` + TUI `life`/`formula`/`waveshape` matches grew with the
  variants, no behavior change).
- `.agents/skills/lib-rust-synth.md` preset section: `PresetKind` now "14
  variants" with the six appended, and a new bullet describing the `fm_voice`
  core (shape, step-gating, `character`/`resonance` semantics, the per-voice
  cfg table values) — including the `reverb_mix` gotcha below.

## 7. Deviations and why

1. **Dev-profile DSP optimization** (root `Cargo.toml`,
   `[profile.dev.package.fundsp]` / `[...rust-synth]` `opt-level = 3`):
   required so the mandated `cargo run --example audition_render` gate
   finishes in seconds instead of ~20 minutes (unoptimized DSP is ~20×
   slower than realtime; full 8-voice graph). Dev-only; release profile and
   every other crate untouched. The render is a measurement tool and never
   ships in a game build.
2. **`reverb_mix` semantics (operational discovery, not a code change):** in
   this codebase `stereo_reverb_mix` multiplies the WHOLE voice (dry AND the
   fixed reverb tail) by `reverb_mix` — it is a per-voice LEVEL TRIM, not a
   wet/dry amount. The director's FM arms use healthy trims
   (kick/snare/bass 0.90, hat/lead 0.80, bell 0.70), not "dry" 0.05 values
   (a 0.05 trim is a −26 dB voice — the first audition round proved it:
   every voice measured ~10× too quiet until the trims were corrected).
   The director's pre-existing `Heartbeat` arm still carries its old 0.05
   trim — left alone (not part of 11c; the bake no longer uses Heartbeat
   for drums, and the TUI operators tuned against that behavior).
3. **`default_track_set` unchanged** (8 tracks; the new kinds are dormant
   there) — per the spec.
4. **Bell slot kept as `Bell`** — documented in §3 with the audited
   alternative numbers.

## 8. Per-file changes

| file | change |
|---|---|
| `vendor/rust-synth-0.23.0/src/audio/preset.rs` | `PresetKind` +6 appended variants, `ALL_KINDS` → 14, `label()` +6, `Preset::build` +6 arms, the shared `fm_voice` core + `FmVoice` cfg + 6 voice configs (+ `fm_sweep_mult`, `fm_step_env` helpers) |
| `vendor/rust-synth-0.23.0/src/persistence.rs` | `kind_to_str` (and its round-trip) gained the 6 new ids |
| `vendor/rust-synth-0.23.0/src/tui/life.rs` | the preset-lifecycle UI match gained the 6 variants (distinct RGB colors) |
| `vendor/rust-synth-0.23.0/src/tui/formula.rs` | the formula-presets match gained the 6 variants |
| `vendor/rust-synth-0.23.0/src/tui/waveshape.rs` | the waveshape-presets match gained the 6 variants (feature-gated `tui`; kept compilable) |
| `vendor/rust-synth-0.23.0/Cargo.toml` | provenance bullet (11c FM kit = first deliberate DSP/preset change) |
| `crates/th-synth/src/director.rs` | role-based `pattern_bits` (drums → tier patterns; bass/lead → `note_line_bits`), 6 new `apply_slot` arms (auditioned values, §3/§7.2), `tracks()` getter, module-doc update, test `the_pattern_bits_follow_the_role_not_the_kind`, `kind_change_reaches_the_engine` snapshot updated for the bake's FM kinds |
| `crates/th-synth/src/score.rs` | `ALL_KIND_IDS` `[&str; 14]`, `kind_id`/`kind_from_id` +6 arms |
| `crates/th-synth/assets/score.json` | baked `kind` for slots 0–4 (`fm_kick`…`fm_lead`) + auditioned gains (0.90/0.90/0.40/0.95/0.60) |
| `crates/th-synth/examples/audition_render.rs` | NEW — the offline WAV audition renderer (director's real tracks, per-slot solo peak/RMS/brightness table + mix, hand-written 44-byte WAV header, optional out path, 0.3 s spin-up flush) |
| `crates/th-web/site/index.html` | `KIND_IDS` 8 → 14 (+ doc comment) |
| `tools/web-tiny-probe.mjs` | track-header check `options.length` 8 → 14 (+ comment) |
| `Cargo.toml` (root) | dev-profile `opt-level = 3` for `fundsp` + `rust-synth` only (audition gate performance; see §7.1) |
| `.agents/skills/lib-rust-synth.md` | preset section: 14 variants + the `fm_voice` core bullet (incl. the `reverb_mix` level-trim gotcha) |

## 9. Gates (exit codes captured, no piping to tail)

```
cargo fmt --all -- --check                                    -> 0
cargo build --workspace                                       -> 0
cargo clippy --workspace --all-targets -- -D warnings         -> 0
cargo test --workspace                                        -> 0 (70 "test result: ok", 0 failed)
cargo check -p th-synth --target wasm32-unknown-emscripten    -> 0
cargo run -q -p th-synth --example audition_render -- /tmp/audition.wav -> 0
```

## 10. Anything I could not verify / caveats

- **No ears in the loop:** the kit was verified by the numeric audition
  (peaks/RMS/brightness per slot + mix), the kick's offline spectral
  analysis (sweep + decaying-index sideband behavior), and the pattern fix's
  mutation check — not by listening. The operator should play a bar or two
  in-game and in the DAW; the `audition_render` example exists precisely so
  any re-tune is a re-measure, not a guess.
- **fundsp 0.20 offline renderer cross-slot quirk:** rendering a SUMMED net
  (`a + b`), a *muted* source's kind changes another source's rendered
  output by ~1–5% (confirmed with a controlled probe: same pure kick renders
  0.1503 alone, 0.1462 with a muted `Bell` summed in, 0.1543 with a muted
  `FmBell`; fully deterministic per graph shape — two consecutive full
  renders of the same score are bit-identical). The live engine is NOT
  affected (each track renders its own persistent graph, mixed after); only
  the offline summed-net path is. Treat solo-vs-mix deltas under ~5% as
  harness noise; every slot in the table is far from any decision boundary,
  so the judgments (no silent/indistinguishable voice; bell-slot call) hold
  under it.
- The web DAW/probe changes are verified by construction (the id list is the
  same 14-order array th-synth exposes; the probe assertion was updated in
  lockstep) — the live browser probe itself is the orchestrator's
  `package-web.sh` step, which this task explicitly does not run.
