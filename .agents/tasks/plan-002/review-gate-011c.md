# Review gate — plan 002 issue 11c (the FM kit)

Read-only gate WITH execution. Run the gates yourself; do not trust the report.
Read this whole file first.

## Authority

- `.agents/plans/002-tinyhexagon/11c-fm-kit.md` — the six voices' design, the
  provenance rule, the role-based `pattern_bits` fix, the audition deliverable.
- `.agents/tasks/plan-002/issue-011c-impl.md` — the task spec.
- `.agents/tasks/plan-002/issue-011c-report.md` — the implementer's report.
- `vendor/rust-synth-0.23.0/src/audio/preset.rs` is the truth for the preset
  idiom — compare the new voices against `bell_preset` (the existing 2-op FM)
  and `pluck_saw`/`heartbeat` (the `pattern_bits` envelope idiom).

## Run yourself

```
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p th-synth --target wasm32-unknown-emscripten
cargo run -q -p th-synth --example audition_render -- /tmp/gate-audition.wav
```
Capture exit codes honestly. **Never pipe the gate to tail.**

## Findings that matter

1. **Is the kit actually FM?** This is the whole point — the previous kit failed
   because nobody auditioned it. Read each voice and confirm it is a carrier
   sine whose phase is modulated by another sine with a decaying index (the
   `(carrier + modulator*scale) >> sine()` shape), not just another saw/square
   with a filter. Call out any voice that is really a subtractive voice in FM
   clothing.
2. **Measure the render — do not take "it sounds good" for an answer.** Run the
   audition renderer yourself. Check the printed table for: any slot with
   peak/RMS at or near ZERO (a silent voice — the exact failure mode of the old
   kit), any slot clipping (peak at 1.0 across the mix), and whether the drums
   are distinguishable from each other (their peaks/RMS should differ; if kick/
   snare/hat print near-identical numbers, they may still be one voice in three
   disguises — the old bug).
3. **Every voice reads `pattern_bits` for its envelope** and every voice goes
   through `stereo_gate_voiced` (so the 11a mute/solo fold still silences it).
   A voice that bypasses the gate is BLOCKING — it would ignore mute/solo.
4. **The role-based `pattern_bits` fix.** Verify: the drum slots get their tier
   patterns REGARDLESS of kind (not only under the Heartbeat arm); the bass/lead
   slots get their note lines; harmony is untouched. Then MUTATION-TEST the new
   test by restoring the old only-in-Heartbeat behaviour and confirming the test
   FAILS. A test that passes both ways is vacuous (this project has shipped
   three of those).
5. **Append-only enum discipline.** The 6 variants are LAST in `PresetKind`, and
   every exhaustive match grew: `label()`, `ALL_KINDS`, `Preset::build`. Check
   that `ALL_KINDS`'s length and `th-synth`'s `ALL_KIND_IDS` are consistent (14
   both) and in the SAME order (a mismatch silently mis-labels the UI picker).
   Grep for any remaining `len() == 8` / `[&str; 8]` / `[PresetKind; 8]`
   assertion that should have grown.
6. **Provenance honesty.** The vendor Cargo.toml provenance block and
   `.agents/skills/lib-rust-synth.md` must state factually that the DSP/presets
   were changed deliberately (the original patch's rule was to keep them
   unchanged). An undocumented DSP change is BLOCKING.
7. **The bake.** `assets/score.json` sets the FM ids for the roles that should
   use them; the score still parses; the `mod legacy` fidelity tests and
   `score_default_matches_the_legacy_constants` still pass (they pin the
   SCORE/SEQUENCER structure — kinds are not part of that, but if a kind change
   broke them, that is a real finding).
8. **Clamp/panic audit**: every new voice clamps its Shared reads (the siblings
   do `.min(...)`, `.clamp(...)`) — no division by a possibly-zero Shared, no
   unwrap on user data.
9. **Non-vacuity sweep** on every new test; mutation-check at least two.

## Verdict contract

Non-blocking suggestions first, then exactly one final line:

```
VERDICT: PASS
```
or
```
VERDICT: BLOCKING -- 1. <file:line — must-fix> 2. <...>
```
