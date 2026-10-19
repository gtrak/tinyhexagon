//! `th-web` — the emscripten (web) entry crate (plan 002).
//!
//! Stub state after issue 02a, made to EXECUTE by issue 03c: the adult
//! web entry (`web::boot` — the retired windowed-client composition over
//! the preloaded `_RELEASE` MEMFS tree) was retired with `th-app`'s
//! windowed front-end. The stub now runs in the browser: two
//! `#[wasm_bindgen]` exports keep the staged page's probe loop alive —
//! `status()` (the page's `window.ohStatus` / `#status` poll, asserted
//! by `tools/web-stub-probe.mjs` in the CI's `wasm` job) and
//! `save_session()` (the page's session-end flush — a no-op: the stub
//! has no storage). The host `main` stays a no-op, keeping the host
//! workspace green. The real web entry (the tiny runtime's
//! `request_animation_frame` loop + the audio-unlock +
//! `th_app::web_fullscreen::install_sinks` wiring) lands in Issue 08,
//! which re-adds the emscripten target dependencies and replaces the
//! minimal stub page.

#[cfg(target_os = "emscripten")]
use wasm_bindgen::prelude::*;

/// The stub's status probe (issue 03c): the page's `window.ohStatus()`
/// / `#status` poll reads this every second — the browser-execution half
/// of the stub (the headless-Chrome CDP probe asserts on its content).
#[cfg(target_os = "emscripten")]
#[wasm_bindgen]
pub fn status() -> String {
    "tinyhexagon stub: the tiny runtime lands in plan 002 issue 07/08".into()
}

/// The stub's session flush (issue 03c): the page's flush calls this
/// (visibility/pagehide); the stub has no storage, so it is a no-op.
#[cfg(target_os = "emscripten")]
#[wasm_bindgen]
pub fn save_session() {}

fn main() {}
