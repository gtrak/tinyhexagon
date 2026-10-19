//! `th-web` — the emscripten (web) entry crate (plan 002 issue 08a):
//! the tinyhexagon tiny runtime's WEB entry — the shared `th_app::tiny`
//! boot over the preloaded `_RELEASE` MEMFS tree, the `window.ohStatus`
//! / `#status` probe hook (the real game's status line) + the
//! first-gesture `resumeAudio()` export (the 08 shell's audio unlock).
//! No IME, no text entry, no IDBFS / persistence (the tiny runtime
//! persists nothing — the read-only `_RELEASE` content stays in
//! MEMFS; the 03c stub's `save_session()` no-op export retired with
//! the persistence machinery).
//!
//! The host `main` stays a no-op (the desktop gate compiles the crate;
//! the real target is `wasm32-unknown-emscripten`).

#[cfg(target_os = "emscripten")]
mod web;

// The 26c.2 trap fix (MANDATORY — the root `.cargo/config.toml`'s
// `wasm32-unknown-emscripten` recipe note): with `-sWASM_BINDGEN` the
// wasm-bindgen glue registers Rust `main` as an `addOnInit` INIT
// FUNCTION, which emscripten runs BEFORE its own `wasmExports['__wasm_call_ctors']()`
// call (the 26c.1/26c.2 root cause — the wasm-bindgen INIT-FUNC-ORDER
// discovery). So `main` must call the C++ static initializers
// explicitly first (Luau's statics — a zero-initialized `blockSize`
// gives a SILENT divide-by-zero wasm trap in `newclasspage` without
// this, the 26c.1 operator symptom), and the web boot runs AFTER.
// (`///` doc comments here are an `unused_doc_comments` warning — `extern`
// blocks take no docs; the gate runs clippy with `-D warnings`.)
#[cfg(target_os = "emscripten")]
extern "C" {
    fn __wasm_call_ctors();
}

fn main() {
    #[cfg(target_os = "emscripten")]
    unsafe {
        __wasm_call_ctors()
    };

    // The web boot (the shared tiny entry's emscripten half —
    // `th_app::tiny::tiny_web`: the `_RELEASE` composition over the
    // page's canvas, the `web_fullscreen` sinks, the rAF loop). RETURNS
    // after scheduling the first frame (the browser owns the loop — the
    // 26c D-029 shape); the loop's self-owning closure keeps the engine
    // + the heap-pinned Lua context alive across `main` returning.
    #[cfg(target_os = "emscripten")]
    web::boot();
}
