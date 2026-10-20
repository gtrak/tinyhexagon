# Issue 08c — packaging/CI/release names (report)

**Worktree:** slot-1 @ `2deca9c4` (master). No commits (the orchestrator
commits). All changes are scripts + workflows + shell/docs — the only
crate touch is `th-web/Cargo.toml` packaging-metadata comments (+ two
probe-reference doc/comment updates in `th-web` — `site/index.html` and
`src/web.rs`). No game/runtime code touched.

## Final artifact names

| Target | Artifact | Produced by |
|---|---|---|
| Linux x86_64 | `tinyhexagon-<tag>-linux-x86_64.tar.gz` | `tools/package.sh --tag <tag>` (the tar is made by the script, at the repo root) |
| Windows x86_64 | `tinyhexagon-<tag>-windows-x86_64.zip` | `tools/package.sh --windows --tag <tag>` |
| Android arm64-v8a | `tinyhexagon-<tag>-android-arm64-v8a.apk` | `tools/package-android.sh` (08b's script, unchanged) + release.yml's rename step |
| Web | `tinyhexagon-<tag>-web.zip` | `tools/package-web.sh` (inner `tinyhexagon-web.zip`, unchanged) + release.yml's rename step |

- The Linux tar / Windows zip contain the `tinyhexagon` binary INSIDE
  the staged `_RELEASE/` directory (`_RELEASE/tinyhexagon[.exe]`
  alongside `Assets/`, `Packs/`, `ConfigOverrides/`, `config.json` —
  the C++ install-root layout `resolve_game_root` walks; verified: the
  tar is 266 entries, `_RELEASE/tinyhexagon` is an ELF x86-64 PIE,
  `Packs/tiny/` is present).
- `<tag>` resolution in `tools/package.sh`: `--tag <t>` / `--tag=<t>` →
  `$ASSET_TAG` (release.yml's workflow env, already normalized
  filesystem-safe) → `dev`. A tag containing `/` or empty is refused.
- Web serve dir / gh-pages contract unchanged: `index.html` +
  `tinyhexagon.js` + `tinyhexagon.wasm` + `tinyhexagon.data` at the
  zip root / branch root; page import `./tinyhexagon.js`; the live page
  is `https://gtrak.github.io/tinyhexagon/` (fork's gh-pages, ci.yml
  pages job + release.yml gh-pages job — both unchanged in behaviour,
  both carry the `github.repository == 'gtrak/tinyhexagon'` fork guard
  where applicable).

## What changed

### 1. Probe consolidation (one tiny probe)

- `tools/web-probe.mjs`, `tools/web-stub-probe.mjs`,
  `tools/web-fs-probe.mjs`, `tools/web-persist-probe.mjs` — DELETED.
  The adult-app IME / persistence / fullscreen-UX probes have no tiny
  twin (no text entry, no persistence, no adult menu geometry).
- `tools/web-tiny-probe.mjs` — NEW, the one probe of the real game
  (evolved from the 08a stub probe, which was already the real-game
  probe):
  - polls `window.ohStatus()` / `#status` for the real status line
    `booted=true;ticks=...;stage=...;started=...;angle=...;audio=...;energy=...;help=...`
    (waits on `booted=true;started=true`, fast-fails on
    `booted=false;error=...`);
  - baseline: `stage` integer ≥ 0, `angle` / `energy` finite,
    `audio` ∈ {live, fallback}, `help=auto`;
  - first gesture (one held `A` key, the page's keydown fires
    `resumeAudio()`): asserts **`audio=live`** (the 10-19 Chrome 148
    measurement — the headless CDP synthetic keydown grants user
    activation), `help=off`, `ticks` advanced, `angle` turned
    counter-clockwise, the `[shell] audio unlock requested` log;
  - clean console (no errors / exceptions; the `/favicon.ico` 404 +
    the boot's "AudioContext was not allowed to start" autoplay
    warning are the documented tolerations), no 4xx/5xx, no failed
    loads (favicon / canceled aborts tolerated), a `<canvas>` with
    non-zero size, title / `ohStatus` / `#status` contract, screenshot
    (`--shot=PATH`, default `target/web-tiny.png`, `--timeout=SECS`
    default 120, same per-PID CDP port + per-run profile + Chrome-kill
    hygiene as before).
- References updated: `.github/workflows/ci.yml` (the wasm job's
  browser-smoke step + the screenshot artifact `web-tiny-screenshot` /
  `target/web-tiny.png` + the file header), `crates/th-web/site/
  index.html` (2 comment refs), `crates/th-web/src/web.rs` (1 doc ref).
- Left as-is: `crates/th-render/src/audio.rs`'s doc comment still
  names the retired `tools/web-probe.mjs` (th-render is 08a/09's lane
  per the 08c constraint — a one-line historical reference, not a
  build input); `tools/fixtures/cube_1_apeirogon.ohr.z` KEPT (the
  plan-001 smoke fixture — the 08c task forbids deleting it); the
  `oh-lanes` external-lane dir in `build-wasm-emsdk.sh`'s podman
  example (03's documented exception — not a repo path).

### 2. `tools/package.sh` (rewritten)

- The whole Linux/Windows package in ONE script: the release build →
  the `_RELEASE` staging (a pristine copy of the tracked `_RELEASE` +
  the binary inside, `target/package/linux-x86_64/_RELEASE` /
  `target/package/windows-x86_64/_RELEASE`) → the smoke → the TAGGED
  tar.gz / zip at the repo root.
- The retired adult replay-fixture headless smoke is gone (the tiny
  runtime has no `-headless`/replay mode). The new smoke is the
  PACKAGED binary run WINDOWED through `tools/smoke-windowed.sh`
  (phases A/B/C: the `--th-desktop-mode-probe` child, the
  shipped-config boot, the auto-resolution reapply — real GL context,
  no one-frame headless fallback, still in the loop at the timeout).
  Linux only; the Windows cross build is smoke-skipped-with-reason on
  a Linux host (no .exe runtime; covered by the windows-latest CI gate
  + a manual run). `--skip-smoke` kept; `--with-steam` + the
  Steam-native staging is removed (the tiny build has no Steam); the
  pristine-copy invariant (`git status --porcelain -- _RELEASE` must be
  clean) is kept.
- zip step: `zip` when present (CI), python3 `zipfile` fallback
  (this box has no `zip` — the fallback ran in the verification).

### 3. Workflows

- `release.yml` — the `linux` job's three steps (build+stage+smoke,
  tar, upload) are now `bash tools/package.sh --tag "${ASSET_TAG}"` +
  the upload; the `windows` job's three (cross build+stage, zip,
  upload) are `bash tools/package.sh --windows --tag "${ASSET_TAG}"` +
  the upload. The `android` / `wasm` / `release` / `gh-pages` jobs are
  UNCHANGED (they already produced / consumed the
  `tinyhexagon-<tag>-*` names). Job IDs, edges, secrets, the
  `ASSET_TAG` normalization steps, the `if-no-files-found: error`
  uploads, and the mingw-vs-MSVC header rationale all kept.
