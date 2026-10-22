# Issue 11c — the FM kit, the role-based pattern_bits fix, the re-timbre, the audition renderer

**Worktree:** `.agents/worktrees/slot-1` (branch `slot-1`), base **`8f69b724`**
(master). Main worktree and slot-2 are OFF-LIMITS. **Never commit.**

**Read first, in this order:**
1. `.agents/plans/002-tinyhexagon/11c-fm-kit.md` — THE SOURCE OF TRUTH (the
   six voices' design, the provenance rule, the pattern_bits fix, the
   audition deliverable).
2. `vendor/rust-synth-0.23.0/src/audio/preset.rs` — the WHOLE file. Study
   `bell_preset` (~714): it is the existing 2-op FM idiom, and your core must
   look like it (`(carrier_freq + modulator*scale) >> sine()`, `lfo(...)`
   closures reading `Shared` params, `FreqMod` for pitch, `follow(0.08)` after
   frequency closures, the `stereo_gate_voiced` tail). Also read `lerp3`,
   `LfoBundle`, `FreqMod`, `stereo_reverb_mix`, `supermass_send`,
   `stereo_gate_voiced`, `heartbeat` (~546, note how it reads `pattern_bits`
   per sample), `pluck_saw` (~845, the amplitude-envelope idiom).
3. `vendor/rust-synth-0.23.0/src/audio/track.rs` — `TrackParams` (every knob
   you may read) and `default_for`.
4. `vendor/rust-synth-0.23.0/Cargo.toml` — the provenance block at the top
   (the existing discipline you must extend) + `[features]`.
5. `vendor/rust-synth-0.23.0/src/audio/engine.rs` — `default_track_set`
   (~445), `build_master`, `EngineHandle`.
6. `crates/th-synth/src/director.rs` — `SLOTS`, `apply_slot` (the per-kind
   parameter blocks), `resolved_kind`, `apply_bar`/`apply_step` (the roles).
7. `crates/th-synth/src/score.rs` — `ALL_KIND_IDS`, `kind_id`/`kind_from_id`,
   validation, the `mod legacy` tests you must not break.
