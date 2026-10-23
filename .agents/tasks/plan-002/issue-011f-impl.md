# Issue 11f — the comp part: a Role enum and an FM electric-piano stab

**Worktree:** `.agents/worktrees/slot-1` (branch `slot-1`), base **`5ed88dd0`**
(master). Main worktree and slot-2 are OFF-LIMITS. **Never commit.**

**Read first, in this order:**
1. `.agents/plans/002-tinyhexagon/11f-arrangement.md` — THE SOURCE OF TRUTH
   (the Role enum, the new slot map, `fm_comp`, the `comp` pattern, the
   backwards-compatibility rules).
2. `crates/th-synth/src/director.rs` — the WHOLE file. Especially `Slot`,
   `SLOTS`, `apply_slot` (note the `index < 3` drum special-casing and the
   `self.score.drum_pattern(...)` calls), `apply_step` (bass/lead note writes),
   `apply_bar` (`slot.chord`), `sequence` (the gate), `note_line_bits`, and the
   tests that pin roles/kinds you must update.
3. `crates/th-synth/src/score.rs` — the schema, validation, `ALL_KIND_IDS`,
   `kind_id`/`kind_from_id`, and the `mod legacy` fidelity tests.
4. `vendor/rust-synth-0.23.0/src/audio/preset.rs` — `fm_voice` + `FmVoice`
   (the table row you add), `Preset::build`, `label()`, `ALL_KINDS`. NOTE:
   every voice now ends with the 11e dry+wet topology
   (`multipass::<U2>() & (wet_chain * wet_amount)`) — your new voice must too.
5. `crates/th-synth/examples/audition_render.rs` — how the real setup is
   rendered and measured.

Write-first: add the `Role` enum + the `Comp` slot + `fm_comp` (a table row),
compile, render, LISTEN to the comp, then do the pattern wiring.

## What to build

### 1. `Role` replaces `Slot.chord` and the index hacks

Per the plan: `Role::{Drum, Bass, Lead, Comp, Harmony}`. `Slot` gains
`role: Role`; `chord: bool` goes away. Update EVERY site that currently branches
on `slot.chord` or `index < 3` — `apply_slot`, `apply_bar`, `sequence`'s gate,
`note_line_bits` callers, `pattern_bits` assignment — to match on the role.
This is a refactor: **the observable behaviour of the existing roles must not
change** (the drum/bass/lead/harmony paths behave as they do today), and the
existing tests must still pass. Mutation-check one of them to prove you did not
soften an assertion.

### 2. `fm_comp` — the seventh FM voice

- `PresetKind::FmComp` APPENDED (so `ALL_KINDS`' existing cycle order is
  untouched; `ALL_KINDS` becomes `[PresetKind; 15]`).
- A `FmVoice` table row for an FM electric-piano stab: ratio ≈2.0, index
  ≈400-500, a fast index decay, and a SHORT amp decay (~0.15-0.25 s) so it
  stabs. Every 11c rule applies (the index-decay envelope, `pattern_bits`,
  `stereo_gate_voiced`), plus 11e's dry+wet topology.
- `label()` → a short label; every exhaustive match grows (`Preset::build`,
  `label`, `ALL_KINDS`, the TUI's `life.rs`/`waveshape.rs`/`formula.rs` colours
  and descriptions — they are exhaustive and WILL fail to compile otherwise).
- `crates/th-synth/src/score.rs`: `ALL_KIND_IDS` → `[&str; 15]` with
  `"fm_comp"` appended in the SAME position as the enum; `kind_id`/
  `kind_from_id` arms.
- Grep for any assertion expecting 14 kinds / `[&str; 14]` and update it.

### 3. The `comp` pattern + the slot map

- `Score` gains `comp: String` (a step pattern, `x` = hit, same parse
  discipline as the drum tiers: char-index, length == `steps_per_bar`).
  Default: the off-beat Genesis stab. Since it must be `#[serde(default)]`-able
  for old scores, give it a sane `Default` (or make it `Option<String>` and
  fall back to an internal default — your call; if you choose `Option`, say so).
- Slot 5 becomes `role: Role::Comp`, `kind: PresetKind::FmComp`, `unlock: 0`
  (the pad's kind/role moves out; slots 6/7 keep Harmony). Apply the comp's
  `pattern_bits` from `score.comp` (the pattern gates the FM envelope; the
  PITCH comes from the bar's chord root × the slot's ratio, written in
  `apply_bar` alongside the harmony slots).
- `assets/score.json`: set `slots[5].kind = "fm_comp"` and the new `comp`
  pattern. Re-balance the comp's gain by MEASURING (the audition table), not by
  guessing — it is a new voice in the mix.

### 4. The sequencer must actually GATE the comp

Today only slots 3/4 are gated by the staccato window. The comp needs its
`pattern_bits` to drive its envelope (the FM core reads `pattern_bits`
per-sample via `step_is_active`, so a `pattern_bits` write is sufficient — no
gate change needed for the sound). But the `fold_mutes`/gate path must include
the Comp role correctly (a muted/soloed comp must silence). Verify by test.

### 5. The audition renderer

Keep the per-slot table + atk/tail + headroom. Ensure the comp appears as its
own row and is audible in the mix. (11g will add the phrase rendering; for this
issue the existing 4-bar render is fine.)

## Tests (non-vacuous — mutation-check at least two)

- `the_comp_role_follows_the_chord_and_takes_the_pattern`: the comp slot's
  `pattern_bits` equals `score.comp`'s set steps, AND its frequency changes
  when the bar's chord changes (i.e. it is chord-following, unlike the lead).
- `existing_roles_are_unchanged`: for a default score, the drum slots' bits are
  the tier patterns, the bass/lead bits are their note lines, the harmony slots
  keep the 4-hit default — the same assertions as 11c's role test, so a refactor
  slip is caught.
- `a_score_without_comp_still_parses_and_plays` (backwards compatibility).
- The mute/solo fold silences the comp.

## Gates (capture exit codes HONESTLY — never pipe the gate to tail)

```
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p th-synth --target wasm32-unknown-emscripten
cargo run -q -p th-synth --example audition_render -- /tmp/comp.wav --solo-dir /tmp/comp-solos
```

## Report (`.agents/tasks/plan-002/issue-011f-report.md`, RELATIVE)

Per-file changes; the `Role` refactor's sites; `fm_comp`'s actual parameter
values; the audition table (the comp's row and the mix's headroom); the
mutation checks; the backwards-compatibility evidence; deviations.
