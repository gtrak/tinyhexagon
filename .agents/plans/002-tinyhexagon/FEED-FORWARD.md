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
  staging dir `target/th-apk-assets`; the Android `package` id is
  `com.gtrak.tinyhexagon` (08b post-review, orchestrator-directed —
  the plan's "stays `info.vittorioromeo.openhexagon`" note was
  superseded; section 08b's application-id bullet).
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
  `tools/android/tinyhexagon-release.keystore` (the committed
  self-distribution key — 08b re-identified it from the retired
  `openhexagon-release.keystore`; section 08b),
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

### 05a — synth host + alsa-sys patch (LANDED)

- Spike result (host): STOCK `rust-synth 0.23.0` compiles on the host —
  the only blocker was the graph-level `links = "alsa"` collision, not the
  crate. No second conflict appeared.
- Patch location: `vendor/alsa-sys-0.3.1/` (registry cache copy, LICENSE
  in; ONLY change `links = "alsa"` → `links = "alsa_0_3"`, + provenance
  comment) wired by the root `Cargo.toml` `[patch.crates-io] alsa-sys =
  { path = "vendor/alsa-sys-0.3.1", version = "0.3.1" }`. Lock: alsa-sys
  0.3.1 (path, no checksum) + 0.4.0 (registry) side by side; alsa 0.9.1
  (cpal 0.15.3) + 0.11.0 (cpal 0.18.2) both present. rodio/th-audio
  untouched.
- Director API (final, `th-synth`): `MusicDirector::start() -> Self`
  (boot: live attempt on desktop/Android, `Err` → silent fallback; on
  emscripten fallback ONLY — autoplay); `set_stage(u32)` (`Shared`
  params only, NEVER `rebuild_graph` — click-free; idempotent);
  `bump()` (duck 0.55 → 0.22, linear recover over 0.6 s, applied via the
  engine's per-buffer `master_gain` `Shared` read); `tick(dt) -> BeatView`
  (live: `t = phase_clock`, energy = `clamp(max(peak_l,peak_r)·3)`,
  playing = live; fallback: `fallback_t += dt`, energy 0, playing false);
  `stop()` (drops the engine, state preserved);
  `start_on_gesture()` (**cfg-gated to emscripten**, idempotent — the web
  boot hook from the first canvas pointer/key event); `resume()` (any
  target, after stop/suspension; cpal 0.15 emscripten = one AudioContext
  per stream, so a fresh engine IS the resume). `BeatView` is exactly the
  contracted shape (`beat_phase`, `bar_phase`, `energy`, `stage`,
  `playing`). The director is a MAIN-THREAD object (cpal 0.15 `Stream` is
  `!Send+!Sync` — `NotSendSyncAcrossAllPlatforms` marker; only the plain
  `BeatView` crosses threads).
- Stage/layer table (final): 8 fixed slots, always `MAX_TRACKS = 8` (the
  engine asserts it): PadZimmer@0 ×1.0 · DroneSub@0 ×0.5 · PluckSaw@2
  ×1.5 · Bell@5 ×2.0 · BassPulse@8 ×0.5 · Shimmer@11 ×2.0 · Heartbeat@14
  ×0.5 (half-time pattern = the "soft kick" role) · SuperSaw@17 ×1.0
  (ratios to the root). `scale_mode = 0`; root A2 110 Hz; `bpm = 70 +
  1.5·min(stage,20)` (cap 100); `brightness = 0.45 + 0.02·min(stage,20)`
  (cap 0.85); root ×(4/3) per 3-stage block (⌊min(stage,20)/3⌋ fourths,
  capped at the stage plan); `master_gain` 0.55, duck 0.22 / 0.6 s. 05a
  tests: stage map monotonic/capped, table match, per-stage unlocks,
  re-tune on shift, beat/bar phases at known times, duck lower + recover,
  start-without-device no-panic + fallback advance, stop/resume, 8
  tracks (12 total, green). Mechanics trap for later workers: `Track` is
  NOT `Clone` (only `TrackParams`, whose `Shared` = `Arc<AtomicU32>` are
  cell-sharing) — `th-synth` hands the engine a rebuilt `Vec<Track>` of
  cloned params so the live graph reads the director's atomics.
- Web gesture hook: `start_on_gesture()` (emscripten-only) / `resume()`;
  issue 08 wires it to the first canvas pointer/key event. A suspended
  AudioContext (hidden tab) freezes `phase_clock` and cpal 0.15.3 exposes
  no suspend query, so `BeatView.playing` means "live" (not "un-suspended")
  — 08 must re-fire the gesture hook on visibility/gesture.
- Target caveats: HOST VERIFIED (check/test + whole-workspace gate green;
  no device on the build host → the fallback path is what every test
  exercised; no listen recorded — 05b/07 on a device-bearing machine).
  Skills written from source: `lib-rust-synth.md` + `lib-cpal.md`
  (indexed; host sections verified; android/wasm notes `TODO(05b)`).
  cpal 0.15.3 emscripten: `Stream::play()` = `AudioContext.resume()` whose
  REJECTED promise PANICS in a spawned task — never start pre-gesture on
  web (the cfg gate exists for this). Oboe (android): mono/stereo only,
  I16/F32, `default_output_device()` always `Some` (a dead device surfaces
  as a stream error → the fallback, same as host).
- Gate (host, 4/4 green): fmt / build / clippy `-D warnings` / test —
  see `.agents/tasks/plan-002/issue-005a-report.md`.

### 05b — synth cross-target (LANDED)

- Hand-off from 05a (host only): run
  `cargo check -p th-synth --target aarch64-linux-android` (the oboe
  host — confirm AAudio-vs-OpenSL in oboe 0.6.1; the question is parked
  in `lib-cpal.md` as `TODO(05b)`) and
  `cargo check -p th-synth --target wasm32-unknown-emscripten` (the
  emscripten host; the risk per plan 05 is rust-synth's UNCONDITIONAL
  TUI/recording deps — ratatui, crossterm, flacenc, vorbis_rs,
  tracing-subscriber — on wasm/Android). If the spike fails: vendor a
  feature-gated rust-synth (plan 05 spike-failure path) and re-run; do
  NOT substitute or drop music. Then re-run `bash tools/gate.sh` and fill
  the `TODO(05b)` markers in `lib-rust-synth.md` / `lib-cpal.md`.
- Spike result (stock vs patched): **Android GREEN on STOCK** rust-synth
  0.23.0 (the whole TUI/recording dep set is in the aarch64 graph and
  checks clean — the wasm-only breakage is in crossterm's `mio 1.2.4`
  ["This wasm target is unsupported by mio"] + vorbis's `ogg_next_sys
  0.1.5` C build script [needs emcc], not in the crates themselves). **
  wasm RED on stock, GREEN patched** (see patch location). `oboe-sys
  0.6.1` compiled the bundled oboe C++ FROM SOURCE with the NDK clang++
  from `tools/android-env.sh` (`aarch64-linux-android28-clang++`, API
  28; the `fetch-prebuilt` GitHub path is OFF by default). oboe host:
  cpal builds from `AudioStreamBuilder::default()` (`AudioApi::
  Unspecified`); oboe 0.6.1 C++ recommends AAudio iff `SDK ≥ O_MR1 (27)
  && isAAudioSupported()` ⇒ **AAudio on all of our minSdk-28 devices**
  (source-verified; the on-device confirmation is 08). SECOND blocker
  (new, not in plan 05's risk list): cpal **0.15.3 is the last 0.15.x**
  and its emscripten host does not compile against the workspace's
  wasm-bindgen 0.2.129 (3 errors in `cpal/src/lib.rs` around
  `BufferSize` — missing `FromWasmAbi` for the `StreamConfig` derive +
  the hand-written `IntoWasmAbi` match no longer type-checking). Fixed
  by the cpal vendor patch (supervisor ruling A); downgrading
  wasm-bindgen was rejected (th-web's 0.2.129 pin matches the installed
  CLI — the 03c recipe's documented drift hazard). Music NOT
  substituted or dropped on any target.
- Patch location: `vendor/rust-synth-0.23.0/` (registry copy; `[features]`
  `default = []` + `recording` [flacenc/vorbis_rs/hound] + `tui`
  [ratatui/crossterm/tracing-subscriber + `recording`]; the 6 deps
  optional; both `[[bin]]`s `required-features`; `lib.rs` gates the
  `recording`/`tui` mods; `audio/engine.rs` 5 cfg-gated `RecorderState`
  touch points; DSP/math byte-identical) + `vendor/cpal-0.15.3/`
  (registry copy; TWO `#[cfg(target_os = "emscripten")]` changes in
  `src/lib.rs` — the missing `FromWasmAbi` impl for `BufferSize`
  (`Option<u32>`: None ⇔ Default, Some(fc) ⇔ Fixed) + dropping the
  trailing `.into_abi()` from `IntoWasmAbi` (identical encoding); all
  other hosts/ABI untouched; residual: 2 upstream dead-code warnings in
  the emscripten host left in place per the surgical ruling). Wired by
  the root `[patch.crates-io]`: `rust-synth = { path = "vendor/
  rust-synth-0.23.0", version = "0.23.0" }` + `cpal = { path = "vendor/
  cpal-0.15.3", version = "0.15.3" }` (version pins limit each patch to
  its line — the 0.18.2 registry cpal rodio pulls stays stock; same
  scheme as the 05a alsa-sys 0.3.x line patch). `th-synth` depends on
  `rust-synth` with the default (no) features ⇒ audio-only graph on
  every target (the lock now drops the TUI/recording subtree).
- Director API: UNCHANGED from 05a (one structural note: `start()`'s
  emscripten early-return was re-cfg'd into two mutually-exclusive
  blocks in `crates/th-synth/src/director.rs` — same calls, same
  signature/behaviour — to drop an `unreachable_code` warning that only
  appears now the emscripten target actually compiles).
- Stage/layer table: UNCHANGED from 05a.
- Web gesture hook: UNCHANGED (`start_on_gesture()` cfg-emscripten-only /`resume()`;
  08 wires the first canvas pointer/key event).
- Target caveats: ALL THREE `cargo check`s green (host re-verified; see
  skills for the per-target evidence). Host gate 4/4 green after the
  patches (fmt/build/clippy `-D warnings`/test). For 08: the emscripten
  build links the PATCHED cpal — its `BufferSize` wasm ABI is
  `Option<u32>` (None = default buffer); if the web entry ever needs a
  custom buffer size use `StreamConfig` as usual (the host API is
  unchanged). The vendored cpal's 2 upstream emscripten-host warnings
  are expected noise. The 05a `TODO(05b)` markers in
  `lib-rust-synth.md` / `lib-cpal.md` are filled (Targets / Oboe host /
  "05b emscripten ABI patch" / "wasm feature patch (05b)").

### 06 — tiny pack (LANDED)

- Level ids / speeds / delays: pack `tinyhexagon_tinyhexagon_tiny_1`
  (author `tinyhexagon`, priority 0, depends on `base`); levels
  `tinyhexagon_tinyhexagon_tiny_1_{meadow,river,stars}` — six sides,
  speed 0.55 / 0.62 / 0.70, delay 1.75 / 1.70 / 1.65, ALL `*_inc` zero,
  `l_setIncTime(999999)` (no in-level increment — the app's stage arc owns
  escalation), rotation 0.04 (max 0.25), `unscored: true`,
  `musicId: "nullMusicId"`, no `difficultyMults` (loader appends 1.0;
  the app passes its own multiplier). Issue 07's journey table and Issue
  09's tuning use these exact values.
- First-pattern delay: `t_wait(180)` as the first `onLoad` timeline entry
  (180 frames ≈ 3 s at 60 fps — the contract value).
- Styles/hues: meadow 100–160 (green), river 190–250 (blue), stars
  270–330 (violet); `3D_*` zero/off; no `Shaders/`, no shader refs, no
  OGG music, no replay/score/Steam.
- Lua globals no-oped: NONE — every `base` helper used exists
  (`utils`/`common`/`commonpatterns`: `shuffle`, `u_rndInt`, `t_wait`,
  `pBarrageSpiral`, `pInverseBarrage`, `pAltBarrage`, `pSpiral`); no
  removed replay/score/Steam globals needed a no-op.
- Load/run test: `th-app/tests/tiny_pack.rs::tiny_pack_loads_and_runs`
  (pack + all three level ids valid in `Assets`; each level runs 30 s
  headless with soft collisions — no death, six sides, authored
  speed/delay landed in `LevelStatus`). Gate-count pins updated for the
  new pack: `th-assets` release_tree (packs 8→9, levels 37→40, styles
  33→36, assets 57→60 headless / 61→64 full, selectable 7→8, discovery
  list +`Packs/tiny/`) and `th-data` load_release (levels 37→40, styles
  33→36).

### 07 — app tiny runtime (LANDED)

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
- 05a handoff: host listen still pending on a device-bearing machine (05b/07); `BeatView.playing` means live, NOT un-suspended — re-fire `start_on_gesture` on visibility/gesture; `stop()` does not seed `fallback_t` from the live phase, so 07 may see a one-frame beat jump after a stop/resume; the Heartbeat half-time/root-fourth choices are 09 tuning.
- Composition signature: `TinyRuntime::new(assets: Arc<Assets>, root: PathBuf, window: bool, config: Config, audio: Option<Arc<TinyAudio>>, music: MusicDirector)` — inside: `compose_with_config(assets, root, window, Arc<RwLock<Config>>, renderer: None, engine_audio: Option<Arc<dyn Audio>>, Arc<NoJoystick>)` (the 02 de-scope) + `set_soft_collisions(true)` ONCE + stage-0 `new_game(PACK, journey.level_id(PACK), true, journey.difficulty())` + `start()`; `DrawOptions` all false (04b); the bin runs `load_and_reapply(DesktopModeSource::Lazy)` + `kid_config` (`no_music` / 3D off / 1024×768; in-memory, never written back) + `audio.set_sound_volume(60.0)`.
- Journey table (`journey.rs`): 30 s × stages 0..=20 (20 transitions = 10 min), plateau at 20; `difficulty = 0.55 + 0.03 * min(stage, 20)`; `bpm = 70 + 1.5 * min(stage, 20)`; one level per 2 stages cycling every 6 (`meadow`/`river`/`stars` — the 06 pack ids `{PACK}_{level}`); base hue `style.hue_min + 47° * (stage / 6)`; music layers 0 pad+drone / 2 pluck / 5 bell / 8 bass / 11 shimmer / 14 heartbeat / 17 supersaw via `MusicDirector::set_stage`; stage-up work: even stages (level change) `new_game` + `start`, odd stages `set_difficulty_mult`, flash 90, `levelUp.ogg`; manual reset = `new_game` SAME level/mult + `start` + `reset_elapsed` + `restart.ogg` (no stage change).
- Touch mapping (`touch.rs`): finger-captured halves (a NEW finger binds to its down-half: `x < width/2` → CCW, else CW; it keeps it for the whole touch — multi-touch holds both sides); a finger on an icon is consumed (one tap on the down-edge, no rotation, held icon fingers keep no tap — the icon-re-fire `Bound` state). Icons ~64×64, margin 8: top-left reset, top-right fullscreen; the mouse icon tap's single source is `FrameEvents.mouse_left` (the window feed's field — `run_windowed` mirrors it from the `InputState` left button; `frame` reads no button state itself).
- SFX ids/volumes: start `go.ogg` (engine `start`), bump `beep.ogg` + the 05a duck (0.55 → 0.22 over 0.6 s) + 0.35× for 0.35 s slow-motion (the `bump_serial` delta arms it; the journey stays REAL time), stage-up `levelUp.ogg`, reset `restart.ogg`, icon tap `select.ogg`, fullscreen `beep.ogg`; sound volume 60 (SFML units; the synth master gain is independent).
- Beat-hue call: EVERY frame `game.apply_music_beat(beat.beat_phase, beat.bar_phase, beat.energy, journey.base_hue())` — `beat_hue` is never cleared (04b P2), so the app keeps feeding it; the beat comes from `MusicDirector::tick(dt)`.
- Headless tests: `th-app/tests/tiny_runtime.rs` — 7 tests: the 65 s run (stage advance at 30/60 s, no death, wall/player/background/flash batches; `LevelData` carries the SHORT level id — assert `pack_id == PACK` + `id == "river"`), the forced collision (`set_always_spin_right(true)`; the first wall spawns ~3 s in and takes ~12 s more at meadow's 0.55× to travel the 1600 px spawn distance — first bump ~16 s, seed-0 stream; 20 s budget), the reset (stage unchanged, clock restarted, same level/mult, angle re-zeroed), the left/right input (keyboard + touch halves), the zero-text recording (all-false `DrawOptions`: 0 text, 6 icon rectangles + the reset triangle + the scene), the no-storage proof, and the 10-minute journey plateau (stage 20, the capped table).
- No-storage proof: a run against a TEMP root (the `_RELEASE` content reached through `Assets`/`Packs` symlinks) leaves the root free of `config.json` / `Profiles/` / `Replays/` and the `_RELEASE/config.json` byte-identical (`the_run_writes_no_config_profiles_or_replays`).
- For 08 (reuse notes): `TinyRuntime::frame(dt, &FrameEvents) -> FrameActions` is the window-agnostic half — feed it from the Android activity / the web rAF (`FrameEvents { input, fingers: [(id, pos)] in the window's `finger_down_positions` order, finger_count, mouse_left, mouse_pos, width, height }`; act on `FrameActions { close, toggle_fullscreen }`); `run_windowed` is the desktop (winit) half only; the dev screenshot arms the bridge's `mustTakeScreenshot` (`--tiny-shot=SECONDS`, the save fires at `draw_frame` step 12 — BEFORE the screen-space icon pass, so the bridge PNG lacks the icons; a post-icon capture would need a new `DrawTarget` hook); the `web_fullscreen` sinks + the audio-gesture re-fire land in 08 (the 05a notes above).

### 08 — Android/web/packaging (pending)

- Windows cross path (03d LANDED): the libsodium cross-prebuild in
  `tools/build-windows.sh` (SODIUM_DIR/SODIUM_LIB_DIR + `--force-sodium`
  + the make prereq) and the `-ladvapi32` rustflag for
  `x86_64-pc-windows-gnu` in `.cargo/config.toml` are REMOVED (th-net
  went in 02b; the dead prebuild failed CI's cross-check before cargo,
  run 39309854944). The cross path now needs only the rustup target +
  mingw-w64 gcc; Luau (luau0-src) is the sole cc-rs dep and compiles in
  place (host-verified: cross `--check --all-targets` and the full
  `--build` link both pass; the .exe imports no advapi32). 08 KEEPS the
  Windows artifact/CI work — release.yml's windows job structure, the
  artifact names, and `tools/ci-deps.sh`'s mingw-w64 prereq are all
  untouched by 03d.
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
- 05a handoff: `BeatView.playing` means live, not un-suspended; a hidden tab freezes `phase_clock` and cpal 0.15 exposes no suspend query — re-fire `start_on_gesture` on visibility/gesture. The web entry must start the synth only on the first gesture.
- 05b handoff: wasm uses the vendored rust-synth (tui/recording features off) + the cpal 0.15.3 emscripten ABI patch; 08 must keep `wasm-bindgen 0.2.129`, never downgrade, and re-fire `start_on_gesture` on visibility/gesture. Runtime audio (device/browser listen, AAudio on-device) remains 07/08. Windows cross was outside 05b; the cpal emscripten host has 2 dead-code warnings.
- 07 handoff: the screenshot is armed inside `bridge_draw` BEFORE `TinyRuntime::draw` appends the two icons, so `evidence/07/screenshot.png` lacks them (the window/test show the 6 rects + triangle); 08/09 capture after the icon pass. The host has no audio device (fallback only). The no-storage test is `#[cfg(unix)]`; Windows coverage is 08/09. The 10-minute end-to-end currently exercises `Journey::advance`, not `TinyRuntime::frame`; 09. `base_hue_for_stage(20)=331` clamps against the river 190–250 range; 09 tuning. `th-app/src/lib.rs` still describes the bin as stripped; 09 docs.
- Web boot/audio-unlock export: 03c landed the STUB's browser half —
  `th-web`'s `#[wasm_bindgen] status()` (returns "tinyhexagon stub: the
  tiny runtime lands in plan 002 issue 07/08"; the page's
  `window.ohStatus` / `#status` 1 s poll reads it) + no-op
  `save_session`; the adult shell page (IDBFS / fullscreen / IME glue)
  is REPLACED by a minimal stub page (`crates/th-web/site/index.html`:
  title / full-viewport `#c` canvas / empty `#status` / the module's
  poll). **08 replaces THIS page with the real tiny entry's shell**
  (re-add the fullscreen / IME / audio-unlock wiring + the rAF loop
  there; the `.cargo/config.toml` `_RELEASE` preload flags are kept).
  The probe/CI path: `tools/web-stub-probe.mjs` (headless-Chrome CDP;
  DOM/status assertions — headless SwiftShader WebGL is flaky and the
  stub draws nothing; OH_CHROME candidates + per-PID debug port +
  SwiftShader flags shared with tools/web-probe.mjs; favicon 404 +
  canceled `ERR_ABORTED` loads tolerated; the wait is the MODULE
  RESOLVE, default 60 s — the 22 MB `.data` preload). CI's `wasm` job
  (ci.yml) runs it against the staged `_WEB` (`python3 -m http.server
  8080`, the runner's `google-chrome` via OH_CHROME) after
  `tools/package-web.sh`, BEFORE the `pages` deploy (pages needs: wasm
  — a red probe never deploys) and uploads `target/web-stub.png`
  (artifact `web-stub-screenshot`). Live-page half (post-push, green
  CI): `OH_CHROME=<chrome> node tools/web-stub-probe.mjs
  https://gtrak.github.io/tinyhexagon/ --shot=target/web-stub-live.png`
  (the screenshot WITHOUT the old `#fs` button is the new-vs-stale-page
  distinguisher). 08 runs the same probe (or the full web-probe.mjs)
  against the real entry.
  - P2 for 08: `web-stub-probe.mjs` parses only `--shot=PATH` (space form ignored;
    the CI default masks it) — accept both or keep `--shot=` everywhere.
  - P2 for 08: `save_session` is a no-op not called by the minimal page; the
    real flush/IDBFS returns with the tiny web entry.
  - P2 for 08: CI hardcodes `OH_CHROME=/usr/bin/google-chrome`; add a
    `command -v` guard or `browser-actions/setup-chrome` for runner drift.
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

### 08a — web (emscripten) tiny entry (DONE — 2026-10-19, final pass fix 4: `#status` visually hidden; probe 19/19, audio=live, gate 567)

- Shared tiny entry: `th_app::tiny::{run, tiny_web, web_status,
  web_request_audio_unlock}` (host `run_windowed` is a thin caller); the
  emscripten boot is `th-web` (`src/main.rs`'s `__wasm_call_ctors()` first,
  then `src/web.rs`'s `boot()`), with `#[wasm_bindgen] status()` (the real
  game's status line → the page's `window.ohStatus` / `#status` 1 s poll)
  and `#[wasm_bindgen(js_name = "resumeAudio")] resume_audio()` —
  first-gesture audio unlock, guarded by `navigator.userActivation.isActive`
  read inside the gesture (headless CDP grants no activation → fallback
  clock stays; a real tap flips to live — the 10-19 Chrome 148 probe run
  granted activation for the synthetic keydown, so it measured `audio=live`;
  the fallback guard is intact for no-activation environments). The shell
  (`crates/th-web/site/index.html`): canvas + `#status` + first
  `pointerdown`/`keydown` fires `resumeAudio()` once (`[shell] audio unlock
  requested` log), `visibilitychange`→visible re-fire; no IME/text/IDBFS/
  persistence. Fix 4: the `#status` line is visually hidden in the page
  CSS (`display: none` — the element + 1 s poll stay, so `ohStatus()` /
  the probe's `#status` contract is unchanged; the game is text-free),
  and the page does NOT declare `#fs` (the `th-render` emscripten
  window's get-or-create fallback creates it — the faint top-right `⛶`).
- web-sys 0.3.106 signatures the 08a guard needs: `Window::navigator()`
  returns `Navigator` (NOT `Option` — use `.map`, not `.and_then`);
  `Navigator::user_activation()` returns `UserActivation` (not Option/Result);
  `UserActivation::is_active() -> bool`. th-app's emscripten web-sys features
  are `Window, Document, Element, Event, Navigator, UserActivation`.
- `js_sys::JsValue` is a PRIVATE re-import in js-sys 0.3.106 (E0603) — use
  `wasm_bindgen::JsValue` (the prelude import) for `web_sys::console::log_*`.
- Do not put `///` doc comments on the `extern "C" { fn __wasm_call_ctors(); }`
  block in `th-web/src/main.rs` (unused_doc_comments warning; gate is
  `-D warnings`).
- RESOLVED (2026-10-19, the three 08a fixes; full evidence in
  `.agents/tasks/plan-002/issue-008a-report.md`):
  1. Fix 1 — the emscripten compile errors (the `navigator()` `.and_then` →
     `.map`; the 4× E0603 `js_sys::JsValue` → prelude `JsValue`; the
     `///`-on-`extern` block → `//`; a fmt collapse).
  2. Fix 2 — vendored cpal 0.15.3 (`src/lib.rs` +
     `src/host/emscripten/mod.rs`): the `BufferSize` emscripten wasm-ABI
     triple must delegate to `Option<FrameCount>` (0.2.129 conventions:
     [OPTIONAL, U32] descriptor — the empty registry `describe()` made the
     CLI's `Descriptor::decode` panic "len is 0" on
     `__wbindgen_describe___wbg_get_streamconfig_buffer_size` — and the
     single-`f64` `2^53-1`-sentinel `Option<u32>` ABI), and the callback's
     `SampleFormat` must be captured by the timer closure instead of
     crossing the JS timer (the 0.2.129 timer coerced the JS argument to
     u32 `0` = `I8` → "host supplied incorrect sample type"). Keep the
     vendor scheme (root `[patch.crates-io] cpal`) — the 0.18.2 registry
     cpal that rodio pulls is NOT patched.
  3. Fix 3 — vendored cpal emscripten host connect direction:
     `AudioDestinationNode` has 0 outputs, so the registry source's
     `context.destination().connect_with_audio_node(&node)` panics
     (IndexSizeError: output index (0) exceeds number of outputs (0)); it
     must be `node.connect_with_audio_node(&context.destination())` (the
     direction `host/webaudio/mod.rs:363` already uses). This was the last
     panic: `tools/package-web.sh` green → probe **19/19 PASS**, clean
     console (no errors / no 4xx-5xx / no failed loads), baseline
     `audio=fallback` → post-gesture `audio=live`, screenshot
     `.agents/evidence/08a/08a-web-probe.png` (real game: meadow background,
     white ball, HUD chips, status line). Host gate: **567 passed / 0
     failed** (baseline). No commit yet (working-tree diff only).
   4. Fix 4 (final pass) — P2 cleanups: the child-visible `#status` line
      hidden (`display: none` in the page CSS — the probe contract is
      unchanged); the unused direct `js-sys` dep dropped from `th-web`
      (`JsValue` rides the `wasm-bindgen` prelude — `Cargo.lock`'s
      `th-web` entry loses it); the stale `#fs` doc in `th-render`'s
      `window.rs` corrected (the shipped page does NOT declare it — the
      get-or-create fallback creates it); the stale "No caller yet" in
      `web_fullscreen.rs` (`tiny_web` calls `install_sinks`); the stale
      "headless probe stays fallback" notes in `tiny.rs` + the page
      (the 10-19 Chrome 148 run measured `audio=live` — activation
      granted — while the no-activation fallback guard remains). Re-
      verified: probe **19/19 PASS**, `audio=live`, clean console,
      screenshot `.agents/evidence/08a/08a-web-probe-fix4.png` (no status
      text); host gate **567 passed / 0 failed** (baseline).

### 08b — android (aarch64) tiny entry (DONE — 2026-10-19: check green + debug APK verified; device smoke = the manual half)

- Android entry: `th_app::tiny::tiny_android(app: AndroidApp)` (a
  `#[cfg(target_os = "android")]` in `th-app/src/tiny.rs` — the
  `tiny_web` boot's android twin): `Assets::new(internal_dir)` →
  `load_and_reapply(Fixed(VideoMode::new(0, 0)))` (**no child probe on
  Android** — the plan's note: winit's monitor API exists only inside a
  RUNNING event loop and the activity's loop is consumed by the game's
  own `run`; inert: both auto-resolution flags default false + the
  platform window is screen-sized) → `kid_config` (in-memory, never
  written back) → `th_audio::Audio::new` + `set_sound_volume(60.0)` →
  `MusicDirector::start()` (the non-emscripten live attempt — oboe/
  AAudio; `Err` → the fallback clock, **never a panic**; the SFX's
  failed-sink-open degrades to the headless device, logged — the
  `th-audio` contract) → `EventLoopBuilderExtAndroid` (`with_android_
  app` + `build`) → `Window::create_android` (physical = config logical
  × `pixel_mult`; the C++ `main.cpp:305-310` setters — the android
  record halves, AA/vsync applied at the first-`resumed`
  materialization; the icon load is the no-op platform half) → the
  shared `run` loop. Every boot failure logs + returns without
  panicking (the activity's main-loop thread stays alive).
- `th-android` (the 02a stub → the real entry, `src/android.rs`):
  `#[no_mangle] pub fn android_main(app: AndroidApp)` (the D-026 entry
  contract — the android-activity glue's C wrapper links this Rust
  symbol; the 02a shape from git history): (1) the `__android_log_
  write` engine sink FIRST, tag **`tinyhexagon`** (the adult
  `OpenHexagon` tag renamed with the product — `adb logcat -s
  tinyhexagon`); (2) the base dir = `internal_data_path()`; (3)
  `extract_content` (the verified SG4 glue carried over: the
  packaging-generated `oh.filelist`-driven `AAssetManager` open/read —
  NO `openDir` enumeration; the atomic `.th_partial` + rename per
  file; the 0-file refusal; **the content marker re-named
  `.oh_content_extracted` → `.th_content_extracted`** — the `.th_`
  prefix is the product-consistent name (the `.th_partial` sibling's);
  with the application-id change below, a device that carried the
  retired adult app's extracted tree gets a FRESH internal data dir
  for the new application anyway (Android isolates app data per
  package id); **the marker's CONTENT is `CONTENT_VERSION` (`u32 = 1`
  — the 08b review fix: the skip check is the marker's content, not
  its presence, so an app UPDATE over the surviving internal data dir
  re-extracts on a version mismatch instead of leaving the stale
  `_RELEASE` — the 09-taste journey level update is exactly that);
  BUMP the const on every `_RELEASE` change (the 09-taste updates are
  the first bump)) —
  (4) `chdir_to_base_dir`; (5)
  `th_app::tiny::tiny_android(app)`.
- Deps: `th-app` gains `[target.'cfg(target_os = "android")'.
  dependencies] winit = {"0.30", features = ["android-native-activity"]}`
  (th-render carries the same version/feature — no lockfile version
  change). `th-android` gains the common deps `th-app` + `th-game` (the
  log-sink FFI names the crate directly) + the android-target `winit`
  + `ndk = "0.9"` (already locked — winit's `android-activity` dep;
  `AndroidApp::asset_manager()` returns this crate's `AssetManager`
  directly). Lock: +7 lines (the `th-android` entry's dep list only —
  zero new packages).
- Conditions: **no SIGINT** (the activity lifecycle is the stop
  mechanism — the back button's `CloseRequested` closes the winit
  window → the loop exit; the `th-app` `sigint` module is never called
  from the android path), **no Steam/Discord/profiles/config/replay**
  (the 02c1-retired de-scoped composition + the kid's in-memory config
  — every run starts stage 0), **no IME / soft input** (the kid has no
  text — the retired adult `ImeController` has no twin).
- Touch / icons: the SHARED 07 `touch.rs` / `tiny.rs` (no android-
  specific geometry — the window layer feeds `FrameEvents` from the
  window's `finger_down_positions` in the shared `run`): the finger-
  captured halves + the two 64×64 corner icons, 8px margin (the plan's
  "48–64 dp" band; the hit tests + the draw share the window's pixel
  space by construction). The fullscreen icon's action is `Window::
  set_fullscreen`'s **record half** (the winit call is
  `cfg(not(target_os = "android"))`-gated in `th-render` — the
  platform activity window owns the fullscreen surface; the record
  drives the bookkeeping).
- **Application id (08b post-review, orchestrator-directed):** the
  first pass kept the plan's "the package id stays" note and shipped
  under the ADULT identity `info.vittorioromeo.openhexagon` — the 08b
  brief forbids the adult APK identity. `[package.metadata.android]
  package` is now **`com.gtrak.tinyhexagon`** (the badging
  `package: name=`; the manifest's `android:label` stays
  `tinyhexagon` — 03's rename); the debug APK was REBUILT +
  re-verified (the Evidence bullet). The 03 "live keystore identity"
  exception is void with the id change — see the keystore half of the
  tools bullet; the §03 entries' "package id stays" / keystore notes
  are superseded by this section (the §03 lines are now updated +
  cross-referenced in place — the review fixes below).
- Review fixes (post-PASS, orchestrator-directed — the 08b final
  pass): (1) the versioned `CONTENT_MARKER` above (the
  blocking-in-practice P2 — the bare `marker.exists()` skip check left
  a stale `_RELEASE` after an app update); (2) the `th-android`
  Cargo.toml's stale `.filelist` comments aligned to the actual
  `oh.filelist`; (3) the `package-android.sh` manual check is
  level-agnostic ("the stage-0 level" — survives the 09-taste journey
  merge); (4) `release.yml`'s keystore-never-committed line clarified
  (a STORE key never; the committed fixed-password
  `tools/android/tinyhexagon-release.keystore` self-distribution key
  is the documented exception); (5) the §03 stale package-id +
  keystore lines updated + cross-referenced here; (6) the evidence
  refreshed — `host-gate.log` carries the `test result:` aggregate
  (TOTAL 567 passed / 0 failed), `check-android.log` is the clean
  re-run (both `Checking th-app` + `Checking th-android` after
  removing both crates' aarch64 artifacts/fingerprints/incremental
  state), the debug APK REBUILT (the marker change moved the `.so` —
  `T android_main` @ 0x4dc41c) + re-verified (`com.gtrak.tinyhexagon`
  badging unchanged). Re-ran: host gate 567/0, `check-android.sh`
  exit 0.
- tools: `tools/check-android.sh` is now `cargo check -p th-app -p
  th-android --target aarch64-linux-android` (the tiny runtime's
  android entry + the cdylib's `android_main`; the NDK r27c pin + the
  self-installing rustup target unchanged) — `ci.yml`'s android job
  header comment updated to match (comment-only; the job runs the same
  script; the workflow behaviour is 08c's lane). `tools/
  package-android.sh` unchanged except the tail "MANUAL checks" list
  reworded to the tiny game (stage-0 meadow, the synth/SFX fallback,
  the icon taps, the no-persistence relaunch, the logcat tag) — the
  release APK path (signing, the 16KB-page `RUSTFLAGS`,
  `tinyhexagon-<tag>-android-arm64-v8a.apk`) + the workflow renames are
  08c. The self-distribution keystore was RE-IDENTIFIED with the
  product (the application-id bullet): `tools/android/tinyhexagon-
  release.keystore` (regenerated — alias `tinyhexagon`, fixed password
  `tinyhexagon`, `CN=TinyHexagon (self-distribution), OU=Rust port,
  O=TinyHexagon`; the old `openhexagon-release.keystore` deleted from
  the tree — the new id has no prior installs, so a fresh signing
  identity costs no update continuity); `package-android.sh`'s
  `KEYSTORE` / `KEYSTORE_PASS` / alias / DN + `release.yml`'s
  keystore-path comment (comment-only) follow.
- Evidence (host, 2026-10-19): `bash tools/check-android.sh` **exit 0**;
  the REBUILT debug APK `target/debug/apk/tinyhexagon.apk` (`cargo apk
  build --manifest-path crates/th-android/Cargo.toml --target
  aarch64-linux-android` AFTER the application-id change; the staging
  step: 232 asset files + `oh.filelist`; cargo-apk 0.10's auto
  debug-keystore signing, `apksigner verify` verified): `aapt dump
  badging` → **package `com.gtrak.tinyhexagon`** v0.1.0 (the first
  pass's badging carried the adult `info.vittorioromeo.openhexagon`
  id — the fix above), `sdkVersion:'28'`, `targetSdkVersion:'34'`,
  `launchable-activity: android.app.NativeActivity`,
  `native-code: 'arm64-v8a'`; `aapt dump xmltree` → `A:
  package="com.gtrak.tinyhexagon"` + `android:label="tinyhexagon"`;
  contents → `lib/arm64-v8a/libth_android.so` + 233 `assets/` entries
  (160 `Packs/`, **10 `Packs/tiny/`** — the 06 pack's levels/scripts/
  styles are IN the APK) + `assets/oh.filelist` (the extractor's
  source); `llvm-nm -D` → **`T android_main`** exported (the
  NativeActivity's dlsym target); the debug .so's 4KB ELF LOAD
  alignment is expected (the 16KB alignment is the release script's
  `RUSTFLAGS` half). Logs: `.agents/evidence/08b/` (`check-android.log`,
  `debug-apk-build.log` = the rebuild, `host-gate.log`,
  `apk-verification.txt` = the rebuilt APK's). Host gate **567 passed
  / 0 failed** (baseline — the new code is android-gated; the fix pass
  is metadata/comment-only: `check-android.sh` + `cargo fmt --check`
  re-ran green after it).
- Device gap (the exact residual): `adb devices` empty on this host —
  the install/boot/touch/audio/rotation smoke is the
  `package-android.sh` tail's documented MANUAL list (08c/09 on a
  device-bearing machine); the 05b parked on-device AAudio
  confirmation stays open with it. No release APK, no workflow edits
  beyond the comment, no web/packaging work (08c's lanes).

### 08c — packaging/CI/release names (DONE — 2026-10-20: gate 570/0, package smoke 3/3, probe 21/21, actionlint clean; report `.agents/tasks/plan-002/issue-008c-report.md`, evidence `.agents/evidence/08c/`)

- Release artifacts (all four, both workflows): `tinyhexagon-<tag>-{linux-x86_64.tar.gz, windows-x86_64.zip, android-arm64-v8a.apk, web.zip}`; the Linux tar / Windows zip contain the `tinyhexagon` binary INSIDE the staged `_RELEASE/` (alongside `Assets/`, `Packs/`, `ConfigOverrides/`, `config.json` — the `resolve_game_root` walk's install-root layout). The pages contract is unchanged: the web zip's root / the gh-pages root carry `index.html` + `tinyhexagon.js` + `tinyhexagon.wasm` + `tinyhexagon.data`; the live page is `https://gtrak.github.io/tinyhexagon/`.
- `tools/package.sh` (rewritten) now OWNS the tagged artifacts: build → stage (`target/package/{linux-x86_64,windows-x86_64}/_RELEASE`) → smoke → the tagged tar.gz / zip at the repo root. `<tag>` = `--tag` > `$ASSET_TAG` (the release workflow's normalized env) > `dev`; a `/`-bearing tag is refused. The retired adult replay-fixture headless smoke is GONE (the tiny runtime has no `-headless` / replay); the package smoke is `tools/smoke-windowed.sh` against the staged binary (phases A/B/C; Linux only — the Windows cross build smoke-skips-with-reason on a Linux host). `--with-steam` + the Steam-native staging removed; the pristine-copy invariant kept.
- Probes: `tools/web-probe.mjs` / `web-stub-probe.mjs` / `web-fs-probe.mjs` / `web-persist-probe.mjs` → ONE `tools/web-tiny-probe.mjs` (the real game's status contract: polls `window.ohStatus()` / `#status`, waits `booted=true;started=true` (fast-fail on `booted=false;error=`), baseline asserts `ticks` numeric / `stage` int ≥ 0 / `angle` finite / `audio` ∈ {live, fallback} / `energy` finite / `help=auto`; after the first gesture (one held `A` key — the page's keydown fires `resumeAudio()`) asserts **`audio=live`** (the 10-19 Chrome 148 headless-CDP activation grant) + `help=off` + CCW steer + ticks advanced + the `[shell] audio unlock requested` log; clean console — favicon 404 + the boot's "AudioContext was not allowed to start" autoplay warning are the documented tolerations; `--shot=PATH` (default `target/web-tiny.png`) / `--timeout=SECS` (120) CLI). The adult IME / persistence / fullscreen-UX probes have no tiny twin (no text, no persistence, no adult menu geometry).
- Workflows: release.yml's `linux` / `windows` jobs now delegate build+stage+smoke+tar/zip to `tools/package.sh [--windows] --tag "${ASSET_TAG}"` (one step each; the android / wasm / release / gh-pages jobs are UNCHANGED — they already named the artifacts `tinyhexagon-<tag>-*`). ci.yml's `wasm` job browser-smoke runs `node tools/web-tiny-probe.mjs` (screenshot artifact `web-tiny-screenshot` / `target/web-tiny.png`). Job IDs / edges / secrets / the ASSET_TAG normalization / the fork guards all kept. actionlint v1.7.7 clean (both files, 2026-10-20).
- Host bin collision (the 03 known P2): RESOLVED BY DOCUMENTATION — `cargo build -p th-web` builds on the host (the no-op main; deps are emscripten-target-gated) and the `--workspace` collision is a NON-FATAL cargo `output filename collision` WARNING (gate stage 2 green, clippy `-D warnings` untouched by it). `th-web/Cargo.toml` now carries the note: the bin name IS the artifact name (tinyhexagon.js/.wasm/.data), the emscripten build is always `-p th-web`, do NOT rename the bin.
- For 09 (sweep): `th-render/src/audio.rs`'s doc comment still cites the retired `tools/web-probe.mjs` (08c's no-runtime-code constraint kept it); the live-page probe half is `OH_CHROME=<chrome> node tools/web-tiny-probe.mjs https://gtrak.github.io/tinyhexagon/ --shot=...`; the local-box port-8080 hazard (an unrelated long-running filebrowser service squats there — a probe run pointed at it measures the stale server, the lane-058 failure mode) — pick a free port locally; CI's clean runners are unaffected.

### 09 — tuning/docs (pending)

- Final tunables:
- 09 tuning notes: Heartbeat half-time pattern / root-fourth shift; the host listen (desktop/Android/web) is still pending; the 05a vendor `.orig` dead file was removed.
- Playtest notes:
- Per-target deviations:
- Dead-surface/doc sweep: `th-render` `PBParticleUpdateParams`/`make_pb_particle`
  spawn branch/export/test; `th-assets` `WorkshopPacks`/`new_with_workshop`;
  `mustShowReplayUI`/`drawTextMessagePBImpl`/`mFT`/`Deferred`/HUD-texture
  comments; any remaining replay/PB/steam doc residue; the stale
  `.gitignore` rule `crates/oh-parity/oh030b-corpus/` (dead since 02).
- 06 P2: `release_tree.rs`'s priority-0 comment still says workshopexample is the last selectable (tiny is also priority 0); update/assert both. The tiny_pack test does not assert the first wall at ~3 s or each level's styleId resolution — 07/09 tuning coverage.
- Plan closed/archived:

### 09-taste — journey feel pass (LANDED 2026-10-19; 572/0)

- Operator feedback: drones, abrupt/empty transitions, flat difficulty,
  always a hexagon, tame colors. Implemented directly in main (not a
  worker): report `.agents/tasks/plan-002/issue-009-taste-report.md`.
- **Continuous level**: `stage_up` no longer calls `new_game`; the run is
  `{pack}_journey` for all 20 stages. `journey.lua` picks patterns from
  the live `u_getDifficultyMult`; `l_setWallSpawnDistance(1050)`,
  `l_setDelayMult(1.05)`, `onLoad t_wait(30)`; the base pattern library's
  trailing waits shortened (the old 1.65–1.75 delay and 6.5× tails left
  holes).
- **Difficulty**: `0.45 + 0.085*t` (0.45..2.15), written every frame
  before the engine ticks; BPM `70 + 2.5*t` (120). Level
  `l_setSpeedMult(0.55)`, `l_setSpeedMax(2.5)` → ~2.7× wall speed.
- **Shapes**: `sides_for_stage` = 6,6,4,4,3,3,5,5,6,6,7,7,4,4,8,8,5,5,6,6,6;
  `set_sides` at even stages (old walls keep geometry → a warp, flash
  masks it); reset restores the stage shape.
- **Music**: `director.rs` bar-wise chord progression (major/minor from
  stage 8) on pad/drone/bass/kick; arp=1 on pluck/bell/shimmer/lead; kick
  pattern 2→4→5→7→9 hits; LFO movement; unlocks 0/2/3/4/7/11; root shift
  every 3 stages. The 05 8-track graph is unchanged.
- **Colors**: new `journey` style (hue 0..360, dynamic arena/walls/3
  background hues with `dynamic_darkness` > 1 = darker, white player,
  pulse 1.4), continuous `(100 + 28*t) mod 360` hue, 6° beat shimmer,
  synthesized pulse (`0.7·energy + 0.45·(1−beat_phase)^2.5`, ≤1.4) so the
  fallback clock still pulses.
- Evidence: `.agents/evidence/09/stage2.png` (stage 2: cyan square arena,
  red complementary walls, dark hue-washed background, white player);
  host gate **572/0**.
- P2: listen/playtest the arp/mix by ear; the shape warp at stage marks;
  the first 10 s with a child; the base pattern library is global (the
  reference levels are denser too); the live web deploy/audio check; 08b/
  08c remain separate.

## 09 operator fix pass (bd983b62)

- **Touch died on an accidental drag (web)**: the canvas had no
  `touch-action` CSS — pointerdown's preventDefault does not stop the
  browser's pan/zoom, so a small vertical drag fired pointercancel
  (folded to TouchEnded) and the hold died. Fixed in the shell:
  canvas `touch-action: none`, page `overscroll-behavior: none`,
  viewport `user-scalable=no`. Verified with a CDP touchStart + 1.8 s
  vertical drift: the hold survives, the angle keeps rotating
  (.agents/evidence/09/web-touch-drag.log).
- **The `#fs` DOM overlay button removed** (two fullscreen controls
  confused the kid): the in-canvas top-right icon is the control; the
  `web_fs_button_get_or_create` seam is inert (every #fs consumer
  no-ops). If a later cleanup wants it gone entirely, delete the
  function + the slot plumbing together.
- **Slower cursor**: journey `speedMult` 0.55→0.40, the difficulty
  ramp softened 0.085→0.06/stage (plateau 2.15→1.65).
- **Live-page lesson**: the 09-taste-2 audio fix reached gh-pages only
  after its CI + pages deploy (~25 min); the operator's "still silent"
  was the stale pre-fix deploy. Live-page verification probe:
  `audio=live;energy=0.0343` on https://gtrak.github.io/tinyhexagon/.
- The probe's baseline `help=auto` assertion assumes no pre-gesture
  input; the helping hand's attract steering is why the game plays
  itself until the first touch.
