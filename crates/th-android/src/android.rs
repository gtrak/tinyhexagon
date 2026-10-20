//! The Android `tinyhexagon` entry (plan 002 issue 08b, D-026) — the
//! native-activity `android_main` over the SAME tiny runtime the desktop
//! `tinyhexagon` bin and the web `tiny_web` boot drive
//! (`th_app::tiny::run`'s shared loop; this crate only does the platform
//! bootstrap and hands the `AndroidApp` to `th_app::tiny::tiny_android`):
//!
//! 1. the engine-log sink (the `__android_log_write` logcat route — the
//!    adult glue's issue 039 STEP 0 salvage; every engine `[channel]`
//!    line + this module's boot diagnostics reach logcat under the
//!    `tinyhexagon` tag);
//! 2. the content base dir (the app's internal data dir — the C++'s
//!    chdir-to-executable-dir has no Android equivalent; the APK's
//!    `_RELEASE` content is extracted there, step 3, issue 25b SG4);
//! 3. the `_RELEASE` extraction (the verified SG4 glue — module notes
//!    below);
//! 4. the CWD step (`set_current_dir` to the base dir — the CWD-relative
//!    fallback paths the port threads explicitly agree with the explicit
//!    `base` path);
//! 5. `th_app::tiny::tiny_android(app)` — the tiny boot (assets / config
//!    + `DesktopModeSource::Fixed(zero)` / the `th-audio` SFX + the
//!    `th-synth` soundtrack (oboe/AAudio, boot-start with the fallback
//!    degradation) / the winit android event loop / the
//!    `Window::create_android` window / the frame loop).
//!
//! # The Android differences (every one documented, none silently faked)
//!
//! * **No SIGINT handler** — the desktop `th-app` `sigint` install has no
//!   Android counterpart (the activity's lifecycle is the stop mechanism:
//!   the native activity's terminate / the platform's
//!   `WindowEvent::CloseRequested` — the back button — closes the winit
//!   window → the loop exit, the `th-render` `Window` event fold); the
//!   `th-app` crate's `sigint` module is never called from here.
//! * **No Steam / no Discord / no profiles / no config / no replays** —
//!   the kid's de-scoped composition (plan 002 issue 02/07): the engine
//!   seams are the null arms (`NoJoystick`, no renderer seam — the
//!   bridge draws), the tiny runtime persists NOTHING (the kid's
//!   in-memory `config.json`-free config; the `th-app` `lifecycle`
//!   no-persistence state: `load_and_reapply` only — the 02c1-retired
//!   config/profile save guards do not exist). Every run starts at
//!   stage 0.
//! * **Audio** — the REAL `th_audio::Audio` (rodio over oboe/AAudio —
//!   the issue 05b source-verified AAudio-on-API-28 note) started at
//!   boot: a failed sink open degrades to the headless device (logged to
//!   logcat, never a panic — the `th-audio` contract), and the
//!   `th-synth` `MusicDirector::start()`'s live attempt's `Err` arm runs
//!   the fallback clock (the `BeatView` keeps advancing — the 05a boot
//!   contract, the plan's "degrade to the fallback clock / no SFX
//!   without panicking").
//! * **Touch / icons** — the 07 shared `touch.rs` halves + the two 64×64
//!   corner icons ride the shared frame loop (`tiny.rs::run` feeds
//!   `FrameEvents` from the window's `finger_down_positions`; the
//!   icon-tap reset is the engine's `new_game`+`start`; the fullscreen
//!   icon's action is the window's record half — the platform activity
//!   window owns the fullscreen surface, the `set_fullscreen`'s
//!   android record note). No IME / no soft input (the kid has no text
//!   entry — the retired adult `ImeController` has no twin here).
//! * **CLI args** — the native activity carries none (the C++ Android
//!   build's `argv` is the loader's); `load_and_reapply` gets the empty
//!   override list.
//! * **Desktop mode** — `DesktopModeSource::Fixed(VideoMode::new(0, 0))`
//!   (the plan's Android note: winit's monitor API exists only inside a
//!   RUNNING event loop and the activity's loop is consumed by the game's
//!   own `run` — a probe here would consume it; inert in practice: both
//!   auto-resolution flags default false and the platform window is
//!   screen-sized independently of the config values).

use std::ffi::CString;
use std::path::{Path, PathBuf};

use ndk::asset::AssetManager;
use winit::platform::android::activity::AndroidApp;

