# Issue 002a report — strip `oh-app` to a headless library

Worker: plan 002, issue 02a (`.agents/tasks/plan-002/issue-002a-impl.md`).
Umbrella: `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`.
HWM at dispatch: `4c31561ac6409e24a1b838043a0bb747f504045d`.

## Result

`oh-app` is now the headless composition library the tiny runtime builds
on. The full gate is green (4/4 stages, 897 tests passed / 0 failed).
Retired crates were NOT removed (02b); `oh-game`/`oh-lua` were not
de-scoped; nothing was renamed. The forked review returned
`VERDICT: PASS` (P2 doc-comment notes deferred to 02b/09); the
orchestrator commits this report with issue 02a.

## `oh-app` module keep-list / removed modules

Kept (final `lib.rs`):

| Module | Gate | Notes |
|---|---|---|
| `bridge` | always | unchanged |
| `camera` | always | unchanged |
| `compose` | always | unchanged signatures (engine de-scope is 02b) |
| `draw_target` | always | NEW — extracted from `windowed.rs` (below) |
| `lifecycle` | always | unchanged |
| `sigint` | `not(emscripten)` | unchanged |
| `joystick` | `not(android) and not(wasm)` | unchanged — still implements `oh_menu::seams::MenuJoystick` (`RealMenuJoystick`), so the `oh-menu` dep is KEPT per the spec's conditional (02b must strip it with the menu crate) |
| `web_fullscreen` | `emscripten` | NEW — salvaged from `oh-additions` (below) |

Removed: `android`, `args`, `client`, `dispatch`, `game_state`,
`lua_console_ui`, `menu_draw`, `print_lua_docs`, `replay_store`, `steam`,
`windowed`, the `oh_additions::touch_overlay` re-export, and the
`open-hexagon` binary (`src/main.rs` + the `[[bin]]` section). The crate
doc was rewritten to the short tiny-runtime composition description.

## `draw_target.rs` public API and copied helpers

`crates/oh-app/src/draw_target.rs` (explicit imports: `std::sync::Arc`,
`oh_math::ssvs::Vec2`, `oh_render::{batching::GlBatchUploader,
draw::{BlendMode, RenderStates, Vertex}, font::Text,
graphics::DrawTarget, pack_shaders::PackShaderProvider,
texture::Sprite, types::RectangleShape, view::View, GlParts}` — no
`glow`/`oh_menu`/`oh_config` needed; the GL context flows through
`GlParts`):

- `pub struct GlDrawTarget<'a>` — fields `parts: &'a GlParts`, `view:
  View`, `provider: Option<Arc<dyn PackShaderProvider>>`; inherent
  `GlDrawTarget::new(&'a GlParts)` (initial view = centre
  `(w/2, h/2)`, size `(w, h)` — the window's pixel space) and
  `GlDrawTarget::with_provider(Arc<dyn PackShaderProvider>)`; full
  `impl DrawTarget for GlDrawTarget<'_>` including `present()`. Private
  helpers copied verbatim: `draw_vertices_with_pack_shader` +
  `draw_text_with_pack_shader` (the 029n1 K6 shader-bound halves). Public
  API unchanged from `windowed::GlDrawTarget`.
- `pub struct AssetsPackShaderProvider` (the `PackShaderProvider` over
  `Arc<oh_assets::Assets>`) — COPIED into `draw_target.rs` too: it is a
  public item of the deleted `windowed` module and
  `examples/pack_shaders_gl.rs` (the 029n1 K6 GL verification example)
  constructs it for `GlDrawTarget::with_provider`.
- `pub struct NullDrawTarget` — the no-op `DrawTarget` fallback, copied
  with it (public family member of the deleted module; `oh-render`'s
  docs reference it by name as the headless degradation).

No windowed loop, menu adapter, client, or event-handling code was copied.

## `web_fullscreen.rs` path / imports / web-sys features

- Path: `crates/oh-app/src/web_fullscreen.rs`, module declared as
  `#[cfg(target_os = "emscripten")] pub mod web_fullscreen;`.
- Salvaged verbatim from `crates/oh-additions/src/fullscreen.rs`
  (158 lines, item-level `#[cfg(target_os = "emscripten")]` gates kept);
  only the module doc gained a salvage note. API kept:
  `pub fn install_sinks(window: &mut oh_render::window::Window,
  config: &Arc<RwLock<oh_config::Config>>)`. Imports as used:
  `oh_config::Config`, `oh_render::window::Window`, `std::sync` — plus
  `oh_game::log_sink::emit_log` (the `[oh-web-fs]` wiring-evidence lines;
  `oh-game` stays a dep). No caller yet — Issue 08 wires the `#fs`
  element + the transition edge.
- `oh-app`'s emscripten `web-sys` features (checked against the source):
  `Window`, `Document`, `Element`, `Event`. The removed
  `HtmlInputElement`/`HtmlElement`/`FocusOptions` belonged to the
  deleted `windowed` soft-input wiring (`oh-additions::soft_input`
  carries its own copy of those features — the additions crate still
  exists until 02b).

## Android / web stub state

- `oh-android`: `src/android.rs` deleted; `src/lib.rs` is now a doc-only
  stub (empty cdylib); the `[target.'cfg(target_os = "android")']`
  `oh-app`/`winit` dependencies removed; `[lib]` (name `oh_android`,
  `crate-type = ["cdylib"]`) + ALL cargo-apk metadata (package
  `info.vittorioromeo.openhexagon`, `apk_name = "open-hexagon"`,
  `build_targets = ["aarch64-linux-android"]`, `assets =
  "../../target/oh-apk-assets"`, minSdk 28 / target 34, the fullscreen
  `NativeActivity` activity block) kept for Issue 08.
- `oh-web`: `src/main.rs` replaced with a no-op `fn main() {}` (+ a doc
  note that the real entry lands in Issue 08); `src/web.rs` deleted;
  the emscripten target deps (`oh-app`/`oh-game`/`wasm-bindgen`/
  `js-sys`/`web-sys`) removed (Issue 08 re-adds the real ones); `site/`
  kept (the retired W5 shell page — its header comment was reworded to
  the retired-boot past tense; Issue 08 ships the new page).

## Removed dependencies / tests; adaptation notes

`oh-app/Cargo.toml` removed: `oh-additions`; non-emscripten `oh-net`,
`oh-steam`, `egui`, `egui-winit`, `egui_glow`, `glow 0.17`, `winit`;
android `ndk`; features `steam` + `produce-lua-metadata`; dev-dep
`tempfile` (only the deleted `replay_store`/`dispatch` tests used it);
the `[[bin]] open-hexagon` target. Kept per the spec keep-list:
`oh-game`, `oh-render`, `oh-config`, `oh-math`, `oh-data`, `oh-lua`,
`oh-assets`, `oh-replay`, `oh-discord`, `oh-audio`, `oh-rng`, `oh-menu`
(joystick's `MenuJoystick`), `libc` + `signal-hook` (sigint), `gilrs`
(joystick), `web-sys` (emscripten). Note: no kept module imports
`oh-audio` in code (the concrete `oh_audio::Audio` is handed into
`compose_with_config` by the caller — Issue 07); kept per the spec's
keep list.

Tests: deleted `tests/windowed.rs`, `tests/headless_replay.rs`,
`tests/lua_console_ui.rs`, `tests/print_lua_docs.rs`,
`tests/profile_store.rs`. Kept `tests/bridge.rs` (7 tests) and
`tests/pack_shaders.rs` (9 tests) UNCHANGED (assertions not weakened).
`tests/common/mod.rs` pruned to what the kept tests use: dropped the
sandbox `sandbox_link`/`copy_dir_recursive` helpers (only the deleted
sandbox binaries used them) and stale binary references in the doc;
kept `compose`, `TestProfiles`, `release_dir`, `test_fonts`,
`DrawEvent`/`Recording`.

Adaptations (compile-driven, no assertion changes):

- `examples/pack_shaders_gl.rs`: import
  `oh_app::windowed::{AssetsPackShaderProvider, GlDrawTarget}` →
  `oh_app::draw_target::{...}` (the only consumer outside the deleted
  modules).
- `compose.rs`: one intra-doc link into the deleted
  `crate::replay_store` module converted to plain text; module doc now
  notes the binary's retirement.
- `README.md`: `cargo run -p oh-app` → `cargo build --workspace`
  (`oh-app` is a library; the `tinyhexagon` binary lands in Issue 07).
- Stub doc comments (`oh-android/src/lib.rs`, `oh-web/src/main.rs`,
  `oh-web/site/index.html`) reworded so no retired `oh_app::…` path is
  referenced.

Deviations: none from the spec. Judgment calls taken within the spec's
leeway: (a) `AssetsPackShaderProvider` + `NullDrawTarget` moved into
`draw_target.rs` instead of deleted (the kept example needs the former;
the latter is the DrawTarget family's fallback) — public API unchanged;
(b) `oh-web`'s now-unused emscripten target deps removed (the spec
allows "leave if harmless, or remove"); (c) `Event` web-sys feature kept
in the emscripten table as the spec's "at minimum" list names it
(the file itself uses Window/Document/Element; `Event` rides the
Issue-08 transition-edge listener).

## Gate result (full, un-piped)

```
== cargo fmt --all -- --check
GATE_EXIT=0
== cargo build --workspace
GATE_EXIT=0
== cargo clippy --workspace --all-targets -- -D warnings
GATE_EXIT=0
== cargo test --workspace
GATE_EXIT=0
log: target/gate.log
gate_rc=0
```

`cargo test --workspace` tail: 108 suites, `test result: ok` in every
one; 897 passed / 0 failed / 0 ignored. `oh-app` contribution: 11 unit
tests (camera/lifecycle/sigint), `tests/bridge.rs` 7,
`tests/pack_shaders.rs` 9. Spec verification block: `no-adult-bin`,
`draw_target.rs` present, `web_fullscreen.rs` present, `lib-clean`,
`platform-stubs-clean`, `gate_rc=0`.

Not run here (environment, not code): the emscripten / android target
compiles — this box has no emsdk (`emcc`/`em++` absent) and no NDK
`aarch64…-clang++`, so `luau0-src`'s C++ build script cannot run for
those targets. `web_fullscreen.rs` is a verbatim copy of
`oh-additions/src/fullscreen.rs`, which compiles for emscripten today
in the workspace with the same imports/superset features. Per the spec,
host green is the bar; target checks are Issue 08.

Residual / doc-only:

- `tools/package*.sh` still reference the retired `open-hexagon`
  binary / web zip (packaging is Issue 08/09 territory — untouched).
- Historical doc comments in `oh-menu`, `oh-render`, `oh-game` cite the
  retired `oh_app::windowed`/`menu_draw` items (doc-only; Issue 09
  docs sweep retargets them, per the issue-01 working agreement).

Committed by the orchestrator after the `VERDICT: PASS`.
