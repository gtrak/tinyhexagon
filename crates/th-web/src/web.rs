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

/// The first-gesture audio unlock (the 08 shell's export; the one-mixer
/// plan, issue 09): the page's first `pointerdown` / `keydown` (and the
/// `visibilitychange` → visible re-fire) calls this. It is idempotent
/// (the in-memory engine start is a no-op while live). It arms the live
/// in-memory `MusicDirector` engine IN THE GESTURE (pure DSP — no
/// `AudioContext` created on this path); the same first input resumes the
/// shared OpenAL context (emscripten's `autoResumeAudioContext` + the
/// SFX device's probe listener), and the next frame's audio pass queues
/// the engine's frames into that context's streaming source — the SFX
/// and the music share ONE context, so ONE gesture unlocks both.
#[wasm_bindgen(js_name = "resumeAudio")]
pub fn resume_audio() {
    th_app::tiny::web_request_audio_unlock();
}

/// The score lab's read (plan 002 issue 10a, the frozen interface): the
/// director's current chiptune score as the canonical pretty JSON —
/// the `?scorelab=1` overlay's grid source and the "Copy JSON" button's
/// bake text. `""` before the web boot installed the director.
#[wasm_bindgen(js_name = "getScore")]
pub fn get_score() -> String {
    th_app::tiny::web_get_score()
}

/// The score lab's write (plan 002 issue 10a, the frozen interface):
/// parse `json` and hot-swap the director's score in place (no graph
/// rebuild — effective within one frame). Returns `""` on success; the
/// human-facing parse/validation error text otherwise (the overlay's
/// status line shows it verbatim — the `?score=` hot-reload's error
/// log).
/// The instrument ids for the DAW's picker (plan 002 issue 11f): the
/// page builds its `<select>` from THIS, so appending a `PresetKind` can
/// never leave the picker stale again (it did: 14 options for 15 kinds).
#[wasm_bindgen(js_name = "getKindIds")]
pub fn get_kind_ids() -> Vec<String> {
    th_app::tiny::web_kind_ids()
}

#[wasm_bindgen(js_name = "setScore")]
pub fn set_score(json: &str) -> String {
    th_app::tiny::web_set_score(json)
}
