# Issue 11f — the comp part: a Role enum and an FM electric-piano stab

**Worktree:** `.agents/worktrees/slot-1` (branch `slot-1`), base `e0d4a268`.
Nothing committed. All gates green (exit codes captured directly, no piping).

## 1. Per-file changes

| File | Change |
|---|---|
| `vendor/rust-synth-0.23.0/src/audio/preset.rs` | `PresetKind::FmComp` **appended** (cycle order untouched); `ALL_KINDS` → `[PresetKind; 15]`; `label()` → "FM Comp"; `Preset::build` + the `FmComp` `FmVoice` table row (values in §3) |
| `vendor/rust-synth-0.23.0/Cargo.toml` | provenance block + the 11f bullet (repo convention for vendored changes) |
| `vendor/rust-synth-0.23.0/src/persistence.rs` | `kind_to_str` exhaustive match + `FmComp => "FmComp"` (this module is unconditional, so it would not compile without the arm) |
| `vendor/rust-synth-0.23.0/src/tui/life.rs` | `color_for` + `FmComp => Rgb(255, 140, 190)` (a hue no other slot-strip colour uses) |
| `vendor/rust-synth-0.23.0/src/tui/waveshape.rs` | both exhaustive FM arm-groups (label + sample) + explicit inner ratio arm `lerp3(1.6, 2.0, 2.4, c)` + the comp colour |
| `vendor/rust-synth-0.23.0/src/tui/formula.rs` | the FM arm-group gains `FmComp` (shared 2-op formula display) |
| `crates/th-synth/src/score.rs` | `ALL_KIND_IDS` → `[&str; 15]` (`"fm_comp"` appended, same position as the enum); `kind_id`/`kind_from_id` + arms; `Score.comp: Option<String>` (`#[serde(default)]`); `COMP_DEFAULT_PATTERN` const; `Score::comp_pattern_bits()`; `validate()` comp length check; test cases (comp length `Err`, absent `comp` `Ok`) |
| `crates/th-synth/src/director.rs` | the `Role` enum; `Slot.chord` → `Slot.role`; the new slot map; `apply_slot` role-match (drums/bass/lead/**comp**/harmony); the `FmComp` kind arm (character/resonance/reverb_mix); `apply_bar` role-match (Comp + Harmony); module docs; 4 existing tests updated (slot-5 map, solo fold, kind-swap vector, 11c grid 6..8); **4 new tests** (§5) |
| `crates/th-synth/assets/score.json` | `slots[5].kind = "fm_comp"` (the pad's kind/role moves out); the new top-level `comp` pattern; comp gain starts at the pad's old 0.0935 and is confirmed by the measured table (§4) |

The audition renderer needed **no change** — it reads the director's real
`Track`s, so the comp appears as its own row automatically (verified in §4).
`MAX_TRACKS` stays 8; no new dependencies; the 11e dry+wet topology is
untouched (the new voice goes through the same shared `fm_voice` core, which
ends in `multipass::<U2>() & (wet_chain * wet_amount)` and
`stereo_gate_voiced`).

## 2. The `Role` refactor — every site

`Role::{Drum, Bass, Lead, Comp, Harmony}` (a `Copy` enum with doc per
variant) replaces `Slot.chord: bool` and the index arithmetic:

| Site | Before | After |
|---|---|---|
| `Slot` struct | `chord: bool` | `role: Role` |
| `SLOTS[5]` | PadZimmer / unlock 4 / ratio 1.0 / chord | **FmComp / unlock 0 / ratio 1.0 / Comp** (slots 6/7 keep Bell/DroneSub, now `Harmony`) |
| `apply_slot` pattern block | `match index { 0..=2 => tiers, 3 \| 4 => line, _ => {} }` | `match slot.role { Drum => tier by index, Bass \| Lead => own line, Comp => `score.comp_pattern_bits()``, Harmony => {} }` |
| `apply_slot` kind params | no FmComp arm | + `FmComp` arm (pulse 0 / LFO off / char 0.50 / res 0.50 / rev 0.25) |
| `apply_bar` | `if slot.chord` | `if matches!(slot.role, Role::Comp \| Role::Harmony)` |
| `sequence()` gate | unchanged | unchanged — only slots 3/4 are staccato-gated; the comp needs no gate change: its FM envelope reads `pattern_bits` per-sample via `step_is_active`, so the pattern IS the gate (verified by the fold test, §5) |
| `fold_mutes` | generic 0..8 | unchanged — program (unlock 0 → live) OR user (`muted`/`solo`) already covers the comp |

The drum tier still selects by index (0/1/2) **inside** `Role::Drum` — that is
data layout (kick/snare/hat), not a special-case: the role now selects the
driver, the index only picks the tier.

## 3. `fm_comp` — actual parameter values

`FmVoice` table row (the placeholder EP stab — the composition boundary:
mechanism + audibility, not the part):

| field | value | note |
|---|---|---|
| ratio (lo/hi) | 2.0 (1.6 / 2.4) | the spec's ≈2.0; modest `character` morph range |
| index | 450.0 | the spec's 400–500, mid |
| index_decay | 10.0 | fast: the index is ~0 within ~30 ms (the EP's tonal collapse) |
| amp_decay | 3.0 | the spec's "short (~0.15–0.25 s)" read by the 11c report's own ms convention (its `FmHat 14.0 (≈29 ms)` ≈ ms-to-near-silence): k = 3.0 → ≈154 ms to 1 % at the shipped 150 bpm (τ ≈ 33 ms) |
| sweep / sweep_decay | 1.0 / 1.0 | no pitch sweep |
| noise_hp / noise_decay | 0.0 / 0.0 | no noise burst |
| filter_cap | 0.0 | no Moog filter |
| detune_carrier | false | single carrier |
| haas | 0.0 | no widening |
| reverb | (16.0, 3.0, 0.88) | a modest room |

Director params for the resolved `FmComp` kind: `character` 0.50 (the tuned
ratio), `resonance` 0.50 (0.5 × 450 Hz index at the hit), `reverb_mix` 0.25
(between the bass's 0.15 and the lead's 0.30), `pulse_depth`/`lfo_depth` 0.

## 4. Audition (the required gate output, `RC=0`)

`cargo run -q -p th-synth --example audition_render -- /tmp/comp.wav
--solo-dir /tmp/comp-solos` — 4 bars @ 150 bpm, the director's REAL tracks
(comp row shown; the comp's `bits 0b0100010001000100` = 0x4444 = steps
2/6/10/14):

```
slot             peak        rms bright (4k+)  atk/tail  onsets
FM Kick        0.4775     0.0611       0.0010      4.46      10
FM Snare       0.5022     0.0298       0.0075     32.42       8
FM Hat         0.1976     0.0135       0.0764     39.36      16
FM Bass        0.2156     0.0384       0.0029      2.42      27
FM Lead        0.3162     0.0433       0.0055      4.22      33
FM Comp        0.1068     0.0151       0.0019      5.23      15
Bell           0.0390     0.0210       0.0059      0.91       0
Drone          0.0836     0.0417       0.0000      0.88      12
mix            0.9107     0.1014       0.0039         —       —

