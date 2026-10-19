# Issue 02c3 report — final engine cleanup (fingerprint, render replay/PB fields)

**Umbrella:** `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`
(02a/02b/02c1/02c2 landed). **Status:** forked review PASS; committed with
issue 02c3. The worker timed out at the very end; the orchestrator re-ran the
gate and committed.

## 1. Fingerprint removal (030b)

Removed, with no game-behaviour change (the path was off by default; the
normal update/draw order is untouched):

- `oh-game/src/hexagon_game.rs`: the `EngineState` fields
  `fingerprint_file` / `fingerprint_frame` / `fingerprint_last_draws`,
  their ctor init, the per-`new_game` reset block, `set_fingerprint_path`,
  and the "Hooks (… / fingerprint)" section header.
- `oh-game/src/update.rs`: the end-of-`update` `maybe_emit_fingerprint()`
  call, `maybe_emit_fingerprint` itself (the whole S1–S15 field assembly),
  and the FNV-1a `FingerprintBuilder`.
- `oh-game/src/lua_console.rs`: the test stand-in's fingerprint init.
- 030b-only accessors (no other caller once the fingerprint went):
  - `SpeedData::min()` / `max()` / `ping_pong()` (the private fields stay —
    `update()` uses them);
  - `CCustomWallManager::alive_handles()` (`temp_alive_handles` stays — the
    live `handle_collision` uses it);
  - `CWall::get_hue_mod()` (`set_hue_mod` + the field stay — `draw()` uses
    the field; the setter is the C++ mirror).
  Genuinely general accessors (`get_speed`, `get_curve`,
  `get_vertex_positions`) were kept.
- `oh-rng`: `state_position()` (wrapper + `Pcg32Fast`), `distance_from_wrapped`,
  the wrapper's `draws` field + `draws()` — parity-only, and after the
  fingerprint removal they had no non-test caller. Kept: `engine_state()`
  (now the K7 test's pin — re-documented as such) and `Pcg32Fast::state()`
  (the C++ `engine()` mirror, not 030b).
- No fingerprint-only tests existed to remove (the path was only
  exercisable through the now-deleted accessor surface).

## 2. `oh-render` replay/PB fields + `oh-app` bridge initializers

- `graphics.rs`: removed `DrawState.replay_icon` / `replay_text` /
  `pb_text` / `pb_text_growth` and `FrameInput.must_show_replay_ui`;
  `init_key_icons` / `update_key_icons` lost the replay-icon + replay-texture
  parameters (and the replay-icon layout block); `draw_key_icons` lost the
  `must_show_replay_ui` parameter + the replay-icon draw; `draw_frame` lost
  the `|| must_show_replay_ui` conjuncts on the key-icon / level-info gates
  and the pass-throughs into `draw_text` / `draw_key_icons`.
- `text_draw.rs`: removed the PB growth sawtooth, the PB sizing, the replay
  text content/size, `draw_text_personal_best`, and the
  `must_show_replay_ui` parameters through `update_text` /
  `update_text_sizes` / `draw_text_time_and_status` / `draw_text`; the
  `TextUpdateInput` replay fields (`must_show_replay_ui`,
  `replay_played_*`, `replay_player_name`) and — dead after the sawtooth
  removal, mirroring the engine's 02c1 `mFT` drop — the `ft` field. The
  surviving time/status/fps/message sizing is bit-identical (same
  `getScaledCharacterSize` calls, same gates).
- `oh-app/src/bridge.rs`: removed the `replay_icon` / `replay_text` /
  `pb_text` / `pb_text_growth` initializers, the `replay_text.set_string("")`
  copy, the neutral `replay` `Texture`, the `must_show_replay_ui: false`
  `FrameInput` field, and the replay arguments at the `init_key_icons` /
  `update_key_icons` call sites; `update_text_sizes` is now the 2-arg form.
- `oh-app/tests/bridge.rs`: the PB/replay size assertions and the
  `replay_icon` texture assertions removed; the surviving time/status/fps/
  message, pass-order, frametime, and texture assertions are untouched.
- `oh-render` tests: PB/replay cases removed (the PB pulse test, the
  replay-UI arm of `draw_key_icons_opacities` — replaced by an on-colour
  re-assertion so the opacity test still covers both states — and the
  replay-icon layout assertions in `key_icon_layout_matches_constants`);
  everything else unchanged.

## 3. `go_to_menu` coverage (keep + cover)

The path is live in the tiny runtime: the `oh-app` bridge's Lippincott
`onRenderStage` / `run_lua_file` error paths call `go_to_menu(false, true)`,
and the C++-facing callers are already pinned (`score_override_error.rs`
for the error form, `joystick.rs` scenario F for the Exit-edge
`go_to_menu(true, false)`). It is NOT dead, so it was kept and covered:

- NEW `crates/oh-game/tests/go_to_menu_callback.rs` (2 tests): arms
  `fn_go_to_menu` (windowed — the C++ head-gate skips the callback
  headless), then
  1. `trigger_go_to_menu()` (the C++ hardcoded-Escape / `Tid::Exit` bind
     body → `goToMenu(true, false)`) → callback fires with `mError = false`;
  2. `go_to_menu(false, true)` (the exact Lippincott form) → callback fires
     with `mError = true`.

Stale go_to_menu docs fixed while there: the `GoToMenuCallback` alias doc
(referenced the removed `oh_app::windowed` module and described atomic-signal
wiring that does not exist in the tree) and the `go_to_menu` method doc
("unset in this port" — false; hosts/tests do set it).

## 4. `oh-assets` dead profile surface — REMOVED

No live `oh-game`/`oh-app` caller remained (the only consumers were
`oh-assets/tests/profiles.rs` itself), so the surface was removed rather
than deferred:

- The `Profiles/` constructor load step (step 5; non-fatal, so the load
  order is unchanged), `load_all_local_profiles` / `reload_local_profiles`
  / `add_local_profile` / `profile_file_path` / `profile_file_contents`,
  the `ProfileStore` struct + `profiles` field, and the public methods
  `any_local_profile_active` / `current_local_profile_name` /
  `get_local_score` / `set_local_score` / `is_level_favorite` /
  `get_favorite_level_ids` / `add_favorite_level` /
  `remove_favorite_level` / `profile_save_current` / `profile_save_all` /
  `profile_create` / `profile_remove` / `profile_set_current` /
  `get_profile_data` / `get_local_profiles_size` / `get_profile_names`;
  `oh-assets/tests/profiles.rs` (9 tests) deleted.
- Config/level/pack data and everything else untouched.
- Spec "keep" note: the spec's keep-list (`profile_data` field,
  `get_highscore`, `save_scores`, `get_game_version`) refers to methods
  that do not exist in this tree — the C++'s singular `profileData`
  high-score store was never ported into `oh-assets` (the PB effect that
  read it was retired in 02c1). `oh-data::ProfileData` (the scores/
  favorites model) stays in `oh-data`, available if a later issue ports a
  menu profile UI.

