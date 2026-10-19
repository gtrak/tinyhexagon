# Issue 02c2 — remove replay from the engine and app

**Umbrella:** `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`
(02a/02b/02c1 landed). This is the largest remaining de-scope.
**Fingerprint stays until 02c3**; do not remove its path. If the fingerprint
serialization reads a replay field that disappears, replace that field with a
neutral constant and record it; 02c3 deletes the fingerprint.

## Goal

Remove replay record/playback and the `oh-replay` crate:

- `oh-game`: the replay types/fields/methods, `ReplayStore` seam, replay
  input/recording branches, replay UI state, replay-save/death paths.
- `oh-lua`: `GameApi::in_replay` (the steam API that used it is already gone).
- `oh-app`: the replay UI in `bridge.rs`, the `replay_store` arg in the
  composition, the replay-seeded test half.
- Delete `crates/oh-replay`; remove it from the workspace and all deps.
- Keep pure simulation/determinism tests; delete replay tests.

## Read first

1. Umbrella + `.agents/tasks/plan-002/issue-002c1-report.md` (signatures,
   fallbacks, 02c2 handoff)
2. `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` §02c1
3. `crates/oh-game/src/{hexagon_game,update,seams,lua_state,lua_console,status}.rs`
4. `crates/oh-lua/src/api/{mod,utils}.rs` and `GameApi` callers
5. `crates/oh-app/src/{compose,bridge}.rs`, `tests/bridge.rs`
6. `crates/oh-replay/` and `crates/oh-game/tests/replay*`

## Steps

### 1. Engine types and fields

- `seams.rs`: remove `ReplayStore`, `NullReplayStore`, `CompressedReplayFile`
  and their docs.
- `hexagon_game.rs`: remove `ReplayFile`, `ReplayData`, `CompressedReplayFile`,
  `ActiveReplay`, the replay cursor, `last_replay_data`, `last_first_play`,
  `last_played_score`, `active_replay`, the `replay_store` field/arg,
  `set_last_replay`, `in_replay`, `must_replay_input`, `must_show_replay_ui`,
  `active_replay_file`, `set_on_death_replay_created`,
  `run_replay_until_death_and_get_score`, `save_replay`,
  `death_create_replay_file`, `death_send_and_save_replay`, and the
  `execute_last_replay` argument/branches in `new_game`. Remove the replay
  UI state/accessors (`replay_text`, replay icon state) and the replay-save
  gate. Remove the 02c1 replay-name fallbacks with the paths.
- `update.rs`: remove the `InputBits` import, the replay-input branch, the
  recording call, and the replay-state checks. Keep the normal input/update
  order; do not reorder the surviving simulation.
- `lua_console.rs`: remove `last_replay_data` and replay references.
- `status.rs`: remove replay-only fields/accessors (keep score/custom score).
- `lua_state.rs`: remove `GameApi::in_replay` and its test mock impl.
- If the fingerprint serialization reads `last_played_score`, replace it with
  a neutral `0.0` (or the current score) and record it; 02c3 removes the
  fingerprint.

### 2. Lua

- Remove `GameApi::in_replay`; `oh-lua` has no live caller after the steam API
  deletion. Remove `in_replay` from any mock/test impl.
- Leave replay-related doc comments for the 02c3/09 sweep; no live code.

### 3. App composition

- `compose.rs`: remove the `replay_store` arg and `NullReplayStore` import
  from `compose_with`/`compose_with_config`; update the engine constructor
  call. `HexagonGame::new` loses `replay_store` (9 → 8 args);
  `compose_with_config` loses it (8 → 7 args).
- `bridge.rs`: remove `replay_icon`, `replay_text`, `must_show_replay_ui`,
  the `replayIcon.png` load, the replay-text draw/update, and
  `game.replay_text()`.
- `tests/bridge.rs`: drop the replay-seeded determinism half; keep the
  pass-order/frametime/texture assertions unweakened. `tests/common` prune.
- `Cargo.toml`: remove `oh-replay` from `oh-game`/`oh-app`.

### 4. Delete the crate

- Remove `crates/oh-replay` and the root workspace member; delete its tests
  (`oh-game/tests/replay_*`, `oh-app/tests/headless_replay.rs` already gone).
- Regenerate `Cargo.lock`; remove `flate2`/`tempfile` workspace pins if
  `oh-replay` was their only consumer (check `rg 'flate2|tempfile' crates`).

### 5. Tests

- Delete replay tests and replay helpers; keep pure simulation, score,
  update-order and determinism tests. Do not weaken surviving assertions.
- Update every `HexagonGame::new` / `compose_with*` call site.

### 6. Gate

```sh
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tools/gate.sh
```

## Verification

```sh
test ! -d crates/oh-replay && echo replay-crate-gone
rg -n 'oh-replay|ReplayStore|NullReplayStore|ActiveReplay|CompressedReplayFile|in_replay|must_show_replay_ui|last_replay_data|set_last_replay|save_replay' \
  Cargo.toml crates --glob 'Cargo.toml' --glob '*.rs' || echo replay-clean
rg -n 'replay' crates/oh-app/src || echo app-replay-clean
rg -n 'fingerprint' crates/oh-game/src | head   # must still exist (02c3)
bash tools/gate.sh; echo "gate_rc=$?"
```

Doc-comment residue may remain (02c3/09 sweep); no live import/dependency or
replay branch may remain. The `fingerprint` path must still be present.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-002c2-report.md` with:

- The final `HexagonGame::new` / `compose_with_config` signatures.
- Removed replay types/fields/methods/branches and tests.
- The fingerprint field adaptation (if any) and RNG/update-order note.
- The `oh-replay` removal, lock delta, `flate2`/`tempfile` decision.
- Gate result/tail and "No commit made."

Update `FEED-FORWARD.md` with `### 02c2 — replay (LANDED)`: signatures,
removed replay surface, any fingerprint adaptation, 02c3 handoff, gate.
Do not mark 02c2 LANDED in `PLAN.md`; the orchestrator does that at commit.

## Risks

- Replay is threaded through `hexagon_game.rs`; remove the record/playback
  branches only and do not reorder the normal update/input path.
- The fingerprint path stays until 02c3; adapt any replay-sourced field
  minimally and record it.
- `oh-app` bridge replay UI removal must not disturb the remaining text/
  texture/pass-order assertions.
- `flate2`/`tempfile` may have other consumers; remove a workspace pin only
  if `rg` proves none remain.
