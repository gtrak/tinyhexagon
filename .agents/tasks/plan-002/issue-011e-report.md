# Issue 11e — the wet-only reverb fix: dry + wet for all 14 voices

**Worktree:** `.agents/worktrees/slot-1` (branch `slot-1`, base
`5093d881`) — nothing committed.
**Result:** all gates green (gate.sh fmt/build/clippy `-D warnings`/test
all `GATE_EXIT=0`; `cargo check -p th-synth --target
wasm32-unknown-emscripten` → 0; the mandated
`cargo run -q -p th-synth --example audition_render -- /tmp/fix.wav
--solo-dir /tmp/fixsolos` → 0).

## 1. The bug, reproduced

fundsp's `reverb_stereo` is a WET-ONLY tank: it discards its dry input.
Every voice ended `… >> split::<U2>() >> [widen] >> reverb_stereo(…)` and
used the tank's output as the voice — so every one of the 14 `PresetKind`
voices was 100% reverb tail, and `reverb_mix` (sitting AFTER
`supermass_send`, multiplying the whole voice) was a level trim whose 0.0
meant SILENCE. 11 grep sites for `reverb_stereo`; 9 of them are the bug
(the other 2 live in `supermass_send`, which was already correct:
`dry & wet_scaled`).

Write-first check (spec-mandated): fixed `pluck_saw` alone, compiled,
probed a solo pluck render (a throwaway example — the shipped score uses
`fm_lead` for slot 4, so the kit renderer never measures a pluck):
atk/tail **1.15 → 12.09**, solo energy 0.0024 → 0.0581. Then applied the
pattern to the other eight sites.

## 2. The fix (vendor `src/audio/preset.rs`) — every site, one construction

All 9 voice sites use the orchestrator's VERIFIED construction
(`multipass::<U2>() & (wet_chain * amount)` — fundsp's own `reverb_stereo`
doc idiom, `multipass() & 0.2 * reverb_stereo(...)`, and the shape
`supermass_send` already used). The wet branch keeps its existing effect
chain (chorus pair / Haas widen → reverb); only the topology changed.
The old whole-voice `voiced = with_super * stereo_reverb_mix(…)` line is
GONE at every site (that multiplication is what made `reverb_mix` a
whole-voice trim instead of a wet amount).

| site | wet branch (unchanged character) | dry source |
|---|---|---|
| `fm_voice` (shared by the six FM voices) | `widen (Haas) >> reverb_stereo(cfg.reverb)` | the step-gated FM `body` |
| `pad_zimmer` | `chorus(0,·,0.015,0.35)\|chorus(1,·,0.020,0.35) >> reverb_stereo(18.0, 4.0, 0.9)` | `filtered` (moog + 3 kHz shelf) |
| `drone_sub` | `chorus(10,·,0.025,0.18)\|chorus(11,·,0.031,0.18) >> reverb_stereo(20.0, 5.0, 0.85)` | `body` |
| `shimmer` | `chorus(20,·,0.008,0.6)\|chorus(21,·,0.011,0.6) >> reverb_stereo(22.0, 6.0, 0.85)` | `bright` |
| `heartbeat` | `(pass()\|delay(0.008)) >> reverb_stereo(10.0, 1.5, 0.88)` | `kick` |
| `bass_pulse` | `(pass()\|delay(0.014)) >> reverb_stereo(14.0, 2.5, 0.88)` | `grooved` |
| `bell_preset` | `chorus(30,·,0.018,0.25)\|chorus(31,·,0.022,0.25) >> reverb_stereo(25.0, 8.0, 0.85)` | `body` |
| `super_saw` | `chorus(0,·,0.012,0.4)\|chorus(1,·,0.014,0.4) >> reverb_stereo(16.0, 3.0, 0.88)` | `filtered` |
| `pluck_saw` | `chorus(0,·,0.010,0.5)\|chorus(1,·,0.013,0.5) >> reverb_stereo(18.0, 3.5, 0.88)` | `plucked` |

Shape (each site, per the verified construction):

```rust
let wet_amount = stereo_reverb_mix(p.reverb_mix.clone(), lb.clone());
let stereo = dry
    >> Net::wrap(Box::new(split::<U2>()))
    >> Net::wrap(Box::new(
        multipass::<U2>()
            & (WET_CHAIN * wet_amount.clone()),
    ));
let voiced = stereo >> supermass_send(p.supermass.clone());
voiced * stereo_gate_voiced( … )   // UNCHANGED tail (the 11a fold)
```