## 5. Doc residue

- `oh-app/src/compose.rs`: the `ReplayStore` seam parenthetical trimmed to
  the one-line history.
- `oh-game/tests/render_hook_rng_stream.rs`: the `set_last_replay` note now
  reads "the C++'s `m_lastReplay` member (the Lua variable `lastReplay`;
  replay removed by plan 002 issue 02c2, the member kept as the `l_replay`
  API target)"; the `state_position()`/`draws()` instrumentation notes were
  rewritten for the raw-state pin.
- `oh-game/tests/common/mod.rs`: the two helper docs that cited the
  removed 029m score-send tests now cite the actual users.
- The `oh-render` replay/PB FIELD docs went with the fields; the
  "swept in 02c3" inert-field notes in `bridge.rs` / `tests/bridge.rs` /
  `text_draw.rs` were deleted as the fields left. Some method-level docs
  still mention the removed UI (`mustShowReplayUI()` in `graphics.rs`,
  `drawTextMessagePBImpl` in `text_draw.rs`); Issue 09 sweeps those.
- `oh-game/src/hexagon_game.rs`: the `go_to_menu` docs (see §3).

## 6. K7 test adaptation (pure simulation test, kept)

`render_hook_rng_stream.rs` now pins the stream through the raw MCG state
(`engine_state()`) instead of the removed `state_position()`/`draws()`:
the pre-hook position check, the one-bump post-hook check (via a fresh
one-draw oracle), the final state equality, and the control-run state
equality each pin the same position/attribution the label + draw counter
used to pin (the value assertions — hook values, side-selection value,
cross-run divergence — are unchanged). No assertion was weakened: the raw
state is a bijection on the stream position the label measured, and the
total-bump counts the draw counter measured are pinned by the final-state
equalities plus the per-frame value checks.

## 7. Tests

- Removed: 9 (`oh-assets/tests/profiles.rs`), 2 (`oh-rng`
  `state_position_matches_cpp_probe` / `state_position_is_affine_in_bumps`),
  1 (`oh-render` `update_text_sizes_pb_pulse_follows_the_growth_sawtooth`;
  the `fps_and_replay_gates` test became `fps_gate_keeps_the_previous_size`
  without the replay arm).
- Added: 2 (`go_to_menu_callback.rs`).
- Net: 511 → 501 passed.

## 8. Gate

`bash tools/gate.sh` — all four stages green (re-run after the final doc
edits):

```
== cargo fmt --all -- --check
GATE_EXIT=0
== cargo build --workspace
GATE_EXIT=0
== cargo clippy --workspace --all-targets -- -D warnings
GATE_EXIT=0
== cargo test --workspace
GATE_EXIT=0
```

501 passed / 0 failed across 59 suites. Spec verification greps:
`rg 'fingerprint' crates/oh-game/src` → none;
`rg 'must_show_replay_ui|replay_icon|replay_text|pb_text|pb_text_growth'
crates/oh-render/src crates/oh-app/src` → none;
`rg 'profile_save_all|profile_set_current|ProfileSaveGuard'
crates/oh-game/src crates/oh-app/src crates/oh-assets/src` → none.

## 9. Deviations / residue

- Nothing renamed (Issue 03's job), as instructed. The one test RENAMED
  (`update_text_sizes_fps_and_replay_gates_keep_the_previous_sizes` →
  `update_text_sizes_fps_gate_keeps_the_previous_size`) is a test whose
  name described a now-removed arm; flag it if the no-rename rule is read
  to cover test names.
- The pre-existing worktree modification to
  `.agents/tasks/plan-002/upstream-228a4cfb-impl.md` (the orchestrator's
  post-02c3 upstream-sync prep) was left untouched.
Committed by the orchestrator after the forked `VERDICT: PASS`.

### Review P2 residue (Issue 09)

- Dead PB-particle update surface in `oh-render`:
  `PBParticleUpdateParams`, the `must_spawn_pb_particles` /
  `next_pb_particle_spawn` spawn branch, `make_pb_particle`, its export and
  test. No `oh-app`/`oh-game` caller remains.
- Dead Steam workshop seam in `oh-assets`: `WorkshopPacks`,
  `new_with_workshop`, and docs naming the removed `oh_steam::SteamManager`.
  No live caller.
- Stale replay/PB/steam doc references listed by the reviewer (Issue 09
  sweep).
