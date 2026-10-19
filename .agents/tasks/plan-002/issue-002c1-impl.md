# Issue 02c1 — remove Steam, Discord, profiles and PB from the engine

**Umbrella:** `.agents/plans/002-tinyhexagon/02-remove-crates-engine.md`
(02a app headless and 02b front-end crates are landed).
**Do not remove replay or fingerprint yet** (02c2/02c3): keep
`oh-replay`, `ReplayStore`, `ActiveReplay`, `in_replay`, the replay
record/playback branches, `last_replay_data`, and the fingerprint path.

## Goal

Remove the non-replay engine seams the tiny runtime does not use:

- Steam: `SteamSink`/`NoneSteamSink`, the `steam_manager` field/arg, rich
  presence, achievements, the Lua `steam_unlockAchievement` API and
  `GameApi::unlock_achievement`.
- Discord: the `oh-discord` crate and everything using it.
- Profiles/scores/PB: the `Profiles`/`NullProfiles` seam, `RealProfiles`,
  profile persistence, the local-score save path, and the PB text/particle
  effects.
- No config/profile persistence in `oh-app::lifecycle`.

Keep the game's score/custom-score state and display; only the local
profile/PB persistence goes.

## Read first

1. Umbrella issue + `.agents/tasks/plan-002/issue-002a-report.md` /
   `issue-002b-report.md`
2. `crates/oh-game/src/{hexagon_game,update,seams,lua_state,lua_console}.rs`
3. `crates/oh-game/src/status.rs` (PB fields)
4. `crates/oh-lua/src/api/{mod,steam,menu}.rs`, `metadata.rs`
5. `crates/oh-app/src/{compose,lifecycle,bridge}.rs`
6. `crates/oh-app/tests/bridge.rs`
7. `crates/oh-game/tests/` (constructor call sites)

## Steps

### 1. Steam

- Delete `crates/oh-lua/src/api/steam.rs`; remove `mod steam;` and
  `steam::register(...)` from `api/mod.rs`; remove the
  `steam_unlockAchievement` metadata/menu entries.
- Remove `GameApi::unlock_achievement` from `oh-game/src/lua_state.rs` and
  its test mock impl. Keep `GameApi::in_replay` (replay stays until 02c2).
- `oh-game/src/seams.rs`: remove `SteamSink`, `NoneSteamSink` and their docs.
- `hexagon_game.rs`: remove `steam_manager` field/arg, the rich-presence
  calls, the `a22_swapdeath` achievement branches, and `unlock_achievement`.
- `update.rs`: remove the Steam rich-presence/achievement blocks.
- `lua_console.rs`: remove the Steam references.

### 2. Discord

- Delete `crates/oh-discord`, remove it from root members, and remove
  `oh-discord` from `oh-game/Cargo.toml` and `oh-app/Cargo.toml`.
- `hexagon_game.rs`: remove `use oh_discord::Discord`, the `discord` field/arg,
  `discord_attempt`/`discord_hung`, `time_until_rich_presence_update`,
  `death_update_rich_presence`, and all `set_rich_presence_*` calls.
- `update.rs`: remove the Discord callback/rich-presence block.
- `oh-app/src/compose.rs`: remove the `discord` argument/import and pass
  nothing to the engine.

### 3. Profiles / scores / PB

- `oh-game/src/seams.rs`: remove the `Profiles` trait and `NullProfiles`.
- `hexagon_game.rs`: remove the `profiles` field/arg,
  `should_save_score`, `death_save_score_if_needed`,
  `death_save_score_if_needed_and_show_pb_effects`, the PB fields
  (`must_spawn_pb_particles`, `next_pb_particle_spawn`, `pb_text_growth`,
  `pb_text`) and their accessors, and the profile lookup in `new_game`
  (use an empty/neutral replay-name fallback if the replay path still needs
  one; record it).
- `update.rs`: remove the profile-active check, the PB particle block,
  `pb_text_growth` and the score-save call. Keep `score` / `custom_score` /
  `score_invalid` and the level-status score override.
- `status.rs`: remove PB-only fields/accessors; keep score fields.
- `oh-app/src/compose.rs`: remove `RealProfiles` and the `profiles` arg from
  `compose_with`/`compose_with_config`; update the engine constructor call.
- `oh-app/src/lifecycle.rs`: remove `ConfigSaveGuard` and
  `ProfileSaveGuard` (no config/profile writes) and their tests; keep
  `load_and_reapply` and the generic folder helper (update its
  Profiles/Replays-specific tests to a neutral folder).
- `oh-app/src/bridge.rs`: remove the `pb_text`/`pb_text_growth` fields,
  init/update/draw, and the `game.pb_text*()` calls; keep the replay text
  and icon (02c2 removes those).
- `oh-app/src/lib.rs` / `Cargo.toml`: remove `oh-discord` and update
  doc comments that name the removed seams.

### 4. Constructors and call sites

`HexagonGame::new` loses `steam_manager`, `profiles` and `discord`; keep the
other args (including `replay_store`). `compose_with`/`compose_with_config`
lose `profiles`, `steam` and `discord`; keep `replay_store`, `renderer`,
`audio`, `joystick`, `config`. Update every test/helper constructor call.

### 5. Tests

- Remove Steam/Discord/profile/PB tests and helpers.
- Keep the replay/determinism/score tests; update their constructors.
- Do not weaken surviving assertions; delete obsolete ones rather than
  `#[ignore]`.

### 6. Gate

```sh
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tools/gate.sh
```

All green. If a test needs a new neutral fixture after profiles are gone,
add it; do not weaken the deterministic score/timing assertions.

## Verification

```sh
test ! -d crates/oh-discord && echo discord-crate-gone
rg -n 'oh-discord' Cargo.toml crates --glob 'Cargo.toml' || echo no-discord-dep
rg -n 'SteamSink|NoneSteamSink|NullProfiles|RealProfiles|unlock_achievement|steam_unlockAchievement|pb_text|should_save_score|death_save_score' \
  crates/oh-game/src crates/oh-app/src crates/oh-lua/src || echo seams-clean
rg -n 'Profiles/|ProfileSaveGuard|ConfigSaveGuard' crates/oh-app/src || echo no-persistence
bash tools/gate.sh; echo "gate_rc=$?"
```

Doc-comment residue may remain (02c2/02c3/09 sweep); no live import or
dependency may remain.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-002c1-report.md` with:

- The final `HexagonGame::new` and `compose_with`/`compose_with_config`
  signatures.
- The removed `GameApi` method, Steam/Discord/profile/PB surfaces.
- The `new_game` replay-name fallback and any RNG-order note (cosmetic PB
  particle RNG changes are acceptable; record them).
- The `lifecycle` no-persistence state.
- Removed tests/adaptations.
- The gate result/tail and "No commit made."

Update `FEED-FORWARD.md` with `### 02c1 — Steam/Discord/profiles/PB (LANDED)`:
the signatures, removed API, fallback/RNG notes, remaining replay/fingerprint
for 02c2/02c3. Do not mark it LANDED in `PLAN.md`; the orchestrator does that
at commit.

## Risks

- Replay and fingerprint stay; do not remove their branches or `oh-replay`.
- Removing profile/PB effects may change the COSMETIC RNG stream; record it.
  Do not change the simulation/update order or the game RNG.
- The score/custom-score/`score_invalid` state stays; only persistence/PB
  effects go.
- The `new_game` replay-name fallback must compile with the replay path still
  present; 02c2 removes it.
