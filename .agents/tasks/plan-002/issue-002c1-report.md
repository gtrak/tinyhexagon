# Issue 02c1 report — remove Steam, Discord, profiles and PB from the engine

**Umbrella:** `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`
(02a app headless and 02b front-end crates were landed; this is the first
02c sub-issue).

## What changed

Removed the non-replay engine seams the tiny runtime does not use:

- **Steam:** the `SteamSink`/`NoneSteamSink` seam, the `steam_manager`
  field/ctor arg, all rich-presence calls, the `a22_swapdeath` achievement
  branches, `GameApi::unlock_achievement`, and the Lua
  `steam_unlockAchievement` global (with its `mod steam` registration).
- **Discord:** the `oh-discord` crate (directory + root member + the
  `oh-discord` deps in `oh-game` and `oh-app`), the engine `Discord` field /
  `discord` ctor arg, `discord_attempt`/`discord_hung`,
  `time_until_rich_presence_update`, `death_update_rich_presence` and all
  `set_rich_presence_*` calls, and the `update.rs` Discord block.
- **Profiles / scores / PB:** the `Profiles`/`NullProfiles` seam,
  `RealProfiles`, the engine `profiles` field/ctor arg,
  `should_save_score`, `death_save_score_if_needed`,
  `death_save_score_if_needed_and_show_pb_effects`, the PB effect state
  (`must_spawn_pb_particles`, `next_pb_particle_spawn`, `pb_text_growth`,
  `pb_text`) + accessors, the `update.rs` profile-active restart/menu-escape
  branch, the PB particle spawn block, the PB text-growth sawtooth, and the
  local-score save path. `score` / `custom_score` / `score_invalid` and the
  level-status score override are untouched.
- **No config/profile persistence in `oh-app::lifecycle`:**
  `ConfigSaveGuard` and `ProfileSaveGuard` (and their tests) are gone; the
  module keeps `create_folder_if_non_existant` + `load_and_reapply` only.

## Final signatures

`crates/oh-game/src/hexagon_game.rs`:

```rust
pub fn new(
    config: Arc<RwLock<Config>>,
    assets: Arc<oh_assets::Assets>,
    audio: Option<Arc<dyn Audio + 'static>>,
    replay_store: Arc<dyn ReplayStore + 'static>,
    script_host: Arc<dyn ScriptHost + 'static>,
    joystick: Arc<dyn Joystick + 'static>,
    cosmetic_rng: Arc<dyn CosmeticRng + 'static>,
    window: bool,
    renderer: Option<Arc<dyn Renderer + 'static>>,
) -> Self
```

`steam_manager`, `profiles` and `discord` args removed; `replay_store`
stays (02c2 removes it).

`crates/oh-app/src/compose.rs`:

```rust
pub fn compose_with(
    assets: Arc<oh_assets::Assets>,
    root: PathBuf,
    window: bool,
) -> Composed

pub fn compose_with_config(
    assets: Arc<oh_assets::Assets>,
    root: PathBuf,
    window: bool,
    config: Arc<RwLock<Config>>,
    renderer: Option<Arc<dyn Renderer + 'static>>,
    audio: Option<Arc<dyn Audio + 'static>>,
    replay_store: Arc<dyn ReplayStore + 'static>,
    joystick: Arc<dyn oh_game::seams::Joystick + 'static>,
) -> Composed
```

`profiles`, `steam` and `discord` args removed; `replay_store`, `renderer`,
`audio`, `joystick` and `config` stay.

Remaining `oh-game` seams (`seams.rs`): `Audio`/`NoAudio`,
`ReplayStore`/`NullReplayStore`, `ScriptError`/`ScriptHost`/`NoScriptHost`,
`Joystick`/`NoJoystick`, `CosmeticRng`/`PcgCosmeticRng`.

## Removed `GameApi` method / Lua surface

- `GameApi::unlock_achievement` removed from `oh-game/src/lua_state.rs`
  (+ the test mock impls in `lua_state.rs`, `lua_console.rs`,
  `oh-game/tests/common/mod.rs`, `oh-lua/tests/api.rs`).
  `GameApi::in_replay` is KEPT (replay stays until 02c2).