headroom: pre-limiter peak 0.9384, post 0.9107 -> trim slot gains by +0.6 dB to leave the limiter alone
```

- **The comp is audible and distinct**: its own peak/RMS row (0.1068 /
  0.0151), 15 detected onsets over 4 bars (the off-beat 8ths), and a
  punchy atk/tail 5.23 (attack dominates the tail — a stab, not a pad).
- **Nothing clips**: pre-limiter peak 0.9384 < 1.0 → +0.6 dB headroom; the
  limiter is not squashing.
- **Nothing else moved**: every pre-existing row matches the 11e follow-on
  balance (the comp replaced the pad's 0.0935 gain slot; the pad's
  sustained energy left, the comp's stabs entered, and the mix peak
  stayed at 0.9107).
- **Genuinely FM** (FFT of the soloed comp, `5-FM_Comp.wav`, aligned for
  the renderer's 0.3 s = 3-step spin-up flush): at a hit the n=1 sideband
  (~330 Hz, ratio 2.0) collapses **16×** over 0→22 ms while the carrier
  only decays 2.2× (n=2 ~550 Hz: 174×) — the decaying-index signature of
  the 2-op core, not a filtered waveform; hit-step peak 3297 vs rest-step
  peak 141 → the envelope is `pattern_bits`-gated.

## 5. Tests + mutation checks

New (all in `crates/th-synth/src/director.rs`, driven through real
fallback directors):

- `the_comp_role_follows_the_chord_and_takes_the_pattern` — slot 5's
  `pattern_bits` == the shipped `comp`'s set steps (computed
  independently from the chars, NOT through `comp_pattern_bits`), the
  comp's freq == the bar's chord root × ratio at bar 0 (I), retunes with
  vi (+9 st) at bar 1, while the lead stays on the key root.
- `existing_roles_are_unchanged` — for the default score: drum slots'
  bits == the stage-0 tier patterns (char-parsed independently),
  bass/lead bits == their note lines (folded independently, asserted
  distinct from each other AND from the Euclidean default), harmony
  slots 6/7 keep `euclidean_bits(4, 0)`. The 11c test
  `the_pattern_bits_follow_the_role_not_the_kind` was adjusted
  accordingly (its untouched-Euclidean loop is now 6..8) and still pins
  the kind-swap behaviour.
- `a_score_without_comp_still_parses_and_plays` (§6).
- `the_mute_solo_fold_silences_the_comp` — a user-muted comp stays
  muted across a full bar of ticks INCLUDING its pattern's hit steps
  (2/6/10/14); soloing the comp silences the other seven while the soloed
  comp follows its program (unlock 0 → live); clearing the solo restores
  program state (the bell still stage-locked at stage 0).
- `score.rs` `a_bad_score_is_an_error_not_a_panic` — + a wrong-length
  `comp` is an `Err` naming the field; an absent `comp` parses `Ok`.

**Mutation checks (three, each on the otherwise-clean tree, each reverted
afterwards, `cargo test` re-green on the restored tree):**

| # | Mutation | Caught by | Failure |
|---|---|---|---|
| M1 | swapped the bass/lead note lines in the `Role::Bass \| Role::Lead` arm (a role-refactor slip) | `existing_roles_are_unchanged` | slot 3 bits 46517 (lead line) ≠ 32089 (bass line) — FAILED |
| M2 | removed the `Role::Comp` grid arm (comp would fall back to the 4-hit Euclidean default) | `the_comp_role_follows_the_chord_and_takes_the_pattern` | slot 5 bits 4369 (Euclidean) ≠ 17476 (comp pattern) — FAILED |
| M3 | dropped `Comp` from `apply_bar`'s chord write (comp would stop following the chord) | same | comp freq 110.0 (key root) ≠ 185.0 (vi) — FAILED |

## 6. Backwards-compatibility evidence

- `Score.comp` is `Option<String>` with `#[serde(default)]` — a score with
  **no `comp` field** parses (`comp == None`) and plays:
  `a_score_without_comp_still_parses_and_plays` asserts the comp slot gets
  the internal off-beat default's bits (0x4444, independently computed),
  a live chord-rooted pitch (unlock 0), and a `0.0` mute.
