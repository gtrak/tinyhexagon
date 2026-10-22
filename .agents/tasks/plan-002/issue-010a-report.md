# Issue 10a fix pass — report (worktree `slot-1`)

## FIX PASS 1

**Date:** the bounded fix pass after the review gate flagged the bake-fidelity
test. Worktree: `/home/gary/dev/tinyhexagon/.agents/worktrees/slot-1`
(branch `slot-1`, base `d97d4277`). Nothing committed.

### The circularity (the blocking finding)

`score_default_matches_the_legacy_constants` (in
`crates/th-synth/src/score.rs`) asserted
`score.drum_pattern(&score.kick, stage) == kick_bits(stage)` — but since the
10a refactor, `crate::director::kick_bits` is *defined* as
`Score::default().drum_pattern(&score.kick, stage)`. Both sides were the same
expression, so the test passed no matter what the JSON contained: it proved
nothing about the bake. The same circularity applied to the `stage_params`
and `chord_offset` portions (their free functions in `director.rs` are now
`Score::default().stage_params(..)` / `Score::default().chord_offset(..)`
shorthands) — and the `minor_for_stage` / bass / lead half of the contract
was not asserted at all.

### The frozen-legacy approach

Added a private `mod legacy` inside the `tests` module of
`crates/th-synth/src/score.rs`, holding **verbatim copies of the d97d4277
(pre-refactor) expressions** (source: `git show d97d4277:crates/th-synth/src/director.rs`),
renamed so there is no ambiguity with the current shorthands:

- consts: `ROOT_A2_HZ 110.0`, `ROOT_SHIFT 4.0/3.0`, `STAGE_RAMP_CAP 20`,
  `BPM_BASE 150.0`, `BPM_PER_STAGE 1.2`, `BRIGHTNESS_BASE 0.45`,
  `BRIGHTNESS_PER_STAGE 0.025`, `CHORD_MAJOR [0,9,5,7]`,
  `CHORD_MINOR [0,8,3,10]`, and the original `BASS_LINE` / `LEAD_LINE`
  arrays;
- `legacy_stage_params(stage) -> (f32, f32, u32, f32)` — the original
  `stage_params` body with `root()` = `ROOT_A2_HZ * ROOT_SHIFT.powi(fourths)`
  folded in (tuple, so the frozen copy carries no dependency on the
  refactored `StageParams` type);
- `legacy_kick_bits(stage)` / `legacy_snare_bits(stage)` /
  `legacy_hat_bits(stage)` — the original literal bitmask expressions
  (kick: `(1<<0)|(1<<7)|(1<<10)` + `1<<3` at stage≥10 + `1<<13` at stage≥16;
  snare: `(1<<4)|(1<<12)` + `1<<14` at stage≥12; hat: the three literal
  branches, including the `0xFFFF & !(1<<4) & !(1<<12)` late tier);
- `legacy_chord_offset(stage, bar)` — the original
  `table[(bar % 4) as usize]` indexing over the frozen tables;
- `legacy_minor_for_stage(stage)` = `(8..16).contains(&stage)`;
- `legacy_bass_line()` / `legacy_lead_line()` — the original literal arrays.

