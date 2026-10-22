# Issue 11e — fix the wet-only reverb wiring (all 11 voices)

**Worktree:** `.agents/worktrees/slot-1` (branch `slot-1`), base **`dda7a3aa`**
(master). Main worktree and slot-2 are OFF-LIMITS. **Never commit.**

**Read first, in this order:**
1. `.agents/plans/002-tinyhexagon/11e-reverb-fix.md` — THE SOURCE OF TRUTH: the
   measured bug, the three proofs, the fix, and the non-negotiables.
2. `vendor/rust-synth-0.23.0/src/audio/preset.rs` — the WHOLE file. Every voice
   ends with the pattern
   `let stereo = X >> split::<U2>() >> ... >> reverb_stereo(...)` and then uses
   `stereo` as the voice. That is the bug. The file ALSO contains the correct
   idiom: `supermass_send` ends `dry & wet_scaled` with
   `let dry = Net::wrap(Box::new(multipass::<U2>()));`. Copy its construction
   style exactly — it compiles and it is right.
3. `crates/th-synth/src/director.rs` — `apply_slot`'s per-kind arms: the
   `reverb_mix` / `gain` values you will re-balance.
4. `crates/th-synth/examples/audition_render.rs` — the renderer you extend with
   the attack/tail metric.

Write-first: fix ONE voice (pick `pluck_saw`), compile, render, verify its
attack/tail flips above 1, THEN apply the pattern to the other ten and
re-balance.

## The bug, in one line

`reverb_stereo` is a **wet-only tank**. `voice >> reverb_stereo(...)` therefore
outputs ONLY reverb tail — the dry instrument signal is discarded and never
re-added. Measured: an impulse through it has no energy for the first 2546
samples (53 ms of predelay). fundsp's own doc shows the intended idiom
(`multipass() & 0.2 * reverb_stereo(...)`), and this file's `supermass_send`
already does it correctly. **11 voices get it wrong** (all 8 original presets +
the 6 FM voices share the core; grep `reverb_stereo` and fix every site where
the reverb output IS the voice).

## What to build

### 1. The dry+wet topology, per voice

For each affected voice, the wet branch keeps its existing effect chain
(chorus → reverb, and any voice-specific shaping), and the voice becomes:

```
dry + wet_amount * wet
```

Follow `supermass_send`'s construction. The `wet_amount` is the voice's
`reverb_mix` (now a true WET AMOUNT — see item 2). Watch out:
- keep the mono/stereo widths correct (`multipass::<U2>()` for the dry stereo
  path, exactly as `supermass_send` does);
- do NOT double-apply a mono→stereo split;
- keep every voice's existing output gain structure otherwise identical
  (except the re-balance in item 3).

### 2. `reverb_mix` becomes a wet amount in [0, 1]

- 0.0 = bone dry (it currently means SILENCE — that inversion is the bug's
  fingerprint, and the 11c report recorded hitting it).
- Clamp to [0, 1].
- Keep the LFO-REVERB modulation working: `stereo_reverb_mix(base, lb)`
  currently produces the multiplier; it must now scale the WET branch only,
  not the whole voice.
- Update its doc comment and any comment claiming it is a send/level.

### 3. Re-balance (this is taste work — do it by measuring, then judgement)

With the dry signal restored the mix becomes louder and punchier, so:
- the FM arms' `reverb_mix` 0.80–0.90 must come DOWN (start ~0.15–0.30 for
  drums; pads may keep more);
- the arms' `gain` values were calibrated against a muffled wet-only signal
  and need re-checking;
- EVERY preset is affected (their `reverb_mix` defaults live in `apply_slot`),
  so re-render the whole kit and re-tune the balance. Record the final numbers.
- **Target: drums PUNCHY.** A kick whose reverb tail is louder than its attack
  is the bug's signature; after the fix the attack must dominate.

### 4. The attack/tail metric in the audition renderer (the permanent guard)

Add to `crates/th-synth/examples/audition_render.rs`: for each SOLOED slot,
detect onsets and compute the ratio of the peak in the first ~10 ms after the
onset to the peak in the ~50–300 ms tail. Print it as a new table column
(`atk/tail`). A percussive voice below ~2 is a failure worth eyeballing; a
purely sustained voice (pad/drone) may legitimately be near 1 — print the
number and let the reader judge, but note the intent in a comment.

