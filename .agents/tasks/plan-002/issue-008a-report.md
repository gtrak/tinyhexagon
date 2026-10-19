# Issue 08a — report (web build fix: DONE — final pass fix 4: `#status` hidden; probe 19/19, audio=live, gate 567)

**Worktree:** `.agents/worktrees/slot-2` (branch `slot-2`, base `c9bf8d23`).
**Status: COMPLETE (web lane).** Four sequential fixes took the emscripten
build from red (compile errors) → red (wasm-bindgen CLI panic) → red
(runtime panics in the audio callback) → **green**: `tools/package-web.sh`
stages `_WEB`, the extended probe passes **19/19** with a clean console and
**`audio=live`** after the gesture, and the host gate re-runs at its
baseline **567 passed / 0 failed**. Fix 4 (this pass) hides the
child-visible `#status` line + lands the P2 cleanups. No commit, no
Android/packaging.

## Fix 4 (this pass) — hide the child-visible status line + P2 cleanups

Reviewer-passed 08a shipped one child-visible artifact: the page rendered
the raw `#status` probe line top-right (the 08a screenshot shows it).
The game is text-free, so the line is hidden visually while the probe
contract stays intact:

1. `crates/th-web/site/index.html` — the `#status` rule gains
   `display: none;` (the element stays in the DOM; the 1 s poll keeps
   filling its `textContent`, so `window.ohStatus()` + the probe's
   `#status` assertions work unchanged). The page header comment's stale
   "a PROBE run stays `audio=fallback` (the 041 measurement)" note is
   corrected: the 10-19 Chrome 148 probe run's synthetic `keydown`
   granted user activation, so it measured `audio=live`; the
   no-activation fallback guard stays.
2. `crates/th-web/Cargo.toml` — the unused direct `js-sys` dependency
   removed (+ its comment mentions): `web.rs` uses only the prelude
   `JsValue` (`wasm-bindgen`) + `web_sys`. `Cargo.lock`: `th-web`'s
   dep list drops `js-sys` (the lockfile update landed with the
   container build).
3. `crates/th-app/src/web_fullscreen.rs` — the stale "No caller yet"
   module note: the web entry's `tiny_web` boot (issue 08a) runs
   `install_sinks` (the `#fs` click + the `fullscreenchange` re-sync).
4. `crates/th-render/src/window.rs` (`web_fs_button_get_or_create` doc,
   ~:3665) — "the committed shell declares it" was stale for this
   build: the shipped 08a tiny shell does NOT declare `#fs`, so the
   get-or-create's absent-element fallback is the button's source there
   (the full inline style contract + the `⛶` glyph, ungated); a found
   element (a custom page that does declare it) is left AS-IS. The
   committed-shell gate JS / `display: none`-by-default / F31-band
   collision-resolution details are kept, attributed to that page.
5. `crates/th-app/src/tiny.rs` — the two stale "headless probe stays
   fallback" notes (the rAF loop's unlock step + the
   `web_request_audio_unlock` docs): corrected to say activation
   granting is the browser's — the 041 measurement saw none (fallback
   clock), while the 10-19 Chrome 148 probe run's synthetic `keydown`
   DID grant it (that run measured `audio=live`); the
   `navigator.userActivation.isActive` guard stays as the no-activation
   half's safety (cpal 0.15.3's emscripten host still panics on the
   rejected resume promise).

### Fix 4 evidence

- **Build** (emsdk container, the pinned recipe): `podman run … 
  docker.io/emscripten/emsdk:latest bash -c 'export
  PATH=/home/gary/.cargo/bin:$PATH; bash tools/package-web.sh'` —
  **exit 0**, `Finished 'release' … in 13.11s` (incremental: the
  dependency removal recompiled `th-app` + `th-web` only; same
  pre-existing warnings — the 2× dead_code in the vendored cpal
  emscripten host, the `WASM_BIGINT` linker note). Staged `_WEB`:
  `index.html` 7,352 B (up from 6,710 — the fix-4 comment + rule),
  `tinyhexagon.js` 218,412 B, `tinyhexagon.wasm` 3,856,150 B,
  `tinyhexagon.data` 22,775,907 B; zip 9.3 M.
  Log: `/tmp/08a-logs/build4.log`.
