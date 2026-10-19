# HISTORY — from the Open Hexagon Rust port to tinyhexagon

This file is the direction summary of the engineering record that preceded
the tinyhexagon fork. The full record is archived, unedited, at
[`.agents/plans/archive/001-rust-port/`](plans/archive/001-rust-port/) (plan,
`DECISIONS.md` D-000…D-039, `PARITY.md`, `deviations.md` F1…F37, the
`mapping/` transcription files, the `parity/` corpus, the `audits/`, the
`evidence/`, the `workflows/`, and every port-era task spec under
`tasks/` — including the pre-plan-002 review protocol,
`tasks/review-gate-port.md`). Git history is the complete source; this page
orients the new work.

## What the port proved

The port was a **literal 1:1 Rust port of SSVOpenHexagon** (the ~42k-line
C++20 Open Hexagon by Vittorio Romeo): file/struct/function structure
mirrored, `float`→`f32`/`double`→`f64` verbatim, every constant copied
(D-000). Verification was by evidence, not by review:

- **Frame-level parity against the instrumented C++.** The C++ reference was
  built in a reproducible container (`tools/cpp-oracle/`) with a minimal,
  off-by-default `-fingerprint` hook emitting a per-frame hash of game state
  (RNG, walls, player, score, custom score, status timers, entities) plus
  the draw count. **3627 frames across 6 corpus cases** were committed
  (`parity/cpp_fingerprints.json`) and re-checked on every gate run.
- **RNG/replay byte-exactness.** `tools/parity.sh` compiled the *real* C++
  sources (`RandomNumberGenerator.cpp`, `Replay.cpp`, `BuildPackId.cpp`,
  `LevelValidator.cpp`) against minimal shims to byte-derive the corpus:
  bit-exact PCG32 streams (7 seeds × int/f32/f64), byte-exact replay
  serialization (zlib level 9, the `sf::Packet` codec), pack ids and level
  validators for all 8 shipped packs, and 4 pinned level-run pairs — with
  deliberately-wrong-pin discrimination so the harness itself was tested.
- **A 43-case Lua A/B corpus** pinned the scripting layer; since the Luau
  unification (D-037) the port shipped a single VM (Luau, every platform)
  with a 5.1-compat prelude, the C++ column remaining ground truth.
- **A real-GPU render stack** (D-011): `winit` 0.30.13 + `glutin` 0.32.3 +
  `glow` 0.18.0, rodio/cpal audio, `fontdue` text — headless-tested with
  GPU tests skipped by design.
- **Android on real devices** (issues 31/34: touch overlay, boot-to-playback,
  NDK r27c pin) and an **emscripten build of the whole app**
  (`wasm32-unknown-emscripten`, D-028/D-036), deployed continuously to
  gh-pages.
- **Packaging and CI for four targets**: Linux, Windows (cross-built),
  Android APK, web — with a per-push gate (fmt / build / clippy `-D
  warnings` / test / Lua parity / additions-boundary ratchet) and release
  workflows producing named artifacts.

Every deliberate divergence from the C++ was registered in
`deviations.md` (F1…F37); Rust sources cite the C++ as `file:line` and
unresolvable citations were defects (D-014). 58+ issues landed under that
discipline.

## What carries forward into tinyhexagon

- The **engine simulation** (`oh-game` → `th-game`): walls, player, status,
  update loop, the fingerprint-free determinism the corpus proved.
- The **Lua content system** (`oh-lua` → `th-lua`): the Luau runtime, the
  game API, the 5.1-compat prelude — custom levels and custom walls stay
  Lua scripts.
- **`oh-render` → `th-render`** and the platform entries: the
  winit/glutin/glow stack, rodio/cpal audio, the Android (`oh-android` →
  `th-android`) and emscripten (`oh-web` → `th-web`) entries.
- **`_RELEASE/`** (assets, packs, configs) and the **packaging/CI** tooling
  (`tools/package*.sh`, `build-windows.sh`, `ci-deps.sh`, `check-android.sh`).
- The `oh-*` → `th-*` crate rename lands in plan 002 Issue 03.

## What is retired

- **The C++ tree and the oracle**: `cpp-original/` (with its submodules and
  the `src`/`include` citation symlinks), `tools/cpp-oracle/`,
  `tools/oracle/` — deleted by plan 002 Issue 01. Git history keeps them.
- **The parity corpus and tooling**: `crates/oh-parity`, `tools/parity.sh`,
  `tools/lua-parity.sh` — deleted by Issue 01; the gate no longer has a
  parity stage.
- **The adult product surfaces**: menus, online leaderboards, Steam,
  Discord, workshop upload, the score server and the replay codec —
  `oh-menu`, `oh-menu-system`, `oh-net`, `oh-server-control`, `oh-steam`,
  `oh-discord`, `oh-workshop-uploader`, `oh-replay` go in Issue 02.
- **The additions ratchet** (`tools/check-additions-boundary.sh` and the
  boundary baseline, retired by Issue 01; `oh-additions` itself goes in
  Issue 02/04) and `docs/` (superseded by generated music).

## The tinyhexagon direction

One game: **no menus, no text, no scores, no death, no persistence.** Hold
left/right to rotate; fullscreen and reset are the only other controls; on
touch, screen halves plus two corner icons. Collisions are soft — the wall
bursts, the screen flashes, the world slows, the music ducks, and play
continues. A run escalates over 20 stages × 30 s (10 minutes), then holds.
Music is **generated** by `rust-synth` (8 tracks, stage mapping, beat
view); colours respond to the music, and the music responds to stages and
collisions. Linux, Windows, Android and web are first-class and packaged
as `tinyhexagon-*`.

The full spec, decisions and issue index:
[`.agents/plans/002-tinyhexagon/PLAN.md`](plans/002-tinyhexagon/PLAN.md).

## Upstream relationship

The fork moved off the original Rust port at SHA
`4c31561ac6409e24a1b838043a0bb747f504045d` (2026-10-19) and now moves
independently. The relationship is tracked manually: the
[upstream skill](skills/upstream/SKILL.md) documents the fork ↔ upstream
workflows and the crate/module mapping, and
[`skills/upstream/changelog.md`](skills/upstream/changelog.md) carries the
high-water mark (its top SHA — the most recent upstream commit fully
triaged into the fork). `tools/upstream-status.sh` prints the delta; the
HWM advances only in a "pull changes from upstream" commit.

## Pointers

- Archived record: `.agents/plans/archive/001-rust-port/` (this issue's
  moves; nothing in the archive was edited).
- Git history: the port landed through `4c31561a`; plan 002 starts at
  `3369fffa`. The pre-plan-002 tree is fully reconstructible (the C++
  tree, submodules and tooling included).
- Active plan: `.agents/plans/002-tinyhexagon/PLAN.md` (+ `FEED-FORWARD.md`).