/// The engine-log sink's liblog write (the NDK `log.h` `ANDROID_LOG_INFO`
/// = 4; the `__android_log_write` non-format form — the format-taking
/// `__android_log_print` is variadic, which stable Rust's FFI cannot
/// call, so the fully-formatted line is written whole). The tag is the
/// spec's `adb logcat -s <tag>` handle (the adult glue's `OpenHexagon`
/// tag renamed with the product — 03's rename; the fork's `tinyhexagon`
/// is the identity here, not the C++'s).
const ANDROID_LOG_INFO: i32 = 4;
const LOG_TAG: &core::ffi::CStr = c"tinyhexagon";

#[allow(clippy::missing_safety_doc)] // the FFI's own docs are the NDK log.h
extern "C" {
    fn __android_log_write(
        prio: i32,
        tag: *const core::ffi::c_char,
        buf: *const core::ffi::c_char,
    ) -> i32;
}

fn android_log_write(line: &str) {
    // The NUL terminator (the liblog `const char*` contract): the
    // `CString` is the bounded allocation of the day (the engine's
    // lines are event-rate, not frame-rate).
    let buf = match std::ffi::CString::new(line) {
        Ok(b) => b,
        // An interior NUL cannot occur in a formatted `[{channel}] {message}`
        // (both halves are Rust `String`s; the C++'s own lines carry none).
        Err(_) => return,
    };
    unsafe {
        __android_log_write(ANDROID_LOG_INFO, LOG_TAG.as_ptr(), buf.as_ptr());
    }
}

/// The marker file the content extraction writes (issue 25b SG4) — its
/// CONTENT (the current [`CONTENT_VERSION`], one line) is the skip check
/// (the 08b review fix): a missing / unreadable marker (the fresh
/// install; a crashed mid-extraction never wrote it, so the next start
/// re-extracts over the partial tree) or one carrying a DIFFERENT value
/// (an app UPDATE — the internal data dir survives the update, so the
/// stale extracted tree would otherwise persist: the new APK's content
/// is skipped and the old `_RELEASE` stays, the 09-taste journey level
/// update being exactly that) both re-extract over the tree; a matching
/// value skips (the spec's "skip extraction when the content is
/// already present").
///
/// Renamed from the adult glue's `.oh_content_extracted` with the
/// product (08b) — the `.th_` prefix is the product-consistent name
/// (the `.th_partial` sibling's). With the 08b application-id change
/// (`info.vittorioromeo.openhexagon` → `com.gtrak.tinyhexagon`) a
/// device that carried the retired adult app's extracted tree (older
/// `_RELEASE` content, WITHOUT the 06 tiny pack the kid's stage-0
/// `new_game` needs) gets a FRESH internal data dir for the new
/// application anyway (Android isolates the app data per package id) —
/// the marker is the new application's own skip check.
const CONTENT_MARKER: &str = ".th_content_extracted";
/// The version of the `_RELEASE` content this APK ships (the marker's
/// stored value — the `CONTENT_MARKER`'s docs): `1` = the original 08b
/// content (the 06 tiny pack). Bump on EVERY `_RELEASE` change (the
/// content's owner does — tools/package-android.sh stages the tree,
/// this const tracks it; the 09-taste journey level updates are the
/// first bump) so an updated APK's boot re-extracts over the previous
/// version's stale tree instead of skipping it.
const CONTENT_VERSION: u32 = 1;
/// The packaging-generated file list (tools/package-android.sh staging):
/// one assets-relative path per line — the extraction's enumeration-free
/// source (module notes: why openDir is not used). `oh.filelist` is the
/// 03 rename exception kept by design (the APK asset marker — the
/// packaging script generates it, the extraction reads it).
const FILELIST: &str = "oh.filelist"; // non-hidden: cargo-apk skips dotfiles (smoke finding)

/// The `_RELEASE`-content base dir for this run (the C++'s CWD).
///
/// SG4 (issue 25b) makes this the EXTRACTED content: the first run copies
/// the APK's `assets/` tree (the `_RELEASE` content — see
/// [`extract_content`]) into the app's internal data dir; later runs
/// detect the marker and skip the copy. The kid's in-memory config (the
/// tiny runtime's no-persistence state) means nothing is ever written
/// back into this dir after boot.
fn assets_base_dir(app: &AndroidApp) -> Result<PathBuf, String> {
    app.internal_data_path()
        .ok_or_else(|| "AndroidApp::internal_data_path is None (activity destroyed)".to_string())
}

