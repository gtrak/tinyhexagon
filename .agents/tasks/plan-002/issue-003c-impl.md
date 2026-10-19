# Issue 03c — early wasm browser smoke + gh-pages stub

**Status:** READY — run after 04b lands, before 05/06.
**Depends:** 03 (rename). **Base:** post-04b `master`.
**Why now:** the user wants to watch the wasm/gh-pages pipeline before the
tiny runtime lands. The current page loads but cannot leave "booting"
because `th-web` is a no-op stub with no `status` export. Make the stub
wasm execute, add a headless-Chrome CDP probe, wire it into CI, and verify
the live page with a screenshot the local model can inspect.

## Local browser

The old agent-browser Chrome is still present:

```
/home/gary/.agent-browser/browsers/chrome-148.0.7778.97/chrome
```

`tools/web-probe.mjs` / `web-fs-probe.mjs` already default to it (or
`OH_CHROME`). Reuse their CDP launch/connect approach; do not add npm deps.

## Steps

### 1. Make the stub wasm execute

- `crates/th-web/Cargo.toml`, target emscripten only:
  ```toml
  [target.'cfg(target_os = "emscripten")'.dependencies]
  wasm-bindgen = "0.2.129"
  ```
  (match the lockfile; the CI installs the same `wasm-bindgen-cli` version).
- `crates/th-web/src/main.rs`:
  ```rust
  #[cfg(target_os = "emscripten")]
  use wasm_bindgen::prelude::*;

  #[cfg(target_os = "emscripten")]
  #[wasm_bindgen]
  pub fn status() -> String {
      "tinyhexagon stub: the tiny runtime lands in plan 002 issue 07/08".into()
  }

  #[cfg(target_os = "emscripten")]
  #[wasm_bindgen]
  pub fn save_session() {} // the old page's flush calls this; no storage in the stub

  fn main() {}
  ```
- The page's `window.ohStatus`/`mod.status()` and `#status` poll then work;
  the old `mod.save_session` no-op keeps the page's flush harmless.
- If the old adult page still throws with the stub (missing adult exports /
  IDBFS), replace `crates/th-web/site/index.html` with a **minimal** page:
  `<title>tinyhexagon</title>`, a full-viewport `<canvas id="c">`, a
  `#status` div, and an ES module:
  ```html
  <script type="module">
    import Module from "./tinyhexagon.js";
    const mod = await Module();
    window.ohStatus = () => mod.status();
    setInterval(() => {
      const s = mod.status();
      document.getElementById("status").textContent = s;
      console.log("status " + s);
    }, 1000);
  </script>
  ```
  Keep the `_RELEASE` preload in `.cargo/config.toml` for the future; the
  minimal page can ignore it. Issue 08 replaces this page with the real
  tiny entry.

### 2. Headless CDP probe

Add `tools/web-stub-probe.mjs` (Node 24 global WebSocket; no npm):

- Launch Chrome: `OH_CHROME` or the agent-browser path candidates; flags
  `--headless=new --no-sandbox --disable-gpu --use-gl=angle
  --use-angle=swiftshader --enable-unsafe-swiftshader` (match the old
  probes), a fresh `--user-data-dir`, a free `--remote-debugging-port`.
- Navigate to the given URL (default `http://127.0.0.1:8080/`), wait for
  `window.ohStatus`/`#status` to leave empty (timeout configurable).
- Assert: `document.title` contains `tinyhexagon`; a `<canvas>` exists with
  non-zero width/height; `window.ohStatus()` and `#status` contain
  `tinyhexagon`; no console errors (a `/favicon.ico` 404 is tolerated);
  capture a screenshot to `--shot target/web-stub.png` (or the given path).
- Print a short summary; exit 0 only on all assertions.
- Serve option: the CI starts `python3 -m http.server` in
  `target/package/web/_WEB`; the probe takes the URL.

### 3. CI browser step

In `.github/workflows/ci.yml`'s `wasm` job, after `tools/package-web.sh`:

- `browser-actions/setup-chrome@v1` (or use the runner's
  `google-chrome`); export `OH_CHROME` from the action path.
- Start a local static server on the staged `_WEB`:
  `python3 -m http.server 8080 --bind 127.0.0.1 --directory target/package/web/_WEB &`
- Run `node tools/web-stub-probe.mjs http://127.0.0.1:8080/ --shot target/web-stub.png`.
- Upload `target/web-stub.png` as an artifact (evidence).
- The `pages` job (needs: wasm) deploys only if the browser probe passes.
- Keep the release workflow unchanged for now; the 08 web slice will run the
  same probe (or the full `web-probe.mjs`) against the real entry.

### 4. Verify the live page

After the push and the green CI/pages run:

```sh
OH_CHROME=/home/gary/.agent-browser/browsers/chrome-148.0.7778.97/chrome \
  node tools/web-stub-probe.mjs https://gtrak.github.io/tinyhexagon/ \
  --shot target/web-stub-live.png
```

Inspect `target/web-stub-live.png` with the local model's image capability:
the page title, canvas, and status text ("tinyhexagon stub …") must be
visible; the canvas may be black (the runtime is 07). The screenshot is the
browser-execution evidence, not just the loader.

## Verification

```sh
cargo build -p th-web --target wasm32-unknown-emscripten   # if emcc/CI; host gate otherwise
bash tools/gate.sh
node tools/web-stub-probe.mjs http://127.0.0.1:8080/ --shot target/web-stub.png
```

CI: the wasm job green (build + browser probe) and the pages job deploys.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-003c-report.md`: the status export, the
probe, the CI step, the local live screenshot result (describe the image),
and the gate. Update `FEED-FORWARD.md` §08: the probe/CI path and that 08
replaces the stub page with the real entry. Orchestrator lands ONE squashed
commit after review; worker does not commit.

## Risks

- The old adult page may throw; the minimal page fallback is the clean fix.
- The `.data` preload is 22 MB; the probe must wait for the module resolve
  (timeout 60s), not just the page load.
- Headless Chrome's WebGL/SwiftShader can be flaky; use the same flags the
  old probes used and keep the probe's assertions DOM/status-based.
- Do not add npm dependencies; the local Node has a global WebSocket.
- Do not change the real game entry (07/08) here.
