**STATUS: DEFERRED (2026-10-19).** Upstream is still advancing past
`228a4cfb`; do NOT land this single-commit sync yet. The HWM stays at
`4c31561a`. When the operator says go, re-run `tools/upstream-status.sh` and
retriage the FULL `HWM..upstream/master` batch; this file is the draft for the
first commit and its mapping table is the starting point.

# Upstream sync — 228a4cfb (desktop-mode probe / winit one EventLoop)

**HWM before:** `4c31561ac6409e24a1b838043a0bb747f504045d`
**Upstream HEAD:** `228a4cfb6690dae58d9a49dd2e412a132791cbe7`
**Direction:** upstream → fork (advance the HWM in the landing commit).
**Base:** apply after Issue 02c2 (replay removal) lands; do not run a second
writer on the same worktree before then.

## What the commit does

The windowed desktop build exited after one frame because
`VideoMode::get_desktop_mode()` built and ran a winit `EventLoop`, and winit
0.30 allows one per process; the game window's later `EventLoop::new()`
returned `RecreationAttempt`. The fix:

- load config first; probe the desktop mode only when
  `windowed_auto_resolution` / `fullscreen_auto_resolution` is on;
- run the winit probe in a child process (`--oh-desktop-mode-probe`), with a
  bounded wait and the zero-mode fallback;
- add a windowed smoke script + CI display packages;
- add an `oh-render` doc note.

## Triage (this fork)

| Upstream path | Disposition | Fork action |
|---|---|---|
| `crates/oh-app/src/desktop_mode.rs` (new) | reimplement | Add it; adapt the `dispatch::run` doc reference to the future tiny entry (Issue 07). Keep the probe/parse/gate tests. |
| `crates/oh-app/src/lib.rs` | integrate | `pub mod desktop_mode;` |
| `crates/oh-app/src/lifecycle.rs` | reimplement | Add `DesktopModeSource` and the `Lazy` gate to `load_and_reapply`; keep our stripped lifecycle (no save guards). Update tests. |
| `crates/oh-render/src/types.rs` | integrate | The one-`EventLoop` doc paragraph. |
| `tools/ci-deps.sh` | integrate | Add `xvfb`, `libgl1-mesa-dri`, `libegl1`, `libegl-mesa0`. |
| `tools/smoke-windowed.sh` (new) | reimplement/adapt | Add the script for Issues 07/08; adapt the default binary to `tinyhexagon` and note the tiny loop's assertions. Do NOT add it to the four-stage gate yet (no binary). |
| `crates/oh-app/src/client.rs` | not-applicable | The adult client is deleted (02a); Issue 07's tiny loop will call the hook/`Lazy`. |
| `crates/oh-app/src/dispatch.rs` | not-applicable | The adult dispatch is deleted; Issue 07's tiny main calls `run_probe_child_if_requested`. |
| `crates/oh-app/src/android.rs` | not-applicable | The adult Android entry is a stub (02a); Issue 08 wires `DesktopModeSource::Fixed(zero)`. |
| `tools/gate.sh` | deferred | Keep the four-stage gate; add a windowed stage in Issue 07/08 once the `tinyhexagon` binary exists. |
| `tools/package.sh` | deferred | The windowed package smoke lands with the tiny binary (07/08). |

The HWM may advance to `228a4cfb` only when the reimplemented pieces land;
record the not-applicable/deferred items in the changelog entry.

## Steps

1. Add `crates/oh-app/src/desktop_mode.rs` (copy the upstream file; adapt the
   doc references from `dispatch::run` to "the tiny binary's main hook
   (Issue 07)"; keep the tests).
2. `crates/oh-app/src/lib.rs`: add `pub mod desktop_mode;` (near `compose`/
   `draw_target`).
3. `crates/oh-app/src/lifecycle.rs`: add the `DesktopModeSource` enum and
   change `load_and_reapply` to take `source`; implement the `Lazy` arm with
   `wants_desktop_mode`/`probe_desktop_mode`; update all tests/callers to
   `DesktopModeSource::Fixed(...)` or `Lazy`. Merge with the 02c1-stripped
   lifecycle (no `ConfigSaveGuard`/`ProfileSaveGuard`).
4. `crates/oh-render/src/types.rs`: add the one-`EventLoop` doc paragraph to
   `VideoMode::get_desktop_mode`.
5. `tools/ci-deps.sh`: add the four display packages and the presence checks.
6. Add `tools/smoke-windowed.sh` from the upstream commit; change the default
   binary to `${CARGO_TARGET_DIR:-target}/debug/tinyhexagon`, add a header note
   that Issues 07/08 wire it into the gate/package smoke, and keep its
   display/xvfb skip logic. Do not add the gate stage now.
7. Do not add the `client`/`dispatch`/`android` diffs; the tiny entry will use
   the hook. Do not add the windowed gate/package stages yet.
8. Update `.agents/skills/upstream/changelog.md`:
   - Prepend the `228a4cfb` entry (date, subject, area, disposition
     `reimplemented (partial; front-end files not-applicable)`, mapping,
     `Fork commit: this commit (plan 002 upstream sync)`, notes listing the
     not-applicable/deferred files).
   - Move the top marker to
     `<!-- high-water-mark: 228a4cfb6690dae58d9a49dd2e412a132791cbe7 -->`.
   - Leave the baseline entry.
9. Update `.agents/plans/002-tinyhexagon/FEED-FORWARD.md`:
   - §07: the tiny main calls `run_probe_child_if_requested(&argv)` at the top;
     `load_and_reapply` uses `DesktopModeSource::Lazy`; the probe child is the
     only `get_desktop_mode` caller.
   - §08: the windowed smoke/gate/package stage lands once the binary exists;
     the CI display packages are already in `tools/ci-deps.sh`.
10. Run the full gate (four stages) and the new `desktop_mode`/`lifecycle`
    tests.

## Verification

```sh
cargo test -p oh-app desktop_mode
cargo test -p oh-app lifecycle
bash tools/gate.sh; echo "gate_rc=$?"
bash tools/upstream-status.sh --no-fetch --check; echo "hwm_rc=$?"   # 0 after the marker move
rg -n 'DesktopModeSource|run_probe_child_if_requested|wants_desktop_mode|probe_desktop_mode' crates/oh-app/src
rg -n 'xvfb|libgl1-mesa-dri|libegl1|libegl-mesa0' tools/ci-deps.sh
```

The windowed smoke is not run (no binary yet); note it as 07/08.

## Report

Write `.agents/tasks/plan-002/upstream-228a4cfb-report.md` with the triage
table applied, the changelog/HWM update, the gate output, and "No commit
made." The orchestrator commits and the changelog's `Fork commit` can be
updated to the commit SHA in a follow-up if desired.
