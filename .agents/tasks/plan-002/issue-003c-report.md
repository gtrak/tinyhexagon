# Issue 03c report — early wasm browser smoke + gh-pages stub

Base: post-04b `master` (`7ac4c603`). The stub wasm now EXECUTES in the
browser, the gh-pages page is the minimal stub shell, the headless-Chrome
CDP probe is wired into CI's `wasm` job before the `pages` deploy, and the
whole chain was verified locally (emscripten build through the pinned emsdk
podman recipe + the probe against the locally staged page). **Committed by the orchestrator after the forked PASS; no worker commit from
the worker — the orchestrator lands ONE squashed commit after review.**

## 1. The status export (the stub wasm executes)

- `crates/th-web/Cargo.toml`: `wasm-bindgen = "0.2.129"` under
  `[target.'cfg(target_os = "emscripten")'.dependencies]` — the lockfile's
  version (CI installs the same `wasm-bindgen-cli`; the host target stays
  dep-free). `Cargo.lock` diff: the single `th-web → wasm-bindgen` edge.
- `crates/th-web/src/main.rs`: `#[wasm_bindgen] status() -> String` returns
  `"tinyhexagon stub: the tiny runtime lands in plan 002 issue 07/08"`;
  `#[wasm_bindgen] save_session()` is a no-op (kept for the old page / the real entry in 08; the minimal page does not call it
  calls it; the stub has no storage). Host `main` stays `fn main() {}`.
  Both exports are `#[cfg(target_os = "emscripten")]` — the host workspace
  is untouched.
- Build (local, the issue-057 emsdk container recipe — `podman run` the
  cached `docker.io/emscripten/emsdk:latest` (emcc 6.0.12) with the host's
  rustup/cargo mounted, `CARGO_TARGET_DIR=/work/target`, `bash
  tools/package-web.sh`): **green in 9.4 s** (the stub's only dep is
  wasm-bindgen). Staged `target/package/web/_WEB`: `index.html` (3.2 KB) +
  `tinyhexagon.js` (90 KB) + `tinyhexagon.wasm` (58 KB) +
  `tinyhexagon.data` (22.8 MB, the preloaded `_RELEASE` tree) + the zip.
  CI (the `setup-emsdk` runner job) remains the canonical wasm gate.
  - One toolchain warning, PRE-EXISTING and out of scope: `emcc:
    WASM_BIGINT is deprecated … [-Wdeprecated]` (a `linker_messages` note).
    `WASM_BIGINT` appears in NO repo flag (grep-verified) — it comes from
    rustc 1.97.1's emscripten target spec against emcc 6.0.12, so any
    emscripten build on the pinned toolchain carries it. The host gate
    never compiles the emscripten target, so it stays invisible to
    `tools/gate.sh`; the build succeeds with it.

## 2. The page (the adult shell → the minimal stub shell)

