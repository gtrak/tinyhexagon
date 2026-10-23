# Issue 11g — the song form: per-bar figures (the phrase)

**Worktree:** `.agents/worktrees/slot-2` (branch `slot-2`), base `c0114fb7`
(the landed 11f commit). Nothing committed (`git log` head is still
`c0114fb7`; `git status` shows exactly the four working-tree changes in §1).
All gates green — exit codes captured directly, no piping of the gate.

**Explicit confirmation: I did NOT author an arrangement.** The shipped
`assets/score.json` carries `"bars": []` (a one-line diff — §5), so the
shipped bake plays exactly today's music. The only phrase that exists in the
tree is the small TEST-ONLY 2-bar phrase inside the unit tests (§4), which
exists to prove resolution/fallback, not to sound good. I made zero
musical decisions (no bar assigned a melody, no re-tuning, no density
choices). Suggestions I had but deliberately did not act on: §8.

## 1. Per-file changes

| File | Change |
|---|---|
| `crates/th-synth/src/score.rs` | the `bars` schema: `Score.bars: Vec<Bar>` (`#[serde(default)]`, declared between `comp` and `slots`); the `Bar` struct (`bass: Option<NoteLine>`, `lead: Option<NoteLine>`, `comp: Option<String>` — each `#[serde(default)]`); `ResolvedBar<'a>` (the resolved figures: `&NoteLine`, `&NoteLine`, `&str`); `Score::resolve_bar(bar)` + `Score::phrase_len()`; a shared `pattern_bits(pattern)` free function (the char-index parse now used by `drum_pattern` AND `comp_pattern_bits` AND the per-bar comp grids — one parse discipline, no drift); `validate()` gains the phrase checks (≤ 64 bars; per-bar `bass`/`lead` note-line length == `steps_per_bar`; per-bar `comp` char count == `steps_per_bar` — all `Err` naming `bars[i].…`, never a panic); module docs; test `a_bad_bar_is_an_error` |
| `crates/th-synth/src/director.rs` | the sequencer: `sequence()`'s gate note-present half reads the RESOLVED bar's lines (booleans pulled out before the `&mut self` gate writes); `apply_bar(bar)` — called exactly when the bar changes — now refreshes the bass/lead `pattern_bits` from the resolved lines and the comp's from the resolved comp pattern (via the new static `write_slot_grid(tracks, i, bits)`, which takes the tracks rather than `&mut self` so the `ResolvedBar` borrow of `self.score` can stay live), alongside the chord-freq writes it always did; `apply_step(step, bar)` writes freqs from the resolved figures (including the per-bar `octave_shift`); module docs; **4 new tests** (§4) |
| `crates/th-synth/assets/score.json` | exactly ONE line added: `"bars": []` between `comp` and `slots` — empty phrase, today's music (§5) |
| `crates/th-synth/examples/audition_render.rs` | the renderer now renders the WHOLE PHRASE director-driven (§3): new `figure_markers(score, b)` (the per-bar marker line), new `render_phrase(d, nets, secs)` (per-sample director ticks through fundsp's 64-sample block `process`), `--bars N` flag, per-pass score-mute soloing + unlock-stage solo passes, updated docs/headers |

`MAX_TRACKS` stays 8; no new dependencies; the 11e dry+wet topology, the
mute/solo fold, and the 11f role map are untouched.

## 2. Resolution + fallback + `pattern_bits`-refresh design

**Schema (`score.rs`).** `Score.bars: Vec<Bar>`, serde-default (an old JSON
with no `bars` field parses to an empty list). `Bar`'s three fields are
`Option`s, each serde-default: `None` = "fall back to the top-level figure".

**Resolution (`Score::resolve_bar`).** One function owns the whole contract:

```text
empty bars  →  every bar resolves to (top-level bass, top-level lead,
               top-level comp, else COMP_DEFAULT_PATTERN)   [today's behaviour]
non-empty   →  entry = bars[bar % bars.len()]; each field is
               entry's override, else the top-level figure;
               comp: entry's, else top-level, else COMP_DEFAULT_PATTERN
```

`phrase_len()` = `bars.len()` or `1` — the modulus the sequencer (and the
renderer) wrap on. The chord cycle is NOT touched: `chord_offset` keeps its
own `bars_per_chord` arithmetic over the same absolute bar (an 8-bar phrase
over a 4-chord table cycles the chords twice — the two are decoupled by
construction; the bar counter is shared, the moduli are not).

**The refresh (the subtle bug, `director.rs::apply_bar`).** The grids
(`pattern_bits` + `pattern_hits` + zero rotation) were computed once — in
`apply_slot` at construction/`set_stage`/`set_score` — from the TOP-LEVEL
lines. With a phrase, that leaves bar 0's (or the default's) bits on every
later bar: a phrase that sounds like one bar. So the refresh lives in
`apply_bar`, which `sequence` calls **exactly when `bar` changes**
(`last_bar != Some(bar)`): the bass/lead slots get `note_line_bits` of the
resolved line, the comp gets `pattern_bits` of the resolved comp pattern.
`apply_slot` keeps writing the top-level figures (the construction /
set_stage / set_score state — corrected on the very first tick, since those
callers re-arm `last_bar = None`).

