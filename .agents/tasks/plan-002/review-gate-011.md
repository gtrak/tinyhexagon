# Review gate — plan 002 issue 11 (the DAW), sub-issue 11a

You are the **gate-reviewer**: read-only, adversarial, WITH execution. You can
and MUST run the gates yourself inside the worktree. Do not accept the
implementer's claims. Read this whole file first.

## Authority

- `.agents/plans/002-tinyhexagon/11-daw.md` — the model (the role/timbre split,
  the mute/solo fold, the schema). THE SPEC OF RECORD.
- `.agents/tasks/plan-002/issue-011a-impl.md` — the task spec.
- `.agents/tasks/plan-002/issue-011a-report.md` — the implementer's report.
- The vendored `vendor/rust-synth-0.23.0/` source is the truth for the engine
  API (`PresetKind`, `ALL_KINDS`, `Track.kind`, `EngineHandle::rebuild_graph`,
  `TrackParams::pattern_bits`). `.agents/skills/lib-rust-synth.md` is
  authoritative over your memory of it.

## Run yourself, in the worktree

```
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p th-synth --target wasm32-unknown-emscripten   # if installed
```
Capture exit codes honestly. **Never pipe the gate to `tail`** — that reports
tail's status and hid a red clippy stage for most of this project's audio arc.

## The findings that matter most

1. **The mute/solo fold is the whole point — verify it adversarially.**
   The refactor exists because `mute` was written by three places and the
   per-tick `gate_voice` would clobber a user mute. So:
   - Read `sequence()` and confirm ALL 8 slots' `mute` is written every tick
     from a fold that includes the user state.
   - Confirm `apply_slot` no longer writes `mute`.
   - **Mutation-test it yourself**: delete the user-mute term from the fold and
     confirm `user_mute_survives_the_gate` FAILS. Restore. Then do the same for
     the solo term against `solo_isolates_one_slot`. If either test still
     passes with its term removed, the test is VACUOUS and that is BLOCKING.
   - Verify the mute folds the STAGE-UNLOCK for drums/harmony (a stage where a
     slot is locked must still be silent even with `muted: false`).

2. **The instrument swap must actually reach the engine, and only when it
   changed.** Read the diff carefully:
   - `set_score` with a changed `slots[i].kind` must mutate the live handle's
     track kind and rebuild the graph.
   - A gain-only or mute-only edit must NOT rebuild (that costs an audible
     discontinuity for nothing). Verify the guard by reading the code, and
     check the test actually distinguishes the two cases (a test that passes
     when the guard is removed is vacuous — mutation-test it).
   - `apply_slot`'s per-preset parameter block must key on the RESOLVED kind,
     not the built-in `SLOTS[i].kind` — otherwise a swapped instrument gets the
     wrong timbre parameters. This is an easy miss; check it specifically.

3. **Backwards compatibility is REQUIRED.** A score JSON saved by issue 10
   (no `kind`, no `muted`, no `solo`) must still parse, and each slot must
   resolve to its built-in kind. Verify with a real issue-10-shaped JSON, not
   just by reading the `#[serde(default)]` attributes. Also confirm the
   SHIPPED `assets/score.json` has every `muted: false` and `solo: null`
   (a stray baked solo would silence the game for a child).

4. **The default score must still render identical music.** The whole bake
   contract. `score_default_matches_the_legacy_constants` and the `mod legacy`
   fidelity tests must pass UNCHANGED. Diff them against the base commit — a
   weakened assertion is BLOCKING.

5. **Validation, not panics.** `from_json` with `"kind": "nope"` and with
   `"solo": 99` → `Err` (naming the slot / the bound), no panic. `kind_from_id`
   on garbage → `None`.

6. **Non-vacuity sweep.** Read every new test and ask "could this pass with the
   feature deleted?" Spot-check at least two by mutation.

## Verdict contract

Non-blocking suggestions first, then exactly one final line:

```
VERDICT: PASS
```
or
```
VERDICT: BLOCKING -- 1. <file:line — must-fix> 2. <...>
```

Numbered, specific, file:line, with the required fix stated. Do not bundle
nitpicks into the blocking list.
