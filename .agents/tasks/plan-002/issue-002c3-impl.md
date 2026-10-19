# Issue 02c3 — final engine cleanup: fingerprint, render replay/PB fields

**Umbrella:** `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`
(02a/02b/02c1/02c2 landed). This is the last engine cleanup before Issue 03
(rename).

## Goal

Remove the remaining parity-only and replay/PB render surface:

- the 030b fingerprint path in `oh-game` (off by default, no longer useful
  without replay parity);
- the `oh-render` `DrawState`/`FrameInput` replay and PB fields
  (`replay_icon`, `replay_text`, `pb_text`, `pb_text_growth`,
  `must_show_replay_ui`) and every draw/update path that uses them;
- the inert `oh-app` bridge initializers and `must_show_replay_ui: false`
  (the 02c1/02c2 accepted deviations);
- the dead `oh-assets` profile surface, if it has no live caller (otherwise
  record it for Issue 09);
- the `go_to_menu`/score-send coverage gap: keep the live path and add a
  non-replay assertion, or remove the path if it is genuinely dead (record).

Do not rename anything; Issue 03 does that.

## Read first

1. Umbrella + `.agents/tasks/plan-002/issue-002c1-report.md` /
   `issue-002c2-report.md`
2. `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` §02c1/§02c2
3. `crates/oh-game/src/{hexagon_game,update,lua_console,speed_data,custom_wall_manager,wall}.rs`
4. `crates/oh-render/src/{graphics,text_draw}.rs`
5. `crates/oh-app/src/bridge.rs`, `tests/bridge.rs`
6. `crates/oh-assets/src/assets.rs` (profile surface)

## Steps

### 1. Fingerprint removal

- `hexagon_game.rs`: remove `fingerprint_file`, `fingerprint_frame`,
  `fingerprint_last_draws`, their init/reset, `set_fingerprint_path`, the
  fingerprint serialization/state, and the `maybe_emit_fingerprint` call.
- `update.rs`: remove `maybe_emit_fingerprint`, the FNV-1a field builder and
  the fingerprint output. Keep the normal update/draw order.
- `lua_console.rs`: remove the test stand-in fingerprint fields.
- `speed_data.rs` / `custom_wall_manager.rs` / `wall.rs`: remove the
  030b-only accessors/comments if no other caller uses them; keep genuinely
  general accessors.
- Remove fingerprint-only tests; keep pure simulation/determinism tests.
- `oh-rng`: remove a parity-only state accessor only if no non-test caller
  uses it; otherwise leave it.

### 2. `oh-render` replay/PB fields

- `graphics.rs`: remove `DrawState.replay_icon`, `replay_text`, `pb_text`,
  `pb_text_growth`; remove `FrameInput.must_show_replay_ui`; remove the
  `init_key_icons` replay-icon parameters and the replay-icon texture; update
  every draw call/condition that reads the removed fields.
- `text_draw.rs`: remove the PB growth/sizing/draw path, the replay text
  sizing/draw path, and the `must_show_replay_ui` parameters/field. Keep the
  surviving score/status/message text sizes exactly as-is.
- `oh-app/src/bridge.rs`: remove the `replay_icon`/`replay_text`/
  `pb_text`/`pb_text_growth` initializers, the `replay_text.set_string("")`,
  `must_show_replay_ui: false`, and the replay/PB draw/update code; update
  the `init_key_icons` call.
- `oh-app/tests/bridge.rs`: remove the PB/replay size assertions; keep the
  pass-order/frametime/texture/other text assertions unweakened.
- `oh-render` tests: remove PB/replay cases; keep the rest.

### 3. `go_to_menu` coverage

The `go_to_menu`/`fn_go_to_menu` path is still live (`bridge.rs`'s screenshot
error and the composition callback); the deleted `go_to_menu_score` tests
only observed the replay store. Keep the path and add a non-replay test: set
`fn_go_to_menu`, call `trigger_go_to_menu`/`go_to_menu`, assert the callback
fires with the error flag. If inspection shows the path is genuinely dead in
the tiny runtime, remove it and record why; do not leave it uncovered.

### 4. `oh-assets` dead profile surface (attempt; record if deferred)

- If no live `oh-game`/`oh-app` caller remains, remove the `Profiles/` load
  step and the public profile read/write methods (`profile_save_all`,
  `profile_set_current`, `get_local_profiles_size`, favorites/score accessors)
  plus their tests. Keep config/level/pack data.
- If removing the profile data model would ripple into unrelated modules,
  leave the dead surface and record it for Issue 09; do not force it.

### 5. Doc residue

Fix live-looking stale comments in the files touched here (the
`oh-app/compose.rs` `ReplayStore` doc, `oh-game/tests/render_hook_rng_stream.rs`'s
`set_last_replay` note, and any `oh-render` replay/PB field docs). Deeper
historical citations stay for the Issue 09 sweep.

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
rg -n 'fingerprint' crates/oh-game/src || echo no-fingerprint
rg -n 'must_show_replay_ui|replay_icon|replay_text|pb_text|pb_text_growth' \
  crates/oh-render/src crates/oh-app/src || echo no-replay-pb-render
rg -n 'profile_save_all|profile_set_current|ProfileSaveGuard' \
  crates/oh-game/src crates/oh-app/src crates/oh-assets/src || echo no-profile-surface
bash tools/gate.sh; echo "gate_rc=$?"
```

Doc-comment history may remain; no live field/path may remain. If the
`oh-assets` profile surface is deferred, record it here explicitly.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-002c3-report.md` with:

- The fingerprint removal, the render replay/PB field removal, the
  `go_to_menu` test/removal decision, the `oh-assets` profile decision.
- Removed/adapted tests and the gate result/tail.
- "No commit made."

Update `FEED-FORWARD.md` with `### 02c3 — final engine cleanup (LANDED)`:
the removed surfaces, the `go_to_menu` coverage, the profile decision, and
the note that Issue 03 (rename) is next. Do not mark 02c3 LANDED in
`PLAN.md`; the orchestrator does that at commit.

## Risks

- The `oh-render` text sizing is pinned by bridge tests; remove only the
  PB/replay branches and keep the surviving score/status/message sizes
  bit-identical.
- The fingerprint path is off by default but its removal changes no game
  behaviour; keep the update order.
- `oh-assets` profile removal may touch the data model; defer rather than
  break unrelated data.
- The `go_to_menu` path may be dead in the tiny runtime; either cover it or
  remove it, but do not leave it silently untested.
