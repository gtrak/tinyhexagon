# Issue 02a — strip `oh-app` to a headless library

**Umbrella contract:** `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`
(this sub-issue is the app half; 02b removes the crates and de-scopes the
engine). Do not remove the retired crates yet; do not de-scope `oh-game` or
`oh-lua`; do not rename anything.

## Goal

Turn `oh-app` from the adult front-end into the small composition library the
tiny runtime will build on: keep `bridge`, `camera`, `compose`, `lifecycle`,
`sigint`, `joystick`, and the extracted `draw_target`; add the salvaged
`web_fullscreen`; delete the adult modules, binary, console, menu and replay
front-end. Stub `oh-android`/`oh-web` so the workspace still builds. The full
gate must be green.

## Read first

1. `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md` (umbrella)
2. `.agents/plans/002-tinyhexagon/PLAN.md` (architecture, issue index)
3. `crates/oh-app/src/lib.rs`, `crates/oh-app/Cargo.toml`, `crates/oh-app/tests/`
4. `crates/oh-app/src/windowed.rs` (the `GlDrawTarget` section and its
   helpers; the rest is adult loop/menu code)
5. `crates/oh-additions/src/fullscreen.rs` and its `Cargo.toml`
6. `crates/oh-android/src/{lib,android}.rs`,
   `crates/oh-android/Cargo.toml`
7. `crates/oh-web/src/{main,web}.rs`, `crates/oh-web/Cargo.toml`
8. `crates/oh-app/src/{compose,bridge,camera,lifecycle,sigint,joystick}.rs`

## Steps

### 1. Extract `draw_target.rs`

From `crates/oh-app/src/windowed.rs`, extract the reusable GL draw target:

- `pub struct GlDrawTarget<'a>` (starts around the `rg -n 'pub struct
  GlDrawTarget'` line; currently ~1416) with its inherent `impl`, the
  `impl DrawTarget for GlDrawTarget<'_>`, and every private helper/constant
  it needs (e.g. `RenderWindowSize` if referenced; copy only what the target
  actually uses).
- Put them in `crates/oh-app/src/draw_target.rs`; make imports explicit
  (`oh_math`, `oh_render`, `glow`, `std`), keep the public API
  (`GlDrawTarget::new`, the `DrawTarget` impl) unchanged.
- Add `pub mod draw_target;` to `lib.rs`.
- Do not copy the windowed loop, menu adapters, client or event handling.
- If `bridge.rs`/tests referenced `windowed::GlDrawTarget`, update them.

### 2. Salvage the web fullscreen install

- Copy `crates/oh-additions/src/fullscreen.rs` to
  `crates/oh-app/src/web_fullscreen.rs`.
- Adjust imports to `oh_config` / `oh_render`; the file is already
  `#[cfg(target_os = "emscripten")]`-gated. Keep its `install_sinks` API.
- Add `#[cfg(target_os = "emscripten")] pub mod web_fullscreen;` to `lib.rs`.
- Add the `web-sys` features the file uses (at least `Window`, `Document`,
  `Element`, `Event`; check the source) to `oh-app`'s emscripten dependency
  table. It is wired in Issue 08; no caller is needed now.

### 3. Strip `lib.rs`

Keep only:

- `pub mod bridge;`
- `pub mod camera;`
- `pub mod compose;`
- `pub mod draw_target;`
- `pub mod lifecycle;`
- `#[cfg(not(target_os = "emscripten"))] pub mod sigint;`
- `#[cfg(not(any(target_os = "android", target_family = "wasm")))] pub mod joystick;`
  (keep `oh-menu` only if `joystick.rs` still needs its `MenuJoystick` trait;
  otherwise strip that impl and the dep — 02b removes the menu crate anyway)
- `#[cfg(target_os = "emscripten")] pub mod web_fullscreen;`

Remove `android`, `args`, `client`, `dispatch`, `game_state`,
`lua_console_ui`, `menu_draw`, `print_lua_docs`, `replay_store`, `steam`,
`windowed`, and the `oh_additions::touch_overlay` re-export. Rewrite the crate
doc to a short tiny-runtime composition description.

### 4. Remove the adult binary and dependencies

- Delete `crates/oh-app/src/main.rs`; remove the `[[bin]]` section from
  `crates/oh-app/Cargo.toml`.