- The pre-existing compat suites all pass unchanged:
  `slots_without_kind_use_the_builtin` + `real_issue_10_json_parses_and_
  resolves_to_the_builtin_kinds` (issue-10 files — no `kind`/`muted`/
  `solo`/`comp` — parse and every slot resolves to its built-in kind, now
  including `FmComp` for slot 5), `score_default_round_trips` (the
  canonical text re-parses, `comp: None` → `"comp": null` omitted-None
  semantics included), and `score_default_matches_the_legacy_constants`
  (the frozen d97d4277 structure — note lines + drum tiers — is untouched;
  `comp` is additive).
- `the_baked_score_is_the_canonical_pretty_text` passes: the shipped
  `assets/score.json` is exactly `Score::default().to_json_pretty() + "\n"`.

## 7. Gates (exit codes captured directly — `"$@" > log 2>&1; rc=$?`)

| Gate | RC |
|---|---|
| `cargo fmt --all -- --check` | 0 |
| `cargo build --workspace` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0 (th-synth lib 39/39; all other crates green) |
| `cargo check -p th-synth --target wasm32-unknown-emscripten` | 0 |
| `cargo run -q -p th-synth --example audition_render -- /tmp/comp.wav --solo-dir /tmp/comp-solos` | 0 (§4) |

