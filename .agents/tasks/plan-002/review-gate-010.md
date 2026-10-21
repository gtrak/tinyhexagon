# Review gate — plan 002 issue 10 (the score lab), sub-issues 10a / 10b

You are the **gate-reviewer**: read-only, adversarial, WITH execution. You can
and must RUN the gates yourself — do not accept the implementer's claims about
green tests. Read this whole file first.

## Authority

- `.agents/plans/002-tinyhexagon/10-scorelab.md` is the spec of record: the
  schema, the frozen interface, the non-negotiables.
- `.agents/tasks/plan-002/issue-010a-impl.md` / `-010b-impl.md` are the task
  specs.
- Skills (`.agents/skills/*.md`, especially `lib-rust-synth.md`) are
  authoritative over your memory of the crate. The vendored
  `vendor/rust-synth-0.23.0/` source is the truth for the engine's API.
- Code reality that contradicts a spec may be a justified deviation — but the
  implementer must have SAID so in its report. An undocumented deviation is a
  blocking finding.

## For 10a — the Rust half

Run yourself, in the worktree:

```
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p th-synth --target wasm32-unknown-emscripten   # if the target is installed
```

Then check, adversarially:

1. **The bake is faithful.** This is the single most important claim.
   Independently verify that `crates/th-synth/assets/score.json` reproduces
   today's music. Do NOT trust
   `score_default_matches_the_legacy_constants` at face value — read it and
   confirm it compares against the REAL legacy expressions, not against
   re-derived copies of the same JSON. Spot-check at least two of:
   the kick's stage-0/10/16 bitmasks, the hat's three tiers (including the
   `exclude` of steps 4 and 12 in the ≥14 tier), `snare_bits(12)`, the bass
   line's 16 entries, the lead line's 16 entries, and `stage_params(7)`
   (bpm, brightness, root).
2. **Nothing rebuilds the graph on a score change.** `set_score` must write
   only `Shared` params / `pattern_bits` / `last_bar` / `last_step`. Grep for
   `rebuild_graph` and confirm no new call site.
3. **The existing director tests are UNCHANGED in substance.** Diff them
   against `git show d97d4277:crates/th-synth/src/director.rs`. If an assertion
   was weakened or deleted, that is BLOCKING unless the report justifies it as
   a genuine spec change.
4. **Non-vacuity** (a repeated failure mode in this repo): read every new test
   and ask "could this pass with the feature absent?" Specifically,
   `set_score_takes_effect_live` must actually assert an OBSERVABLE change
   caused by `set_score` (a mute/frequency/timing delta), not merely that the
   call returns.
5. **Validation, not panics.** `from_json` on `{}`, on a wrong-length pattern,
   on a 7-entry `slots` array, and on `{"steps_per_bar":16}` → all `Err`, no
   panic, no `unwrap`. Check the `steps_per_bar` bounds and the `chords` length
   bound exist.
6. **The melodic/octave semantics.** The lead must NOT follow the chord offset
   (the legacy behaviour; `the_bar_chord_moves_the_harmony_only` asserts it)
   and the bass must keep its `root * 0.5` (now `octave_shift: -1`). A
   refactor that silently makes the lead chord-relative is BLOCKING.
7. **No new deps beyond serde/serde_json** in `th-synth`, and no `std::fs` /
   file reads (the wasm target must not need a filesystem). Confirm
   `include_str!` is the only file access.
8. **wasm first-classness**: `th-web/src/web.rs`'s two new exports delegate
   correctly and the non-emscripten stubs exist so the host gate compiles.

## For 10b — the lab page

1. **Default path untouched.** With no query string, the page must behave
   exactly as before (text-free, `#status` hidden, the gesture unlock intact).
   Diff `index.html` and confirm every new branch is behind the query check.
2. **No new network/fetch on the default path.** The hot reload's `fetch` must
   only run when `?score=` is present.
3. **The exports are the frozen names** (`getScore` / `setScore`) and no
   temporary shim remains in the final file (grep for the shim's guard).
4. **The probe still passes** on the default path. If the probe's default
   assertions were weakened, BLOCKING.
5. **The overlay's edits map to real schema fields** — read the mapping and
   verify one drum toggle and one note cycle produce the documented JSON
   change (trace it by hand through the code; you may run node on an extracted
   script with a stubbed `mod` if that is cheaper than the browser run).
6. `?scorelab=1` and `?score=` combined must not double-fetch or double-apply.

## Verdict contract

Write your findings as: **non-blocking suggestions first**, then exactly one
final line, either:

```
VERDICT: PASS
```

or

```
VERDICT: BLOCKING -- 1. <file:line — must-fix> 2. <...>
```

Numbered, specific, each pointing at a file:line and stating the required fix.
Do not bundle unrelated nitpicks into the blocking list. If the only issues are
cosmetic, PASS with suggestions.
