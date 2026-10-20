# Issue 08b — report (Android tinyhexagon entry: DONE — check green, debug APK verified; device smoke = the manual half)

**Worktree:** `.agents/worktrees/slot-1` (base `b23c378a`).
**Status: COMPLETE (Android lane).** The 02a stub is replaced with the real
`android_main` over the shared `th_app::tiny` runtime:
`bash tools/check-android.sh` green (`th-app` + `th-android` against the
pinned NDK r27c), the debug APK builds via `cargo apk build`
(`target/debug/apk/tinyhexagon.apk`, `android_main` exported, signed, the
`_RELEASE` tree + the 06 tiny pack in `assets/`), and the host gate
re-runs at its baseline **567 passed / 0 failed**. No commit, no
web/packaging/CI-workflow work (the one workflow edit is a comment in
`ci.yml`'s job description — the android job runs the updated
`tools/check-android.sh` script unchanged).

**Application-id fix (post-review, orchestrator-directed):** the first
pass kept the plan's "the package id stays" note and shipped under the
adult app identity `info.vittorioromeo.openhexagon` — the 08b brief
forbids the adult APK identity. The application id is now
**`com.gtrak.tinyhexagon`** (`[package.metadata.android] package`), the
`tools/android` keystore carries the tiny identity too (renamed +
regenerated — see "Packaging metadata / tools"), and the debug APK was
REBUILT + re-verified (`aapt dump badging` →
`package: name='com.gtrak.tinyhexagon'` — below). Everything else is
unchanged.

**Review fixes (post-PASS, orchestrator-directed; no commit):** the
08b review passed with six pre-landing fixes — (1) the content marker
is now VERSIONED (the `.th_content_extracted` content carries
`CONTENT_VERSION: u32 = 1`; the skip check is the marker's content,
not its presence — an app UPDATE over the surviving internal data dir
re-extracts on a version mismatch instead of leaving the stale
`_RELEASE`; bump the const on every `_RELEASE` change — the 09-taste
journey level updates are the first bump); (2) the `th-android`
Cargo.toml's stale `.filelist` comments aligned to the actual
`oh.filelist`; (3) the `package-android.sh` manual check is
level-agnostic ("the stage-0 level" — survives the 09-taste journey
merge); (4) `release.yml`'s "a keystore must NEVER be committed"
clarified — a STORE-distribution key never; the committed
fixed-password `tools/android/tinyhexagon-release.keystore`
self-distribution key is the documented exception; (5) the
FEED-FORWARD §03 stale package-id line updated + cross-referenced to
§08b (the keystore-exception line likewise); (6) the evidence
refreshed (the host-gate log carries the `test result:` aggregate —
567/0; the android-check log is a clean re-run showing BOTH
`Checking th-app` + `Checking th-android`; the debug APK rebuilt +
re-verified after the marker change). Re-verified: host gate 567/0,
`check-android.sh` exit 0, rebuilt-APK badging unchanged
(`com.gtrak.tinyhexagon`).

## The Android entry function

`crates/th-app/src/tiny.rs` (new, `#[cfg(target_os = "android")]`):
**`th_app::tiny::tiny_android(app: AndroidApp)`** — the `tiny_web`
emscripten boot's android twin over the shared [`run`](shared `run`
windowed entry, 08a's target-agnostic half):

1. `base` = `app.internal_data_path()` (the app's internal data dir — the
   extracted `_RELEASE` content; the `th-android` entry extracts + chdirs
   before calling this);
2. `th_assets::Assets::new(&base, false, false)` (the kid's windowed,
   no-workshop shape — the 02c3-retired workshop seam's discovery half
   has no android caller);
3. `Config::default()` → `lifecycle::load_and_reapply(&mut config, &base,
   &[], DesktopModeSource::Fixed(VideoMode::new(0, 0)))` → `kid_config`
   (the plan's Android note: **`Fixed(zero)` — no child probe on
   Android**: winit's monitor API exists only inside a RUNNING event
   loop and the activity's loop is consumed by the game's own `run`;
   inert: both auto-resolution flags default false and the platform
   window is screen-sized independently of the config values). Never
   written back (no config/profile/replay persistence);
4. `th_audio::Audio::new(assets)` + `set_sound_volume(60.0)` (the SFX —
   audio fallback below);
5. `th_synth::MusicDirector::start()` (the non-emscripten boot attempt —
   live on oboe/AAudio, fallback clock on failure);
