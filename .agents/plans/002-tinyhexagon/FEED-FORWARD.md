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

### 01 — `.agents` history (pending)

- Archive path:
- Gate stages:
- Test files removed with parity:

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