The functions are `pub` inside the private `legacy` module (Rust privacy:
the parent `tests` module may only name a child module's `pub` items); the
module itself stays private, test-only, and the frozen code is
independent of anything the refactor touched.

`score_default_matches_the_legacy_constants` now compares `Score::default()`
against these frozen expressions for stages 0..=20:

- kick/snare/hat: `score.drum_pattern(&score.<drum>, stage)` == the frozen
  literal bitmasks (63 independent comparisons);
- stage params: bpm, brightness, root within an absolute 1e-4 (the JSON
  stores `1.3333334` for `4.0/3.0`); fourths compared exactly;
- `score.minor_for_stage(stage)` == `legacy_minor_for_stage(stage)`;
- bars 0..8: `score.chord_offset(stage, bar)` == `legacy_chord_offset(stage, bar)`;
- `score.bass.notes == legacy_bass_line()` and
  `score.lead.notes == legacy_lead_line()`.

**Not vacuous (verified by mutation):** with the kick tier-0 pattern
temporarily corrupted (`x......x..x.....` → `x......x.x......`), the test
fails immediately — `panicked ... assertion left == right failed: kick
stage 0` — and the asset was restored byte-identical afterwards
(`cmp` clean). The old circular test could not have failed this way.

### Did the JSON need correcting?

**No.** Every assertion passed against the frozen legacy behaviour on the
first run, so `crates/th-synth/assets/score.json` was **not touched** (it is
byte-identical to the 10a bake; the `the_baked_score_is_the_canonical_pretty_text`
test confirms the file is still `Score::default().to_json_pretty()` + newline).
Fields checked and confirmed faithful: all three drums' tier patterns +
stage boundaries + the hat late-tier `exclude: [4, 12]`; `bpm`
(150.0 / 1.2 / cap 20), `brightness` (0.45 / 0.025), `root_hz` 110.0,
`root_shift` 1.3333334 (parses to the exact same f32 as `4.0/3.0`, so root
matches within the 1e-4 tolerance at every fourth), `minor_from` 8 /
`minor_until` 16, the chord tables (major `[0,9,5,7]`, minor `[0,8,3,10]`),
`bars_per_chord` 1, and the 16-entry bass/lead arrays.

### Octave semantics (step 3)

Approach taken: **driven through a real director.** `MusicDirector::tracks`
is a private field of the `director` module, so reading
`tracks[3]/[4].params.freq/mute` is only possible from a test inside
`director.rs` — one **new** test
`director::tests::the_bass_and_lead_write_the_legacy_octaves` was appended to
the existing `tests` module (additions only; every existing test and all
production code in `director.rs` is untouched by this pass). It:

- asserts the baked octaves: `score.bass.octave_shift == -1`,
  `score.lead.octave_shift == 1`;
- stage 3 (fourths = 1, root = `ROOT_A2_HZ * ROOT_SHIFT.powi(1)`), ticks to
  bar 0 / step 0 and asserts `tracks[3].params.freq` ==
  `root * 2f32.powi(-1) * semitone_ratio(offset + note)` (offset = I, note 0)
  with `mute == 0.0`, and `tracks[4].params.freq` ==
  `root * 2f32.powi(1) * semitone_ratio(note)` (note 12, **no** chord offset)
  with `mute == 0.0`;
- ticks to bar 1 / step 0 and asserts the bass moved with the chord
  (vi: offset +9) while the lead kept its exact bar-0 frequency
  (chord-independent).

### Gates (real output lines)

```
$ cargo fmt --all
(exit 0, no output)

$ cargo build --workspace
    Finished `dev' profile [unoptimized + debuginfo] target(s) in 1.09s

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev' profile [unoptimized + debuginfo] target(s) in 0.65s

$ cargo test --workspace
test result: ok. 592 passed; 0 failed; ... (69 `test result: ok` suites,
                                            0 `test result: FAILED`)
```

Key lines: `score::tests::score_default_matches_the_legacy_constants ... ok`,
`director::tests::the_bass_and_lead_write_the_legacy_octaves ... ok`,
`score::tests::the_baked_score_is_the_canonical_pretty_text ... ok`,
`score::tests::score_default_round_trips ... ok`,
`score::tests::a_bad_score_is_an_error_not_a_panic ... ok`, all other
pre-existing tests unchanged and passing.

### Deviations from the fix-pass brief (all minimal, all flagged)

1. **One new test appended to `crates/th-synth/src/director.rs`** (the
   octave-semantics test). Step 3's primary approach ("drive through a real
   director, read `tracks[3]/[4].params.freq/mute`") is only possible from
   inside the `director` module (`tracks` is private); the brief's
   do-not-change-`director.rs` clause was read as "do not alter the 10a
   refactor or weaken existing tests", which holds: zero existing lines
   changed there.
2. **Pre-existing clippy blockers fixed** (all present at HEAD `8abe61b6`
   / `d97d4277`, all in files the 10a work did NOT touch, all
   behaviour-neutral; required for the mandated
   `cargo clippy --workspace --all-targets -- -D warnings` to pass):
   - `crates/th-audio/src/lib.rs:600` — `clippy::doc_lazy_continuation`:
     the `/// + unlock ...` doc line started a lazy list; indented the
     continuation line two spaces (the lint's own suggested fix);
   - `crates/th-game/src/shrapnel.rs` lines 797/814/830/853/868/874 —
     6× `#[must_use]` `Shrapnel::update` return unused in
     `update_rides_velocity_drags_tumbles_and_burns_life`: `let _ = ...`
     (the lint's suggested fix);
   - `crates/th-game/tests/shrapnel.rs:141` — dead `let impact = ...`:
     renamed to `_impact` (keeps the computation);
   - `crates/th-synth/src/clink.rs:79` — `clippy::unnecessary_cast`:
     `crack_len as f32` with `crack_len: f32` → dropped the cast;
   - `crates/th-app/src/tiny.rs:340` — dead `let plain = ...` in
     `play_shard_clinks` (dead since `d97d4277` stripped the
     `?clinksplain` instrument): renamed to `_plain` (keeps the pure URL
     query read; no behaviour change).
3. No other scope: `crates/th-synth/assets/score.json` untouched,
   `director.rs` production code untouched, all other tests as-is.

### Changed files (this fix pass)

- `crates/th-synth/src/score.rs` — `mod legacy` (frozen d97d4277 copies) +
  rewritten `score_default_matches_the_legacy_constants` + dropped the
  now-unused director-shorthand imports;
- `crates/th-synth/src/director.rs` — one new test
  `the_bass_and_lead_write_the_legacy_octaves` (additions only);
- gate-enabling lint fixes: `crates/th-audio/src/lib.rs`,
  `crates/th-game/src/shrapnel.rs`, `crates/th-game/tests/shrapnel.rs`,
  `crates/th-synth/src/clink.rs`, `crates/th-app/src/tiny.rs`.

## POLISH PASS 1

**Date:** the bounded polish pass after the reviewer's audit of the 10a
work (spec: `.agents/tasks/plan-002/issue-010a-impl.md`; plan:
`.agents/plans/002-tinyhexagon/10-scorelab.md`). Worktree:
`/home/gary/dev/tinyhexagon/.agents/worktrees/slot-1` (branch `slot-1`,
base `d97d4277`). Nothing committed. ONLY the three accepted findings
below were changed; no other behavior was added.

### 1. Stale module docs (both fixed)

**`crates/th-synth/src/score.rs`** (module doc, top of file): the old
text said `score_default_matches_the_legacy_constants` asserted
"against the default-score shorthands in `crate::director`" — the
PRE-fix-pass circular design. It now states the truth: the
bake-fidelity test compares the parsed default score against the
FROZEN pre-refactor (d97d4277) expressions in this module's private
`mod legacy` (verbatim copies of the d97d4277 code), so it is
independent of the `Score::default()` shorthands in `crate::director`
(asserting through them would make the test vacuous: both sides the
same expression).

**`crates/th-synth/src/director.rs`** (module doc, tail of the score
paragraph): the old text said the consts/free functions are "the
DEFAULT-score shorthands the tests' tick arithmetic **and the
bake-fidelity test** ... keep asserting against." It now states: the
shorthands are what the tests' tick arithmetic asserts against; the
bake-fidelity test (`score_default_matches_the_legacy_constants`, in
`score.rs`) does **NOT** — it compares the parsed default score
against the frozen pre-refactor (d97d4277) expressions in `score.rs`'
private `mod legacy` (verbatim copies), since asserting through these
shorthands would make it vacuous (they are `Score::default()` lookups).

Both rewrites keep the existing comment style and the ~76-col wrap.

### 2. New test: the `set_score` re-arm pair

Added `director::tests::set_score_reapplies_the_current_step_and_bar`
to `crates/th-synth/src/director.rs`. The finding: `set_score` forces
`self.last_bar = None; self.last_step = None;` so the next tick
rewrites the bar chord (`apply_bar`) and the step's notes
(`apply_step`) — and that pair was UNCOVERED: deleting both lines left
the whole suite green, because the existing `set_score_*` tests assert
mutes/gains/pattern-bits, and `sequence()`'s `gate_voice` rewrites the
mutes every tick (masking the gap). The new test therefore asserts
note FREQUENCIES — the state ONLY rewritten by
`apply_bar`/`apply_step` when the re-arm fires:

- fallback director, `tick(0.01)` → bar 0, step 0 (the default bass's
  note 0 is on, chord I): asserts `tracks[3].params.freq` ==
  `root * 2^(-1) * semitone_ratio(0)` and `tracks[5].params.freq` ==
  `root` (the pad, chord I, offset 0);
- `set_score` with a score that CHANGED the step's note
  (`bass.notes[0] = Some(5)`) AND the bar's chord
  (`chords.major[0] = 7`) — the same score otherwise;
- `tick(0.001)` — still step 0 (the tiny dt means the step index does
  NOT advance): asserts the bass freq is now
  `root * 2^(-1) * semitone_ratio(7 + 5)` and the pad freq is now
  `root * semitone_ratio(7)`.

The step half pins the `last_step` re-arm (only `apply_step` rewrites
the bass/lead freqs); the bar half pins the `last_bar` re-arm (only
`apply_bar` rewrites the pad/bell/sub freqs) — so the test fails if
EITHER line is removed.

**Mutation check (performed, as required):** I commented out BOTH
`self.last_bar = None; self.last_step = None;` lines in `set_score`'s
body (the `set_stage` and `try_start_live` copies left untouched —
verified by grep before running) and re-ran the new test: it FAILED,
exactly as the review predicted:

```
running 1 test
test director::tests::set_score_reapplies_the_current_step_and_bar ... FAILED

failures:

---- director::tests::set_score_reapplies_the_current_step_and_bar stdout ----

thread 'director::tests::set_score_reapplies_the_current_step_and_bar' (795758) panicked at crates/th-synth/src/director.rs:1043:9:
the step's note freq re-applies within the same step (got 55, want 110)
note: run with `RUST_BACKTRACE=1` environment variable to display backtrace

failures:
    director::tests::set_score_reapplies_the_current_step_and_bar

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.00s
error: test failed, to rerun pass `-p th-synth --lib`
```

(stale freq 55.0 vs the new note's 110.0). Under the SAME mutation the
pre-existing `set_score_takes_effect_live` and
`set_score_rewrites_the_slot_gains` both still passed (`test result:
ok. 1 passed` each) — confirming they are what masked the gap. The two
lines were then RESTORED and the test re-run:
`test director::tests::set_score_reapplies_the_current_step_and_bar ... ok`
(restoration verified by grep: the re-arm pair is present again in
`set_stage`, `set_score`, and `try_start_live`).

### 3. Honest `expect` on unvalidated input

`crates/th-synth/src/score.rs`, `drum_pattern`: the old
`.expect("tiers start at stage 0 (validated by from_json)")` could
panic if a caller hand-built a `Score` (bypassing
`from_json`/`set_score`'s validation) with an empty `tiers` vec or a
first tier whose `stage` is above the queried stage. The BEHAVIOR is
unchanged (panicking is acceptable for programmer-error input); the
message now names the invariant AND the fix:

```rust
.expect(
    "drum tiers must be non-empty with tiers[0].stage == 0 (Score::from_json validates this; a hand-built Score must too)",
);
```

(the single long literal follows the repo's existing style for long
string arguments — rustfmt keeps it on one line).

### Gates (real output lines, final/restore state)

```
$ cargo fmt --all
(exit 0, no output)

$ cargo build --workspace
warning: output filename collision at .../target/debug/tinyhexagon
  = note: the bin target `tinyhexagon` in package `th-web` has the same
           output filename as the bin target `tinyhexagon` in package `th-app`
  = note: this may become a hard error in the future; see <https://github.com/rust-lang/cargo/issues/6313>
  = help: consider changing their names to be unique or compiling them separately
(same collision note repeated for `tinyhexagon.dwp`)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
(exit 0 — the collision warning is pre-existing at d97d4277 and is a
 cargo-level note, not a rustc warning)

$ cargo clippy --workspace --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.16s
(exit 0)

$ cargo test --workspace
69 suites: `test result: ok`, zero `FAILED` lines;
TOTAL: 593 passed; 0 failed; 1 ignored   (was 592 passing before this
 pass — the +1 is the new test)

$ cargo test -p th-synth set_score
running 3 tests
test director::tests::set_score_rewrites_the_slot_gains ... ok
test director::tests::set_score_reapplies_the_current_step_and_bar ... ok
test director::tests::set_score_takes_effect_live ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 25 filtered out; finished in 0.00s
```

### Changed files (this polish pass)

- `crates/th-synth/src/score.rs` — module doc (top) + the
  `drum_pattern` `expect` message (no functional change);
- `crates/th-synth/src/director.rs` — module doc (tail) + one new test
  `set_score_reapplies_the_current_step_and_bar` (additions only;
  production code identical to pre-polish after the mutation check
  restored the re-arm lines);
- `.agents/tasks/plan-002/issue-010a-report.md` — this section.