(The pre-existing `tinyhexagon` bin-name collision warning between
th-app/th-web still prints on `cargo build`; it is not from 11f.)

## 8. Deviations / decisions (all inside the spec's "your call" margins)

1. **`comp` is `Option<String>`** (the spec offered the choice and said to
   say so): `None` = "no field" → the internal off-beat default
   `COMP_DEFAULT_PATTERN` (`"..x...x...x...x."` — the 8th-note off-beats,
   steps 2/6/10/14; the plain off-beat Genesis stab). A present pattern is
   validated to exactly `steps_per_bar` chars, same char-index discipline
   as the drum tiers.
2. **Shipped pattern**: the bake carries the same off-beat 8ths explicitly
   (data the operator edits). It deliberately overlaps the stage-0 hat
   tier — that is the *obvious* placeholder, not a judgement call.
3. **`amp_decay` = 3.0**: the spec's 0.15–0.25 s window read through the
   11c report's own ms convention (≈ ms-to-near-silence); see §3.
4. **Comp gain 0.0935** (the pad's old gain) — confirmed by the measured
   table, not re-tuned: the comp is audible (peak 0.1068) and the mix
   keeps +0.6 dB headroom.
5. **Untouched by design**: `sequence()`'s staccato gate (pattern_bits is
   the comp's gate — no sound change needed), the 11e dry+wet topology,
   `th-web`'s DAW UI (its `KIND_IDS` select stays at 14 options and the
   `OH_SCORELAB=1` probe pins `options.length === 14` — the comp's kind is
   reachable via the score JSON / `getScore` round trip, just not
   picker-selectable; a comp lane in the DAW is a UI follow-up, not
   11f's mechanism), and `offline_render.rs` (a pre-11f hand-mirrored
   level-theft snapshot, explicitly noted as drift-prone in its own docs).
6. The TUI `short_tag` map still renders "FM Comp" as "—" — every 11c FM
   label does; consistency over novelty.

## 9. Musical suggestions I deliberately did NOT act on

(Per the composition boundary: numbers are mine, taste is yours. All of
these are `assets/score.json` / knob edits, not code.)

- **Comp presence**: at gain 0.0935 the comp sits under the lead (0.3162)
  and bass (0.2156) — a quiet-but-present row. If the comp should push
  into the mix, raise `slots[5].gain` (headroom allows roughly +3–4 dB
  before the raw peak approaches 1.0).
- **Pattern density**: the off-beat 8ths (4 hits/bar) are the sparsest
  obvious figure. Off-beat 16ths (`"x.x.x.x.x.x.x.x"`), a 2-hit pickup
  figure, or moving hits onto the kick's "and" (step 7) are all one-line
  data edits.
- **Stab octave/interval**: the comp stabs the chord root (slot ratio
  1.0). A fifth-up stab (root+7) is a different texture — ratio is
  engine-side (`SLOTS[5].ratio`), a one-constant change if you want it.
- **Air**: `reverb_mix` 0.25 and the (16 s, 3.0, 0.88) tank are middling;
  the 11c table's reverb re-audit method applies if you want the comp
  drier or roomier.
