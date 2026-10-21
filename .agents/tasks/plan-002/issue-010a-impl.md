# Issue 10a — the `Score` type, the live setter, the baked asset

**Worktree:** `.agents/worktrees/slot-1` (branch `slot-1`), base
**`d97d4277`** (master). The main worktree and slot-2 are OFF-LIMITS. **Never
commit** — the orchestrator lands one squashed commit after review.

**Read first (in this order):**
1. `.agents/plans/002-tinyhexagon/10-scorelab.md` — the full schema, the frozen
   interface, the non-negotiables. THE SOURCE OF TRUTH for this task.
2. `crates/th-synth/src/director.rs` — the whole file. You are replacing the
   hardcoded content (`BASS_LINE`, `LEAD_LINE`, `kick_bits`/`snare_bits`/
   `hat_bits`, `CHORD_MAJOR`/`CHORD_MINOR` usage, `stage_params` internals,
   `minor_for_stage`, `chord_offset`, `apply_slot`'s gains) with lookups into a
   `Score`. Keep `SLOTS`, `Slot`, the preset-character/LFO/reverb blocks, the
   ducking, the engine start paths and the seconds-per-step clock logic.
3. `crates/th-synth/src/lib.rs`, `crates/th-synth/src/beat.rs`,
   `crates/th-synth/Cargo.toml`.
4. `crates/th-app/src/tiny.rs` — the `web` module (bottom of the file): the
   `thread_local DIRECTOR` handle and `resume_audio()`; you add the two
   `web_get_score` / `web_set_score` helpers next to them.
5. `crates/th-web/src/web.rs` — the `#[wasm_bindgen]` exports pattern
   (`status` / `resumeAudio`); you add `getScore` / `setScore` in the same
   style, delegating to the `th-app` helpers under `#[cfg(target_os =
   "emscripten")]`.

Write-first: create `crates/th-synth/src/score.rs` and
`crates/th-synth/assets/score.json` BEFORE reading further afield.

## What to build

1. **`crates/th-synth/assets/score.json`** — the shipped score, exactly the
   schema in plan 10, and it MUST encode today's music. Derive the drum
   patterns by RUNNING the current functions, not by hand: the existing tests
   in `director.rs` assert `kick_bits(0) == (1<<0)|(1<<7)|(1<<10)`,
   `hat_bits(0).count_ones() == 4`, `hat_bits(6).count_ones() == 8`,
   `snare_bits(0)` = `(1<<4)|(1<<12)`, and the stage-10/16 kick + stage-12
   snare + stage-14 hat additions. Add a temporary `#[test]` that prints each
   `format!("{:016b}", bits)` for stages 0..=16 and delete it after transcribing
   (or reason from the code — either way the round-trip test below is what
   proves it).

2. **`crates/th-synth/src/score.rs`** — the `Score` structs per the frozen
   interface: `Score`, `BpmRamp`, `BrightnessRamp`, `DrumTiered`, `DrumTier`,
   `NoteLine`, `SlotGain`, `Chords`. Serde `rename_all = "snake_case"` (the
   field names in plan 10 are already snake_case; keep them). Parse pattern
   strings via `char_indices`/`chars().position()` — NEVER byte arithmetic
   (the strings are ASCII today but the parse must not rely on it).
   `from_json` must return `Result<Self, String>` with a human-readable
   message (include serde's error text; prefix with the field path if serde
   gives one). Validate: `steps_per_bar` in 1..=64, every pattern string's
   length equals `steps_per_bar`, `chords.major`/`chords.minor` length is 1..=16, `slots`
   has exactly 8 entries. A failed validation is an `Err`, never a panic.

3. **`MusicDirector` rewiring** (`director.rs`):
   - Field `score: Score`, initialised from `Score::default()`; `new_fallback`
     and every constructor path sets it.
   - `set_score(&mut self, score: Score)` — assign, then re-apply: the 8 slots'
     gains + drum patterns (`pattern_bits`, `pattern_hits`, `pattern_rotation`),
     and force `last_bar = None; last_step = None` so the next tick rewrites
     the chord/step. Do NOT rebuild the graph, do NOT touch `self.live`'s
     engine beyond the existing `Shared` writes (if live, also re-push
     `global.bpm`/`brightness`/`scale_mode` like `set_stage` does).
   - `score(&self) -> &Score`.
   - `stage_params` / `minor_for_stage` / `chord_offset` at MODULE level are
     used by tests and by `apply_slot`/`set_stage`/`apply_bar`/`apply_step`.
     Keep module-level free functions for the DEFAULT score working (the
     existing tests call `stage_params(3)`, `chord_offset(...)`,
     `minor_for_stage(...)`, `kick_bits(...)` etc.) — implement them as thin
     wrappers over `Score::default()` so the existing test bodies and the
     `slots_unlock_per_stage` / `the_drum_patterns_are_distinct_and_evolve` /
     `the_bass_and_lead_are_sequenced_per_step` / `the_gate_releases_between_notes`
     / `the_bar_chord_moves_the_harmony_only` tests pass UNCHANGED. Add
     `#[deprecated]`-free doc notes that the director's live path reads
     `self.score`, and these free functions are the default-score shorthands.
   - `BPM_BASE`, `BPM_PER_STAGE`, `BRIGHTNESS_BASE`, `BRIGHTNESS_PER_STAGE`,
     `STAGE_RAMP_CAP`, `ROOT_A2_HZ`, `ROOT_SHIFT` stay as `pub const`s **only
     if** the wrappers need them; otherwise re-express them from
     `Score::default()` and keep the names re-exported so existing tests
     (`stage_map_monotonic_and_capped` asserts `prev.bpm == BPM_BASE`) compile.
   - `GATE_OPEN` / `SWING` are read by `sequence()` — they must now come from
     `self.score` (the tests `the_gate_releases_between_notes` /
     `the_bass_and_lead_are_sequenced_per_step` use the module consts for both
     the tick math and the assertion; keep the consts as the default-score
     values and have `sequence()` read `self.score.gate_open`/`.swing`, with a
     test proving a non-default score changes the gate).

4. **wasm plumbing**: `th-app/src/tiny.rs` gets `web_get_score` /
   `web_set_score` (under the existing `#[cfg(target_os = "emscripten")]` web
   module, using the `DIRECTOR` thread-local; a missing director returns `""` /
   `"score lab: no director"`). `th-web/src/web.rs` adds the two exports.
   Non-emscripten: `web_get_score`/`web_set_score` must still exist as safe
   stubs (returning `""` / an error string) so the host gate compiles — mirror
   how `web_request_audio_unlock` is declared for all targets in
   `th-app/src/tiny.rs` (check the exact cfg shape there and match it).

5. **Tests** (in `score.rs` and `director.rs`):
   - `score_default_round_trips`: `Score::from_json(&Score::default().to_json_pretty())
     == Score::default()`.
   - `score_default_matches_the_legacy_constants`: for stages 0..=20 the
     default score's kick/snare/hat bitmasks equal
     `kick_bits`/`snare_bits`/`hat_bits`, its `stage_params` equals the legacy
     one (bpm/brightness/root), and its `chord_offset` equals
     `chord_offset`. THIS is the test that proves the bake is faithful.
   - `set_score_takes_effect_live`: build a fallback director, `tick` to step 0,
     then `set_score` with a score whose `bass.notes[0]` is `None` and assert
     the bass's `mute` becomes 1.0 on the next tick (and a changed `swing`
     changes the swung-note window timing).
   - `a_bad_score_is_an_error_not_a_panic`: `from_json("{}")`,
     `from_json("{\"steps_per_bar\":16}")`, a pattern string of the wrong
     length, and a `slots` array of 7 → all `Err`, no panic.
   - Keep `Score::to_json_pretty` stable enough that the lab can diff it: use
     `serde_json::to_string_pretty` and assert the output re-parses.

## Gates you must run before reporting

- `cargo fmt --all`
- `cargo build --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo check -p th-synth --target wasm32-unknown-emscripten` (if the target
  is not installed: `rustup target add wasm32-unknown-emscripten`)

Do NOT run `tools/package-web.sh` (it is slow and the orchestrator does it).

## Report (write to `/home/gary/dev/tinyhexagon/.agents/tasks/plan-002/issue-010a-report.md`)

- What changed, file by file.
- The EXACT command output lines for each gate (the `GATE_EXIT=` / test
  summary lines — paste, do not paraphrase).
- Any deviation from this spec and why (a deviation is acceptable when the
  code contradicts the spec; say so explicitly and either correct the spec file
  in the plan (minimal, factual) or flag it).
- Any skill-file correction you made (`.agents/skills/*.md`) and why.
- The `cargo test` count before vs after (baseline today: 588 tests passing).
- `TODO(skill-gap)` markers you left behind, if any.