Notes: the Haas delays (heartbeat/bass/fm) stay in the WET branch, so the
dry kick/bass land mono-compatible — punch is preserved. Mono/stereo
widths: `multipass::<U2>()` carries the stereo dry exactly as
`supermass_send` does; no double split.

**`reverb_mix` semantics** (`stereo_reverb_mix`, same file): now the
voice's WET AMOUNT in [0, 1] — 0 = bone dry. The base is clamped to
[0, 1] (previously only the LFO-modulated path clamped, so an
out-of-range editor value could boost the wet branch); the LFO-REVERB
modulation is unchanged (±0.4 at depth=1, clamped again) and now scales
the wet branch ONLY, as required. Doc comment updated to say so.

## 3. Before / after — attack/tail (the metric that should have caught this)

All three tables are my own renders (`audition_render`, 4 bars @ 150 bpm,
stage 0, shipped score, soloed slots + mix; atk/tail = median across
detected onsets of the first-10 ms peak over the 50–300 ms tail peak).

**BEFORE (the bug, pristine tree — reproduced before touching the DSP):**

```
slot             peak        rms bright (4k+)  atk/tail  onsets
FM Kick        0.2723     0.0521       0.0015      1.35      52
FM Snare       0.1505     0.0302       0.0068      1.14      52
FM Hat         0.0568     0.0120       0.0759      0.90      50
FM Bass        0.2179     0.0415       0.0029      1.23      50
FM Lead        0.1993     0.0441       0.0055      1.00      58
Pad            0.0078     0.0019       0.0029      1.16      27
Bell           0.0147     0.0058       0.0053      0.78       1
Drone          0.0313     0.0141       0.0000      0.49       0
mix            0.3955     0.0875       0.0048
```

**Plan-file BEFORE (orchestrator's probe, the shipped bake's render):**
FM Kick 0.86 · FM Snare 0.47 · FM Hat 0.94 · FM Bass 0.87 · FM Lead 0.74,
median 0.58 across 40 onsets. **My reproduction agrees in the verdict**
(no percussive voice anywhere near 5–20×; hat/lead/bell/drone at or
below 1; every tail ≥ the hit that caused it) **but not in the exact
numbers** — the plan's probe anchored its attack window at the pattern
grid's hit time, where the pre-fix signal is still inside the reverb's
~53 ms predelay silence; my detector anchors at the detected envelope
rise (the reverb's early reflections). Different anchors + different
render (shipped bake vs my 4-bar stage-0 solo) = different numbers, same
bug. I report my numbers as the honest before/after pair.

**INTERMEDIATE (fixed, OLD reverb_mix 0.8–0.9 — the re-balance proof):**

```
FM Kick        0.9118     0.1290      atk/tail  1.81
FM Snare       0.9126     0.0640              7.61
FM Hat         0.4306     0.0321              7.59
FM Bass        0.4825     0.0912              1.42
FM Lead        0.7028     0.0963              3.40
mix            0.9146
```

Reproduces the orchestrator's probe: mix peak 0.40 → **0.92** (mine
0.3955 → 0.9146), snare 0.94 → 5.15 (mine 1.14 → 7.61), hat 1.46 → 6.09
(mine 0.90 → 7.59). Dry restored, per-slot RMS ≈ doubled — and the mix
nearly clips. This is why item 3 (re-balance) is mandatory.

**AFTER (fixed + re-balanced, final):**

```
slot             peak        rms bright (4k+)  atk/tail  onsets
FM Kick        0.9113     0.1219       0.0011      4.41       9
FM Snare       0.9124     0.0580       0.0082     33.55       8
FM Hat         0.4247     0.0296       0.0764     39.96      16
FM Bass        0.4509     0.0823       0.0029      6.00      33
FM Lead        0.6804     0.0859       0.0055      6.18      35
Pad            0.0837     0.0327       0.0027      1.28      23
Bell           0.0836     0.0449       0.0059      0.91       0
Drone          0.1855     0.0890       0.0000      0.58      16
mix            0.9145     0.1736       0.0046
```

Read-out: every slot non-silent; every PERCUSSIVE voice's attack now
dominates its tail (kick 4.4×, snare/hat 33–40× — capped-tail territory,
bass 6.0×); the sustained voices sit near or below 1 as expected (pad
1.28, bell 0.91 = steady-state fallback, drone 0.58 — its beat-rate
amplitude wobble is below its own steady level + tail). No slot clipped:
mix peak 0.9145 < 1.0, and the 16-bit WAV has zero clipped samples.

## 4. The re-balance (director.rs `apply_slot`)