- Remove dependencies that no kept module uses: at minimum `oh-additions`,
  `egui`, `egui-winit`, `egui_glow`, `glow 0.17`, `oh-net`, `oh-steam`, and
  the `steam` / `produce-lua-metadata` features. Remove `winit`/`ndk` if the
  kept modules do not import them; keep `gilrs` (joystick), `signal-hook`
  (sigint), `libc` (sigint), `oh-menu` only if `joystick.rs` needs it, and
  the engine/render composition deps (`oh-game`, `oh-render`, `oh-config`,
  `oh-math`, `oh-data`, `oh-lua`, `oh-assets`, `oh-replay`, `oh-discord`,
  `oh-audio`, `oh-rng`). Let `cargo check`/`clippy` catch anything unused.
- Keep `web-sys` (emscripten) for `web_fullscreen`.
- If `compose.rs` or `lifecycle.rs` uses a removed module's type, adapt it
  minimally; do not de-scope the engine yet.

### 5. Tests

- Delete the adult test files: `lua_console_ui.rs`, `print_lua_docs.rs`,
  `profile_store.rs`, `windowed.rs`, `headless_replay.rs` (it drives the
  removed client). Keep `bridge.rs` and `pack_shaders.rs` if they compile;
  prune `tests/common/` to what the kept tests use.
- Do not weaken surviving assertions; delete obsolete tests rather than
  `#[ignore]` them.

### 6. Stub the platform entries

- `oh-android`: delete `src/android.rs` and the `mod android;` line; leave a
  short `src/lib.rs` (empty cdylib is fine). Remove the now-unused target
  dependencies on `oh-app`/`winit` from `Cargo.toml`; keep the cargo-apk
  metadata (package name, assets, SDK, activity) for Issue 08.
- `oh-web`: replace `src/main.rs` with a minimal no-op `fn main() {}` (with a
  doc comment that the real entry lands in Issue 08) and delete `src/web.rs`.
  Leave `Cargo.toml` target deps if harmless, or remove the now-unused
  `oh-app`/`oh-game`/wasm deps; Issue 08 re-adds the real ones. The host
  workspace must build.

### 7. README

The `cargo run -p oh-app` command no longer works (the binary is removed).
Change the README's build line to `cargo build -p oh-app` (library; the
`tinyhexagon` binary lands in Issue 07) or `cargo build --workspace`.

### 8. Gate

```sh
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tools/gate.sh
```

All green. If a kept test/API needs a tiny adaptation, record it.

## Verification

```sh
test ! -e crates/oh-app/src/main.rs && echo no-adult-bin
test -f crates/oh-app/src/draw_target.rs
test -f crates/oh-app/src/web_fullscreen.rs
rg -n 'pub mod (android|args|client|dispatch|game_state|lua_console_ui|menu_draw|print_lua_docs|replay_store|steam|windowed)' crates/oh-app/src/lib.rs || echo lib-clean
rg -n 'oh_app::(windowed|client)|oh_app::android' crates/oh-android crates/oh-web || echo platform-stubs-clean
bash tools/gate.sh; echo "gate_rc=$?"
git status --short
```

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-002a-report.md` with:

- The final `oh-app` module keep-list and removed modules.
- The `draw_target.rs` public API and copied helpers.
- The `web_fullscreen.rs` path/imports and web-sys features.
- The android/web stub state.
- Removed dependencies/tests, adaptation notes, any deviation.
- The full gate result and tail.
- "No commit made."

Update `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` with a `### 02a —
app headless (LANDED)` entry: keep-list, `draw_target` API, `web_fullscreen`
path, stub state, removed deps/tests, and anything 02b must preserve. Do not
mark Issue 02a LANDED in `PLAN.md`; the orchestrator does that at commit.

## Risks

- `windowed.rs` mixes the reusable target with adult UI; extract only the
  target and helpers, and compile after each deletion.
- `compose.rs` still carries the replay/profile/Steam/Discord seams; keep them
  until 02b. Do not accidentally change the engine's input/update order.
- The emscripten/android target checks are Issue 08; host green is the bar
  here. Note any target-only code left unwired.
- Do not remove the retired crates in this step; 02b does that once their
  consumers are gone.