`crates/th-web/site/index.html` is now the spec's **minimal** page:
`<title>tinyhexagon</title>`, a full-viewport `<canvas id="c">` (640×480
backing store, CSS 100vw/100vh — the stub never resizes it), an EMPTY
`#status` div (the probe's wait is `#status` leaving empty), and the ES
module (`import Module from "./tinyhexagon.js"; const mod = await
Module(); window.ohStatus = () => mod.status();` + the 1 s poll into
`#status` + console).

**Decision recorded (reviewer's call):** the old adult page does NOT
mechanically throw under the stub — its only wasm dependencies were
`status()` / `save_session()`, both now exist (the minimal page only polls status; 08 re-adds the real flush/IDBFS glue that reads
`cfg.FS`/`cfg.IDBFS`, which the unchanged link flags still export, but
nothing calls `ohWebPersistBoot` anymore). It was replaced anyway because
(a) the adult shell would have put a DEAD `#fs` fullscreen button on the
live page (the Rust side that owned the button was retired in 02a and the
old page has no JS click handler) plus an invisible `#ime` field and
IDBFS glue nothing calls — a misleading live page; (b) the spec names the
minimal page the "clean fix" and says 08 "replaces THIS page with the
real tiny entry" — i.e. the minimal page is the stub-era page; (c) the
probe's assertions pass under either page, so the gate is unaffected. The
`_RELEASE` preload flags in `.cargo/config.toml` are KEPT for 07/08 (the
`.data` still ships and preloads into MEMFS — harmless for the stub).

## 3. The probe (`tools/web-stub-probe.mjs`)

Node ≥ 22 (global WebSocket; no npm deps). Reuses `web-probe.mjs`'s CDP
launch/connect: `OH_CHROME` or the agent-browser Chrome candidates,
per-PID `--remote-debugging-port` (the lane-058 stale-Chrome lesson), a
fresh per-run `--user-data-dir`, `--headless=new --no-sandbox
--disable-gpu --disable-dev-shm-usage --use-gl=angle
--use-angle=swiftshader --enable-unsafe-swiftshader`. Navigates to the
given URL (default `http://127.0.0.1:8080/`), waits (default **60 s** —
configurable via `--timeout`) for `window.ohStatus` to be a function AND
`#status` to leave empty (the wait is the MODULE RESOLVE, not the page
load — the 22 MB `.data` preloads during the factory's boot), then
asserts:

- `document.title` contains `tinyhexagon`
- a `<canvas>` exists with non-zero width/height
- `window.ohStatus()` and `#status` both contain `tinyhexagon`
- no console errors/exceptions (`error` AND `warning` policy, same as
  `web-probe.mjs`; a `/favicon.ico` 404 is tolerated)
- no 4xx/5xx responses (favicon tolerated)
- no failed loads (the favicon's 404-abort is tolerated — see below)
- the screenshot captured (`--shot`, default `target/web-stub.png`)

It prints the short summary (status + PASS/FAIL per check + console /
non-200 / failed-loads lists + the shot path) and exits 0 only on
all-pass. `Network.loadingFailed` entries carry the URL (requestId→URL map
from `requestWillBeSent`) and the `canceled` flag: a CANCELED
`net::ERR_ABORTED` (the browser's own abort — the favicon after its 404,
a re-navigation) is tolerated; an UNCANCELED failure (a refused
connection, a dropped fetch) still fails the gate. This tolerance was
discovered in local verification: the first probe run failed
"no failed loads" purely on the favicon's canceled abort (its entry had no
URL to match against).

## 4. The CI browser step (`.github/workflows/ci.yml`, `wasm` job)

After `Build + stage` (`tools/package-web.sh`), before the default-branch
`web-publish` upload:

- `Browser smoke (the stub's headless-Chrome CDP probe)`: starts
  `python3 -m http.server 8080 --bind 127.0.0.1 --directory
  target/package/web/_WEB &` (readiness loop + an EXIT trap to kill it),
  then `OH_CHROME=/usr/bin/google-chrome node tools/web-stub-probe.mjs
  http://127.0.0.1:8080/ --shot=target/web-stub.png`. The runner's
  preinstalled `google-chrome` is the browser (no setup action — the probe
  is DOM/status-based, not version-sensitive pixels; the spec offered
  either).
- `Upload the stub screenshot (evidence)`: `target/web-stub.png` →
  artifact `web-stub-screenshot` (per push/PR — the local sibling's
  artifact).
- A failing probe fails the `wasm` job, and `pages` (needs: wasm) never
  deploys a page the browser cannot run — the spec's gate, unchanged job
  graph. The file header's `wasm` job line now records the browser smoke.
  `release.yml` untouched (the spec's "for now"; 08's web slice runs the
  same probe or the full `web-probe.mjs` against the real entry).

## 5. Local verification (done, pre-push)

- **Wasm build + staging**: the emsdk podman recipe above — green
  (compile, link, wasm-bindgen, the 22.8 MB preload pack, the staging,
  the zip).
- **Probe against the locally staged page**: served
  `target/package/web/_WEB` on 127.0.0.1:18080 (port 8080 is held by an
  unrelated `filebrowser` service on this host — the probe's URL argument
  makes the port a parameter; CI's clean runner has no collision) and ran
  the probe twice with the agent-browser Chrome
  (`/home/gary/.agent-browser/browsers/chrome-148.0.7778.97/chrome`):
  both runs **8/8 PASS, exit 0** (status line
  `tinyhexagon stub: the tiny runtime lands in plan 002 issue 07/08`,
  title `tinyhexagon`, canvas 640×480, no console errors, no non-200
  except the tolerated favicon 404). The negative path is proven too:
  pointing the probe at the stale 8080 server (a foreign page without the
  stub status) fails the wait and exits 1 with the console/log dump.
- **Screenshot inspection** (`target/web-stub.png`, 1280×720, the
  probe's own `Page.captureScreenshot`): a full-viewport near-black
  canvas (the stub draws nothing — the runtime lands in 07/08) with the
  light-gray monospace `#status` text top-right reading exactly
  `tinyhexagon stub: the tiny runtime lands in plan 002 issue 07/08`.
  That is the browser-execution evidence: the wasm module resolved,
  `status()` returned the stub string, and the page's 1 s poll wrote it
  into the DOM. (The title bar is not in the capture — headless CDP
  captures the layout viewport — so the title assertion rides on the
  probe's `document.title` read, not pixels.)
- **Host gate**: `bash tools/gate.sh` — fmt / build / clippy
  (`-D warnings`) / test all `GATE_EXIT=0`; `cargo test --workspace`
  **539 passed / 0 failed / 1 ignored** (the pre-existing th-lua doc-test
  ignore; the 04b baseline was the same 539 — this issue adds no host
  tests and weakens none).

## 6. PENDING — the live page (after the orchestrator's push)

CI (`wasm` job green including the browser smoke + `pages` deploy) lands
on the push. Then the spec's step 4, from this host:

```sh
OH_CHROME=/home/gary/.agent-browser/browsers/chrome-148.0.7778.97/chrome \
  node tools/web-stub-probe.mjs https://gtrak.github.io/tinyhexagon/ \
  --shot=target/web-stub-live.png
```

Inspect `target/web-stub-live.png` with the image capability: the title,
canvas, and the stub status text must be visible (the canvas may be
black). Note: the gh-pages branch may still serve the OLD adult page's
bytes until this push's `pages` deploy completes — a probe run against
the stale deployment would show the adult page (its `#status` reads
`status: <…>` from the old composition's last deploy); re-run after the
deploy's green check. (The old page would still pass the probe's
assertions — title / canvas / status text / `ohStatus` all survive — so
a PASS is not, by itself, proof of the NEW page: the screenshot's
top-right text WITHOUT the old page's `#fs` button is the distinguisher.)

## 7. Feed-forward

`FEED-FORWARD.md` §08's "Web boot/audio-unlock export" line now carries
the probe/CI path and the 08 contract (08 replaces the stub page with the
real tiny entry's shell; 08 re-runs the same probe or the full
`web-probe.mjs` against the real entry).

## Residual risks

- CI's `wasm` job has not yet run (it runs on the push): the runner's
  `google-chrome` + `python3 http.server` + the probe's SwiftShader path
  are each locally verified only in combination (agent-browser Chrome,
  same probe logic) — the first CI run is the runner-environment
  confirmation. If the runner's Chrome emits a boot console `warning`
  (the probe gates warnings, per `web-probe.mjs`'s policy), the smoke
  step will say so verbatim and the tolerance can be narrowed in a one-
  line follow-up.
- The `WASM_BIGINT` deprecation warning (toolchain-level, §1) may become
  an ERROR in a future emcc — a `.cargo/config.toml` flag, not a 03c
  concern.
- Port 8080 on this host is held by `filebrowser` (pid 2409) — local
  serve/probe runs here should use another port (the probe's URL arg);
  the many stale lane `http.server` processes from 054–058 are
  left alone.
