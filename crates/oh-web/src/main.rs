//! `oh-web` — the emscripten (web) entry crate (plan 002).
//!
//! Stub state after issue 02a: the adult web entry (`web::boot` — the
//! retired windowed-client composition over the preloaded `_RELEASE`
//! MEMFS tree) was retired with `oh-app`'s windowed front-end. This no-op
//! `main` keeps the host workspace green; the real web entry (the tiny
//! runtime's `request_animation_frame` loop + the audio-unlock +
//! `oh_app::web_fullscreen::install_sinks` wiring) lands in Issue 08,
//! which re-adds the emscripten target dependencies.

fn main() {}
