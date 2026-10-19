# Feed-forward — plan 002 (tinyhexagon)

Living document. After every landed issue, the orchestrator appends the
lessons and updates the affected later issue files and the next task spec.
This is how a fresh worker on a later issue inherits the earlier work without
re-reading the whole history.

## Rules

- Record only what changes later work: accepted deviations, final API
  signatures, renamed/removed files, new caveats, per-target behaviour, and
  shifts in the upstream mapping.
- Update the later issue file(s) in the same pass; do not leave a note here
  that contradicts a frozen issue file.
- The reviewer checks that the feed-forward was applied.
- Keep entries short and dated; link the issue/commit.

## Log

### 00 — upstream (LANDED)

- Baseline SHA / default branch:
  4c31561ac6409e24a1b838043a0bb747f504045d / master
- Remote verified: `upstream` = `https://github.com/gtrak/SSVOpenHexagon`
  (fetch/push); upstream HEAD == HWM == fork HEAD; delta 0.
- Mapping corrections: none. All 22 baseline crates accounted for:
  12 kept/repurposed (`oh-*` → `th-*`, android/web repurposed), 10 removed,
  1 new (`th-synth`).
- Open questions: none. `shellcheck` is unavailable in this environment; the
  script was checked with `bash -n` and synthetic-repo cases (behind/current,
  bad marker, missing remote).

### 01 — `.agents` history (LANDED)

- Archive path: `.agents/plans/archive/001-rust-port/` — the old plan
  files, `DECISIONS.md`, `PARITY.md`, `deviations.md`, `STATUS.md`,
  `ISSUE-019-REPORT.md`, `WORKER-HYGIENE.md`, `mapping/`, the `parity/`
  corpus, `audits/`, `evidence/`, `workflows/`, and every port-era task
  spec under `tasks/` (the pre-plan-002 review protocol restored there as
  `tasks/review-gate-port.md` from `4c31561a`). Nothing in the archive was
  edited; the plan-002 task files live in `.agents/tasks/plan-002/`.
- Gate stages (4, final result all green, gate rc 0): 1. `cargo fmt --all
  -- --check` 2. `cargo build --workspace` 3. `cargo clippy --workspace
  --all-targets -- -D warnings` 4. `cargo test --workspace` (1014 tests
  passed, 0 failed). The Lua-parity and additions-boundary stages are
  gone; the failing-stage-tail rule and `target/gate.log` are kept.
- Test files removed with the parity crate: all 12 under
  `crates/oh-parity/tests/` (`common/mod.rs`, `dialect_census.rs`,
  `gen_corpus.rs`, `lua51_compat_prelude.rs`, `lua51_snippet.lua`,
  `lua_engine_parity.rs`, `lua_vm_probes.rs`,
  `parity_cpp_fingerprints.rs`, `parity_level_runs.rs`,
  `parity_pack_ids.rs`, `parity_replay.rs`, `parity_rng.rs`) plus the
  crate's `src/lib.rs` + `Cargo.toml`; the lockfile dropped only the
  `oh-parity` entry (17 lines). No other test read a deleted path at
  runtime — oh-parity was the only corpus consumer.
- Reference fixes (doc comments only, no assertions weakened): the
  `crates/oh-parity/tests/lua51_compat_prelude.rs` citations in
  `crates/oh-lua/src/prelude.rs`, `crates/oh-lua/src/context.rs` and
  `crates/oh-lua/lua51_prelude.lua` (×2) now point at git history; the
  `lua51_prelude.lua` task-spec pointer retargeted to
  `.agents/plans/archive/001-rust-port/tasks/issue-030c-three-way-parity.md`.
- Deviations: none from the issue contract. `.git/modules/extlibs` state
  was removed by `git submodule deinit` itself; the 5 gitlinks were
  dropped from the index with `git rm --cached` so `git submodule status`
  is empty. Historical doc comments elsewhere in `crates/` that cite the
  pre-archive `.agents/plans/001-rust-port/...` or `tools/oracle/...`
  paths were left in place (history, not build inputs — working
  agreement); a docs sweep (Issue 09) can retarget them. `.github`
  workflows were not in scope; `ci.yml`'s gate job runs `tools/gate.sh`
  (now 4 stages) and is unaffected, though its header comments still
  describe the retired lua-parity/cpp-oracle stages; the active gate job
  already calls the four-stage `tools/gate.sh`, and the comment cleanup
  belongs to Issue 08/09.

### 02a — app headless (LANDED)

- `oh-app` keep-list (library only; the `open-hexagon` binary +
  `src/main.rs` are gone): `bridge`, `camera`, `compose` (signatures
  unchanged — the engine de-scope is 02b), `draw_target` (NEW, extracted
  from `windowed.rs`), `lifecycle`, `sigint` (not-emscripten),
  `joystick` (not android/wasm), `web_fullscreen` (emscripten-only;
  NO CALLER YET — wired in Issue 08). Removed: `android`, `args`,
  `client`, `dispatch`, `game_state`, `lua_console_ui`, `menu_draw`,
  `print_lua_docs`, `replay_store`, `steam`, `windowed`, the
  `oh_additions::touch_overlay` re-export. Crate doc rewritten to the
  tiny-runtime composition description. Gate 4/4 green, 897 passed / 0
  failed.
