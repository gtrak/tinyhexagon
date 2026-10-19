# Feed-forward — plan 002 (tinyhexagon)

Living document. After every landed issue, the orchestrator appends the
lessons and updates the affected later issue files and the next task spec.
This is how a fresh worker on a later issue inherits the earlier work without
re-reading the whole history.

## Rules

- Record only what changes later work: accepted deviations, final API
  signatures, renamed/removed files, new caveats, per-target behaviour, and
  shifts in the upstream mapping.
- Update the later issue file(s) in the same pass; do not leave a note here
  that contradicts a frozen issue file.
- The reviewer checks that the feed-forward was applied.
- Keep entries short and dated; link the issue/commit.

## Log

### 00 — upstream (LANDED)

- Baseline SHA / default branch:
  4c31561ac6409e24a1b838043a0bb747f504045d / master
- Remote verified: `upstream` = `https://github.com/gtrak/SSVOpenHexagon`
  (fetch/push); upstream HEAD == HWM == fork HEAD; delta 0.
- Mapping corrections: none. All 22 baseline crates accounted for:
  12 kept/repurposed (`oh-*` → `th-*`, android/web repurposed), 10 removed,
  1 new (`th-synth`).
- Open questions: none. `shellcheck` is unavailable in this environment; the
  script was checked with `bash -n` and synthetic-repo cases (behind/current,
  bad marker, missing remote).

### 01 — `.agents` history (LANDED)

- Archive path: `.agents/plans/archive/001-rust-port/` — the old plan
  files, `DECISIONS.md`, `PARITY.md`, `deviations.md`, `STATUS.md`,
  `ISSUE-019-REPORT.md`, `WORKER-HYGIENE.md`, `mapping/`, the `parity/`
  corpus, `audits/`, `evidence/`, `workflows/`, and every port-era task
  spec under `tasks/` (the pre-plan-002 review protocol restored there as
  `tasks/review-gate-port.md` from `4c31561a`). Nothing in the archive was
  edited; the plan-002 task files live in `.agents/tasks/plan-002/`.
- Gate stages (4, final result all green, gate rc 0): 1. `cargo fmt --all
  -- --check` 2. `cargo build --workspace` 3. `cargo clippy --workspace
  --all-targets -- -D warnings` 4. `cargo test --workspace` (1014 tests
  passed, 0 failed). The Lua-parity and additions-boundary stages are
  gone; the failing-stage-tail rule and `target/gate.log` are kept.
- Test files removed with the parity crate: all 12 under
  `crates/oh-parity/tests/` (`common/mod.rs`, `dialect_census.rs`,
  `gen_corpus.rs`, `lua51_compat_prelude.rs`, `lua51_snippet.lua`,
  `lua_engine_parity.rs`, `lua_vm_probes.rs`,
  `parity_cpp_fingerprints.rs`, `parity_level_runs.rs`,
  `parity_pack_ids.rs`, `parity_replay.rs`, `parity_rng.rs`) plus the
  crate's `src/lib.rs` + `Cargo.toml`; the lockfile dropped only the
  `oh-parity` entry (17 lines). No other test read a deleted path at
  runtime — oh-parity was the only corpus consumer.
- Reference fixes (doc comments only, no assertions weakened): the
  `crates/oh-parity/tests/lua51_compat_prelude.rs` citations in
  `crates/oh-lua/src/prelude.rs`, `crates/oh-lua/src/context.rs` and
  `crates/oh-lua/lua51_prelude.lua` (×2) now point at git history; the
  `lua51_prelude.lua` task-spec pointer retargeted to
  `.agents/plans/archive/001-rust-port/tasks/issue-030c-three-way-parity.md`.
- Deviations: none from the issue contract. `.git/modules/extlibs` state
  was removed by `git submodule deinit` itself; the 5 gitlinks were
  dropped from the index with `git rm --cached` so `git submodule status`
  is empty. Historical doc comments elsewhere in `crates/` that cite the
  pre-archive `.agents/plans/001-rust-port/...` or `tools/oracle/...`
  paths were left in place (history, not build inputs — working
  agreement); a docs sweep (Issue 09) can retarget them. `.github`
  workflows were not in scope; `ci.yml`'s gate job runs `tools/gate.sh`
  (now 4 stages) and is unaffected, though its header comments still
  describe the retired lua-parity/cpp-oracle stages; the active gate job
  already calls the four-stage `tools/gate.sh`, and the comment cleanup
  belongs to Issue 08/09.

### 02 — crate removal / engine de-scope (pending)

- Final `HexagonGame::new` signature:
- Removed `GameApi` methods:
- `draw_target.rs` API:
- Lua globals removed/no-oped:
- RNG-order note:
- `th-app` keep-list / temporary stubs:

### 03 — rename (pending)

- Final crate names:
- Binary/artifact names:
- Rename exceptions:
- `default-members`:
- Tinyhexagon bin timing (renamed here or deferred to 07):

### 04 — engine kid changes (pending)

- Soft collision API/bump accessor:
- `set_difficulty_mult` range:
- `apply_music_beat` / `StyleData::apply_beat_hue`:
- `DrawOptions` shape:
- Wall-gap API:
- Tests added:

### 05 — synth (pending)

- Spike result (stock vs patched):
- Patch location:
- Director API:
- Stage/layer table:
- Web gesture hook:
- Target caveats:

### 06 — tiny pack (pending)

- Level ids / speeds / delays:
- First-pattern delay:
- Styles/hues:
- Lua globals no-oped:
- Load/run test:

### 07 — app tiny runtime (pending)

- Composition signature:
- Journey table:
- Touch mapping:
- SFX ids/volumes:
- Beat-hue call:
- Headless tests:
- No-storage proof:

### 08 — Android/web/packaging (pending)

- Android entry:
- Web boot/audio-unlock export:
- Artifact names:
- Target degradations:
- Pages URL:

### 09 — tuning/docs (pending)

- Final tunables:
- Playtest notes:
- Per-target deviations:
- Plan closed/archived:
