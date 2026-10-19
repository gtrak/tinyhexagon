<!-- high-water-mark: dcd9fdeb9aacbba08209918620dd0ec4e7c56a7a -->

# Upstream changelog (tinyhexagon ↔ gtrak/SSVOpenHexagon Rust port)

The **most recent upstream SHA below is the high-water mark (HWM)**. The
marker line (first line of this file) is its single source of truth and is
parsed by `tools/upstream-status.sh`. Upstream → fork entries are listed
newest-first; the HWM advances only after every commit from the old HWM up to
the new SHA has been triaged (see `SKILL.md`).

## 2026-10-19 — dcd9fdeb — fix(ci): install the winit X11 -dev libs the windowed smoke dlopens

- Area: CI (`tools`)
- Disposition: integrated
- Mapping: `tools/ci-deps.sh` → `tools/ci-deps.sh`
- Fork commit: this commit
- Notes: integrated into the same `tools/ci-deps.sh` update as `228a4cfb` (per
  the sync spec). Adds `libxkbcommon-dev`, `libxkbcommon-x11-dev`,
  `libx11-dev`, `libxcursor-dev`, `libxrandr-dev`, `libxi-dev` (guarded
  `dpkg -s` presence checks) — winit's X11 backend dlopens unversioned `.so`
  names (xkbcommon-dl: `libxkbcommon-x11.so`), which only the `-dev` packages
  provide; upstream gate run 39283564865 (job 118163145033) died in
  xkbcommon-dl without them. The windowed smoke that consumes these packages
  lands with the `tinyhexagon` binary (Issues 07/08); the packages are in
  place now.

## 2026-10-19 — 228a4cfb — fix(windowed): the desktop-mode probe no longer eats winit's one EventLoop

- Area: `oh-app` (lifecycle), `oh-render` (docs), `tools`
- Disposition: reimplemented (partial; front-end files not-applicable)
- Mapping: `crates/oh-app/src/desktop_mode.rs` → `crates/oh-app/src/desktop_mode.rs` (new);
  `crates/oh-app/src/lifecycle.rs` → same (`DesktopModeSource`);
  `crates/oh-app/src/lib.rs` → same (`pub mod desktop_mode;`);
  `crates/oh-render/src/types.rs` → same (one-`EventLoop` doc on
  `VideoMode::get_desktop_mode`); `tools/ci-deps.sh` → same (xvfb + Mesa
  EGL/DRI; the `dcd9fdeb` X11 -dev libs integrated into the same update);
  `tools/smoke-windowed.sh` → `tools/smoke-windowed.sh` (new, adapted)
- Fork commit: this commit
- Notes: the probe/parse/gate tests are kept verbatim (7 `desktop_mode` lib
  tests) and the `lifecycle` tests move to `DesktopModeSource` (+ new
  `load_and_reapply_lazy_with_both_auto_flags_off_keeps_defaults`). Adapted:
  the `desktop_mode.rs` doc references to `dispatch::run` now point at
  "the tiny binary's main hook (Issue 07)"; `lifecycle.rs` references to the
  ported `main_client` / `oh-app` `android::run` now point at Issue 07's
  tiny `main` (`DesktopModeSource::Lazy`) and Issue 08's Android entry
  (`DesktopModeSource::Fixed(zero)`); `tools/smoke-windowed.sh` defaults to
  `${CARGO_TARGET_DIR:-target}/debug/tinyhexagon` with a header note that
  Issues 07/08 wire it into the gate/package smoke. Not-applicable (fork
  files deleted/stubbed by 02a): `crates/oh-app/src/client.rs` (Issue 07's
  tiny loop calls the hook + `Lazy`), `crates/oh-app/src/dispatch.rs`
  (Issue 07's tiny `main` calls `run_probe_child_if_requested`),
  `crates/oh-app/src/android.rs` (stub; Issue 08 wires
  `DesktopModeSource::Fixed(zero)`). Deferred: `tools/gate.sh` (the windowed
  gate stage lands in Issues 07/08 once the `tinyhexagon` binary exists) and
  `tools/package.sh` (the windowed package smoke lands with the tiny binary).
  The windowed smoke is not run in this sync (no binary yet).

## 2026-10-19 — 4c31561a — land(058): the menu's mouse surface, ported site-by-site — and the first-mouse-frame divergence the measurement caught

- Area: baseline (whole upstream repository)
- Disposition: baseline (integrated — fork point)
- Mapping: n/a
- Fork commit: 4c31561ac6409e24a1b838043a0bb747f504045d
- Notes: verified at plan 002 dispatch — `upstream/master` == `HEAD` ==
  `git merge-base HEAD upstream/master` (delta 0).

## Entry format (upstream → fork)

One entry per triaged upstream commit, newest-first, above this block:

```
## <date> — <sha> — <subject>
- Area: …
- Disposition: integrated | reimplemented | deferred | not-applicable
- Mapping: upstream path/crate → fork path/crate (from SKILL.md)
- Fork commit: <sha>
- Notes: …
```

`deferred` entries must name the tracking issue; `not-applicable` entries
state the reason.

## Fork → upstream

- (no entries yet)