| arm | `reverb_mix` before (level trim) | after (WET AMOUNT) |
|---|---|---|
| `FmKick` | 0.90 | **0.25** (roomy low tail, attack first) |
| `FmSnare` | 0.90 | **0.20** (punch over room) |
| `FmHat` | 0.80 | **0.15** (tight, almost dry) |
| `FmBass` | 0.90 | **0.15** (dry bass weight) |
| `FmLead` | 0.80 | **0.30** (melodic air) |
| `FmBell` | 0.70 | **0.35** (marimba room) |
| `Heartbeat` / `BassPulse` / `PluckSaw` | 0.05 / 0.08 / 0.20 | unchanged — now read as wet amounts: a 5–20 % tail on a DRY voice (pre-11e the 0.05 was a −26 dB whole-voice trim; the backwards-compat built-in drums are now actually audible) |
| `PadZimmer` / `Bell` / `DroneSub` | 0.28 / 0.28 / 0.15 | unchanged (pads keep more — 0.28 is a healthy wet amount) |
| `Shimmer` / `SuperSaw` | 0.50 | unchanged (ambient voices, more wet is right) |

The arm's doc comment (which claimed `reverb_mix` was a level trim) was
rewritten to the new semantics. **Gains** are score data
(`score.slots[i].gain` = 0.90/0.90/0.40/0.95/0.60/0.20/0.20/0.28) and
were re-checked, not changed (non-negotiable: no score-data edits): with
the dry restored they now drive the mix to a pre-limiter peak of 2.13
(1879 samples > 1.0 pre-limiter; post-limiter 0.9145, no clip). The DAW's
per-slot gain (the 11d editor) is the operator's headroom lever — the
shipped bake peaks at −0.8 dBFS post-limiter by construction.

## 5. The attack/tail metric (`crates/th-synth/examples/audition_render.rs`)

New `atk/tail` + `onsets` columns in the per-slot table.
`attack_tail(left, right, sr)`:

1. 5 ms peak-envelope frames over |L|,|R|;
2. onset = a frame that jumps > 20 % over its predecessor AND peaks
   locally above 25 % of the render's own peak, with 30 ms min spacing
   (one hit, one onset — a decaying tail is never such a peak; a
   gate re-opening is a jump from silence);
3. per onset: attack = peak in [onset, onset+10 ms); tail = peak in
   [onset+50 ms, min(onset+300 ms, NEXT onset)) — the tail window never
   reaches past the next onset (the next hit's attack is not this hit's
   tail);