**The freqs + the gate.** `apply_step` writes the bass/lead freqs from the
resolved lines (a bar's override carries its own `octave_shift`). The
staccato gate's note-present half (`sequence`) reads the resolved lines
too — a phrase bar that RESTS where the top-level line plays must stay
gated; the `ResolvedBar` temporaries are dropped (booleans extracted) before
the `&mut self` `gate_voice` writes. The drums are untouched by the phrase
(their grids come from the stage tiers, not the bars — pinned by the
kick-identical audio check in §5).

## 3. The audition renderer: the PHRASE

The pre-11g renderer rendered 4 bars of ONE static setup — a phrase would
have been inaudible by construction (the graph never saw a bar change). Now:

- **Director-driven render** (`render_phrase`): the fallback director ticks
  SAMPLE by SAMPLE while the offline nets run through fundsp's BLOCK
  `process` (64-sample chunks — the same chunking `Wave::render` uses; a
  per-sample `get_stereo()` tick would be ~10× slower). The director's
  `sequence` rewrites the tracks' `Shared` params — the SAME atomics the
  live graph reads — so the resolved per-bar figures, the per-bar
  `pattern_bits` refresh and the absolute-bar chord offsets all vary inside
  the WAV.
- **Bars rendered**: `max(4, score.phrase_len())` (the resolved phrase
  length, default 4 for the shipped no-phrase score), `--bars N` overrides
  (added — the spec's optional flag; lets the operator loop a short phrase).
- **Soloing now lives in the SCORE**: the director's per-tick mute fold
  rewrites every slot's `mute` from the score's `muted`/`solo` every tick,
  so the pre-11g raw `params.mute` writes would have been clobbered by the
  first tick. Each solo pass clones the score with every other slot
  `muted = true` and `set_score`s it. Consequence: a stage-LOCKED voice
  (the bell, unlock 2) would read 0.0 forever and could never expose a
  broken voice — so each solo pass additionally runs at the max slot unlock
  (stage 2 — where the drum tiers, the root and the chord table are
  stage-0-identical; only bpm/brightness differ, and the pass gets its own
  `GlobalParams`). The MIX pass stays the shipped stage-0 mix. The header
  says so: `== per-slot (soloed, all voices unlocked @ stage 2, 6.40 s) +
  mix (the shipped stage-0 mix) ==`.
- **Kept**: the per-slot table (peak/rms/bright/atk-tail/onsets + the
  `--solo-dir` dumps), the atk/tail column, the headroom line, the 44-byte
  hand-written WAV header, the 0.3 s spin-up flush.

**Per-bar marker output** — the section header that lets a listener follow
the form (`figure_markers`: which figure each part resolves to on bar `b` —
`bar[i]` = phrase bar `i`'s override, `i = b % bars.len()`; `default` = the
top-level figure; `internal` = the off-beat comp fallback when the score
itself has no `comp`):

Shipped score (no phrase — the backwards-compat shape):

```
phrase: 1 bar(s); rendering 4 (markers: default = the top-level figure; bar[i] = phrase bar i's override; internal = the off-beat comp fallback):
[bar 0] bass=default lead=default comp=default
[bar 1] bass=default lead=default comp=default
[bar 2] bass=default lead=default comp=default
[bar 3] bass=default lead=default comp=default
```

A 2-bar phrase (the test-only shape, rendered for verification then
reverted):

```
phrase: 2 bar(s); rendering 4:
[bar 0] bass=bar[0] lead=default comp=bar[0]
[bar 1] bass=default lead=bar[1] comp=default
[bar 2] bass=bar[0] lead=default comp=bar[0]
[bar 3] bass=default lead=bar[1] comp=default
```

**The shipped-score table** (the required gate run — `/tmp/phrase.wav`,
`/tmp/phrase-solos`):

```
slot             peak        rms bright (4k+)  atk/tail  onsets
FM Kick        0.4799     0.0615       0.0009      4.49      10
FM Snare       0.5185     0.0311       0.0089     29.47       7
FM Hat         0.2122     0.0133       0.0764     34.47      14
FM Bass        0.1275     0.0186       0.0042      1.71      28
FM Lead        0.1678     0.0189       0.0236      4.18      28
FM Comp        0.1122     0.0166       0.0029      4.58      12
Bell           0.0406     0.0221       0.0104      0.98       1
Drone          0.0910     0.0432       0.0001      0.99      28
mix            0.6674     0.0865       0.0047         —       —

headroom: pre-limiter peak 0.7679, post 0.6674 -> trim slot gains by +2.3 dB to leave the limiter alone
```

- **Nothing clips**: pre-limiter peak 0.7679 < 1.0 (+2.3 dB headroom).
- **The comp is audible and distinct**: its own row (0.1122/0.0166), 12
  onsets, atk/tail 4.58 — within noise of the 11f numbers (0.1068/0.0151,
  5.23), i.e. the comp voice itself is unchanged; the mechanism around it
  is new.
- **The table is now HONEST about the staccato**: the pre-11g table
  measured a static setup with the melodic voices permanently on; the
  director-driven render plays them gated (70 % of their note steps), so
  the melodic rows are lower than the 11f reference (bass 0.2156 → 0.1275,
  lead 0.3162 → 0.1678, mix 0.9107 → 0.6674). That is the contract change
  this issue's renderer makes — a number for the operator, not a
  re-tuning: no gain moved.

## 4. Tests + mutation checks

New (all driven through real fallback directors unless noted):

- **`an_empty_bars_list_is_todays_behaviour`** (director) — the shipped
  score asserts `bars.is_empty()`; then, for a FULL chord cycle (bars 0–3,
  stage 3 so the root is shifted), bar after bar, the bass/lead/comp
  `pattern_bits` equal the top-level figures' independently computed bits
  (note-line fold + char parse — NOT through `resolve_bar`/`pattern_bits`/
  `note_line_bits`) and the step-0 freqs equal the top-level lines over the
  absolute bar's chord (bass follows the chord, lead ignores it, comp = the
  chord root).
- **`bars_resolve_modulo_and_fall_back`** (director) — a TEST-ONLY 2-bar
  phrase (bar 0: its own bass [step 0, +5] + its own comp [16ths]; bar 1:
  its own lead [step 0, +15]; every `None` field falls back). Asserts the
  pure `resolve_bar` resolution (0,1,0,1 + per-field fallbacks +
  `phrase_len == 2`) AND the director's per-bar grids across 4 real bars —
  the `apply_bar` refresh is exactly what keeps bar 1's grid from being
  bar 0's stale bits. The override figures are asserted DISTINCT from the
  top-level ones (so the grid asserts are inequalities).
- **`the_phrase_actually_changes_what_plays`** (director) — two bars with
  DIFFERENT bass lines (bar 0: note only at step 0, +0; bar 1: note only at
  step 4, +7): different `pattern_bits` (bit 0 vs bit 4) AND different
  freqs (independently computed: `root·2^oct·semitone(0)` vs
  `root·2^oct·semitone(9+7)` — vi's offset + the line's fifth), and the
  grid RETURNS on bars 2/3 (the modulo). Mutation-sensitive by
  construction (spec's named mutation: resolution always returning bar 0
  fails the bar-1 asserts).
- **`the_gate_reads_the_resolved_bar`** (director) — bar 1 overrides the
  bass with a line that RESTS at step 0 (where the top-level line plays):
  bar 0 step 0 live, bar 1 step 0 GATED (mute 1.0), bar 1 step 1 (the
  override's own note, swung odd step, in-step 0.25 inside the 0.15–0.85
  window) live.
- **`a_bad_bar_is_an_error`** (score.rs) — wrong-length per-bar bass line
  → `Err` containing `bars[0].bass`; wrong-length per-bar comp (on bar 1)
  → `Err` containing `bars[1].comp`; 65 bars → `Err` naming `bars`; and the
  VALID degenerate case (all-`None` bars) parses and resolves to the
  top-level figures.

**Mutation checks (three, each applied to the otherwise-clean tree, each
reverted afterwards, `cargo test -p th-synth --lib` re-green on restore):**

| # | Mutation | Caught by | Observed failure |
|---|---|---|---|
| M1 | `resolve_bar` always returns `bars[0]` (the spec's named mutation: resolution always returns bar 0) | `bars_resolve_modulo_and_fall_back` + `the_phrase_actually_changes_what_plays` + `the_gate_reads_the_resolved_bar` | 3 tests FAILED (bar-1 figure/grid/gate all wrong) |
| M2 | `apply_bar`'s grid refresh commented out (no per-bar `pattern_bits` write — the STALE-grid bug) | `bars_resolve_modulo_and_fall_back` + `the_phrase_actually_changes_what_plays` | 2 tests FAILED (the phrase plays as one bar's grid) |
| M3 | the gate's note-present half reads `score.bass.notes` (the pre-11g expression) instead of the resolved line | `the_gate_reads_the_resolved_bar` | FAILED at the bar-1 step-0 assert (gate stayed open: mute 0.0 where 1.0 was required) |

(Two test-side timing bugs found and fixed while writing the suite —
documented for the reviewer: a per-tick `-0.01 s` that made later ticks
land on the previous bar's last step, and a swung-step tick landing at
in-step 0.15 = exactly the `SWING` window bound, where float noise decides
the side. Both now land ~0.10 in-step units clear of every window edge.)

## 5. Backwards-compatibility evidence

- **The shipped asset is the reference, untouched musically**:
  `git diff crates/th-synth/assets/score.json` is EXACTLY one line —
  `"bars": []` between `comp` and `slots`. No figure, gain, kind, pattern
  or chord moved.
- **`an_empty_bars_list_is_todays_behaviour`** (§4): bars 0–3 of the
  shipped score resolve to the top-level figures — same grids, same freqs,
  bar after bar, at a shifted stage.
- **The pre-existing suites pass unchanged**: `mod legacy`'s
  `score_default_matches_the_legacy_constants` (the frozen d97d4277
  structure — note lines, drum tiers, ramps, chords),
  `the_baked_score_is_the_canonical_pretty_text` (the asset IS
  `Score::default().to_json_pretty() + "\n"` — the new `"bars": []` is part
  of the canonical text, verified byte-for-byte by that test),
  `score_default_round_trips`, `slots_without_kind_use_the_builtin`, and
  every 11f/11c/11a/11e director test (44/44 in the th-synth lib;
  `cargo test --workspace` fully green).
- **Audio-level proof** (the two renders in §3, compared per-bar from the
  `--solo-dir` WAVs; bar = 1.575 s — the solo passes' stage-2 bpm):
  - **no-phrase lead solo** (the key-root line — the "one bar, forever"
    probe, the lead being the one figure the chord cycle cannot move):
    per-bar RMS `[681, 592, 601, 605]` — flat bar to bar (the bar-0
    edge is the spin-up settling).
  - **the kick is byte-for-byte identical in energy** between the
    no-phrase and the phrase renders (`[2025, 2067, 2036, 2003]` both
    times, < 5 % tolerance) — the phrase does not touch the drum grid.
  - **with the TEST-ONLY 2-bar phrase**: lead per-bar RMS
    `[619, 49, 482, 34]` (top-level line vs the 1-note override, ~12×
    alternation, bar 0 ≈ bar 2), bass `[23, 511, 28, 510]` (override vs
    fallback, ~20× alternation; the fallback bars' energy ≈ the no-phrase
    same-bar energy — 511 vs 577, i.e. the fallback really IS the
    top-level line; the 12 % gap is the differing reverb tail from the
    preceding bars). Modulo repeats hold within tolerance on both
    (the override-note segments are reverb-tail-dominated, which sets the
    looser tolerance — the resolution itself is pinned exactly by the
    unit tests).

## 6. Gates (exit codes captured directly — `> log 2>&1; echo $?`, never piped)

| Gate | RC |
|---|---|
| `cargo fmt --all -- --check` | 0 |
| `cargo build --workspace` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `cargo test --workspace` | 0 (th-synth lib 44/44; all other crates green) |
| `cargo check -p th-synth --target wasm32-unknown-emscripten` | 0 |
| `cargo run -q -p th-synth --example audition_render -- /tmp/phrase.wav --solo-dir /tmp/phrase-solos` | 0 (§3 table; ~16.5 s wall) |

(The pre-existing `tinyhexagon` bin-name collision warning between
th-app/th-web still prints on `cargo build`; it is not from 11g — same as
the 11f report.)

## 7. Decisions inside the spec's margins

1. **`ResolvedBar<'a>`** (refs into the score) is the resolution's shape —
   `Copy` + `PartialEq`, so tests compare whole resolutions. The comp's
   resolved value is always a concrete `&str` (the fallback chain is baked
   into the resolution, so the sequencer never writes a None pattern).
2. **The refresh lives in `apply_bar`** (the spec offered `apply_bar` or
   "where `apply_step` first sees a new bar" — `apply_bar` is exactly the
   bar-change point `sequence` already has, and it also writes the chord
   freqs, so the bar's whole state lands in one call).
3. **Bars rendered = `max(4, phrase_len)`** (the spec's "resolved
   `bars.len()`, default 4"), `--bars N` added (the spec's optional flag).
4. **Solo passes at the max-unlock stage** (§3) — the pre-11g table
   measured locked voices with raw atomic writes; the score-based soloing
   would have made the bell a permanent 0.0 row. Documented in the output
   header and the module docs.
5. **`pattern_bits(pattern)` became the one shared char-index parser** —
   `drum_pattern` and `comp_pattern_bits` now call it too (same discipline,
   one less copy of the 11c convention).
6. The in-memory test phrases bypass `validate` (struct-literal `Score`s) —
   the JSON-side discipline is pinned by `a_bad_bar_is_an_error`. (During
   verification I accidentally fed a 15-char comp pattern to the SHIPPED
   asset and `Score::default()` correctly refused it at build time:
   `bars[0].comp: must be 16 chars, one per step (got 15)` — the validation
   does its job; the test-only phrase now uses 16-char patterns.)

## 8. Musical suggestions I deliberately did NOT act on

(Per the composition boundary: numbers are mine, taste is the
orchestrator's. All of these are `assets/score.json` edits, not code.)

- **The test-only phrase is arbitrary and should stay out of the bake**:
  bar 0's bass [step 0, +5] + 16th comp, bar 1's lead [step 0, +15], were
  chosen only to be mutually distinct figures. When writing the real
  arrangement, the orchestrator will replace them — the mechanism (modulo
  resolution, per-field fallback, the per-bar grid refresh) is
  figure-agnostic, and `--bars N` lets a short phrase be looped while
  auditioning.
- **First-phrase idea, purely as a hint for the 8-bar pass**: the
  fallback semantics make a "comp/lead trade" a one-line-per-bar edit —
  e.g. keep the top-level bass the whole phrase and override only
  `bars[i].lead`/`bars[i].comp` where the trade happens (a bar with
  `lead: null` stays on the motif for free). I am NOT proposing that as
  the song; it is just the shape the mechanism makes cheapest.
- **Nothing else.** No re-tuning thoughts surfaced beyond what §3's
  numbers already report (the mix has +2.3 dB of headroom under the
  limiter — more than the 11f reference's +0.6, because the gated
  melodic voices now play as they actually play).