/// Issue 25b SG4: extract the APK's `assets/` tree (the `_RELEASE`
/// content — cargo-apk's `assets` metadata places the `_RELEASE` entries
/// at the APK assets root) into `base_dir` (the app's internal data dir)
/// on first run (or when the marker's content version is stale — an app
/// update), then write the marker with the current [`CONTENT_VERSION`];
/// SKIP (the Ok no-op) when the marker already carries the current
/// version (the spec's "skip extraction when the content is already
/// present" — versioned, the 08b review fix).
///
/// This is the port-specific necessity the C++ Android build avoids
/// (SFML's `AssetManager` wrapper reads the APK on the fly): `th-assets`
/// threads CWD-relative filesystem paths, so the content must land on
/// disk before the `config.json` load + the `Assets` scan in
/// [`th_app::tiny::tiny_android`].
///
/// The walk: the packaging-generated `oh.filelist` (one assets path per
/// line, written by tools/package-android.sh into the staging assets
/// dir). NO `AAssetManager_openDir` enumeration: the on-device smoke
/// (emulator, API 34) caught `openDir("")` returning an EMPTY root
/// listing (the classic NDK root-enum quirk), which silently extracted 0
/// files. Each listed path is opened (`AAssetManager_open`, which is
/// reliable where openDir is not), read whole (streaming mode) and
/// written ATOMICALLY (`.th_partial` + rename — a crash mid-extraction
/// never leaves a half-file at the real name; parents are created on
/// demand). Returns `Err` (the `[FATAL ERROR]` in [`android_main`] — the
/// C++'s `std::terminate` on missing content) when an open/read/write
/// fails OR when the filelist is missing/empty (never write the marker
/// over a zero-file extraction — the original smoke bug also wrote the
/// marker on the empty walk, permanently skipping a future good run).
/// The marker's skip check (the 08b review fix): `true` ONLY when the
/// marker exists AND carries the current [`CONTENT_VERSION`] — a
/// missing / unreadable marker (the fresh install) or a stale value (an
/// app update whose internal data dir survived) both re-extract.
fn marker_current(base_dir: &Path) -> bool {
    std::fs::read_to_string(base_dir.join(CONTENT_MARKER))
        .is_ok_and(|v| v.trim() == CONTENT_VERSION.to_string())
}

fn extract_content(asset_manager: &AssetManager, base_dir: &Path) -> Result<(), String> {
    // The 08b review fix: the skip check is the marker's CONTENT (the
    // current version) — a missing marker (the fresh install) or a stale
    // one (an app update over the surviving internal data dir) both
    // re-extract (the `CONTENT_MARKER`'s docs).
    if marker_current(base_dir) {
        return Ok(());
    }
    let filelist_asset = asset_manager
        .open(cstr(FILELIST)?.as_c_str())
        .ok_or_else(|| format!("the packaging-generated {FILELIST:?} is missing from the APK assets (stale/absent staging — rebuild via tools/package-android.sh)"))?;
    let mut filelist_bytes = Vec::new();
    std::io::Read::read_to_end(&mut { filelist_asset }, &mut filelist_bytes)
        .map_err(|e| format!("asset read({FILELIST:?}): {e}"))?;
    let filelist = std::str::from_utf8(&filelist_bytes)
        .map_err(|e| format!("{FILELIST:?} is not UTF-8: {e}"))?;
    let mut file_count = 0usize;
    for path in filelist.lines().filter(|l| !l.is_empty()) {
        extract_asset_file(asset_manager, path, base_dir, &mut file_count)?;
    }
    if file_count == 0 {
        return Err(format!(
            "the {FILELIST:?} extraction copied 0 files — refusing to write the content marker (stale/absent APK assets)"
        ));
    }
    // The marker's CONTENT is the current version (the 08b review fix —
    // the next boot's `marker_current` compare: a matching value skips,
    // a bumped one re-extracts over the stale tree).
    std::fs::write(base_dir.join(CONTENT_MARKER), CONTENT_VERSION.to_string())
        .map_err(|e| format!("write the content marker {CONTENT_MARKER:?}: {e}"))?;
    eprintln!(
        "[::mainClient] extracted {file_count} content files from the APK assets into {base_dir:?}"
    );
    Ok(())
}