4. the MEDIAN across onsets is printed (one value per hit, the "median
   across onsets" of the plan's tables); onsets are printed next to it
   (honesty about the sample). When no onset is detected (a purely
   sustained voice) a steady-state reading is returned instead (render
   head's first 10 ms over its 50–300 ms window; onset count reads 0).
   Ratios cap at 99.90.

The doc comment says explicitly why the metric exists: **peak, RMS and
brightness all look HEALTHY when a signal is 100% reverb** — the
pre-11e kit measured fine on every one of those and was still nothing
but reverb tail. A percussive voice below ~2 is a failure worth
eyeballing; a sustained voice (pad/drone/sub) may legitimately sit near
or below 1 — its energy is continuous, not transient — so the number is
printed and the reader judges.

Also: `--solo-dir` now `create_dir_all`s the directory (the dump used to
`eprintln` on a missing dir — harness robustness, one line).

## 6. Non-negotiables, checked

- **`stereo_gate_voiced` tail** — kept on all 9 sites (the 11a
  mute/solo fold); the last `* stereo_gate_voiced(…)` of every voice is
  untouched.
- **The six FM voices stay genuinely FM** — only the tail topology
  changed in `fm_voice`; the decaying index envelope, the ratio morph,
  the sweep, the noise burst and the filter are byte-identical.
- **No new dependencies**; `PresetKind`'s 14 variants unchanged;
  `ALL_KINDS`/`label()` untouched.
- **Provenance** — `vendor/rust-synth-0.23.0/Cargo.toml`: new bullet
  naming 11e as the SECOND deliberate DSP change and a CORRECTNESS fix
  (a wet-only tank used as if it were a mix), citing fundsp's own doc
  idiom (`multipass() & 0.2 * reverb_stereo(10.0, 5.0, 0.5)` — "Add 20%
  Reverb") and this file's own `supermass_send` as the reference.
- **Skill file** — `.agents/skills/lib-rust-synth.md`: the old
  "`reverb_mix` is a whole-voice LEVEL TRIM" gotcha (which now
  described the bug as the intended behavior) is replaced by the
  dry+wet topology bullet: the wet-only bug, the 9-site fix, the new
  WET-AMOUNT semantics + clamp, and the atk/tail guard.
- **Score JSON + `mod legacy`** — untouched (verified: `git diff` empty
  for `assets/score.json`, `score.rs`, `persistence.rs`; the
  `score_default_matches_the_legacy_constants` fidelity test passes in
  the green test run).

**The plan's "reverb feeding reverb" note:** `supermass_send`'s
`reverb_stereo(35,15) >> reverb_stereo(50,28)` stage — its `amount` is
verified 0.0 in the shipped score (`TrackParams::default_for`:
`supermass: shared(0.0)`; the score's `SlotGain` has no supermass field,
so the default stands) → inert; its topology/cost is UNCHANGED by 11e
(it was already in every voice's path pre-fix). Not touched, per the
spec's scope.

## 7. Gates (exit codes captured honestly — the gate was never piped to tail)

```
bash tools/gate.sh > /tmp/g.log 2>&1; echo $?
  fmt     GATE_EXIT=0
  build   GATE_EXIT=0
  clippy  GATE_EXIT=0
  test    GATE_EXIT=0          (th-synth: 35+3 passed, 0 failed)
  -> overall 0
cargo check -p th-synth --target wasm32-unknown-emscripten -> 0
  (pre-existing warnings from the vendored cpal 0.15.3 only — untouched)
cargo run -q -p th-synth --example audition_render -- /tmp/fix.wav --solo-dir /tmp/fixsolos -> 0
  (final table in §3; 8 solo WAVs written to /tmp/fixsolos/)
```

## 8. Per-file changes

| file | change |
|---|---|
| `vendor/rust-synth-0.23.0/src/audio/preset.rs` | all 9 voice sites → `multipass::<U2>() & (wet_chain * wet_amount)` (the verified construction); `stereo_reverb_mix` base clamped to [0,1] + doc rewrite (WET AMOUNT semantics); the whole-voice `* stereo_reverb_mix` line deleted at every site; per-site 11e comments (the full explanation lives at `fm_voice`) |
| `crates/th-synth/src/director.rs` | FM arms' `reverb_mix` re-balanced (0.90/0.90/0.80/0.90/0.80/0.70 → 0.25/0.20/0.15/0.15/0.30/0.35) + the arm's doc comment rewritten from "level trim" to WET AMOUNT |
| `crates/th-synth/examples/audition_render.rs` | `atk/tail` + `onsets` columns, the `attack_tail` metric (with the "why this exists" comment), `--solo-dir` directory creation |
| `vendor/rust-synth-0.23.0/Cargo.toml` | provenance bullet (second deliberate DSP change; correctness fix; fundsp doc idiom cited) |
| `.agents/skills/lib-rust-synth.md` | the `reverb_mix` gotcha replaced by the dry+wet topology + atk/tail guard bullet |

## 9. Deviations and anything I could not verify

1. **BEFORE numbers are my own reproduction, not the plan's 0.58
   verbatim** — see §3: same verdict (no percussive voice above 1.35
   pre-fix; tail ≥ attack), different anchors (detected envelope rise vs
   the plan's grid-anchored probe) and render (my 4-bar stage-0 solo
   pass vs the shipped bake's render). The plan's per-voice numbers are
   quoted alongside mine for reference.
2. **The pluck write-first check used a throwaway probe example**
   (deleted; no trace left) because `PluckSaw` is not in the shipped
   score — its atk/tail 1.15 → 12.09 and the solo-energy jump (0.0024 →
   0.0581) verified the construction before the other eight sites.
3. **Mix headroom:** post-limiter peak 0.9145 (pre-limiter 2.13) — the
   11c score gains were calibrated against the muffled wet-only signal
   and the score JSON is off-limits (non-negotiable), so the shipped
   bake runs hot; the master limiter engages on the biggest transients
   (fast attack, no clipping in the output). The DAW's per-slot gain is
   the operator's lever. Flagged for the operator's ear-check.
4. **No ears in the loop** — the balance is verified by the numeric
   audition (no silent slot, no clip, atk/tail per slot, brightness
   still spectrally distinct: hat 0.076 the bright one, sub 0.0000 the
   dark one). The operator should play a bar in-game/in the DAW;
   `audition_render` makes any re-tune a re-measure.
5. **fundsp offline-renderer cross-slot quirk (from 11c, still true):**
   rendering a summed net, a muted source's kind shifts another source's
   output by ~1–5 %; solo-vs-mix deltas under ~5 % are harness noise.
   The live engine (per-track graphs) is unaffected. Every table entry
   here is far from any decision boundary.
