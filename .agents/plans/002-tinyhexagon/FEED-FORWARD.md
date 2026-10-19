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
  unchanged — the engine de-scope is 02c), `draw_target` (NEW, extracted
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

### 02b — front-end/online crates (LANDED)

- Removed 7 workspace members + directories: `oh-menu`,
  `oh-menu-system`, `oh-additions`, `oh-net`, `oh-steam`,
  `oh-server-control`, `oh-workshop-uploader` (14 crates remain).
  Lockfile: −189 lines, 0 added — the seven entries + their exclusive
  transitive deps (`steamworks`(-sys), `sodiumoxide`, `libsodium-sys`,
  `rusqlite`, `libsqlite3-sys`, `hashlink`,
  `fallible-iterator`, `fallible-streaming-iterator`, `ed25519`,
  `signature`, `paste`; `hashbrown` survives via `fontdue`/`indexmap`).
  The root `[workspace.dependencies]` pins
  `steamworks`/`rusqlite`/`sodiumoxide` went with them (the deleted
  crates were the only consumers); `tempfile` stays (oh-replay's 29m1
  dev-dep) with its comment retargeted to oh-replay.
- `oh-app`: the `oh-menu` dep is gone; `joystick.rs` no longer
  implements `oh_menu::seams::MenuJoystick` — `RealMenuJoystick`,
  `impl MenuJoystick` and the now-dead `RealJoystick::pressed_focus` /
  `::rising_jdir` are removed. The engine's `Joystick` seam +
  `RealJoystick` (gilrs backend, `oh_config::joystick` machine
  feeding) are UNCHANGED; zero engine edits in 02b.
- `oh-replay`/`oh-discord` stay until 02c: the engine's `ReplayStore` /
  `SteamSink` / `Discord` / `Profiles` seams and the
  `compose_with_config` signature still carry them; oh-app
  `tests/bridge.rs` still pins replay-seeded determinism through
  `HexagonGame::set_last_replay` + `oh_replay::ReplayFile` — 02c must
  adapt or drop that half WITHOUT weakening the pass-order / frametime
  / texture assertions.
- Doc-only residue (02c/09 sweep): `oh_steam::SteamManager` mentions in
  oh-game `seams.rs`/`hexagon_game.rs`, oh-app `compose.rs` (the `steam`
  arg), oh-assets `assets.rs` (workshop surface); `oh-workshop-uploader`
  pattern citations in oh-discord docs; the "salvaged from
  `oh-additions`" notes in oh-app `lib.rs`/`web_fullscreen.rs`/
  `Cargo.toml` + oh-render `texture.rs`; tools/*.sh + .github still name
  the retired binaries (08/09). No live import/dep on any removed
  crate remains anywhere.
- Gate 4/4 green (fmt/build/clippy/test): 68 suites, 569 passed / 0
  failed / 0 ignored (02a was 108 / 897 — the 40 removed suites / 328
  tests were the seven crates'). Host-only again: no emscripten/android
  target compiles (no toolchains); the `oh-android`/`oh-web` stubs are
  untouched and depend on none of the removed crates — 08 re-checks the
  targets.

### 02c1 — Steam/Discord/profiles/PB (LANDED)

- Removed: `oh-discord` (crate + root member + `oh-game`/`oh-app` deps);
  the `SteamSink`/`NoneSteamSink` seam, `steam_manager` field/arg,
  rich-presence calls, `a22_swapdeath` branches, `discord` field/arg,
  `discord_attempt`/`discord_hung`, `time_until_rich_presence_update`,
  `death_update_rich_presence`, all `set_rich_presence_*`; the
  `Profiles`/`NullProfiles` seam + `RealProfiles` + `profiles` field/arg;
  `should_save_score` / `death_save_score_if_needed` /
  `death_save_score_if_needed_and_show_pb_effects`; the PB effect state
  (`must_spawn_pb_particles`, `next_pb_particle_spawn`, `pb_text_growth`,
  `pb_text`) + accessors; the profile-active restart/menu-escape branch;
  `oh-app` `ConfigSaveGuard`/`ProfileSaveGuard`.
  `score` / `custom_score` / `score_invalid` + the level-status score
  override are UNTOUCHED.
- Final signatures (02c2 still to de-scope replay):
  `HexagonGame::new(config: Arc<RwLock<Config>>, assets,
  audio: Option<Arc<dyn Audio + 'static>>,
  replay_store: Arc<dyn ReplayStore + 'static>,
  script_host: Arc<dyn ScriptHost + 'static>,
  joystick: Arc<dyn Joystick + 'static>,
  cosmetic_rng: Arc<dyn CosmeticRng + 'static>, window: bool,
  renderer: Option<Arc<dyn Renderer + 'static>>) -> Self` —
  `steam_manager`/`profiles`/`discord` gone.
  `oh_app::compose::compose_with(assets, root, window)` and
  `compose_with_config(assets, root, window, config,
  renderer: Option<Arc<dyn Renderer + 'static>>,
  audio: Option<Arc<dyn Audio + 'static>>,
  replay_store: Arc<dyn ReplayStore + 'static>,
  joystick: Arc<dyn Joystick + 'static>) -> Composed` —
  `profiles`/`steam`/`discord` args gone.
  Remaining `seams.rs`: `Audio`/`NoAudio`, `ReplayStore`/`NullReplayStore`,
  `ScriptError`/`ScriptHost`/`NoScriptHost`, `Joystick`/`NoJoystick`,
  `CosmeticRng`/`PcgCosmeticRng`.
- Removed `GameApi` method: `unlock_achievement` (the `GameApi` mock impls
  in `oh-game` `lua_state.rs`/`lua_console.rs`, `oh-game/tests/common`
  and `oh-lua/tests/api.rs` all follow). `GameApi::in_replay` KEPT (02c2
  removes it with replay).
- Lua globals: `steam_unlockAchievement` removed (the `oh-lua` `api/steam.rs`
  module + `mod steam` registration + menu no-op entry + metadata);
  registered-global count 285 → 284 (the `284` doc references in oh-lua
  are current).
- Replay-name fallback (recorded): `new_game`'s last-replay path records
  `String::new()` (the C++ `no_profile`-equivalent headless state); `death_create_replay_file`
  records `"no_profile"` always. `determinism.rs`'s `assert_replay_pins`
  player_name pin re-based to `"no_profile"`; its `stream_fixture()`
  serialization fixture still carries `"testProfile"` (byte-oracle data,
  not a seam).
- RNG-order note: the removed PB-particle spawn drew 7× `CosmeticRng` per
  spawn (x/rotation/scale/alpha + velocity/angular draws, every 2.75 s
  while due) — those draws no longer occur. Surviving cosmetic draws:
  swap/kill particles, the random music-segment pick, draw-time
  camera-shake jitter. Simulation RNG + input/update order UNCHANGED.
- `oh-app` lifecycle no-persistence state: public API is exactly
  `create_folder_if_non_existant` + `load_and_reapply`; no config/
  profile writes anywhere in `oh-app`.
- ACCEPTED DEVIATION (supervisor ruling 2026-10-24): `oh-app/src/bridge.rs`
  `AppDraw::new` keeps two inert `DrawState` initializers —
  `pb_text: graphics::init_text(&font_bold, "", 65.0, zoom)` +
  `pb_text_growth: 0.0` (lines 198-199). `oh_render::graphics::DrawState`
  (02c3's scope) still carries both fields with no field-skipping
  constructor, so the exhaustive literal must name them; dead defaults
  until 02c3. CONSEQUENCE: `rg pb_text crates/oh-app/src` is expected to
  hit EXACTLY those two lines until 02c3 — do not "fix" them in 02c2.
  02c3 must remove the two lines + the `DrawState` fields + the
  `oh-render` `text_draw`/`graphics` PB residue together.
- 02c3/09: `oh-assets` still loads/saves profiles (`Assets::profile_save_all` /
  `profile_set_current`, and the loader reads `Profiles/`); no engine/app
  caller remains. Dead-surface cleanup, not a 02c1 seam.
- Doc-comment residue (02c2/02c3/09 sweep): the `oh-lua` `api/mod.rs`
  module doc names `steam_unlockAchievement` (C++ group note); the
  `oh-app` `lifecycle.rs` docs mention the retired `Profiles/` folder
  (one quotes the C++ log-message string the generic helper mirrors).
- Removed tests: `oh-game/tests/discord_retry.rs` +
  `oh-game/tests/pb_particle_rng_order.rs` (whole files); the
  `go_to_menu_score.rs` no-profile death/restart `go_to_menu` test +
  `NoLocalProfiles` fixture; the `oh-app` lifecycle save-guard test
  modules (folder test retargeted to neutral `Data/`/`Packs/` folders);
  the `oh-app/tests/bridge.rs` PB text-size assertion.
  Surviving deterministic score/timing assertions unweakened.
- For 02c2 (remove replay): `oh-replay` + `ReplayStore`/
  `NullReplayStore` + `ActiveReplay` + `in_replay` +
  `last_replay_data` + the replay record/playback branches +
  `set_last_replay` + `save_replay` (gate now
  `!in_replay() && !status.has_died`) + `death_create_replay_file` /
  `death_send_and_save_replay` + `new_game`'s `execute_last_replay` param
  all still present; `oh-app` `tests/bridge.rs` still pins
  replay-seeded determinism through `set_last_replay` +
  `oh_replay::ReplayFile` (adapt or drop that half WITHOUT weakening the
  pass-order / frametime / texture assertions); `GameApi::in_replay` +
  the `new_game`/`death` replay-name fallbacks go with it.
- For 02c3 (fingerprint/RNG + final engine cleanup): the 030b
  fingerprint path (`fingerprint_file`/`fingerprint_frame`/
  `fingerprint_last_draws` in `hexagon_game.rs`, OFF by default) + the
  `oh-render` `DrawState` PB fields (see accepted deviation) + the doc
  residue above.
- Gate 4/4 green (fmt/build/clippy/test): 63 suites, 552 passed / 0
  failed / 1 ignored (the pre-existing `oh-lua` sandbox doc-test
  ignore). 02b's tail was 569 passed; the 17-test delta is the removed
  Discord/PB/lifecycle/PB-text/Steam coverage. `Cargo.lock`
  regenerated (`oh-discord` entry gone). Host-only again (no
  emscripten/android toolchains on this box — 08 re-checks the targets).

### 02c2 — replay (LANDED)

- Removed `oh-replay` (crate + root member + `oh-game`/`oh-app` deps;
  35 tests) and the replay surface: `ReplayStore`/`NullReplayStore`/
  `CompressedReplayFile`, `ReplayFile`/`ReplayData`/`ActiveReplay`, the
  replay cursor/`last_replay_data`/`last_first_play`/`last_played_score`/
  `active_replay`/`replay_store`, `set_last_replay`/`in_replay`/
  `must_replay_input`/`must_show_replay_ui`/`save_replay`/
  `death_create_replay_file`/`death_send_and_save_replay`, the `update.rs`
  replay input/recording branches, `lua_console` replay state,
  `GameApi::in_replay`, and `new_game`'s `execute_last_replay`.
- Final signatures:
  `HexagonGame::new(config, assets, audio, script_host, joystick,
  cosmetic_rng, window, renderer)` (8 args; `replay_store` gone);
  `new_game(pack_id, id, first_play, difficulty_mult)`;
  `compose_with(assets, root, window)` and
  `compose_with_config(assets, root, window, config, renderer, audio,
  joystick)` (7 args; `replay_store` gone).
- Fingerprint: the 030b path stays for 02c3; its replay-sourced
  `last_played_score` byte was removed (off-by-default parity debug only).
- Replay UI: engine side gone; `oh-render` `DrawState` still carries
  `replay_icon`/`replay_text`/`must_show_replay_ui`, and `bridge.rs` keeps
  the corresponding inert initializers (`must_show_replay_ui: false`) — the
  02c3 render sweep removes both, like the PB fields.
- Tests: 552 → 511 passed (35 `oh-replay` + 2 `go_to_menu_score` + 4
  replay-determinism); `oh-app/tests/bridge.rs` keeps 7 tests and injects
  `SEED = 0` directly into `state_handle().rng` for its two live runs; no
  surviving assertion weakened. Two basis notes: the D-019 determinism
  survivor is now self-consistent (direct seed) rather than
  C++-oracle-verified; the live `go_to_menu`/`send_scores` path keeps no test
  after its replay-store instrument was deleted — 02c3/07 decides remove vs.
  a non-replay assertion.
- `flate2`/`tempfile` workspace pins removed (only `oh-replay` used them);
  `Cargo.lock` regenerated.
- Gate 4/4 green (orchestrator re-ran; worker timed out after the gate while
  writing the report).
- For 02c3: remove the `oh-render` replay + PB `DrawState` fields, the
  `bridge.rs` inert initializers (`replay_icon`/`replay_text`/`pb_text`/
  `pb_text_growth`/`must_show_replay_ui: false`), the fingerprint/RNG path,
  and the replay/PB/steam doc residue.

### 02c3 — final engine cleanup (LANDED)

- Fingerprint (030b) removed: the `EngineState` `fingerprint_*` fields +
  `set_fingerprint_path` + the `new_game` reset, `maybe_emit_fingerprint`
  + the FNV-1a builder in `update.rs`, the `lua_console` stand-in init, and
  the 030b-only accessors with no other caller (`SpeedData::min/max/
  ping_pong`, `CCustomWallManager::alive_handles`, `CWall::get_hue_mod`).
  `oh-rng` lost the parity-only `state_position` (wrapper + `Pcg32Fast`),
  `distance_from_wrapped` and the wrapper `draws` counter; `engine_state()`
  (raw MCG state) + `Pcg32Fast::state()` (the C++ `engine()` mirror) stay.
- `oh-render` `DrawState`/`FrameInput` replay + PB fields removed
  (`replay_icon`/`replay_text`/`pb_text`/`pb_text_growth`/
  `must_show_replay_ui`) with every draw/update path that used them
  (`init_key_icons`/`update_key_icons`/`draw_key_icons` replay arms, the
  `draw_frame` gate conjuncts, `update_text` PB sawtooth + replay content,
  `update_text_sizes` PB/replay sizes, `draw_text_time_and_status` replay
  arm, `draw_text_personal_best`, `TextUpdateInput` replay + dead `ft`
  fields); the surviving time/status/fps/message sizing is bit-identical.
  The inert `oh-app` bridge initializers + `must_show_replay_ui: false`
  went with the fields.
- `go_to_menu` coverage: the path is live (the bridge Lippincott error
  paths + the C++-facing callers pinned by `score_override_error.rs` /
  `joystick.rs`), so it was kept and covered — new
  `oh-game/tests/go_to_menu_callback.rs` (2 tests): `trigger_go_to_menu()`
  fires the callback with `mError = false`, `go_to_menu(false, true)` with
  `mError = true`. The K7 single-stream test (`render_hook_rng_stream.rs`)
  now pins through the raw `engine_state()` (same position/attribution
  pins, no assertion weakened).
- `oh-assets` profile surface: REMOVED (no live `oh-game`/`oh-app`
  caller — only its own test file consumed it): the `Profiles/` load step,
  `ProfileStore`, and the 16 public read/write methods; `oh-data
  ::ProfileData` stays in `oh-data` (the C++'s singular `profileData`
  high-score store was never ported into `oh-assets` — the spec's
  keep-list items do not exist in this tree; if a menu profile UI is
  ever ported, Issue 09+ rebuilds it). Tests: 511 → 501 passed (−9
  profiles, −2 oh-rng state-position, −1 PB pulse; +2 go_to_menu).
- Gate 4/4 green (fmt / build / clippy -D warnings / test).
- Review P2 residue for Issue 09: the dead `oh-render` PB-particle update
  surface (`PBParticleUpdateParams`/`make_pb_particle`/spawn branch/export/
  test), the dead `oh-assets` `WorkshopPacks`/`new_with_workshop` seam (docs
  name the removed `oh_steam::SteamManager`), and stale replay/PB/steam doc
  references (`mustShowReplayUI`, `drawTextMessagePBImpl`, the 02c1 `mFT`
  note, etc.).
- Committed by the orchestrator after the forked PASS.

### 02 — crate removal / engine de-scope (02c1/02c2/02c3 LANDED)

- 02c2 (remove replay; remove `oh-replay`): see §02c1's "For 02c2"
  note — the replay surface, `GameApi::in_replay` and the
  replay-name fallbacks are 02c2's removal list.
- 02c3 (remove the parity fingerprint/RNG + final engine cleanup):
  landed — see §02c3. The engine cleanup phase is complete; Issue 03
  (rename) is next.
- `draw_target.rs` API / `th-app` keep-list: landed in 02a/02b (see
  those sections); the composition signatures moved again in 02c1 (see
  §02c1).

### 03 — rename (LANDED)

- Final crate names: the 12 kept crates are `th-math`, `th-rng`,
  `th-data`, `th-config`, `th-lua`, `th-assets`, `th-game`, `th-audio`,
  `th-render`, `th-app`, `th-android`, `th-web` (directories, manifests,
  `th_*` identifiers, tools, CI all renamed; `cargo metadata` lists zero
  `oh-*` packages; `Cargo.lock` regenerated).
- Binary/artifact names: desktop bin `tinyhexagon` (the `th-app`
  `[[bin]]`, `crates/th-app/src/bin/tinyhexagon.rs`); `th-web` carries
  `[[bin]] name = "tinyhexagon" path = "src/main.rs"` so the wasm output is
  `tinyhexagon.js` / `tinyhexagon.wasm` (+ `tinyhexagon.data` in deps/ —
  the page import, `tools/package-web.sh` staging/zip and
  `tools/build-wasm-emsdk.sh` all use those names; the inner web zip is
  `tinyhexagon-web.zip`); `th-android`: `apk_name = "tinyhexagon"`,
  `[lib] name = "th_android"` (`libth_android.so` — `package-android.sh`
  checks + manifest comments updated), app `label = "tinyhexagon"`,
  staging dir `target/th-apk-assets`; the Android `package` id stays
  `info.vittorioromeo.openhexagon` (the plan does not rename it).
  Release artifacts: `tinyhexagon-<tag>-{linux-x86_64.tar.gz,
  windows-x86_64.zip, android-arm64-v8a.apk, web.zip}` (both workflows).
- Rename exceptions (left as `oh-*`, 22 doc-only lines — see the report's
  exception table): removed-crate citations `oh-net` (×3), `oh-steam` (×6)
  + `oh_steam` (×1), `oh-discord` (×1), `oh-server-control` / `oh-
  workshop-uploader` (×2, `tools/package.sh` header), `oh-additions` (×6,
  the "salvaged from" notes); `oh-lanes` (the EXTERNAL lane dir
  `/home/gary/dev/oh-lanes` in `build-wasm-emsdk.sh`'s podman example —
  not a repo path); plus pattern-invisible names kept by design:
  `SSVOpenHexagon` strings, `"Open Hexagon"`, `ohr.z` / `ohdb.sqlite` /
  `ohvrvanilla_*`, `oh.filelist` (APK asset marker), `OH_*` env vars,
  the page-JS handles `window.ohStatus` / `ohWebModule` /
  `ohWebPersistBoot` / `ohstatechange` (the probes rely on them),
  `tools/android/openhexagon-release.keystore` (live keystore identity),
  and the stale `.gitignore` rule `crates/oh-parity/oh030b-corpus/`
  (inert; 09 sweep candidate).
- `default-members`: `["crates/th-app"]` (root `Cargo.toml`) — `cargo run`
  runs the stub, exit 0.
- Tinyhexagon bin timing: the STUB lands NOW (issue 03, per the spec's
  decision — a one-line `tinyhexagon` bin in `th-app` prints
  "the tiny runtime lands in plan 002 Issue 07" and exits 0); Issue 07
  replaces `src/bin/tinyhexagon.rs` in place (same bin name/path). Known
  cosmetic consequence: on the host target dir the `th-app` and `th-web`
  bins both named `tinyhexagon` collide — cargo prints an "output filename
  collision" WARNING (not an error) during `--workspace` builds; `cargo
  run` is unaffected (explicit unit). If 08 needs the host no-op th-web bin
  gone, drop it there (the wasm bin name is what the artifacts need).
- CI: job IDs + dependency edges UNCHANGED (ci.yml: gate / windows-cross /
  android / wasm / pages; release.yml: gate / linux / windows / android /
  wasm / release / gh-pages). Pages URL is
  `https://gtrak.github.io/tinyhexagon/` (the GitHub repo is
  `gtrak/tinyhexagon` — `git remote` verified; the two
  `github.repository == 'gtrak/SSVOpenHexagon'` fork-guards in ci.yml are
  now `'gtrak/tinyhexagon'` — the old value would have silently disabled
  the wasm artifact upload + pages jobs). `CODEOWNERS` needs no change
  (no crate names). Log prefixes / CLI flag renamed consistently:
  `[th-render]` / `[th-web-fs]` / `[th-audio]`, `--th-desktop-mode-probe`
  (`th_app::desktop_mode::PROBE_ARG` + `tools/smoke-windowed.sh`).

### 04 — engine kid changes (04a LANDED, 04b pending)

- Soft collision API/bump accessor: `set_soft_collisions` /
  `soft_collisions`; `bump_serial() -> u64` — one increment per killed
  wall + `flash_effect = max(·, 120.0)`; see the §04a log entry.
- `set_difficulty_mult` range: `clamp(0.1, 4.0)` on the shared
  `difficultyMult` (all existing speed/delay/style paths read it).
- `apply_music_beat` (final, 04b): `HexagonGame::apply_music_beat(
  beat_phase: f32, bar_phase: f32, energy: f32, base_hue: f32)` — stores
  `EngineState.beat_hue: Option<f32>` / `beat_energy` (±1.5° deterministic
  shimmer on the beat/bar sines over the app's base hue), sets
  `status.beat_pulse = energy` + re-arms `beat_pulse_delay`; applied to
  the style in `update` AFTER `style_data.update(...)` (same frame);
  `status.pulse` (simulation/RNG) untouched. `StyleData::apply_beat_hue(
  f32)` / `apply_beat_pulse(f32)` (clamp to the style's hue/pulse ranges,
  unordered-bounds clamp safe for wrapping styles, `compute_colors`
  inside). See §04b.
- `DrawOptions` shape (final, 04b): `th_render::graphics::DrawOptions
  { show_text, show_level_info, show_key_icons, show_fps: bool }`
  (`Clone, Copy, Debug, PartialEq, Eq`; `Default` = all true = the legacy
  rendering). `FrameInput.draw_options` gates the draw-side passes in
  `draw_frame`; `AppDraw::set_draw_options` + the update-side
  `update_key_icons` / `update_level_info` / `update_text_sizes` gates
  live in the `th-app` bridge. The kid app (07) sets all false. See §04b.
- Wall-gap API: `WallGap { center_angle, half_width_angle, radius }` +
  pure `compute_wall_gaps(&[CWall])` (`th-game::wall`, crate-root
  re-export) + `HexagonGame::wall_gaps()` /
  `nearest_gap_angle(player_angle) -> Option<f32>`; see §04a.
- Tests added: `tests/soft_collisions.rs` (9) + `tests/wall_gaps.rs`
  (9); 509 → 527 passed (04a; the 1 ignored is the pre-existing th-lua
  sandbox doc-test). 04b added 12 more (→ 539): `tests/music_beat.rs`
  (4), `th-data` style beat/fmod (4), `th-render` draw_options (3),
  `th-app` bridge plumbing (1).

### 04a — engine kid mechanics (LANDED)

- API (final): `HexagonGame::set_soft_collisions(bool)` /
  `soft_collisions()` / `set_difficulty_mult(f32)` (writes the shared
  `difficultyMult`, clamped `0.1..=4.0`) / `bump_serial() -> u64` /
  `wall_gaps() -> Vec<WallGap>` / `nearest_gap_angle(player_angle) ->
  Option<f32>`; `WallGap { center_angle, half_width_angle, radius }` +
  pure `compute_wall_gaps(&[CWall])` in `th-game::wall` (re-exported at
  the crate root); `CWall::kill()` (sets the existing `killed` flag —
  the existing `retain` drops the wall after the update) +
  `CWall::angle_span() -> Option<(start, end, radius)>` from the two
  distance vertices (angles normalized to `[0, TAU)`);
  `CCustomWallManager::soft_kill_overlapping(&Vec2) -> usize`.
- State: `EngineState.soft_collisions` (init false) + `bump_serial` (init
  0); the kid composition (07) calls `set_soft_collisions(true)` once.
  Non-kid path byte-identical (every added line is `soft`-gated; no
  update reordering — the second "always deadly" round is simply
  skipped in soft mode).
- Bump semantics: per killed wall (CWall: first collision round; custom
  wall: the manager kill count) set
  `status.flash_effect = max(·, 120.0)` + `bump_serial += n`; a killed
  wall is retained out, so repeated updates never double-bump. The app
  compares serial deltas (05 synth-duck / 07 slow-motion + SFX); the
  death flash (255) is untouched and sets no bump.
- Gap algorithm: per-wall span from `v0`/`v1` (atan2 → `[0, TAU)`, may
  wrap the seam; `None` on degenerate vertices), flatten → sort → merge
  (EPS 1e-5 rad, tracking the bounding wall radius at each merged edge)
  → complement; gap `radius` = min of the two bounding walls' radii
  (the gap closes once the nearer bound reaches it); gaps ordered by
  start angle; empty / fully-covered ring → `[]`. `nearest_gap_angle`:
  signed diff normalized to `[-PI, PI]`, smallest |distance|, exact ties
  keep the first gap in order, `None` when no gap.
- Custom-wall limitation: `handle_collision` returns one bool, so the
  soft path re-derives the hit set as "alive + `canCollide` +
  overlapping the post-push player position" (the pushes still run; a
  `deadly` custom wall is killed too — soft = no death by design). The
  tiny pack (06) should not rely on custom walls.
- RNG-order: unchanged (the soft branch draws no game RNG; the
  non-soft path is untouched).
- Tests: 18 new — `tests/soft_collisions.rs` (9: soft vs non-soft CWall
  twins — flash 120 vs 255, wall vs player death, moving wall dying on
  the SAME tick in both modes, one-bump-per-killed-wall + no
  double-bump + two-walls-in-one-frame, custom-wall twins, difficulty
  write + clamp) + `tests/wall_gaps.rs` (9: `angle_span` incl. seam
  wrap, gap centres/widths incl. the 0/TAU-seam gap, sparse-ring order,
  full/empty ring, engine delegation + nearest-gap picks). Gate 4/4
  green: 62 suites, 527 passed / 0 failed / 1 ignored (baseline at
  `5740e9bc` measured in a temp worktree: 509 — the delta is exactly
  the 18 new tests).
- Committed by the orchestrator after the forked PASS.
- For 04b: `status.pulse` / `beat_pulse` stay engine-driven (the kid
  beat API is additive — do not rewrite the pulse path); `DrawOptions`
  gating lives in the th-app bridge / th-render — 04a touched no
  text/draw surface.
- For 07: call `set_soft_collisions(true)` once after composition; poll
  `bump_serial()` for slow-motion/SFX/duck; steer the helping hand with
  `nearest_gap_angle(get_player_angle())` (or widest of `wall_gaps()`);
  drive the stage arc with live `set_difficulty_mult` (engine clamps
  0.1..=4.0, the app owns the 0.55..1.15 arc).

### 04b — beat palette/pulse + DrawOptions (LANDED)

- Beat API (final): see the §04 bullets. Application point: `update.rs`,
  directly after the `style_data.update(m_ft, dm.powf(0.8))` block
  (non-B&W half) and before `update_player_position` — the beat's
  `compute_colors` runs in the SAME frame. `EngineState.beat_hue` /
  `beat_energy` init `None` / `0.0` (constructor + the `lua_console.rs`
  test stand-in). Shimmer: `±BEAT_HUE_SHIMMER_DEG` (1.5°, two half-
  amplitude sines on `beat_phase·TAU` / `bar_phase·TAU`; phase 0 ⇒ 0).
  `status.beat_pulse_delay` is re-armed to
  `max(·, levelStatus.beatPulseDelayMax)` so the engine's own beat-pulse
  retrigger doesn't clobber the synth value; stop calling and the engine
  animation takes over.
- **Intentional C++-parity fix (supervisor ruling A)**: 
  `StyleData::calculate_color`'s fmod emulation was fixed from
  `total - total/360.0*360.0` (collapsed every hue in [0, ~720°) to ~0 —
  dynamic style colours were frozen near red; the beat palette would
  have been visually inert) to `total - (total/360.0).trunc()*360.0`
  (true `std::fmod`, keeps the dividend's sign). Dynamic style colours
  now animate through the hue range as the C++ intended; the frozen-red
  behaviour was the bug. Pinned by
  `calculate_color_hue_wraps_like_the_cpp_fmod` (90→0.25, 120→1/3,
  270→0.75, 451→91/360, 350+shift30→20/360, −90→−0.25). Nothing pinned
  the old behaviour. LATER-ISSUE CONSEQUENCE: 06 (tiny pack) styles can
  rely on dynamic colours + the beat hue actually reaching the screen;
  05/07 golden-frame expectations must use the animated (C++-correct)
  palette, not the frozen-red one.
- `DrawOptions` (final): see the §04 bullets. Gating: `draw_frame` skips
  `text_draw::draw_text` when `!show_text`; key icons / level info are
  AND-gated with `Config` + the option; `show_fps` ANDs with
  `Config::getShowFPS()` inside `draw_text_time_and_status` (new `show_fps
  : bool` parameter on `draw_text` / `draw_text_time_and_status` — the
  only call site is `draw_frame`). Background / walls / player /
  particles / flash are NEVER gated. Bridge: `AppDraw.draw_options`
  (private, default) + `set_draw_options` / `draw_options()`; the
  update-side `update_key_icons` / `update_level_info` / `update_text_
  sizes` calls are AND-gated too.
- Review P2 handoffs:
  - 06: `clamp_range` treats a reversed range (300/30) as the numeric
    [30,300], not the wrap arc [300,360]∪[0,30]; use normal `hue_min <
    hue_max` styles or add a true wrap clamp.
  - 07: `beat_hue` is never cleared, so the style hue stays at the last
    beat unless the app calls `apply_music_beat` every frame; either call
    every frame or add a `clear_music_beat`.
  - The beat block sits outside the `!black_and_white` style-update
    branch, so B&W recomputes beat colours while the style animation is
    skipped; inert for the kid config (B&W false), but guard or document.
  - 04a soft burst remains the flash+bump (no reusable burst helper); the
    custom-wall push limitation is recorded; no 04a mechanics changed.
- 04a P2 outcomes: (1) the DELAY consumer is asserted in
  `set_difficulty_mult_writes_the_shared_mult` (`get_delay_mult_dm()`,
  `delayMult / dm^0.10`, delayMax-capped when `hasDelayMaxLimit`); (2)
  SOFT WALL BURST: the death path has NO reusable wall-burst particle
  helper (the only burst spawners are the swap-specific
  `SwapParticleSpawnInfo` bursts; PB-star is a dead surface since 02c1)
  ⇒ the engine's soft-collision visual IS the flash (120) + bump serial;
  the app's SFX / slow-motion lands in 07 — no new particle system;
  (3) SOFT CUSTOM-WALL PUSH: `handle_collision`'s push still runs
  before the kill (single 3-phase push-then-report algorithm; no
  kill-before-push path on the manager) — the tiny pack (06) should not
  use custom walls.
- RNG / order: unchanged. The beat block in `update.rs` is additive +
  inert without a beat; `apply_music_beat` / `apply_beat_*` draw no game
  RNG; `status.pulse` (the RNG feed) is bit-identical with/without the
  beat (pinned by twin-engine `engine_state()` + `status.pulse` compare).
  The non-soft collision path is byte-identical. Only legacy-observable
  change: the fmod parity fix above.
- Tests: 12 new (see §04). `cargo test -p th-game soft_collisions` (the
  04b spec's verification filter) matches ZERO test names (the 04a tests
  are `soft_collision_*`; file is `soft_collisions.rs`) — use
  `--test soft_collisions` (9/9). Gate 4/4 green: 539 passed / 0 failed /
  1 ignored (04a baseline 527 — delta exactly the 12 new tests).
- Committed by the orchestrator after the forked PASS.
- For 05 (synth): drive `apply_music_beat(beat_phase, bar_phase, energy,
  base_hue)` once per frame from the synth's beat/bar clocks (phases in
  `[0, 1)`); `base_hue` = the stage hue the app owns; read
  `status.beat_pulse` back if you need the engine-side mirror.
- For 07 (tiny runtime): call `AppDraw::set_draw_options(DrawOptions {
  show_text: false, show_level_info: false, show_key_icons: false,
  show_fps: false })` once after composition (default stays the legacy
  rendering for the classic path); `bump_serial()` deltas drive
  slow-motion / SFX / the synth duck (05); the soft-collision flash (120)
  is the only engine-side bump visual.

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

- Desktop-mode probe (upstream `228a4cfb`, landed in the fork's
  `th-app`): the tiny `main` calls
  `th_app::desktop_mode::run_probe_child_if_requested(&argv)` at the top
  (before any FS/config/window work — exit with the returned code when it
  returns `Some`); `load_and_reapply` is called with
  `DesktopModeSource::Lazy` (both the windowed and the headless branch —
  the C++ `mainClient` does the load + reapply once). The probe child is
  the ONLY `VideoMode::get_desktop_mode` caller (winit 0.30 allows one
  `EventLoop` per process — the `th-render` `types.rs` doc paragraph +
  the `th-app::desktop_mode` module docs). The probe's CLI flag is
  `--th-desktop-mode-probe` (renamed from `--oh-desktop-mode-probe` in 03).
- Composition signature:
- Journey table:
- Touch mapping:
- SFX ids/volumes:
- Beat-hue call:
- Headless tests:
- No-storage proof:

### 08 — Android/web/packaging (pending)

- Android entry: pass `DesktopModeSource::Fixed(VideoMode::new(0, 0))`
  to `load_and_reapply` (winit's monitor API exists only inside a RUNNING
  event loop and the activity's loop is consumed by the game's own `run`
  — inert: both auto-resolution flags default false and the platform
  window is screen-sized independently of the config values).
- Windowed smoke/gate/package: `tools/smoke-windowed.sh` exists (default
  binary `${CARGO_TARGET_DIR:-target}/debug/tinyhexagon`, adapted header
  notes; the `[th-render] context GL_SAMPLES` assertion + the
  `--th-desktop-mode-probe` probe arg are the 03-renamed forms). The
  windowed gate stage (tools/gate.sh) and the windowed package smoke
  (tools/package.sh) land HERE once the `tinyhexagon` binary exists. The
  CI display packages (xvfb, libgl1-mesa-dri,
  libegl1, libegl-mesa0 + the winit X11 -dev libs) are already in
  `tools/ci-deps.sh` (upstream `228a4cfb` + `dcd9fdeb`). NOTE:
  `tools/smoke-windowed.sh` still asserts the old `[::mainClient] Finished`
  string; that half is inert until 07/08 adapts it to the tiny loop.
- Web boot/audio-unlock export:
- Artifact names: `tinyhexagon-<tag>-{linux-x86_64.tar.gz,
  windows-x86_64.zip, android-arm64-v8a.apk, web.zip}` (release.yml job
  artifact names + the inner `tinyhexagon-web.zip`); web serve dir carries
  `index.html` + `tinyhexagon.js` + `tinyhexagon.wasm` + `tinyhexagon.data`
  (the zip-root / gh-pages inline-deploy contract — page import is
  `./tinyhexagon.js`).
- Target degradations:
- Pages URL: `https://gtrak.github.io/tinyhexagon/` (repo renamed — the
  gh-pages site follows; the ci.yml fork-guards use
  `github.repository == 'gtrak/tinyhexagon'` since 03).

### 09 — tuning/docs (pending)

- Final tunables:
- Playtest notes:
- Per-target deviations:
- Dead-surface/doc sweep: `th-render` `PBParticleUpdateParams`/`make_pb_particle`
  spawn branch/export/test; `th-assets` `WorkshopPacks`/`new_with_workshop`;
  `mustShowReplayUI`/`drawTextMessagePBImpl`/`mFT`/`Deferred`/HUD-texture
  comments; any remaining replay/PB/steam doc residue; the stale
  `.gitignore` rule `crates/oh-parity/oh030b-corpus/` (dead since 02).
- Plan closed/archived:
