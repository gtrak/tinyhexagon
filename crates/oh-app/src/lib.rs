//! `oh-app` — the tiny-runtime composition library (plan 002 issue 02a).
//!
//! The C++ `HexagonGame::draw()` (`HGGraphics.cpp:52-316`) is already
//! ported as `oh_render::graphics::draw_frame`; nothing in `oh-game` may
//! call it (D-013/D-020: `oh-game` and `oh-render` deliberately do not
//! depend on each other — `oh-lua` depends on `oh-game`, so `oh-game`
//! cannot gain a render dependency, and `oh-render` is a consumer of
//! vertex data, not of the simulation). The composition — building
//! `FrameInput`/`DrawState` from the engine's state each frame and
//! calling `draw_frame` — therefore lives here, in the one crate that
//! depends on BOTH.
//!
//! The adult front-end (windowed loop, menu, console, replay client, the
//! `open-hexagon` binary) was stripped by plan 002 issue 02a; what remains
//! is the small composition library the tiny runtime (Issue 07) builds
//! on:
//!
//! * [`bridge`] — `AppDraw` (the per-frame draw state + scratch) and
//!   `draw()` (the `HexagonGame::draw` bridge over a `DrawTarget`);
//! * [`camera`] — the `oh_game::camera::Camera` → `oh_render::view::Camera`
//!   conversion (both port the same C++ `ssvs::Camera`);
//! * [`compose`] — the headless engine composition (the two-phase
//!   `HexagonGame` build over a real `oh-lua` `ScriptHost` + the real
//!   `oh_assets::Assets` registry);
//! * [`draw_target`] — the reusable GL draw target extracted from the
//!   retired windowed module (`GlDrawTarget` over a live `GlParts`, the
//!   no-op `NullDrawTarget`, the 029n1 (K6) pack-shader provider);
//! * [`lifecycle`] — the `mainClient` folder creation + config load /
//!   reapply (`main.cpp:275-291`; the config/profile exit scope guards
//!   were retired by plan 002 issue 02c1);
//! * [`sigint`] — the headless / server SIGINT handlers (desktop only;
//!   the web loop is a plain `request_animation_frame`);
//! * [`joystick`] — the desktop `RealJoystick` controller poller (gilrs;
//!   the Android/wasm targets keep the engine's no-op seam);
//! * [`web_fullscreen`] — the web's `#fs` click / change-sink install
//!   (emscripten only; the web entry wires it in Issue 08).

pub mod bridge;
pub mod camera;
pub mod compose;
pub mod draw_target;
#[cfg(not(any(target_os = "android", target_family = "wasm")))]
pub mod joystick;
pub mod lifecycle;
// 29o1 — the headless / server SIGINT handlers (`signal-hook`, which is
// gated to `not(emscripten)` in this crate's Cargo.toml — the web loop
// is a plain `request_animation_frame`, no signal handler).
#[cfg(not(target_os = "emscripten"))]
pub mod sigint;
// The web's `#fs` sink install (salvaged from `oh-additions` by issue
// 02a; no caller yet — the web entry lands in Issue 08).
#[cfg(target_os = "emscripten")]
pub mod web_fullscreen;
