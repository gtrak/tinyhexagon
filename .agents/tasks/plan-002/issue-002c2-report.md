# Issue 02c2 report — remove replay from the engine and app

**Umbrella:** `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`
(02a/02b/02c1 landed). **Status:** forked review PASS; committed with
issue 02c2. The worker finished the implementation and the full gate, then hit
its 30-minute wall-clock timeout while writing this report; the orchestrator
wrote this report from the run log and independently re-ran the gate.

## Final signatures

`crates/oh-game/src/hexagon_game.rs` (8 args; `replay_store` gone):

```rust
pub fn new(
    config: Arc<RwLock<Config>>,
    assets: Arc<oh_assets::Assets>,
    audio: Option<Arc<dyn Audio + 'static>>,
    script_host: Arc<dyn ScriptHost + 'static>,
    joystick: Arc<dyn Joystick + 'static>,
    cosmetic_rng: Arc<dyn CosmeticRng + 'static>,
    window: bool,
    renderer: Option<Arc<dyn Renderer + 'static>>,
) -> Self
```

`new_game(pack_id, id, first_play, difficulty_mult)` — the
`execute_last_replay` argument and the replay-name fallbacks are gone.

`crates/oh-app/src/compose.rs` (7 args; `replay_store` gone):

```rust
pub fn compose_with(assets, root, window) -> Composed
pub fn compose_with_config(
    assets, root, window, config,
    renderer: Option<Arc<dyn Renderer + 'static>>,
    audio: Option<Arc<dyn Audio + 'static>>,
    joystick: Arc<dyn Joystick + 'static>,
) -> Composed
```

## Removed

- `crates/oh-replay` (whole crate: 35 tests — 33 in `tests/oracle.rs`, 2 in
  the lib unit tests) and its root workspace member; `oh-game`/`oh-app` deps.
  `Cargo.lock` regenerated; the now-unused `flate2`/`tempfile` workspace pins
  were removed (the pins were `oh-replay`'s only consumers).
- `oh-game/src/seams.rs`: `ReplayStore`, `NullReplayStore`,
  `CompressedReplayFile`.
- `hexagon_game.rs`: `ReplayFile`/`ReplayData`/`CompressedReplayFile`/
  `ActiveReplay`, the replay cursor, `last_replay_data`, `last_first_play`,
  `last_played_score`, `active_replay`, `replay_store`, `set_last_replay`,
  `in_replay`, `must_replay_input`, `must_show_replay_ui`,
  `active_replay_file`, `set_on_death_replay_created`,
  `run_replay_until_death_and_get_score`, `save_replay`,
  `death_create_replay_file`, `death_send_and_save_replay`, and the
  `execute_last_replay` `new_game` branches.
- `update.rs`: the `InputBits` import, the replay-input branch, the
  recording call, and the replay-state checks. The normal input/update order
  is unchanged.
- `lua_console.rs`: `last_replay_data` and replay references.
- `lua_state.rs`: `GameApi::in_replay` (+ the mock impls).
- `status.rs`: replay-only fields/accessors.
- `oh-app`: the replay-seeded composition/test half; `replay_store` args;
  the replay icon PNG load; the engine-side replay text/UI. `FrameInput`
  still carries the `oh-render` `must_show_replay_ui: false` field and
  `AppDraw` still initializes the inert `replay_icon`/`replay_text` — the
  `oh-render::graphics::DrawState` fields are 02c3's sweep (same pattern as
  the 02c1 PB fields). No replay is ever drawn or recorded.

## Fingerprint

The 030b fingerprint path (`fingerprint_file`/`fingerprint_frame`/
`fingerprint_last_draws`, OFF by default) remains for 02c3. Its
replay-sourced `last_played_score` byte was removed with the replay state;
the fingerprint line is therefore one field shorter. This is acceptable
because the fingerprint is a parity-only, off-by-default debug path that
02c3 deletes; no game behaviour depends on it.

## Tests

Removed / adapted (no surviving assertion weakened):

- `oh-replay`: 35 tests deleted with the crate.
- `oh-game/tests/go_to_menu_score.rs` (2 tests) — both drive
  `ReplayStore::save`/`CompressedReplayFile`; the replay store they observe
  no longer exists. The live `go_to_menu`/`send_scores` path remains but is
  now uncovered; 02c3/07 must either remove the dead path or add a
  non-replay assertion.
- `oh-game/tests/determinism.rs`: 4 replay-execution/serialization tests
  removed (7 → 3); the pure determinism/reuse tests remain. Oracle basis
  change: the D-019 survivor now injects `SEED = 0` directly — exact
  equality is intact, but it is self-consistent, not C++-oracle-verified.
- `oh-app/tests/bridge.rs`: the replay-seeded determinism test now runs TWO
  live games with the seed injected directly into `state_handle().rng`
  (`SEED = 0`, `alwaysSpinRight`); the pass-order/frametime/texture
  assertions are unchanged and still 7 bridge tests.
- Constructor call sites updated across `oh-game`/`oh-app` tests,
  `tests/common`, `examples/pack_shaders_gl.rs`, `pack_shaders.rs`.
- `oh-lua/tests/api.rs` mock `in_replay` removed.

Test totals: 552 → 511 passed (the 41 removed = 35 `oh-replay` + 2
`go_to_menu_score` + 4 replay-determinism tests), 0 failed, 1 pre-existing
ignored.

## Gate

`bash tools/gate.sh` (orchestrator re-run):

```
== cargo fmt --all -- --check        GATE_EXIT=0
== cargo build --workspace           GATE_EXIT=0
== cargo clippy --workspace --all-targets -- -D warnings  GATE_EXIT=0
== cargo test --workspace            GATE_EXIT=0
gate_rc=0   (511 passed / 0 failed / 1 ignored)
```

## Deviations / residue

1. **`oh-render` replay UI fields** (`DrawState.replay_icon`,
   `replay_text`, `must_show_replay_ui`) and the two inert bridge
   initializers remain until 02c3, exactly like the 02c1 PB fields. 02c3
   must remove the `oh-render` replay + PB fields and the bridge
   initializers together.
2. Doc-comment residue: `oh-app/src/compose.rs` still mentions the retired
   `ReplayStore` seam in a doc string; `oh-game/tests/render_hook_rng_stream.rs`
   mentions `set_last_replay` in a doc note; `oh-render`'s replay field
   docs. 02c3/09 sweep.
3. The worker timed out before writing this report; no code was left
   uncommitted in a red state. Orchestrator re-ran the gate.

Committed by the orchestrator after the forked `VERDICT: PASS`.