- `crates/oh-lua/src/api/steam.rs` deleted; `mod steam;` +
  `steam::register(...)` gone from `api/mod.rs`;
  `steam_unlockAchievement` removed from the menu no-op list, the metadata
  table and the `tests/api.rs` registered-name set (285 → 284 registered
  globals; the `284` doc-count references updated in `oh-lua`).

## `new_game` replay-name fallback (recorded per spec)

- `new_game`'s last-replay replay-name path
  (`assets.getCurrentLocalProfile().getName()` in the C++): the profiles
  seam is gone, so the replay's `playerName` is the neutral empty string —
  `let rf_name = String::new();` (the C++'s `no_profile`-equivalent
  headless state).
- `death_create_replay_file` always records the C++'s no-active-profile
  spelling — `let rf_name = "no_profile".to_string();`.
- `determinism.rs`'s `assert_replay_pins` player_name pin moved
  `"testProfile"` → `"no_profile"` to match (the C++ test's
  `pSetCurrent("testProfile")` premise is retired). The
  `stream_fixture()` serialization fixture still carries
  `player_name: "testProfile"` — that is byte-oracle fixture data (a
  `ReplayFile` field), not a seam reference.

## RNG-order note (recorded per spec)

The removed PB particle spawn block drew 7 times from the `CosmeticRng`
seam per spawn (every 2.75 s while PB particles were due):
`get_rnd_r(-64.0, width + 64.0)` (x), `get_rnd_r(0.0, 360.0)` (rotation),
`get_rnd_r(0.75, 1.35)` (scale), `get_rnd_i(90, 145)` (alpha),
`get_rnd_r(-12.0, 12.0)`, `get_rnd_r(4.0, 18.0)`, `get_rnd_r(-6.0, 6.0)`.
Those draws no longer occur. Surviving cosmetic draws: the swap/kill
particle spawns (`update.rs`), the random music-segment pick, and the
draw-time camera-shake jitter (`get_rnd_r`). The game (simulation) RNG and
the input/update order are UNCHANGED.

## `lifecycle` no-persistence state

`oh-app/src/lifecycle.rs` public API is now exactly:

- `create_folder_if_non_existant(base_dir: &Path, folder_name: &str)` — the
  generic folder helper (its tests now use neutral `Data/` / `Packs/`
  folders instead of `Profiles/` / `Replays/`).
- `load_and_reapply(...)` — reads config; no writes.

No `ConfigSaveGuard` / `ProfileSaveGuard`, no `Profiles/` or config
writes anywhere in `oh-app`.

## Removed tests / adaptations

Removed (whole files):

- `crates/oh-game/tests/discord_retry.rs` — the Discord retry behaviour
  (seam deleted).
- `crates/oh-game/tests/pb_particle_rng_order.rs` — the PB-particle
  cosmetic-RNG draw-order test (effect deleted).

Adapted (no surviving assertion weakened):

- `crates/oh-game/tests/common/mod.rs` — `TestProfiles` fixture and the
  profiles/steam/discord compose-helper args gone.
- `crates/oh-game/tests/go_to_menu_score.rs` — the no-profile
  death/restart `go_to_menu` test (its premise — a profile becoming
  active — no longer exists) and its `NoLocalProfiles` fixture removed;
  surviving score tests untouched.
- `crates/oh-game/tests/determinism.rs` — constructor call sites updated;
  the `player_name` pin re-based to `"no_profile"` (see fallback note);
  module docs updated.
- `crates/oh-game/tests/{bind_ctor_effects,fps_text,joystick,restart_replay_hints}.rs`
  — constructor call sites updated only.
- `crates/oh-game/src/{lua_state,lua_console}.rs` — mock `GameApi` impls
  lose `unlock_achievement`.
- `crates/oh-app/tests/common/mod.rs` — `TestProfiles` + old compose args
  gone.
- `crates/oh-app/tests/bridge.rs` — the PB text-size assertion removed
  (the engine never sets PB text anymore; the replay-seeded determinism
  pins are unweakened).
- `crates/oh-app/tests/pack_shaders.rs` + `examples/pack_shaders_gl.rs` —
  compose call sites updated.