8. `crates/th-synth/examples/offline_render.rs` — the existing offline
   measurement example; your audition renderer is its descendant (WAV output,
   the director's REAL slot values).

Write-first: add the 6 `PresetKind` variants + the `fm_voice` core + one voice,
compile, render it to a WAV, listen/measure, THEN do the other five and the
bake.

## What to build

### 1. The FM core + the six voices (in `preset.rs`)

The YM2612 idiom: a sine carrier whose phase is modulated by a sine, with a
**decaying modulation index**. In this codebase that is:

```rust
let mod_freq = lfo(move |t| carrier_hz * ratio(t)) >> follow(0.08) >> sine();
let index = lfo(move |t| /* depth * decay(t) */);   // the FM envelope
let sig = (carrier_freq_net + mod_freq * index) >> sine();
```

Add a small `struct FmVoice { ratio, index, decay, sweep, noise... }` (name and
shape your choice) and `fn fm_voice(p, g, cfg) -> Net` implementing the shared
structure; the six `PresetKind`s dispatch to it with their own cfg.

Per-voice requirements (plan 11c's table):
- `fm_kick`: sine carrier with a FAST downward pitch sweep (≈4× → 1× in ~60 ms
  — implement with the pitch-sweep idiom the heartbeat's `body_osc` uses, i.e.
  a `phi`-driven `if active` branch, or an `lfo` on `pattern step phase`), plus
  a brief high-ratio modulator for the click.
- `fm_snare`: a noise burst + a short pitched FM body; `character` balances
  noise vs tone.
- `fm_hat`: high inharmonic ratio with a very short (20-40 ms) decay.
- `fm_bass`: carrier + modulator ratio ≈1..2, a decaying index, low-passed
  like `bass_pulse` (reuse its `moog` idiom).
- `fm_lead`: 2-op with a moderate index, slight detune, a pluck-ish amplitude
  envelope.
- `fm_bell`: short percussive FM at ratio ≈3.5-4.

Every voice MUST:
- read `p.gain`, `p.mute` through the SAME `stereo_gate_voiced` tail the other
  presets use (so the fold keeps working), and keep `stereo_reverb_mix` +
  `supermass_send` like its siblings;
- drive its amplitude envelope from `p.pattern_bits` via
  `rhythm::step_is_active` (as `pluck_saw`/`heartbeat` do) so the grid times it;
- let `character` morph the ratio (0.5 = the tuned default) via `lerp3`, and
  `resonance` scale the modulation index (mirroring `bell_preset`);
- never panic on any parameter value (clamp everything, as the siblings do).

### 2. `PresetKind` is APPEND-ONLY, and every exhaustive match grows

Add the 6 variants (names: `FmKick, FmSnare, FmHat, FmBass, FmLead, FmBell`) at
the END of the enum. Update:
- `label()` (short labels: "FM Kick", "FM Snare", "FM Hat", "FM Bass",
  "FM Lead", "FM Bell" — check how the UI shows them and keep them short);
- `ALL_KINDS` (append; its type becomes `[PresetKind; 14]`);
- `Preset::build`'s match;
- `default_track_set` in engine.rs — you MAY add the new kinds as dormant
  slots, but if `MAX_TRACKS` is 8 you cannot add tracks; leave the set at 8 and
  simply do not use the new kinds there (state this in your report);
- `vibe.rs` — it uses only explicit kinds, so it should not need changes, but
  verify it compiles.
- `crates/th-synth/src/score.rs`: `ALL_KIND_IDS` becomes `[&str; 14]` with the
  six new snake_case ids appended (`fm_kick`, `fm_snare`, `fm_hat`, `fm_bass`,
  `fm_lead`, `fm_bell`), in the SAME order as the enum. `kind_id`/
  `kind_from_id` gain the arms. The `score_json.rs`/validation tests must
  still pass (a 14-long array is fine; check nothing asserts len == 8 — grep
  for that and fix any such assertion to the new length).

### 3. The provenance block (REQUIRED — same discipline as before)

In `vendor/rust-synth-0.23.0/Cargo.toml`'s top provenance comment, add a bullet
in the existing style naming this change: the FM kit is a plan-002 issue-11c
addition (append-only `PresetKind`, a new `fm_voice` core + six voices), and it
is the first deliberate change to the DSP/presets (the original patch kept them
unchanged). Be factual and brief. Also note it in
`.agents/skills/lib-rust-synth.md`'s preset section.

### 4. The role-based `pattern_bits` fix (in `director.rs`)

Today `apply_slot` sets `pattern_bits` ONLY inside the `Heartbeat` arm, so the
melodic slots fall back to `default_for`'s 4-hit Euclidean pattern — and any
`step_is_active`-driven envelope (pluck_saw, and now the FM voices) fires on
times unrelated to that line's notes. Fix it **by role, not by kind**:

- slots 0,1,2 (drums): `pattern_bits` = the score's kick/snare/hat tier pattern
  for the stage, ALWAYS (move this out of the `Heartbeat` arm into the role
  logic).
- slot 3 (bass): `pattern_bits` bit `i` set iff `score.bass.notes[i].is_some()`.
- slot 4 (lead): likewise from `score.lead.notes[i]`.
- slots 5,6,7 (harmony): the existing 4-hit Euclidean default is fine (they
  follow the chord, not steps) — leave `pattern_bits` untouched.
- Keep `pattern_hits` consistent with whatever you store (the vendor comment
  says it is fed into `euclidean_bits`, and `pluck_saw`/`heartbeat` read only
  `pattern_bits`, but keep them coherent).
- Where the note lines are longer/shorter than 32 bits, follow
  `Score::drum_pattern`'s documented convention (indices ≥ 32 do not fit).

Add a test proving: with the shipped score, slot 4's `pattern_bits` equals the
lead line's set steps (not the Euclidean default), and slot 0's equals the
stage-0 kick tier. **Mutation-check it** (revert to the old only-in-Heartbeat
behaviour and confirm the test fails).

### 5. The re-timbre bake (`crates/th-synth/assets/score.json`)

Set `slots[i].kind` so the roles get the FM voices: kick→`fm_kick`,
snare→`fm_snare`, hat→`fm_hat`, bass→`fm_bass`, lead→`fm_lead`. The harmony
slots (pad/bell/sub) keep their existing kinds unless your audition says
`fm_bell` is clearly better for the bell slot — your call, documented.

**Audition before you bake.** The renderer (next item) is how. Record the final
numbers (gains, character, etc.) in your report.

### 6. The audition renderer (the deliverable that prevents another bad kit)

`crates/th-synth/examples/audition_render.rs`:

- Builds the SAME graph the director builds: read the shipped
  `Score::default()`, resolve each slot's kind + the `apply_slot`-equivalent
  parameters, `Preset::build` each, sum, `master_bus`. Do not hand-mirror the
  numbers (the old `offline_render.rs` warns that drift is possible) — if a
  private director field forces mirroring, PRINT the values used and say so in
  the report.
- Renders ~4 bars at the shipped bpm to `out.wav` (16-bit stereo 48 kHz;
  `hound` is NOT available — `th-synth` has no audio-encoding dep. Write the
  WAV header by hand (~44 bytes) and the PCM; it is 30 lines and dependency
  free. Do not add a dependency for this.)
- Prints a per-slot table: peak and RMS with each slot soloed, and the mix's
  peak — so a silent or clipped voice is visible as a number, not a guess.
- Takes an optional output path argument; default `audition.wav`.

Run it yourself and include its output table in your report.

## Gates (capture exit codes HONESTLY — never pipe the gate to tail)

```
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p th-synth --target wasm32-unknown-emscripten
cargo run -q -p th-synth --example audition_render -- /tmp/audition.wav
```
Do NOT run `tools/package-web.sh` (the orchestrator does).

## Report (write to `.agents/tasks/plan-002/issue-011c-report.md` RELATIVE — in
YOUR worktree)

Per-file changes; the six voices' actual parameter values; the audition
renderer's printed table; the pattern_bits mutation check; the kind-count
changes touched outside the vendor; deviations and why; the skill/provenance
edits; anything you could not verify.
