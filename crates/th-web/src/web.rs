//! The emscripten (web) boot + the two probe exports (plan 002 issue 08a).

use std::sync::atomic::{AtomicBool, Ordering};

use wasm_bindgen::prelude::*;
// `JsValue` is `wasm_bindgen::JsValue` (the prelude's import above):
// `js_sys::JsValue` is a private re-import in js-sys 0.3.106
// (`E0603` — the struct is not re-exported publicly from `js_sys`).

/// The single-init guard (the 26c.1/26c.2 "main ran twice" artifact):
/// `true` once the boot has run — a re-entered `main` / re-invoked boot
/// skips the second init (one composition / one canvas / one WebGL2
/// context / one rAF loop).
static BOOT_STARTED: AtomicBool = AtomicBool::new(false);

/// The boot (called from `main` AFTER `__wasm_call_ctors()`): the engine
/// log lines to the browser console (Rust stdio is a black hole in the
/// emscripten glue), the single-init guard, and the shared tiny entry's
/// emscripten half (the `th_app::tiny::tiny_web` boot — the `_RELEASE`
/// composition over the page's canvas, the `web_fullscreen` sinks, the
/// rAF loop). Returns after scheduling the first frame.
pub fn boot() {
    th_game::log_sink::set_log_sink(|channel, message| {
        web_sys::console::log_1(&JsValue::from_str(&format!("[{channel}] {message}")));
    });

    // The single-init guard (a re-entered boot no-ops — the second
    // composition would build a second engine over the same canvas).
    if BOOT_STARTED.swap(true, Ordering::SeqCst) {
        web_sys::console::log_1(&JsValue::from_str(
            "[th-web] boot re-entered - second init skipped (single-init guard)",
        ));
        return;
    }
    web_sys::console::log_1(&JsValue::from_str(
        "[th-web] boot started (after __wasm_call_ctors)",
    ));

    th_app::tiny::tiny_web();
    web_sys::console::log_1(&JsValue::from_str(
        "[th-web] tiny boot scheduled (the rAF loop owns the game from here)",
    ));
}

/// The status probe (the 03c `status()` hook, now the REAL game's status
/// line — `th_app::tiny::web_status`: the boot state, the rAF tick
/// count, the journey stage, the engine's started flag, the player
/// angle, the synth's live-vs-fallback state, the helping-hand state).
/// The page's `window.ohStatus` / `#status` poll reads this every second
/// (the `tools/web-tiny-probe.mjs` wait + assertions ride on it).
#[wasm_bindgen]
pub fn status() -> String {
    th_app::tiny::web_status()
}

/// The first-gesture audio unlock (the 08 shell's export): the page's
/// first `pointerdown` / `keydown` (and the `visibilitychange` → visible
/// re-fire) calls this once. It is idempotent (the Rust side re-checks
/// the browser's user activation and the live-engine state). It arms the
/// live `MusicDirector` engine (`start_on_gesture`) on the next frame;
/// the `th-audio` SFX device (emscripten's OpenAL) unlocks on the SAME
/// first gesture through its own listeners (emscripten's
/// `autoResumeAudioContext` + the module's probe listener) — there is no
/// `th-audio` call to make.
#[wasm_bindgen(js_name = "resumeAudio")]
pub fn resume_audio() {
    th_app::tiny::web_request_audio_unlock();
}