6. `EventLoopBuilderExtAndroid`: `EventLoop::<()>::with_user_event()` +
   `builder.with_android_app(app)` + `build()` (the `AndroidApp`-bound
   winit loop — MUST exist before any window/GL work; the platform
   `ANativeWindow` only exists between `InitWindow` and
   `TerminateWindow`);
7. `th_render::Window::create_android(event_loop, physical_w.max(1),
   physical_h.max(1), "tinyhexagon")` (physical = config logical ×
   `pixel_mult` — the C++ `recreateWindow` sizing; the EGL/GLES context
   materializes on the first `resumed` — `ensure_android_gl` →
   `create_android_parts`), then the C++ `main.cpp:305-310` config
   setters (`set_size`/`set_pixel_mult`/`set_fullscreen`/
   `set_antialiasing_level`/`set_vsync`/`set_fps_limited`/`set_max_fps` —
   the android record halves; the AA level + vsync are applied at the
   materialization) + the `Assets/icon.png` load (no-op on the platform
   window, the C++'s load-failure-tolerant shape);
8. `run(window, assets, base, config, audio, music, None)` — the shared
   frame loop (the 07 touch halves + corner icons + bump/journey/beat/
   SFX + the bridge draw + the swap).

Every boot failure (internal-data-dir, assets, config, event-loop
build) logs (`eprintln!` → the `RustStdoutStderr` logcat tag + the
engine lines through the logcat sink) and returns without panicking —
the activity's main-loop thread stays alive (the C++'s
no-crash-on-failed-boot posture).

**`crates/th-android/src/android.rs`** (new): `#[no_mangle] pub fn
android_main(app: AndroidApp)` (the D-026 entry contract — the
android-activity glue's C wrapper links this Rust symbol; the verified
02a shape, `git show b7718ed6^:crates/oh-android/src/android.rs`):
(1) the engine-log sink install (`__android_log_write`,
`ANDROID_LOG_INFO`, tag **`tinyhexagon`** — `adb logcat -s
tinyhexagon`; the adult glue's `OpenHexagon` tag renamed with the
product; installed FIRST, before any boot failure below), (2)
`assets_base_dir` (the internal data dir), (3) `extract_content` (below),
(4) `chdir_to_base_dir`, (5) `th_app::tiny::tiny_android(app)`.

The no-SIGINT / no-Stream / no-Discord conditions: the tiny composition
is the 02/07 de-scoped one (null seams — `NoJoystick`, no renderer
seam, the 02c1-retired config/profile save guards do not exist; the
`th-app` `sigint` module is never called from the android path — the
activity lifecycle is the stop mechanism: the back button's
`CloseRequested` closes the winit window → the loop exit). **No IME /
soft input** (the kid has no text entry — the retired adult
`ImeController` has no twin; the `th-app` emscripten web-sys features
are untouched).

## Asset extraction path

The verified SG4 glue, carried into `th-android` (`src/android.rs`):
`extract_content(asset_manager: &AssetManager, base_dir: &Path)` —

* **versioned marker skip (the 08b review fix):** `.th_content_
  extracted` in the internal data dir stores the current
  `CONTENT_VERSION` (`u32 = 1` — the original 08b content). The skip
  check is the marker's CONTENT, not its presence
  (`marker_current`): a missing / unreadable marker (the fresh
  install; a crashed mid-extraction never wrote it) OR one carrying a
  DIFFERENT value (an app UPDATE — the internal data dir survives the
  update, so a bare presence check would skip the new APK's content
  and the stale `_RELEASE` would stay: the 09-taste journey level
  update is exactly such an update, where
  `TinyRuntime::new`'s `get_level_data(...).expect(...)` would panic
  on the missing level) both re-extract over the tree, and the walk
  writes the new value after. (The walk overwrites / creates every
  listed file but does not delete unlisted ones — a level REMOVED from
  the pack stays as an orphan in the internal dir: inert for the
  journey, which references explicit level ids; a 09/08c sweep
  candidate if the content ever deletes a level.) Bump
  `CONTENT_VERSION` on every
  `_RELEASE` change (the content's owner does — tools/
  package-android.sh stages the tree, the const tracks it; the
  09-taste journey level updates are the first bump). RE-NAMED from
  the adult's `.oh_content_extracted`, 08b (the `.th_` prefix is the
  product-consistent name, the `.th_partial` sibling's); with the
  application-id change below, a device that carried the retired
  adult app's extracted tree gets a FRESH internal data dir for the
  new application anyway (Android isolates app data per package id).