- `ci.yml` — the `wasm` job's browser-smoke step now runs
  `node tools/web-tiny-probe.mjs` (step renamed "the tiny entry's
  headless-Chrome CDP probe"; the comment updated — the probe asserts
  the status contract incl. the `audio=live` flip, not the retired
  "no activation" wording) and uploads `web-tiny-screenshot`
  (`target/web-tiny.png`). Gate matrix / windows-cross / android /
  pages jobs untouched.
- Both workflows: YAML-parsed + actionlint-checked (below). One real
  bug caught pre-check: a `: ` inside an unquoted step `name:` scalar
  in release.yml (invalid YAML) — fixed by rewording.

### 4. Host bin collision (the known P2) — resolved minimally

- `cargo build -p th-web` on the host: **builds** (the host main is
  the no-op; deps are emscripten-target-gated).
- `cargo build --workspace` (the gate's stage 2): both bins named
  `tinyhexagon` (th-app's real one + th-web's no-op) → cargo prints a
  NON-FATAL `warning: output filename collision at
  target/debug/tinyhexagon[.dwp]`; the build SUCCEEDS (gate stage 2
  GATE_EXIT=0) and clippy `-D warnings` does not trip on it (it is a
  cargo-level warning, not a rustc lint).
- Resolution per the task's decision tree: it builds ⇒ a clear comment
  suffices. `th-web/Cargo.toml` now carries the documentation: the bin
  name IS the artifact name (tinyhexagon.js/.wasm/.data), the
  emscripten build is always `-p th-web` (no companion bin), the host
  `--workspace` collision is an expected cosmetic warning, and the bin
  must NOT be renamed (the artifact names are what it buys). No
  gating / path rename needed.

### 5. Stale comments

- `tools/ci-deps.sh`: the xvfb bullet said "the gate's windowed smoke …
  the desktop build" — now "the windowed smoke … the package smoke
  tools/package.sh runs in the release linux job … the tinyhexagon
  build".
- `tools/smoke-windowed.sh`: header said "NOT wired into
  tools/gate.sh or tools/package.sh yet: … lands in Issues 07/08" —
  now "IS wired into tools/package.sh (the release linux job's package
  smoke — 08c); NOT a tools/gate.sh stage".