- **Probe** (free port 8087, `OH_CHROME=/home/gary/.agent-browser/
  browsers/chrome-148.0.7778.97/chrome node tools/web-stub-probe.mjs 
  http://127.0.0.1:8087/ --shot=.agents/evidence/08a/08a-web-probe-fix4.png`):
  **exit 0, `RESULT: PASS`, 19/19 assertions PASS**, `console: no
  errors`, `non-200: none`, `loads: none`:

  ```
    baseline : tinyhexagon web: booted=true;ticks=12;stage=0;started=true;angle=0.0000;audio=fallback;help=auto
    post     : tinyhexagon web: booted=true;ticks=116;stage=0;started=true;angle=-11.7103;audio=live;help=off
  ```

  **`audio=live`** after the gesture (the 10-19 Chrome 148 run again —
  activation granted); the `#status` assertions (14 / 15) pass against
  the hidden element's `textContent` — the probe contract is intact.
  Log: `/tmp/08a-logs/probe-4.log`.
- **Screenshot** `.agents/evidence/08a/08a-web-probe-fix4.png`:
  the real game (meadow background, white ball, the in-canvas
  fullscreen icon top-right, the `#fs` fallback's faint `⛶` below it)
  and **no status text anywhere** — the raw probe line no longer shows
  to the child.
- **Host gate**: `bash tools/gate.sh` — **fmt 0 / build 0 / clippy 0 /
  test 0 — 567 passed, 0 failed** (baseline; the edits are comments /
  an emscripten-scoped dependency removal / a page CSS rule — the host
  target is unaffected). Log: `/tmp/08a-logs/host-gate4.log` +
  `target/gate.log`.

## Fix 3 (this pass) — cpal emscripten host: connect direction

`vendor/cpal-0.15.3/src/host/emscripten/mod.rs` (output-stream timer
callback, ~line 330). The previous run passed both earlier panics and died
here on the first live-audio buffer:

```
Could not connect the audio node to the destination: JsValue(IndexSizeError:
Failed to execute 'connect' on 'AudioNode': output index (0) exceeds number
of outputs (0))
```

Root cause: the registry source connected **from** the destination —
`context.destination().connect_with_audio_node(&node)` — but an
`AudioDestinationNode` has **0 outputs**, so the browser rejects the call.
Changed to the source→destination direction (the one the webaudio host
already uses, `host/webaudio/mod.rs:363`):

```rust
node.connect_with_audio_node(&context.destination())
    .expect("Could not connect the audio node to the destination");
```

(`node.start()` kept after it, unchanged.) One-line semantic change;
provenance bullet added to the vendored `Cargo.toml` header (fix-3 entry,
matching the fix-2 style). No other connect-direction bug in the vendored
cpal (grep: only `emscripten/mod.rs:333` was reversed; the webaudio host is
correct). This was the LAST panic — the probe then ran clean.

## The earlier passes (kept for the audit trail)

- **Fix 1** (compile errors, all `cfg(target_os = "emscripten")`-scoped):
  `th-app/tiny.rs` `web_request_audio_unlock` guard — `Window::navigator()`
  returns `Navigator` (never `Option`) in web-sys 0.3.106, so
  `.and_then(|w| w.navigator())` → `.map(|w| w.navigator())`; `th-web/web.rs`
  — four `js_sys::JsValue::from_str` (private re-import, E0603 ×4) → the
  prelude `JsValue::from_str`; `th-web/main.rs` — `///` doc on the
  `extern "C" { fn __wasm_call_ctors(); }` block (unused_doc_comments under
  `-D warnings`) → `//`; `th-web/web.rs` cargo-fmt collapse.
- **Fix 2** (the wasm-bindgen 0.2.129 CLI descriptor panic + the live
  sample-type panic), both in the vendored cpal:
  1. `src/lib.rs` — the whole `BufferSize` emscripten wasm-ABI triple
     (`WasmDescribe` + `FromWasmAbi`/`IntoWasmAbi` `Abi`) now delegates to
     `Option<FrameCount>` the way the 0.18.2 registry cpal does. The
     registry source's empty `describe()` wrote zero bytes → CLI
     `Descriptor::decode` panic (`descriptor.rs:324`, "len is 0") on
     `__wbindgen_describe___wbg_get_streamconfig_buffer_size`; the
     pre-0.2.129 `Option<u32>`-as-`Abi` two-word `WasmRet` return did not
     match the descriptor's `f64` glue. Now: [OPTIONAL, U32] descriptor +
     the 0.2.129 single-`f64` `2^53-1`-sentinel ABI.
  2. `src/host/emscripten/mod.rs` — the `SampleFormat` the callback's
     `Data` carries is captured by the timer closure instead of crossing
     the JS timer as a `Closure<FnOnce() -> SampleFormat>` argument (the
     0.2.129 timer invocation coerced the JS argument to the u32 ABI `0`
     = `I8` → `traits.rs` panic "host supplied incorrect sample type").

## Build evidence (emsdk container, emcc 6.0.12, the pinned recipe)

- `podman run … docker.io/emscripten/emsdk:latest bash -c 'export
  PATH=/home/gary/.cargo/bin:$PATH; bash /work/tools/package-web.sh'` —
  **exit 0**, `Finished 'release' … in 14.96s` (incremental over the fix-2
  build; only the vendored cpal + downstream recompiled). Warnings only
  (pre-existing: 2× dead_code in the vendored cpal emscripten host,
  `WASM_BIGINT` deprecation linker note) — none are `-D warnings` host
  gate issues; the gate's host build is unaffected (all edits are
  emscripten-scoped / vendored-for-emscripten).
