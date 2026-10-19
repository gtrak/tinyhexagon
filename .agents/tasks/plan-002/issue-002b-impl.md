# Issue 02b — remove the retired front-end / online crates

**Umbrella:** `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`
(02a app headless is landed at `b7718ed6`; 02c de-scopes the engine).
Do not de-scope `oh-game`/`oh-lua`, do not remove `oh-replay`/`oh-discord`,
do not rename anything.

## Goal

Delete the crates that the tiny runtime no longer uses now that `oh-app` is
headless, and remove the last `oh-menu` coupling in `oh-app`:

- `crates/oh-menu`
- `crates/oh-menu-system`
- `crates/oh-additions`
- `crates/oh-net`
- `crates/oh-steam`
- `crates/oh-server-control`
- `crates/oh-workshop-uploader`

Keep `oh-replay` and `oh-discord` until 02c (the engine still uses them).
The workspace must build, clippy and test green.

## Read first

- `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md` (umbrella)
- `.agents/tasks/plan-002/issue-002a-report.md` (keep-list; the 02b handoff)
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` §02a
- root `Cargo.toml`, `crates/oh-app/Cargo.toml`,
  `crates/oh-app/src/joystick.rs`,
  `crates/oh-app/src/{compose,bridge,lifecycle}.rs`

## Steps

1. Remove `"crates/oh-menu"`, `"crates/oh-menu-system"`,
   `"crates/oh-additions"`, `"crates/oh-net"`, `"crates/oh-steam"`,
   `"crates/oh-server-control"`, `"crates/oh-workshop-uploader"` from the
   root `Cargo.toml` members and delete the seven directories.
2. `oh-app`:
   - `src/joystick.rs` imports `oh_menu::seams::MenuJoystick` and implements
     `RealMenuJoystick`. Remove that menu-side view/import (the engine's
     `Joystick` seam and `RealJoystick` stay).
   - Remove the `oh-menu = { path = ... }` dependency from
     `crates/oh-app/Cargo.toml` and update/remove comments that name it as a
     live dependency.
   - Check `compose.rs`/`bridge.rs`/`lifecycle.rs` for `oh_menu`,
     `oh_additions`, `oh_net`, `oh_steam`, `oh_server_control`,
     `oh_workshop_uploader` code references; remove/adapt them. Historical
     doc comments may stay (Issue 09 sweep) but no live import/dependency.
3. Regenerate `Cargo.lock` (`cargo build --workspace`); the removed crates'
   entries should disappear.
4. Run the full gate.

Do not delete `_RELEASE`, `tools/`, the packaging scripts, or the
`oh-android`/`oh-web` stubs.

## Verification

```sh
ls crates
rg -n 'oh-menu|oh-menu-system|oh-additions|oh-net|oh-steam|oh-server-control|oh-workshop-uploader' \
  Cargo.toml crates --glob 'Cargo.toml' --glob '*.rs' || echo no-live-refs
rg -n 'oh_menu|oh_additions|oh_net|oh_steam|oh_server_control|oh_workshop_uploader' \
  crates/oh-app/src || echo app-clean
bash tools/gate.sh; echo "gate_rc=$?"
```

`ls crates` should show the seven removed directories absent and
`oh-replay`/`oh-discord` still present. The gate must be 4/4 green.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-002b-report.md` with:

- The crates removed and the `joystick` app change.
- The lockfile delta.
- The gate result/tail.
- Any doc-comment residue left for 02c/09.
- "No commit made."

Update `FEED-FORWARD.md` with a `### 02b — front-end/online crates (LANDED)`
entry: removed crates, the joystick change, remaining `oh-replay`/`oh-discord`
consumers (for 02c), and the gate result. Do not mark 02b LANDED in
`PLAN.md`; the orchestrator does that at commit.

## Risks

- Do not remove `oh-replay`/`oh-discord` yet; `oh-game`/`oh-app` compose still
  use them until 02c.
- Do not remove the engine's replay/profile/Steam/Discord code here.
- `oh-app`'s comments may still mention the retired crates; that is doc-only,
  but no dependency/import may remain.
- The `oh-android`/`oh-web` stubs must keep compiling; they do not depend on
  the removed crates.
