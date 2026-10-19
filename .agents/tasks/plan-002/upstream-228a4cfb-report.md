# Upstream sync report — 228a4cfb..dcd9fdeb (desktop-mode probe + X11 dev libs)

**Batch:** `4c31561a..dcd9fdeb` (2 upstream commits)
**HWM:** `4c31561a` → `dcd9fdeb9aacbba08209918620dd0ec4e7c56a7a`
**Base:** post-02c3 HEAD (`30b4d18e`)
**Status:** forked review PASS; committed.
**Direction:** upstream → fork

## Triage applied

| Upstream path | Disposition | Fork action (applied) |
|---|---|---|
| `crates/oh-app/src/desktop_mode.rs` (new) | reimplemented | Added verbatim from `git show 228a4cfb`; the two `dispatch::run` doc references adapted to "the tiny binary's top-of-`main` hook (Issue 07)". Probe/parse/gate tests kept (7 lib tests). |
| `crates/oh-app/src/lib.rs` | integrated | `pub mod desktop_mode;` (near `compose`/`draw_target`) + the upstream module comment + a crate-doc bullet for the module list. |
| `crates/oh-app/src/lifecycle.rs` | reimplemented | `DesktopModeSource` enum (`Fixed(VideoMode)` / `Lazy`) added; `load_and_reapply` now takes `source`; the `Lazy` arm probes via `crate::desktop_mode::wants_desktop_mode` / `probe_desktop_mode` only when an auto flag is on (zero-mode fallback otherwise). Merged into the 02c1-stripped lifecycle (no save guards — none were added). Doc references to the ported `main_client` / `oh-app` `android::run` retargeted to Issue 07's tiny `main` (`Lazy`) and Issue 08's Android entry (`Fixed(zero)`). All 4 existing tests moved to `DesktopModeSource::Fixed(...)`; new `load_and_reapply_lazy_with_both_auto_flags_off_keeps_defaults` added. |
| `crates/oh-render/src/types.rs` | integrated | The one-`EventLoop` doc paragraph on `VideoMode::get_desktop_mode`, byte-identical to the upstream hunk (matching git index hashes `78502857..01bd8fbf`). |
| `tools/ci-deps.sh` | integrated (both commits) | `xvfb`, `libgl1-mesa-dri`, `libegl1`, `libegl-mesa0` (228a4cfb) + `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libx11-dev`, `libxcursor-dev`, `libxrandr-dev`, `libxi-dev` (dcd9fdeb) — comment block, guarded presence checks, and the final "all prerequisites present" echo, per the upstream end-state. |
| `tools/smoke-windowed.sh` (new) | reimplemented/adapted | Added from the upstream commit; default binary `${CARGO_TARGET_DIR:-target}/debug/tinyhexagon` (usage + `BIN` line); header note that Issues 07/08 wire it into the gate/package smoke and that the assertions target the tiny runtime's frame loop; sandbox-rationale line reworded (the fork's game persists no `Profiles/`/`Replays/` — 02c1). Display/xvfb skip logic kept verbatim. `bash -n` clean; executable bit set. NOT added to the four-stage gate (no binary yet). |
| `crates/oh-app/src/client.rs` | not-applicable | Adult client deleted (02a). Issue 07's tiny loop calls the hook + `Lazy`. |
| `crates/oh-app/src/dispatch.rs` | not-applicable | Adult dispatch deleted. Issue 07's tiny `main` calls `run_probe_child_if_requested`. |
| `crates/oh-app/src/android.rs` | not-applicable | Adult Android entry is a stub (02a). Issue 08 wires `DesktopModeSource::Fixed(zero)`. |
| `tools/gate.sh` | deferred | Four-stage gate kept as-is; the windowed stage lands in Issue 07/08 once the `tinyhexagon` binary exists. |
| `tools/package.sh` | deferred | The windowed package smoke lands with the tiny binary (07/08). |

## Changelog / HWM / feed-forward

- `.agents/skills/upstream/changelog.md`: marker moved to
  `<!-- high-water-mark: dcd9fdeb9aacbba08209918620dd0ec4e7c56a7a -->`; two
  entries prepended, newest first — `dcd9fdeb` (disposition `integrated`,
  mapping `tools/ci-deps.sh`, `Fork commit: this commit`) and `228a4cfb`
  (disposition `reimplemented (partial; front-end files not-applicable)`,
  mapping per file, `Fork commit: this commit`, notes listing the
  not-applicable/deferred files + the adaptations). The baseline entry is
  untouched.
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md`:
  - §07: the tiny `main` calls `run_probe_child_if_requested(&argv)` at the
    top; `load_and_reapply` is called with `DesktopModeSource::Lazy`
    (windowed + headless branch); the probe child is the only
    `VideoMode::get_desktop_mode` caller.
  - §08: the Android entry passes `DesktopModeSource::Fixed(zero)`; the
    windowed smoke/gate/package stage lands once the binary exists; the CI
    display packages (xvfb + Mesa EGL/DRI + the X11 -dev libs) are already
    in `tools/ci-deps.sh`.

## Verification

```
$ cargo test -p oh-app desktop_mode   # 7 tests (4 parse + 3 gate) — ok
$ cargo test -p oh-app lifecycle      # 5 tests (4 adapted + 1 new Lazy) — ok
$ bash tools/gate.sh; echo "gate_rc=$?"
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
# 59 test suites: 509 passed, 0 failed (02c3 tail was 501 — +7 desktop_mode, +1 lifecycle Lazy)

$ bash tools/upstream-status.sh --no-fetch --check; echo "hwm_rc=$?"
HWM:      dcd9fdeb9aacbba08209918620dd0ec4e7c56a7a
upstream: upstream/master @ dcd9fdeb9aacbba08209918620dd0ec4e7c56a7a
commits:  0

check: current — 0 upstream commits since HWM
hwm_rc=0

$ rg -n 'DesktopModeSource|run_probe_child_if_requested|wants_desktop_mode|probe_desktop_mode' crates/oh-app/src
# hits: lifecycle.rs (enum, signature, Lazy arm, all 5 tests) + desktop_mode.rs (the module)
$ rg -n 'xvfb|libgl1-mesa-dri|libegl1|libegl-mesa0' tools/ci-deps.sh
# comment block + 4 guarded checks + the all-present echo (the 6 X11 -dev checks ride along)
$ bash -n tools/ci-deps.sh && bash -n tools/smoke-windowed.sh   # both clean
```

The windowed smoke is NOT run (no `tinyhexagon` binary yet) — noted for
Issues 07/08. `desktop_mode.rs` differs from the upstream source only in
the two `dispatch::run` → "the tiny binary's top-of-`main` hook (Issue 07)"
doc adaptations (verified by `diff` against `git show 228a4cfb:...`);
`smoke-windowed.sh` differs only in the binary name + the fork header note.

## Changed files

- `crates/oh-app/src/desktop_mode.rs` (new)
- `crates/oh-app/src/lib.rs`
- `crates/oh-app/src/lifecycle.rs`
- `crates/oh-render/src/types.rs`
- `tools/ci-deps.sh`
- `tools/smoke-windowed.sh` (new)
- `.agents/skills/upstream/changelog.md`
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md`

Committed by the orchestrator after the forked `VERDICT: PASS`.

## Review P2 (07/08)

`tools/smoke-windowed.sh` still asserts the old `[::mainClient] Finished`
string; the tiny runtime will not print it, so that half is inert until Issues
07/08 adapt the script. The GL-context/probe assertions remain meaningful; the
script is not in the four-stage gate or the package smoke yet.