* **enumeration-free walk:** the packaging-generated `oh.filelist`
  (tools/package-android.sh's staging step: `find . -type f` + sort, one
  assets-relative path per line — the 03 rename exception kept by
  design) instead of `AAssetManager_openDir("")` (the on-device smoke's
  empty-root quirk — a silent 0-file extraction).
* **per file:** `AAssetManager_open` + read whole + atomic write
  (`.th_partial` + rename; parents created on demand; the partial
  suffix re-named with the product from `.oh_partial`).
* **zero-file refusal:** a missing/empty filelist or a 0-file walk is an
  `Err` (never the marker over nothing — the original smoke bug);
  `[FATAL ERROR]` + return in `android_main` (the C++'s
  `std::terminate` equivalent).

Extraction target: the app's **internal data dir**
(`AndroidApp::internal_data_path()` — private, writable; the C++'s
chdir-to-executable-dir has no Android equivalent). `ndk = "0.9"` (the
lockfile's existing version — winit's `android-activity` dependency;
`AndroidApp::asset_manager()` returns this crate's `AssetManager`
directly — no lockfile version change).

## Audio fallback

* **`th-synth` (the soundtrack):** `MusicDirector::start()` at boot —
  the non-emscripten arm attempts the live `AudioEngine` (cpal 0.15.3 →
  oboe 0.6.1 → AAudio on the minSdk-28 floor, the 05b source-verified
  note). `try_start_live`'s `Err` arm logs (`th-synth: live audio start
  failed (…)`) and stays on the **fallback clock** (`BeatView` keeps
  advancing, `is_live()` false) — **never a panic** (the 05a boot
  contract; the 05a no-device test pins the no-panic half).
* **`th-audio` (the SFX):** `Audio::new` opens the rodio/oboe output
  eagerly; a failed sink open (a dead AAudio device) degrades to the
  **headless device** — every one-shot no-ops while the object state
  (volume/pitch/offset) stays correct; the failure is logged so a dead
  sink is diagnosable in logcat, **never a panic** (the `th-audio`
  module contract — "Audio::new never panics on a missing device").
  SFX at the plan's gentle volume: `set_sound_volume(60.0)`.
* If both fail: the game runs fully silent on the fallback beat clock —
  the plan's "degrade to the fallback clock/no SFX without panicking".
* No audio-gesture unlock (that is the web's autoplay policy, 08a's
  `resumeAudio`); Android boots live-or-fallback at boot, exactly the
  05a table (desktop/Android: live attempt; emscripten: fallback until
  gesture).

## Touch / icon DP

The 07 shared `touch.rs` + `tiny.rs` code — no android-specific
geometry (the window layer feeds `FrameEvents` from the window's
`finger_down_positions` in the shared `run` loop):

* **the two hold halves:** a finger's down-half (`x < width/2` → CCW,
  else CW) captures it for the whole touch (multi-touch holds both
  sides);
* **the two corner icons:** 64×64 squares, 8px margin (07's
  `ICON_SIZE`/`ICON_MARGIN` — the plan's "48–64 dp" band), top-left
  **reset** (the engine's `new_game` same-level + `start` +
  `restart.ogg`), top-right **fullscreen** — on Android the toggle is
  `Window::set_fullscreen`'s **record half** (the winit call is
  `cfg(not(android))`-gated in `th-render`: the platform activity
  window owns the fullscreen surface; the record drives the bookkeeping
  the way the adult glue documented), `select.ogg` / `beep.ogg` SFX;
* a finger on an icon is consumed (one tap on the down-edge, no
  rotation);
* **no IME / no text** (all-false `DrawOptions`, the 04b kid config);
* the back button → `CloseRequested` → window close → loop exit (no
  SIGINT half — the module notes).

## Packaging metadata / tools

`crates/th-android/Cargo.toml` (03's renames already in place — verified,
no `open-hexagon` APK/asset paths remained):

* **application id** `[package.metadata.android] package =
  "com.gtrak.tinyhexagon"` (the badging `package: name=` — 08b
  post-review, orchestrator-directed: the adult
  `info.vittorioromeo.openhexagon` id is the adult app's identity the
  brief forbids; the new id installs the tiny APK as its own
  application and supersedes the plan's "the package id stays" note —
  superseded likewise in the 03 entry's keystore-identity exception,
  see below);

* package `th-android`, **`apk_name = "tinyhexagon"`**, cdylib **`th_android`**
  (`lib/arm64-v8a/libth_android.so` — the `android.app.lib_name` cargo-apk
  writes), `build_targets = ["aarch64-linux-android"]` (arm64-only), SDK
  bounds **min 28 / target 34**, `assets = "../../target/th-apk-assets"`
  (the script's staging dir — the `_RELEASE` tree + `oh.filelist`),
  fullscreen landscape `NativeActivity` (`label = "tinyhexagon"`,
  `Theme.DeviceDefault.NoActionBar.Fullscreen`, `sensorLandscape`,
  the `config_changes` set, `exported = true`) — the verified 02a
  metadata kept.
* **the self-distribution release keystore re-identified with the
  product (08b post-review):** `tools/android/
  openhexagon-release.keystore` → `tools/android/
  tinyhexagon-release.keystore` (REGENERATED with keytool — alias
  `tinyhexagon`, fixed password `tinyhexagon`,
  `CN=TinyHexagon (self-distribution), OU=Rust port, O=TinyHexagon`;
  the old file deleted from the tree). The old identity was the adult
  app's (the `openhexagon` alias / the `O=SSVOpenHexagon` DN); the
  03 "live keystore identity" rename-exception is void now that the
  application id changed — the new id has no prior installs, so a
  fresh signing identity costs no update continuity. `tools/
  package-android.sh`'s `KEYSTORE` / `KEYSTORE_PASS` / alias / DN all
  follow; `.github/workflows/release.yml`'s keystore-path comment
  follows (comment-only).
* `[dependencies]`: `th-app` (the tiny runtime boot) + `th-game` (the
  log-sink FFI names the crate directly); `[target.'cfg(target_os =
  "android")'.dependencies]`: `winit` 0.30 (`android-native-activity` —
  the `AndroidApp` type + the event-loop builder; th-render carries the
  same version/feature — no lockfile version change) + `ndk 0.9`
  (the extraction's `AssetManager`).
* `Cargo.lock`: only the `th-android` entry gains its four deps
  (all already-locked packages — +7 lines, zero new packages).

`tools/check-android.sh`: now `cargo check -p th-app -p th-android
--target aarch64-linux-android` (the tiny runtime's android entry + the
cdylib's `android_main`; the NDK r27c pin + the self-installing rustup
target unchanged). `.github/workflows/ci.yml`: the android job's header
comment updated to the new check (comment only — the job runs the same
script; the workflow's behaviour is 08c's lane). `tools/package-android.sh`:
already `tinyhexagon.apk` / `target/th-apk-assets` / `oh.filelist`
(03's renames) — only the tail "MANUAL checks" list reworded to the tiny
game (stage-0 meadow, the synth/SFX fallback, the icon taps, the no-
persistence relaunch) + the `adb logcat -s tinyhexagon` handle + the
08b keystore rename (above). The release APK path (signing, the 16KB-
page `RUSTFLAGS`, the `tinyhexagon-<tag>-android-arm64-v8a.apk` asset)
is 08c's lane.

## Verification (exact evidence)

* **Baseline (pre-edit):** `bash tools/check-android.sh` — **exit 0**
  (`cargo check -p th-app --target aarch64-linux-android`, NDK r27c
  27.2.12479018, API 28).
* **`bash tools/check-android.sh` (final — the clean re-run, post
  review fixes): exit 0** — `cargo check -p th-app -p th-android
  --target aarch64-linux-android` from a clean th-app/th-android
  target state (both crates' aarch64 artifacts + fingerprints + the
  incremental dir removed) so the log shows BOTH `Checking th-app`
  and `Checking th-android` (the full `th-app` + `th-android`
  android graph: winit 0.30.13 android backend, oboe 0.6.1 (the
  bundled oboe C++ from source via the NDK clang++), vendored cpal
  0.15.3 + rust-synth 0.23.0, rodio/cpal 0.18). Log:
  `.agents/evidence/08b/check-android.log` (both `Checking` lines +
  `Finished dev profile`; the host is fast in check mode — with the
  dependency rmetas cached the two re-checks are sub-second).
* **Debug APK:** `cp -a _RELEASE/. target/th-apk-assets/` + the
  `oh.filelist` generation (232 asset files — the script's staging
  step), then
  `cargo apk build --manifest-path crates/th-android/Cargo.toml --target aarch64-linux-android`
  (debug profile — cargo-apk 0.10's auto debug-keystore signing; the
  16KB-page `RUSTFLAGS` rides on the release script's path only) —
  **exit 0**, `target/debug/apk/tinyhexagon.apk` (174 MB; debug
  `libth_android.so` 157 MB), `apksigner` "Verification succesful".
  Log: `.agents/evidence/08b/debug-apk-build.log`.
* **APK verification — the REBUILT APK (post the review fixes — the
  versioned marker changed the `.so`)**
  (`.agents/evidence/08b/apk-verification.txt` +
  `debug-apk-build.log` are the rebuild; the application-id fix above
  was the second build): `aapt dump badging` →
  **`package: name='com.gtrak.tinyhexagon'`** `versionCode='16777472'
  versionName='0.1.0'`, `sdkVersion:'28'`, `targetSdkVersion:'34'`,
  `launchable-activity: name='android.app.NativeActivity'`,
  `native-code: 'arm64-v8a'` (the first pass's badging carried the
  adult `info.vittorioromeo.openhexagon` id — the fix above); the
  manifest (`aapt dump xmltree`) confirms `A: package="
  com.gtrak.tinyhexagon"` + `android:label="tinyhexagon"` (03's label
  intact); contents: `lib/arm64-v8a/libth_android.so` +
  `libc++_shared.so` + 233 `assets/` entries (160 `Packs/`, **10
  `Packs/tiny/`** — the 06 pack's meadow/river/stars levels + scripts
  + styles), `assets/oh.filelist` present (the extractor's source);
  `llvm-nm -D libth_android.so` → **`00000000004dc41c T android_main`**
  (the NativeActivity's dlsym target exported); `apksigner verify` →
  **verified** (debug keystore). (The debug .so's 4KB ELF LOAD
  alignment is expected — the 16KB-page alignment is the release
  script's `RUSTFLAGS` half, 08c's path.)
* **Host gate (re-run post review fixes):** `bash tools/gate.sh` —
  **fmt 0 / build 0 / clippy 0 / test 0 — 567 passed, 0 failed** (the
  08a baseline; the new code is `target_os = "android"`-gated — the
  host compiles none of it; the fix pass is metadata/comment-only
  otherwise). Log: `.agents/evidence/08b/host-gate.log` (carries the
  per-binary `test result:` lines + the TOTAL aggregate).
* **Device gap (the exact residual):** `adb devices` → no device / no
  running emulator on this host — the install/boot/touch/audio/rotation
  smoke is the `package-android.sh` tail's documented MANUAL list
  (reworded for the tiny game): `adb install -r` (the first install
  runs the extraction — the `[::mainClient] extracted N content files`
  line + `adb logcat -s tinyhexagon`), launch → the stage-0 meadow
  level (GLES 3.0 + the translated shaders; the synth live-or-fallback
  + the SFX at volume 60), left/right halves = CCW/CW, the icon taps,
  the relaunch (marker skip + stage 0 again — no persistence), the
  rotation (sensorLandscape, no game-state loss). The plan's own
  acceptance surface is the same: build-level here, on-device the
  manual half (the 25b spec's posture, carried forward).

## State / next steps

* **Done, stopping here per the task brief.** No commit — the working
  tree carries the 08b diff (incl. the application-id fix + the six
  review fixes) on base `b23c378a`; the debug APK + the staging dir
  live under the gitignored `target/` (the evidence copies are in
  `.agents/evidence/08b/`; `apk-verification.txt` +
  `debug-apk-build.log` are the REBUILT (post-review-fixes) APK's,
  `check-android.log` the clean re-run, `host-gate.log` the gate with
  the aggregate). The keystore rename shows as an unstaged deletion +
  untracked new file (`tools/android/`), left for the commit pass.
* FEED-FORWARD §08b appended + updated (the entry function, the
  versioned marker, the application-id fix, the audio-fallback halves,
  the DP, the APK evidence, the device gap, the six review fixes).
* 08c (the workflows/packaging lane) picks up: the release APK path
  (`tools/package-android.sh` — signing with the re-identified
  `tools/android/tinyhexagon-release.keystore`, the 16KB `RUSTFLAGS`,
  the `tinyhexagon-<tag>-android-arm64-v8a.apk` asset), the remaining
  workflow renames if any, the device smoke when a device/emulator is
  available, AND the `CONTENT_VERSION` bump whenever the 09-taste
  `_RELEASE` content lands (the review fix's bump discipline — the
  versioned marker's re-extraction depends on it).