- `.cargo/config.toml`: checked — the emscripten section's
  `tinyhexagon.js`/`tinyhexagon.wasm` artifact names and the 08a boot
  references are current; the only `stub` mention is the historical
  "the 03c stub dropped the page's persistence glue" fact (accurate as
  history). No change needed.
- `tools/build-windows.sh` / `check-android.sh` /
  `package-android.sh` / `package-web.sh` / `build-wasm-emsdk.sh`:
  checked — all name the `tinyhexagon` artifacts / `th-app` +
  `th-android` already (08b's lane did the Android half); no
  SSVOpenHexagon/open-hexagon/oh- residue in any tools script except
  the documented exceptions (`upstream-status.sh`'s upstream remote is
  `gtrak/SSVOpenHexagon` — the actual upstream C++ repo — and the
  `oh-lanes` external lane dir above).

## Verified

- **Gate**: `bash tools/gate.sh` 4/4 green, **570 passed / 0 failed /
  0 ignored** (the expected baseline; nothing in `crates/` changed
  behaviour — the th-web changes are comments/doc-refs). The
  collision warning is visible in `target/gate.log` (stage 2) as a
  warning with GATE_EXIT=0.
- **Package smoke (Linux)**: `bash tools/package.sh` (tag default
  `dev`) — release workspace build, staging, `tools/smoke-windowed.sh`
  against the staged binary (phase A `OH_DESKTOP_MODE=1280x720`, phase
  B shipped-config boot, phase C auto-resolution reapply — all OK),
  tar produced (`tinyhexagon-dev-linux-x86_64.tar.gz`, 4.8 MB, 266
  entries: `_RELEASE/tinyhexagon` ELF x86-64 PIE + the full data
  including `Packs/tiny/`), pristine-copy invariant clean.
- **Probe (browser execution)**: `node tools/web-tiny-probe.mjs`
  against the staged `target/package/web/_WEB` (the 08a release
  artifacts — no emscripten toolchain on this host, so the wasm build
  is 08a's, unchanged by 08c; serving on a clean port 18321):
  **21/21 PASS** — baseline `booted=true;ticks=11;stage=0;started=
  true;angle=0.0000;audio=fallback;energy=0.0000;help=auto` →
  post-gesture `audio=live;help=off;angle=-12.1226`, clean console,
  screenshot `.agents/evidence/08c/web-tiny-probe.png`. (The first
  local run on port 8080 failed measuring a STALE unrelated
  filebrowser service squatting on the port — the lane-058 failure
  mode; re-run on a free port: PASS. CI uses a clean runner + its own
  http.server, where this cannot happen.)
- **Windows cross-check**: `bash tools/build-windows.sh
  --check --all-targets` (the ci.yml windows-cross job's exact
  command) exit 0.
- **actionlint v1.7.7**: both workflows, rc 0, no findings.
- **YAML**: both workflows `yaml.safe_load`-clean (the check that
  caught the step-name colon bug).

Evidence: `.agents/evidence/08c/` — `gate.log` (stage rcs + the 570/0
aggregate + the collision-warning lines), `package.log` (the
package.sh run tail: smoke phases + layout + invariant),
`package-tar-listing.txt` (the full 266-entry tar listing),
`web-tiny-probe.log` + `web-tiny-probe.png` (the 21/21 run + the
screenshot), `actionlint.txt`, `win-cross-check.log`.

## Deferred / open

- **Full Windows release cross-BUILD** (`package.sh --windows` end to
  end) not run locally — the check passed, the build path is the
  pre-existing `build-windows.sh --build --release` (unchanged by 08c),
  and the release windows job runs it on a runner. The zip step's
  python3-fallback branch was exercised locally in the shape it would
  run (the same code path as the Linux tar's python3-free tar).
- **Wasm rebuild**: not re-run (no emcc on this host; the recipe /
  flags / artifact names are 08a's and 08c touched none of them —
  `package-web.sh` / `build-wasm-emsdk.sh` are unchanged). The probe
  ran against 08a's verified artifacts.
- **Android release APK**: `package-android.sh` + the release android
  job already produce / rename `tinyhexagon-<tag>-android-arm64-v8a.
  apk` (08b's lane); 08c made no change there.
- The CI `OH_CHROME=/usr/bin/google-chrome` hardcode (the 08 P2) is
  KEPT: the probe errors loudly with a set-OH_CHROME hint if the
  runner's chrome disappears, and adding a setup-chrome action is a
  runner-policy call for the orchestrator.
- The `th-render` doc-comment reference to the retired
  `tools/web-probe.mjs` (task-6's "docs" sweep vs the 08c constraint
  "do not change game/runtime code") — left for the 09 sweep.