- Staged `_WEB` (`target/package/web/_WEB`): `index.html` 6,710 B,
  `tinyhexagon.js` 218,412 B, `tinyhexagon.wasm` 3,856,150 B,
  `tinyhexagon.data` 22,775,907 B; zip `target/package/web/
  tinyhexagon-web.zip` 9.3 M.
- Log: `/tmp/08a-logs/build3.log`.

## Probe evidence (19/19, clean console, audio=live, screenshot)

Serve: `python3 -m http.server 8086 --bind 127.0.0.1 --directory
target/package/web/_WEB`. Run:

```
OH_CHROME=/home/gary/.agent-browser/browsers/chrome-148.0.7778.97/chrome \
  node tools/web-stub-probe.mjs http://127.0.0.1:8086/ \
  --shot=.agents/evidence/08a/08a-web-probe.png
```

Result (**exit 0, `RESULT: PASS`**, all 19 assertions PASS):

```
  baseline : tinyhexagon web: booted=true;ticks=12;stage=0;started=true;angle=0.0000;audio=fallback;help=auto
  post     : tinyhexagon web: booted=true;ticks=114;stage=0;started=true;angle=-11.8752;audio=live;help=off
  console  : no errors
  non-200  : none
  loads    : none
```

Notes:
- **`audio=live` after the gesture** — the 041 headless measurement
  predicted `fallback` (no user activation under CDP), but this Chrome 148
  run granted activation for the synthetic `keydown` gesture, so the
  `resumeAudio()` unlock flipped the fallback clock to the live
  AudioContext. The fallback path remains intact as the no-activation
  guard (the `navigator.userActivation.isActive` check in
  `web_request_audio_unlock`); either outcome satisfies the probe,
  `live` is the stronger result.
- The status line in the screenshot matches the `post` values
  (`ticks=114; …;audio=live;help=off`), and the frame shows the real game
  (meadow background, white ball, HUD chips).
- Screenshot: `.agents/evidence/08a/08a-web-probe.png` (1280×577 PNG).
- Log: `/tmp/08a-logs/probe3.log`.

## Host gate (baseline preserved)

`bash tools/gate.sh` — **fmt 0 / build 0 / clippy 0 / test 0 — 567 passed,
0 failed** (the baseline 567; every edit is `cfg(target_os =
"emscripten")`-scoped or in the vendored cpal, which the host target does
not compile). Log: `/tmp/08a-logs/host-gate3.log` + `target/gate.log`.

## State / next steps

- **Done, stopping here per the task brief.** No Android/packaging work,
  no commit — the working tree carries the 08a diff (fix 1 + 2 + 3 + 4)
  on top of the base 08a code; `_WEB` is staged under the gitignored
  `target/` (the fix-4 build: `index.html` 7,352 B with the hidden
  `#status`).
- FEED-FORWARD §08a updated with the fix-2/3 root causes + the green
  probe/gate evidence.
- If a later lane wants to drop the vendor patch: the upstream cpal
  connect-direction bug would need an upstream fix (the webaudio host is
  already correct in 0.15.3; only the emscripten host is reversed) — until
  then keep the 05b/08a vendor scheme (root `[patch.crates-io]`).
- The pre-existing 2× dead_code warnings in the vendored cpal emscripten
  host are cosmetic; leave them (touching them widens the vendor diff).
