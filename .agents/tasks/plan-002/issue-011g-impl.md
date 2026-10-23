# Issue 11g — the song form: per-bar figures (the phrase)

**Worktree:** `.agents/worktrees/slot-2` (branch `slot-2`), base **the landed
11f commit** (the orchestrator gives the exact SHA — do NOT start from an older
base). Main worktree and slot-1 are OFF-LIMITS. **Never commit.**

**Read first:**
1. `.agents/plans/002-tinyhexagon/11f-arrangement.md` — THE SOURCE OF TRUTH,
   including **THE COMPOSITION BOUNDARY (operator directive)**: you build the
   MECHANISM, you do NOT author the music.
2. `.agents/tasks/plan-002/issue-011f-impl.md` + the 11f report — what landed.
3. `crates/th-synth/src/director.rs` — `sequence` (`bar = step / steps_per_bar`,
   then `apply_step(step % steps_per_bar)`), `apply_step`, `apply_bar`, the
   `Role` enum from 11f.
4. `crates/th-synth/src/score.rs` — the schema, validation, and the fidelity
   tests that must keep passing.
5. `crates/th-synth/examples/audition_render.rs` — the renderer you extend to
   render a phrase.

## What to build

### 1. The `bars` phrase (schema + sequencer)

`Score` gains:

```rust
#[serde(default)]
pub bars: Vec<Bar>,

pub struct Bar {
    #[serde(default)]
    pub bass: Option<NoteLine>,
    #[serde(default)]
    pub lead: Option<NoteLine>,
    #[serde(default)]
    pub comp: Option<String>,
}
```

Semantics (all four are REQUIRED):

- **Empty `bars` = exactly today's behaviour**: one bar whose figures are the
  top-level `bass`/`lead`/`comp`. A score written before this issue parses and
  plays unchanged — that is the backwards-compatibility contract, and the
  shipped bake is the reference.
- **Non-empty**: the bar index is resolved modulo `bars.len()`, and each field
  falls back to the top-level value when `None`. So a `Bar { lead: None, .. }`
  means "this bar uses the default lead line".
- The **phrase length is independent of the chord cycle**: `chord_offset` keeps
  its own `bars_per_chord` + table arithmetic over the ABSOLUTE bar counter, so
  an 8-bar phrase over a 4-chord progression cycles the chords twice. Do not
  couple them.
- **Validation**: every `bars[i].bass`/`.lead` `notes.len()` must equal
  `steps_per_bar`; every `bars[i].comp` pattern must be exactly
  `steps_per_bar` chars. Violations are `Err` naming the bar index, never a
  panic. `bars.len()` must be `<= 64` (a sanity bound; an unbounded phrase is a
  score bug, not a feature).

### 2. The sequencer must actually vary

`sequence()` currently derives `bar` and calls `apply_step(step % steps_per_bar,
bar)`. The figure for a bar must now come from the resolved `Bar` (with the
fallbacks above), and `apply_step`/the gate must read THAT figure — including
the bass/lead `note_line_bits`-derived `pattern_bits`, which currently are
computed from the top-level lines and would go stale per bar.

Careful: `pattern_bits` for bass/lead/comp is derived from the notes/pattern of
the bar NOW PLAYING. Recompute it when the bar changes (in `apply_bar`, or where
`apply_step` first sees a new bar) — a stale `pattern_bits` is exactly the
subtle bug that would make a phrase sound like one bar. Test it.

### 3. The audition renderer renders the PHRASE

`audition_render` currently renders 4 bars of ONE static setup. Extend it to
render the whole phrase (the resolved `bars.len()` bars, default 4) and print a
per-bar marker line naming which bar index and which figure each bar used, so a
listener can follow the form. Keep the per-slot table, the atk/tail column and
the headroom line. Optionally add a `--bars N` flag.

The point: the operator must be able to HEAR that bar 5 differs from bar 1.

### 4. DO NOT COMPOSE

Per the plan's composition boundary: do NOT write an 8-bar arrangement and do
NOT re-tune anything by taste. Leave `assets/score.json`'s `bars` EMPTY (or omit
it) so the shipped bake keeps playing today's music, and provide the mechanism
plus a small TEST-ONLY phrase in the tests (a couple of bars that differ) to
prove resolution and fallback work. The orchestrator composes the real
arrangement afterwards by editing the JSON.

If you find yourself deciding which bar should have the melody, stop — that is
not this task.

## Tests (non-vacuous — mutation-check at least two)

- `an_empty_bars_list_is_todays_behaviour`: a score with no `bars` produces the
  same `pattern_bits`/frequencies as before, bar after bar.
- `bars_resolve_modulo_and_fall_back`: a 2-bar phrase over 4 bars resolves
  bars 0,1,0,1 and a `None` field falls back to the top-level value.
- `the_phrase_actually_changes_what_plays`: two bars with DIFFERENT bass lines
  produce different bass `freq`/`pattern_bits` on the corresponding steps.
  Mutation-check this — if you make the resolution always return bar 0, it must
  FAIL.
- `a_bad_bar_is_an_error`: a wrong-length note line or pattern string, and an
  over-long `bars` list → `Err` naming the bar.
- Backwards compatibility: the shipped `assets/score.json` (no `bars`) still
  parses, and the `mod legacy` fidelity tests still pass.

## Gates (capture exit codes HONESTLY — never pipe the gate to tail)

```
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p th-synth --target wasm32-unknown-emscripten
cargo run -q -p th-synth --example audition_render -- /tmp/phrase.wav --solo-dir /tmp/phrase-solos
```

## Report (`.agents/tasks/plan-002/issue-011g-report.md`, RELATIVE)

Per-file changes; the resolution + fallback + `pattern_bits`-refresh design;
the per-bar marker output; the mutation checks; the backwards-compatibility
evidence; and an explicit confirmation that you did NOT author an arrangement.
