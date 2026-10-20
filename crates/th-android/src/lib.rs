//! `th-android` — the Android (aarch64) entry crate (plan 002 issue 08b).
//!
//! The thin, target-gated glue between the Android runtime and the shared
//! tiny runtime:
//!
//! * the `android_main(app: AndroidApp)` native-activity entry (D-026:
//!   `native-activity`, not `game-activity` — the smaller surface, no Java
//!   game framework) — android-activity 0.6 through Winit's re-export
//!   (`winit::platform::android::activity::AndroidApp`), exactly the
//!   signature the glue's C wrapper expects (`fn android_main(app)`);
//! * the engine-log sink install (the `__android_log_write` logcat route —
//!   the engine's `[channel]` lines + the boot diagnostics reach
//!   `adb logcat -s tinyhexagon`; the `th-audio`'s failed-sink-open log
//!   half lands there too);
//! * the `_RELEASE` content extraction into the app's internal data dir
//!   (the verified issue 25b SG4 glue: the packaging-generated
//!   `oh.filelist` drives the `AAssetManager` open/read — the
//!   on-device-smoke fix for the `AAssetManager_openDir("")` empty-root
//!   quirk — with the content marker's skip + the atomic per-file write);
//! * the tiny boot hand-off: `th_app::tiny::tiny_android(app)` — the
//!   shared tiny entry's android half (assets / config / audio / synth /
//!   the winit event loop + `Window::create_android` / the frame loop),
//!   the `tiny_web` emscripten boot's twin;
//! * the `[package.metadata.android]` cargo-apk section (issue 25b SG6 —
//!   package name, the aarch64-only target, the `_RELEASE` content as APK
//!   assets, the SDK bounds, the fullscreen `NativeActivity`);
//! * the cdylib name: the `libth_android.so` cargo-apk ships into the
//!   APK's `lib/arm64-v8a/` (the `android.app.lib_name` meta-data cargo-apk
//!   writes is this lib name).
//!
//! On every non-Android target the crate builds to an empty cdylib (the
//! entry symbol + the winit android modules are `target_os = "android"`-
//! gated), so the desktop gate (`cargo build/clippy/test --workspace`)
//! never sees it.

#[cfg(target_os = "android")]
mod android;
