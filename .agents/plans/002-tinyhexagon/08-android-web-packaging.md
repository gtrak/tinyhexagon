# Issue 08 — Android and web entries + packaging/CI/release names

**Status:** 08a LANDED (web entry; probe 19/19, gate 567). 08b LANDED (`2deca9c4`: android_main + content extraction w/ versioned marker, app id `com.gtrak.tinyhexagon`, keystore re-identified, APK verified; gate 570/0). 08c LANDED (`ab5d9872`: one web-tiny-probe (21/21), package.sh owns the tagged tar/zip + windowed smoke, release assets `tinyhexagon-<tag>-*`, actionlint clean; the timed-out first attempt was discarded).
**Depends:** 07

## Objective

Ship the same tinyhexagon loop on Android (arm64-v8a) and wasm/emscripten,
and update the packaging scripts, CI, release workflow and artifact names.
All four targets use the same `th-render`/`bridge`/`GlDrawTarget` path.

## Android

- `th-android` remains the cdylib crate. Keep the existing, device-verified
  glue:
  - `android_main` + `EventLoopBuilderExtAndroid`;
  - the internal-data-dir asset extraction for `_RELEASE`;
  - `th_render::Window::create_android` (EGL/GLES) and the activity lifecycle;
  - the no-SIGINT/Steam/Discord conditions.
- Change the app entry: call the tinyhexagon runtime
  (`th_app::tiny_android` or a shared `th_app::run` with the Android window)
  instead of the adult `main_client`.
- Config: no profiles/replays/config writes; the kid defaults.
- Audio: `th-synth` starts at boot (oboe); `th-audio` SFX starts at boot.
  If a stream fails, degrade to the fallback clock/no SFX without panicking.
- Touch: the two hold zones + corner icons from Issue 07.
- APK/lib names: package `th-android`, `apk_name = "tinyhexagon"`,
  cdylib `th_android` (or `tinyhexagon`); update `tools/android/` metadata and
  `tools/package-android.sh`.
- Remove the adult `open-hexagon` APK/asset paths.

## Web (emscripten)

- `th-web` remains the emscripten bin crate. Keep the verified build recipe:
  - `wasm32-unknown-emscripten`, `emcc`, `-sWASM_BINDGEN`,
    `-sDEFAULT_TO_CXX`, WebGL2 min/max, `--preload-file=_RELEASE@/RELEASE`,
    `-sFORCE_FILESYSTEM=1`, the `CXXFLAGS=-fwasm-exceptions` invocation from
    `tools/build-wasm-emsdk.sh`.
- Change the boot: call the tinyhexagon runtime (`th_app::tiny_web`) instead
  of the adult `main_client`. The composition reads `_RELEASE` through
  emscripten's libc.
- Shell (`crates/th-web/site/index.html`):
  - Keep the canvas, the WebGL2 boot, and the fullscreen button (`#fs`).
  - Keep a minimal `ohStatus`/log hook for the probes.
  - Remove the IME/`#ime`, text-entry and IDBFS/persistence machinery; there
    is no text entry and no local storage.
  - Add a **first-gesture audio unlock**: the first pointerdown/keydown calls
    the exported `resumeAudio()` (which starts/resumes `th-synth` and
    `th-audio`) and is idempotent. The fallback clock runs until then.
  - Keep the browser Fullscreen API handlers (the salvaged
    `web_fullscreen.rs` + the shell button).
- Artifact names: `tinyhexagon.js` / `tinyhexagon.wasm` (rename the crate
  output in the packaging script if the crate name is `th_web`); update
  `.cargo/config.toml` comments/names and `tools/package-web.sh`.
- Pages: update `ci.yml`'s pages job and `release.yml` to publish the new
  artifact/URL (the fork's gh-pages).

## Packaging and CI

Keep every workflow and script; rename only.

| File | Change |
|---|---|
| `.github/workflows/ci.yml` | Gate matrix (ubuntu/windows), windows-cross, android, wasm, pages. Update package/binary/artifact names. |
| `.github/workflows/release.yml` | Keep the release matrix and gh-pages; assets become `tinyhexagon-<tag>-{linux-x86_64.tar.gz,windows-x86_64.zip,android-arm64-v8a.apk,web.zip}`. |
| `tools/package.sh` | Linux/Windows package of `tinyhexagon` + `_RELEASE`. |
| `tools/package-android.sh` | `tinyhexagon.apk`. |
| `tools/package-web.sh` / `build-wasm-emsdk.sh` | tinyhexagon JS/wasm + shell. |
| `tools/build-windows.sh` | `tinyhexagon` binary/check. |
| `tools/check-android.sh` | `th-app`/`th-tiny` android check (update the package). |
| `tools/ci-deps.sh` | Unchanged deps (ALSA, NDK, mingw, emsdk as applicable). |
| `tools/web-*.mjs` | Point at the new artifact/page; keep the probes useful. |
| `.cargo/config.toml` | Update crate/artifact comments and names; keep the target flags. |

- Release artifacts must be produced for all four targets. The release
  workflow stays tag-driven.
- The pages URL/repo name is the fork's; use `tinyhexagon` everywhere the
  old `SSVOpenHexagon` page path appears.

## Steps

1. Make the tiny runtime target-agnostic enough for Android/web:
   `th_app::run(config)` plus platform window construction; no desktop-only
   calls in shared code.
2. Android: repurpose `th-android`'s entry to the tiny runtime; update the
   metadata/packaging; verify `check-android.sh` and a debug APK.
3. Web: repurpose `th-web`'s boot; simplify the shell; add the audio-unlock
   export; wire the fullscreen install; verify the emscripten build.
4. Update packaging scripts and the two workflows; run `actionlint`.
5. Update the web probes/screenshots if they still reference the adult app.
6. Gate + forked reviewer; then device/browser smoke.

## Verification

- `bash tools/check-android.sh` green; `tools/package-android.sh` produces
  `tinyhexagon.apk`; a device/emulator smoke: boot to gameplay, touch
  left/right, icons, audio (synth + SFX), no text, no persistence.
- `bash tools/build-wasm-emsdk.sh` / `package-web.sh` green; browser smoke:
  canvas, first-gesture audio, touch/mouse, fullscreen, no IDBFS writes,
  gameplay, no text.
- `tools/package.sh` produces the Linux and Windows packages with the
  `tinyhexagon` binary and `_RELEASE`.
- `actionlint` clean; `release.yml` asset names are `tinyhexagon-*`; the
  pages job publishes the tinyhexagon build.
- Host gate green (fmt/build/clippy/test).

## Risks

- Autoplay: the audio context/stream may be suspended until a gesture. The
  shell export and the fallback clock are mandatory.
- `rust-synth` on the target: Issue 05's spike/patch must be in place; do not
  start this issue if the target check fails.
- Android asset extraction paths and the APK packaging metadata differ from
  the adult app; copy the verified glue rather than inventing a new one.
- Web memory/preload: keep the existing `--preload-file` recipe and the
  emscripten flags; the shell simplification must not break the module boot.
- Two audio backends on web/Android (`th-audio` + `th-synth`); if they
  conflict, prefer the synth and degrade SFX, and record the deviation.

## Feed-forward

Record: the Android entry function, the web boot/audio-unlock export, the
final artifact names, and any target-specific degradation. Issue 09 uses the
packaged artifacts for the final smoke.