**This metric is why the bug shipped:** peak, RMS and brightness all look
healthy when a signal is 100% reverb. Say so in the comment.

## Non-negotiables

- Every voice keeps the `stereo_gate_voiced` tail (the 11a mute/solo fold).
- The six FM voices stay genuinely FM (the decaying index envelope and the
  index-dependent spectrum).
- No new dependencies; `PresetKind`'s variants are unchanged.
- Extend the vendor `Cargo.toml` provenance comment: a SECOND deliberate DSP
  change, and a CORRECTNESS fix (a wet-only tank used as if it were a mix),
  citing fundsp's own doc idiom as the reference. Also note it in
  `.agents/skills/lib-rust-synth.md`.
- The score JSON and the `mod legacy` fidelity tests must be unaffected.

## Gates (capture exit codes HONESTLY — never pipe the gate to tail)

```
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p th-synth --target wasm32-unknown-emscripten
cargo run -q -p th-synth --example audition_render -- /tmp/fix.wav --solo-dir /tmp/fixsolos
```

## Report (`.agents/tasks/plan-002/issue-011e-report.md`, RELATIVE)

- The before/after attack/tail table (BEFORE the fix: median 0.58, and the
  per-voice numbers in the plan file — reproduce them yourself first so the
  comparison is honest).
- Every site changed, with the construction used.
- The re-balanced `reverb_mix`/`gain` values per arm.
- The metric's implementation and a comment explaining why it exists.
- Deviations, and anything you could not verify.

## VERIFIED CONSTRUCTION (the orchestrator compiled and measured this)

I already compiled and measured the exact topology, so do not improvise on the
types. For the FM core (`fm_voice`), this compiles and flips the metric:

```rust
let (rt60_l, rt60_r, damp) = cfg.reverb;
let wet_amount = stereo_reverb_mix(p.reverb_mix.clone(), lb.clone());
let stereo = gated
    >> Net::wrap(Box::new(split::<U2>()))
    >> Net::wrap(Box::new(
        multipass::<U2>()
            & (Net::wrap(Box::new(widen)) >> reverb_stereo(rt60_l, rt60_r, damp)
                * wet_amount.clone()),
    ));

let with_super = Net::wrap(Box::new(stereo)) >> supermass_send(p.supermass.clone());
let voiced = with_super;   // the wet scaling moved INTO the branch
```

and for a plain voice (probed on `pluck_saw`):

```rust
let wet_amount = stereo_reverb_mix(p.reverb_mix.clone(), lb.clone());
let stereo = plucked
    >> Net::wrap(Box::new(split::<U2>()))
    >> Net::wrap(Box::new(
        multipass::<U2>()
            & ((chorus(0, 0.0, 0.010, 0.5) | chorus(1, 0.0, 0.013, 0.5))
                >> reverb_stereo(18.0, 3.5, 0.88))
                * wet_amount.clone(),
    ));
let with_super = stereo >> supermass_send(p.supermass.clone());
let voiced = with_super;
```

Note the shape: `multipass::<U2>() & (wet_chain * amount)` — a stereo DRY
passthrough summed with the scaled wet branch. The old
`voiced = with_super * stereo_reverb_mix(...)` line DISAPPEARS (that
multiplication is what made `reverb_mix` a whole-voice trim).

### Measured effect (my probe, FM core only)

| voice | attack/tail before | after |
|---|---|---|
| FM Kick | 1.04 | 1.62 |
| FM Snare | 0.94 | **5.15** |
| FM Hat | 1.46 | **6.09** |
| FM Bass | 0.54 | 0.64 |
| FM Lead | 0.74 | 0.47 |
| mix peak | 0.40 | **0.92** |

So it works — AND the mix nearly clips (0.92), which is why the re-balance in
item 3 is mandatory, not optional. Sustained voices (bass/lead) legitimately
stay near or below 1: their energy is continuous, not transient.

Do the same for the other voices (pad, drone, shimmer, heartbeat, bass_pulse,
bell, super_saw) — they share the bug in their own shapes.