- `crates/oh-app/src/lifecycle.rs` — the two save-guard test modules
  removed; folder test retargeted to neutral folders.

## Accepted deviations

1. **`oh-app/src/bridge.rs` keeps two inert `DrawState` initializers**
   (supervisor ruling, 2026-10-24):
   `pb_text: graphics::init_text(&font_bold, "", 65.0, zoom)` and
   `pb_text_growth: 0.0` in `AppDraw::new`. `oh_render::graphics::DrawState`
   (02c3's scope) still carries both fields and has no field-skipping
   constructor, so the exhaustive literal must name them; they are dead
   defaults (the engine no longer sets the STRING; `oh-render`'s
   `text_draw` still advances/resizes the inert empty PB text) until 02c3
   removes the fields. Consequence: the spec's
   `rg 'pb_text' crates/oh-app/src` verification is expected to hit
   EXACTLY these two lines until 02c3.
2. **Doc-comment residue** (allowed by the spec; 02c2/02c3/09 sweep):
   the `oh-lua` `api/mod.rs` module doc names
   `steam_unlockAchievement` in its C++-group note, and
   `oh-app/src/lifecycle.rs` doc comments mention the retired `Profiles/`
   folder (one quoting the C++ log-message string the generic helper
   mirrors). No live import/dependency on any removed surface remains.

## Verification (spec block)

```text
$ test ! -d crates/oh-discord && echo discord-crate-gone
discord-crate-gone
$ rg -n 'oh-discord' Cargo.toml crates --glob 'Cargo.toml' || echo no-discord-dep
no-discord-dep
$ rg -n 'SteamSink|NoneSteamSink|NullProfiles|RealProfiles|unlock_achievement|
  steam_unlockAchievement|pb_text|should_save_score|death_save_score' \
  crates/oh-game/src crates/oh-app/src crates/oh-lua/src
crates/oh-app/src/bridge.rs:198:  pb_text: graphics::init_text(&font_bold, "", 65.0, zoom),
crates/oh-app/src/bridge.rs:199:  pb_text_growth: 0.0,
crates/oh-lua/src/api/mod.rs:34: //! ... `steam_unlockAchievement` global ...  (doc comment)
-> oh-game/src + oh-app/src clean of live seam/profile/PB code; the two
   bridge.rs lines are the accepted deviation above; the oh-lua line is a
   doc comment (allowed residue).
$ rg -n 'Profiles/|ProfileSaveGuard|ConfigSaveGuard' crates/oh-app/src
crates/oh-app/src/lifecycle.rs:21: ...'Profiles/' folder does not exist, creating... (doc comment)
crates/oh-app/src/lifecycle.rs:109: ... (doc comment: 02c1 retired the Profiles/ folder)
-> no live persistence code; doc-comment residue only.
```

Replay + fingerprint retention (spec "keep" list, verified present):
`crates/oh-replay` (workspace member), `ReplayStore`/`NullReplayStore`,
`ActiveReplay`, `in_replay`, `last_replay_data`, the replay
record/playback branches, `set_last_replay`, `save_replay` (gate now
`!in_replay() && !status.has_died`) and the 030b fingerprint
path (`fingerprint_file`/`fingerprint_frame`/`fingerprint_last_draws`) —
none renamed or reordered.

## Gate result

`bash tools/gate.sh` → rc 0 (4/4 stages green, `target/gate.log`):

```text
== cargo fmt --all -- --check        -> GATE_EXIT=0
== cargo build --workspace           -> GATE_EXIT=0
== cargo clippy --workspace --all-targets -- -D warnings
                                     -> GATE_EXIT=0
== cargo test --workspace            -> GATE_EXIT=0
```

Test totals (gate.log): 63 suites (test binaries + doc-tests),
**552 passed / 0 failed / 1 ignored** (the single ignored is the
pre-existing `oh-lua` `sandbox.rs` doc-test `ignore`d since issue 007).
02b's tail was 569 passed; the 17-test delta is the removed
Discord/PB/lifecycle/PB-text/Steam test coverage. `Cargo.lock`
regenerated after the manifest changes (`oh-discord` entry gone).

Committed by the orchestrator after the forked `VERDICT: PASS`.