- `draw_target` API: `GlDrawTarget<'a>::new(&'a GlParts)` +
  `.with_provider(Arc<dyn PackShaderProvider>)` + the full `DrawTarget`
  impl (incl. `present`) — public API unchanged from
  `windowed::GlDrawTarget`; the 029n1 K6 pack-shader provider
  `AssetsPackShaderProvider` (over `Arc<oh_assets::Assets>`) and the
  no-op `NullDrawTarget` fallback moved into the same module (the kept
  `examples/pack_shaders_gl.rs` imports both from
  `oh_app::draw_target` now).
- `web_fullscreen` path: `crates/oh-app/src/web_fullscreen.rs`
  (`#[cfg(target_os = "emscripten")] pub mod web_fullscreen;`),
  `install_sinks(&mut oh_render::window::Window, &Arc<RwLock<oh_config::Config>>)`
  — verbatim salvage of `oh-additions/src/fullscreen.rs` (imports:
  `oh_config` / `oh_render` / `oh_game::log_sink::emit_log`); oh-app's
  emscripten web-sys features: `Window`, `Document`, `Element`, `Event`
  (the old `HtmlInputElement`/`HtmlElement`/`FocusOptions` died with the
  windowed soft-input wiring). 02b may delete the original
  `oh-additions/src/fullscreen.rs` — this copy is the survivor.
- Stub state: `oh-android` = doc-only empty cdylib (`android_main` + the
  `oh-app`/`winit` android target deps removed; ALL cargo-apk metadata
  kept: package name, assets, SDK 28/34, the fullscreen NativeActivity).
  `oh-web` = no-op `fn main() {}` (`src/web.rs` deleted; the emscripten
  target deps oh-app/oh-game/wasm-bindgen/js-sys/web-sys removed — Issue
  08 re-adds them; `site/` shell page kept, header reworded).
- Removed `oh-app` deps/tests: `oh-additions`, `oh-net`, `oh-steam`,
  `egui`, `egui-winit`, `egui_glow`, `glow 0.17`, `winit`, android
  `ndk`, the `steam` + `produce-lua-metadata` features, dev `tempfile`;
  tests `windowed.rs`, `headless_replay.rs`, `lua_console_ui.rs`,
  `print_lua_docs.rs`, `profile_store.rs` (the sandbox helpers in
  `tests/common` went with them). Kept `tests/bridge.rs` (7) +
  `tests/pack_shaders.rs` (9) unweakened. Kept deps despite no kept
  module importing them: `oh-audio` (Issue 07 hands `oh_audio::Audio`
  into the composition), `oh-replay` (tests), `oh-discord` (compose
  seam), `oh-menu` (see 02b note).
- 02b must preserve / watch:
  * `joystick.rs` still implements `oh_menu::seams::MenuJoystick`
    (`RealMenuJoystick` + its `Arc` share) — removing `oh-menu` in 02b
    must strip that impl + the `oh-menu` dep from `oh-app`.
  * `compose_with_config` still carries the engine's `steam` /
    `discord` / `profiles` / `replay_store` seams (the signatures are
    02b's to de-scope, together with `HexagonGame::new`).
  * `tests/bridge.rs` pins replay-seeded determinism through
    `HexagonGame::set_last_replay` + `oh_replay::ReplayFile` — 02b's
    replay de-scope must adapt or drop that half (do not weaken the
    pass-order / frametime / texture assertions).
  * The stubs' packaging metadata (oh-android) and no-op mains
    (oh-android/oh-web) are Issue 08's starting point; `web_fullscreen`
    is 08's first caller.
  * The engine's input/update order is UNCHANGED by 02a (no engine edit).
  * No target compiles were run for emscripten/android on this box (no
    emsdk / NDK C++ toolchains); host green is the bar — 08 re-checks
    the targets.

### 02 — crate removal / engine de-scope (pending)

- Final `HexagonGame::new` signature:
- Removed `GameApi` methods:
- `draw_target.rs` API:
- Lua globals removed/no-oped:
- RNG-order note:
- `th-app` keep-list / temporary stubs:

### 03 — rename (pending)

- Final crate names:
- Binary/artifact names:
- Rename exceptions:
- `default-members`:
- Tinyhexagon bin timing (renamed here or deferred to 07):

### 04 — engine kid changes (pending)

- Soft collision API/bump accessor:
- `set_difficulty_mult` range:
- `apply_music_beat` / `StyleData::apply_beat_hue`:
- `DrawOptions` shape:
- Wall-gap API:
- Tests added:

### 05 — synth (pending)

- Spike result (stock vs patched):
- Patch location:
- Director API:
- Stage/layer table:
- Web gesture hook:
- Target caveats:

### 06 — tiny pack (pending)

- Level ids / speeds / delays:
- First-pattern delay:
- Styles/hues:
- Lua globals no-oped:
- Load/run test:

### 07 — app tiny runtime (pending)

- Composition signature:
- Journey table:
- Touch mapping:
- SFX ids/volumes:
- Beat-hue call:
- Headless tests:
- No-storage proof:

### 08 — Android/web/packaging (pending)

- Android entry:
- Web boot/audio-unlock export:
- Artifact names:
- Target degradations:
- Pages URL:

### 09 — tuning/docs (pending)

- Final tunables:
- Playtest notes:
- Per-target deviations:
- Plan closed/archived:
