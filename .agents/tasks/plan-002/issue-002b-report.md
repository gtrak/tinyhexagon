# Issue 002b report — remove the retired front-end / online crates

Worker: plan 002, issue 02b (`.agents/tasks/plan-002/issue-002b-impl.md`).
Umbrella: `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`.
HWM at dispatch: `b7718ed60c9d7a70d71f313f5a90a30eed2c8d4d` (02a landed).

## Result

The seven retired crates are gone from the workspace, the last `oh-menu`
coupling is out of `oh-app`, the lockfile is regenerated, and the full
gate is green (4/4 stages; 569 tests passed / 0 failed / 0 ignored).
`oh-replay`/`oh-discord` were NOT removed (02c); `oh-game`/`oh-lua`
were not de-scoped; nothing was renamed; no engine file was edited.
The orchestrator commits this report with issue 02b.

## Crates removed

Deleted directories + root `Cargo.toml` members: `crates/oh-menu`,
`crates/oh-menu-system`, `crates/oh-additions`, `crates/oh-net`,
`crates/oh-steam`, `crates/oh-server-control`,
`crates/oh-workshop-uploader`. `ls crates` now shows the 14 surviving
crates; `oh-replay`/`oh-discord` still present.

The only live dependency on the seven was `oh-app -> oh-menu` (the
joystick's `MenuJoystick` view); the rest were intra-set
(`oh-additions`/`oh-menu` -> `oh-menu`, `oh-menu` -> `oh-menu-system`,
`oh-server-control`/`oh-steam` -> `oh-net`). No kept crate had a
dependency on any of the seven (verified by `rg` over all
`crates/**/Cargo.toml` before deletion).

Also removed from the root `Cargo.toml`'s `[workspace.dependencies]`:
the `steamworks`, `rusqlite` and `sodiumoxide` pins — the deleted
crates were their only consumers (grep-proven), and their comments
cited the deleted crates/paths. The `tempfile` pin STAYS (oh-replay's
29m1 bounded-read dev-dep uses it); its now-stale issue-17 comment was
retargeted to oh-replay. `flate2` stays (oh-replay). The members-list
history comment had the four removed names stripped.

## `oh-app` changes

- `Cargo.toml`: the `oh-menu = { path = ... }` dependency and its
  comment block are removed; the gilrs target-table comment now says
  the Android/wasm targets keep the no-op `NoJoystick` seam (no more
  `NoMenuJoystick`).
- `src/joystick.rs`: the menu-side view is stripped —
  `use oh_menu::seams::MenuJoystick;`, the `RealMenuJoystick`
  struct + `#[derive(Clone)]`, `impl MenuJoystick for
  RealMenuJoystick`, and `impl RealMenuJoystick` are deleted, along
  with the two inherent helpers that lost their only caller
  (`pressed_focus` — the menu's `Jid::Focus` `focusHeld` query;
  `rising_jdir` — the menu's `risingEdge(Jdir)`). The engine's
  `Joystick` seam impl on `RealJoystick` (update / pressed_dir /
  pressed_id / rising_edge_id / vendor_id) is UNCHANGED, as is the
  gilrs backend and the `oh_config::joystick` machine feeding (the
  engine side 02a/02c keep). Module/struct/`unsafe impl Send+Sync`
  docs re-pointed from "both seams" to the engine-only seam.
- `compose.rs`/`bridge.rs`/`lifecycle.rs`: no live code references to
  any retired crate (only doc comments — see residue below). No
  signature changes anywhere (the engine's `steam`/`discord`/
  `profiles`/`replay_store` seams stay until 02c).

## Lockfile delta

`Cargo.lock`: **189 lines deleted, 0 added**. Gone: the seven crate
entries + their exclusive transitive deps — `steamworks`,
`steamworks-sys`, `sodiumoxide`, `libsodium-sys`, `rusqlite`,
`libsqlite3-sys`, `hashlink`, `fallible-iterator`,
`fallible-streaming-iterator`, `ed25519`, `signature`, `paste`.
`hashbrown` is NOT removed (still used by `fontdue`/`indexmap`). No
orphaned third-party entries remain (checked: none of the removed
families still appear in the lock).

## Verification (spec block)

```sh
ls crates   -> 14 crates; the seven absent; oh-replay/oh-discord present
rg -n 'oh-menu|oh-menu-system|oh-additions|oh-net|oh-steam|oh-server-control|oh-workshop-uploader' \
  Cargo.toml crates --glob 'Cargo.toml' --glob '*.rs'
  -> doc-comment hits ONLY (list below); no live dep/import
rg -n 'oh_menu|oh_additions|oh_net|oh_steam|oh_server_control|oh_workshop_uploader' \
  crates/oh-app/src
  -> compose.rs:535 `oh_steam::SteamManager` (doc; the steam seam is
     live until 02c)
bash tools/gate.sh; echo "gate_rc=$?"  -> gate_rc=0
```

## Gate result (full, un-piped)

```
== cargo fmt --all -- --check
GATE_EXIT=0
== cargo build --workspace
GATE_EXIT=0
== cargo clippy --workspace --all-targets -- -D warnings
GATE_EXIT=0
== cargo test --workspace
GATE_EXIT=0
log: target/gate.log
gate_rc=0
```

`cargo test --workspace`: 68 suites, all `test result: ok`;
**569 passed / 0 failed / 0 ignored** (02a was 108 suites / 897 —
the 40 removed suites / 328 tests belonged to the seven crates:
oh-menu's 16 integration files + unit/doc tests, oh-menu-system
`navigation`, oh-net `database`/`wire`, oh-server-control
`loopback`, oh-steam `manager`/`workshop_cache`/`workshop_packs_seam`,
oh-workshop-uploader `uploader`). `oh-app` contribution unchanged:
lib unit tests (camera/lifecycle/sigint), `tests/bridge.rs` (7),
`tests/pack_shaders.rs` (9), Doc-tests — none weakened.

## Doc-comment residue left for 02c/09

No live import/dependency on any removed crate remains; the
spec-permitted historical doc comments:

- `oh_steam::SteamManager` mentions (the `SteamSink` seam lives until
  02c): `oh-game/src/seams.rs:455,468`, `oh-game/src/hexagon_game.rs:498`,
  `oh-app/src/compose.rs:535` (the `steam` arg's doc).
- `oh-steam` workshop-surface docs in `oh-assets/src/assets.rs`
  (lines 27, 78–89, 351, 364) — 02c's Steam de-scope territory.
- `oh-workshop-uploader` pattern citations in `oh-discord` docs
  (`src/manager.rs`, `src/core.rs`, `src/no_discord.rs`,
  `tests/manager.rs`) — kept crate, doc-only.
- "Salvaged from `oh-additions`" notes: `oh-app/src/lib.rs`,
  `oh-app/src/web_fullscreen.rs`, `oh-app/Cargo.toml` (web-sys
  comment); `oh-render/src/texture.rs:552` (the touch-overlay history
  of `load_texture_or_null`).
- `tools/package.sh` / `tools/build-windows.sh` / `.github` still name
  the retired binaries (packaging/CI sweep is Issue 08/09).

## Not run here (environment, not code)

Emscripten / Android target compiles — no emsdk/NDK on this box; the
`oh-android`/`oh-web` stubs are untouched by 02b (they do not depend
on any removed crate). Host green is the bar; Issue 08 re-checks the
targets (as in 02a).

## Working-tree note

At dispatch the working tree already carried uncommitted
orchestrator edits to `PLAN.md` and `02-remove-crates-engine.md`
(the 02a/02b/02c re-scope: 02b = the seven crates, 02c = the engine
de-scope). They pre-date this issue, match its spec, and were left
in place for the orchestrator's commit.

Committed by the orchestrator after the forked `VERDICT: PASS`.
