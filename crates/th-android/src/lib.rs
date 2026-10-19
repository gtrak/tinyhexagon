//! `th-android` — the Android (aarch64) entry crate (plan 002).
//!
//! Stub state after issue 02a: the adult `android_main` entry (the
//! windowed-pipeline hand-off into `th-app`'s android module) was
//! retired with `th-app`'s windowed front-end; the `[lib]` cdylib target
//! and the cargo-apk packaging metadata (package name, the aarch64-only
//! build target, the `_RELEASE` assets, the SDK bounds, the fullscreen
//! `NativeActivity`) are kept so Issue 08 lands the real Android entry
//! over the tiny runtime without re-doing the packaging.
