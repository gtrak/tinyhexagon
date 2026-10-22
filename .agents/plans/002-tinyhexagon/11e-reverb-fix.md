# Issue 11e — the reverb wiring bug: every voice is 100% wet

**Status:** IN PROGRESS
**Depends:** 11c (the FM kit), the vendored preset file
**Operator directive (verbatim):** "the instruments are still not great, and
it's way too much reverb" → "fix it".

## The bug (MEASURED, not inferred)

`fundsp`'s `reverb_stereo(room, time, damping)` is a **WET-ONLY tank**: it
discards its dry input. Evidence, three independent ways:

1. An impulse through `split::<U2>() >> reverb_stereo(16.0, 3.0, 0.88)` has
   **zero** energy in its first 10 samples and its first non-silent sample at
   **2546 samples (53 ms)** — the room's predelay. If the dry passed through,
   sample 0 would be ~1.0.
2. fundsp's OWN doc for `reverb_stereo` shows the intended idiom:
   ```rust
   /// ### Example: Add 20% Reverb
   /// multipass() & 0.2 * reverb_stereo(10.0, 5.0, 0.5);
   ```
   i.e. dry **and** wet, summed by the CALLER.
3. This very file's `supermass_send` already does it correctly — it ends with
   `dry & wet_scaled`. So the codebase contains both the bug and the fix.

**Every voice** pipes straight through the reverb and its output goes to the
speakers: `gated >> split >> widen >> reverb_stereo(...)` — the dry signal is
dropped and never re-added. 11 occurrences in
`vendor/rust-synth-0.23.0/src/audio/preset.rs` (including all six FM voices).

### The audible consequence (measured on the shipped bake's render)

Attack (0–10 ms) vs reverb tail (50–300 ms), 40 onsets:

| voice | attack/tail |
|---|---|
| FM Kick | 0.86 |
| FM Snare | 0.47 |
| FM Hat | 0.94 |
| FM Bass | 0.87 |
| FM Lead | 0.74 |
| **median across onsets** | **0.58** |

A punchy drum should be **5–20×**, never below 1. Every reverb tail is louder
than the hit that caused it. The operator has never heard the instruments —
only their reverb tails. That is why no amount of re-tuning helped.

### The compounding error

`reverb_mix` (0.8–0.9 at the FM arms) was (mis)used as a whole-voice LEVEL
trim to stop the drowned mix being too loud — a compensating hack for a
wet-only tank. With a correct dry+wet mix, **`reverb_mix` must become a true
WET AMOUNT** where 0 = bone dry. That is also what makes the 11d audition
editor's reverb knob meaningful.

Additionally `supermass_send` runs `reverb_stereo(35.0, 15.0, 0.88) >>
reverb_stereo(50.0, 28.0, 0.90)` — reverb feeding reverb. Its `amount` is 0.0
by default in the shipped score, so it is inert today, but the default must be
verified and the stage's cost considered.

## The fix

1. **Every voice becomes `dry + reverb_mix * wet`**, using the pattern the file
   already contains (`multipass::<U2>() & (effect * amount)`, or a mono
   `Shared` multiplier via `lfo` — mirror `supermass_send`'s construction
   exactly, since it compiles and is correct).

   The wet branch should keep the existing effect chain (chorus → reverb), so
   the character is preserved; only the topology changes.

2. **`reverb_mix` becomes the wet amount in [0, 1]** (0 = dry only). Clamp it.
   The LFO-REVERB modulation path (`stereo_reverb_mix`) must keep working.

3. **Re-audition and re-balance.** Once the dry signal is present:
   - the FM arms' `reverb_mix` 0.80–0.90 becomes absurd — it must come down to
     a tasteful wet amount (start ~0.15–0.30 for drums, higher for pads);
   - the arms' `gain` values (calibrated against a muffled wet-only signal)
     must be re-checked, since the mix gets louder and punchier;
   - `reverb_mix`'s meaning change affects EVERY preset, so re-render the whole
     kit and re-tune.

4. **The audition renderer gains an ATTACK/TAIL metric per slot** — the guard
   that makes this class of bug impossible to ship again. Report the ratio; a
   percussive voice below ~2 is a failure. Print it in the table.

## Non-negotiables

- Keep the `stereo_gate_voiced` tail on every voice (the 11a mute/solo fold).
- No new dependency. No change to `PresetKind`'s variants.
- The FM voices must stay genuinely FM (the index-decay envelope and the
  index-dependent spectrum — see 11c).
- Extend the vendor's provenance comment: this is the second deliberate
  DSP change, and it is a CORRECTNESS fix (a wet-only tank used as if it were a
  mix), not a taste change. Say what was wrong and what the reference is
  (fundsp's own doc idiom).
- The `mod legacy` score-fidelity tests and the score JSON must be unaffected
  (this is a synth-graph change, not a score change).

## Verification (the orchestrator's, not the worker's)

1. `bash tools/gate.sh > /tmp/g.log 2>&1; echo $?` + grep `GATE_EXIT` — never
   `| tail`.
2. `cargo check -p th-synth --target wasm32-unknown-emscripten`.
3. Run the audition renderer; **the median attack/tail must go from 0.58 to
   well above 1** (record before/after), no slot silent, no clipping.
4. Re-render the listenable mix and confirm by ear + by the metric.
5. Probe: default PASS + `OH_SCORELAB=1` PASS.

## Issues

| # | Issue | Owner | Slot |
|---|-------|-------|------|
| 11e | the dry+wet fix across all 11 voices, the `reverb_mix` semantics change, the re-balance, the attack/tail metric | worker | slot-1 |
