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

### 02 — crate removal / engine de-scope (02c1/02c2 LANDED; 02c3 pending)

- 02c2 (remove replay; remove `oh-replay`): see §02c1's "For 02c2"
  note — the replay surface, `GameApi::in_replay` and the
  replay-name fallbacks are 02c2's removal list.
- 02c3 (remove the parity fingerprint/RNG + final engine cleanup):
  fingerprint path + the `oh-render` `DrawState` replay AND PB fields +
  the inert `bridge.rs` initializers (`replay_icon`/`replay_text`/
  `pb_text`/`pb_text_growth`/`must_show_replay_ui: false`) + the doc
  residue (see §02c1/§02c2).
- `draw_target.rs` API / `th-app` keep-list: landed in 02a/02b (see
  those sections); the composition signatures moved again in 02c1 (see
  §02c1).

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