/// One listed file `path` (from the packaging-generated `oh.filelist`,
/// e.g. `Assets/OpenSquare-Bold.ttf` or `Packs/tinyhexagon/.../level.json`):
/// open, read whole (streaming mode), write ATOMICALLY (module notes:
/// the walk).
fn extract_asset_file(
    asset_manager: &AssetManager,
    path: &str,
    base_dir: &Path,
    file_count: &mut usize,
) -> Result<(), String> {
    let cpath = cstr(path)?;
    let mut asset = asset_manager
        .open(cpath.as_c_str())
        .ok_or_else(|| format!("AAssetManager_open({path:?}) failed"))?;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut asset, &mut bytes)
        .map_err(|e| format!("asset read({path:?}): {e}"))?;
    let local = base_dir.join(path);
    if let Some(parent) = local.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create_dir_all({parent:?}): {e}"))?;
    }
    let partial = local.with_extension("th_partial");
    std::fs::write(&partial, &bytes).map_err(|e| format!("write({partial:?}): {e}"))?;
    std::fs::rename(&partial, &local)
        .map_err(|e| format!("rename({partial:?} -> {local:?}): {e}"))?;
    *file_count += 1;
    Ok(())
}

fn cstr(s: &str) -> Result<CString, String> {
    CString::new(s).map_err(|e| format!("CString({s:?}): {e}"))
}

/// `mainClient`'s CWD step on Android: point the process CWD at the assets
/// base dir so the CWD-relative paths the port threads explicitly
/// (`base_dir`) AND any CWD-relative fallback (`std::env::current_dir()`)
/// agree (the C++'s `ssvu::FileSystem` CWD model). `std::env::set_current_dir`
/// on the app's internal dir (writable, private).
fn chdir_to_base_dir(base_dir: &Path) -> Result<(), String> {
    std::env::set_current_dir(base_dir).map_err(|e| format!("set_current_dir({base_dir:?}): {e}"))
}

/// The Android `tinyhexagon` native-activity entry (module docs): called
/// ONCE per `android_main` invocation (the activity's main-loop thread);
/// the shared loop's exit (the platform `CloseRequested` — the back
/// button — or the window close) returns through it.
///
/// `#[no_mangle]` + the default (Rust) ABI: the android-activity glue's C
/// `android_main` wrapper links this Rust symbol directly (android-
/// activity 0.6's entry contract — its lib docs: "an exported, unmangled,
/// `\"Rust\"` ABI function with the signature `fn android_main(app:
/// AndroidApp)`"); the cdylib's `lib/arm64-v8a/libth_android.so` exports
/// it (the `android.app.lib_name` meta-data's dlsym target).
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    // The engine-log sink (installed FIRST — before the base-dir /
    // extraction / boot failures below, so the engine's lines exist from
    // the earliest possible frame; the composition's own `eprintln!`
    // boot-failure lines already reach logcat on the `RustStdoutStderr`
    // tag, a second logcat-visible route).
    th_game::log_sink::set_log_sink(|channel, message| {
        android_log_write(&format!("[{channel}] {message}"));
    });
    th_game::log_sink::emit_log(
        "android",
        "engine boot: the tinyhexagon logcat sink is installed",
    );

    // The base dir (the app's internal data dir — the C++'s CWD; the
    // extracted `_RELEASE` content from SG4 on).
    let base_dir = match assets_base_dir(&app) {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("[::mainClient] {e}");
            return;
        }
    };
    // SG4 (issue 25b): the `_RELEASE` content must be ON DISK before the
    // `config.json` load + the `Assets` scan in the tiny boot (the module
    // notes: the port-specific necessity — the C++ reads the APK on the
    // fly). The first run extracts (the marker check skips later runs).
    if let Err(e) = extract_content(&app.asset_manager(), &base_dir) {
        // The C++ would `std::terminate()` (missing/unreadable content).
        eprintln!("[FATAL ERROR] Asset extraction failed: {e}");
        return;
    }
    if let Err(e) = chdir_to_base_dir(&base_dir) {
        eprintln!("[::mainClient] {e}");
        return;
    }

    // The tiny boot (the module docs' step 5 — the assets / config /
    // audio / synth / window / loop, all in `th_app::tiny`; the failure
    // arms log + return without panicking, the `tiny_android`'s docs).
    th_app::tiny::tiny_android(app);
}
