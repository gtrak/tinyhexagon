# Issue 02 — Remove retired crates; de-scope the engine

**Status:** LANDED (02a/02b/02c1/02c2/02c3)
**Depends:** 01

**Execution split (2026-10-19):** this issue is executed as **02a** (strip
`oh-app` to a headless library; extract `draw_target.rs`; salvage
`web_fullscreen.rs`; stub `oh-android`/`oh-web`), **02b** (remove the retired
front-end/online crates: menu, menu-system, additions, net, steam,
server-control, workshop-uploader), and **02c1** (remove Steam/Discord/profiles/PB; remove `oh-discord`),
**02c2** (remove replay; remove `oh-replay`), and **02c3** (remove the parity
fingerprint/RNG and finish the engine cleanup).
Each starts after the previous lands and consumes its feed-forward. The
contract below is the umbrella; the sub-issue task specs carry the per-step
detail.

## Objective

Remove the retired crates and the engine surfaces they supported: replay,
Steam, Discord, profiles/scores, and the parity fingerprint. The remaining
engine must compile with the kid-relevant seams only. This is the largest
mechanical issue; do it compile-driven, one module at a time.

## Removed crates

`th-parity`, `th-replay`, `th-net`, `th-steam`, `th-discord`,
`th-server-control`, `th-workshop-uploader`, `th-menu`, `th-menu-system`,
`th-additions`.

- `th-additions` is removed per the plan's elaboration. Before deleting it,
  copy `crates/oh-additions/src/fullscreen.rs` to
  `th-app/src/web_fullscreen.rs` and adjust imports (it uses
  `th-render::Window` + `th-config`); it is wired in Issue 08.
- The touch overlay/soft input/menu injection are not needed; the kid touch
  mapping is implemented in `th-app` (Issue 07).
- Remove the crates from the workspace members and from every dependent
  `Cargo.toml`.

## Engine de-scope

### Replay (`th-game`)

Remove imports and paths in:

- `hexagon_game.rs`: `ReplayFile`, `ReplayData`, `CompressedReplayFile`,
  `ActiveReplay`, the replay cursor, `last_replay_data`, `last_first_play`,
  `last_played_score`, `active_replay`, `set_last_replay`, `in_replay`,
  `must_replay_input`, `must_show_replay_ui`, `active_replay_file`,
  `last_replay_data`, `set_on_death_replay_created`,
  `run_replay_until_death_and_get_score`, `save_replay`,
  `death_create_replay_file`, `death_send_and_save_replay`,
  `death_save_score_if_needed`; `new_game` loses `execute_last_replay`.
- `update.rs`: the `InputBits` import, the replay input branch, the recording
  call `update_input_record_current_input_to_last_replay_data`, and the
  replay-state checks.
- `seams.rs`: `ReplayStore`, `NullReplayStore`, `CompressedReplayFile`.
- `lua_console.rs`: the `last_replay_data` field.
- Remove replay-related tests; keep pure simulation tests.
- Remove the parity fingerprint path (`fingerprint_frame`,
  `fingerprint_last_draws`, `set_fingerprint_path`, the fingerprint output in
  `update`) and the parity-only RNG state accessors if no other caller needs
  them.

### Profiles / scores / PB

- Remove the `Profiles` seam, `NullProfiles`, `RealProfiles`, and the engine's
  local-score/PB paths (`should_save_score`, `death_save_score_if_needed`,
  PB particle spawning if it depends on profile scores).
- Remove `th-app`'s `RealProfiles`; the composition passes no profile seam.
- No `Profiles/` writes, no score persistence.

### Steam / Discord

- `HexagonGame::new` loses `steam_manager` and `discord`; remove the
  `SteamSink` trait, `NoneSteamSink`, the `Discord` field/import, the
  rich-presence calls (`start`, death, `update_rich_presence_callbacks`),
  `discord_attempt`/`discord_hung`, and the achievement hooks
  (`a22_swapdeath`, Lua `steam_unlockAchievement`).
- `th-game` drops its `th-discord` dependency.
- `th-app` loses `steam.rs`; `th-lua/src/api/steam.rs` is removed and the
  registration list updated. Remove `GameApi::in_replay` and
  `GameApi::unlock_achievement`.

### `th-app` cleanup

Keep the pieces the tiny runtime needs; remove the adult front-end.

**Keep/adapt:** `compose.rs` (de-scoped seams), `bridge.rs`, `camera.rs`,
`lifecycle.rs` (root resolution only; no config save), `sigint.rs`,
`joystick.rs` (desktop optional), and a new `draw_target.rs` extracted from
`windowed.rs` (the `GlDrawTarget` implementation and its helpers).

**Remove:** `menu_draw.rs`, `windowed.rs` (after extracting
`GlDrawTarget`), `lua_console_ui.rs`, `client.rs`, `dispatch.rs`,
`print_lua_docs.rs`, `replay_store.rs`, `steam.rs`, `game_state.rs`,
`args.rs` (unless a minimal `--headless` test switch is useful), `android.rs`
(the Android entry is repurposed in `th-android`), the `open-hexagon` binary
target (the binary becomes `tinyhexagon` in Issue 07), the egui dependencies,
and the `steam`/`produce-lua-metadata` features.

**Temporary state:** `th-app` may be a headless library plus a stub binary
until Issue 07; `th-android`/`th-web` may be stubbed entry points until Issue
08. The workspace must still build and test.

### Lua

- Remove `steam` registration and `in_replay`/`unlock_achievement` from the
  `GameApi` trait.
- Remove score/replay globals that call removed engine methods, or make them
  no-ops if a shipped base script still references them. The tiny pack does
  not use them; the base helper scripts are checked in Issue 06.
- `th-lua` keeps `th-game` as its dependency; `th-game` no longer depends on
  replay/discord.

### Composition

- `compose_with_config` loses the profile, replay-store and Discord args;
  `HexagonGame::new` loses the Steam/Discord args.
- `th-app`'s composition passes `None` audio (or `th-audio` later), no
  profiles, no replay store, `NoJoystick` on mobile/web.
- `th-app`'s `th-synth` dependency is added in Issue 05, not here.

## Files

| File | Change |
|---|---|
| root `Cargo.toml` | Remove retired members. |
| `th-game/src/{hexagon_game,update,seams,lua_console}.rs` | De-scope. |
| `th-game/src/{lua_state,status}.rs` | Remove replay/profile/steam hooks if present. |
| `th-game/Cargo.toml` | Remove `th-replay`, `th-discord`. |
| `th-lua/src/api/{mod,steam}.rs`, `th-game/src/lua_state.rs` | Remove steam/replay hooks. |
| `th-app/src/*` | Keep/adapt/remove per above; add `draw_target.rs` and `web_fullscreen.rs`. |
| `th-app/Cargo.toml` | Remove retired/egui deps; keep engine/render/audio. |
| `th-android/`, `th-web/` | Temporary stubs if needed. |
| Tests | Delete replay/parity/profile/PB tests; update constructors. |

## Steps

1. Remove the retired crates and workspace members.
2. Copy `fullscreen.rs` → `th-app/src/web_fullscreen.rs`; delete
   `th-additions`.
3. De-scope `th-game` replay, then profiles/scores/PB, then Steam/Discord,
   compiling after each group.
4. Update `th-lua` registrations and the `GameApi` trait.
5. Strip `th-app` to the tiny-runtime keep-list; extract `draw_target.rs`.
6. Update `compose.rs` constructors and all call sites/tests.
7. Stub `th-android`/`th-web` entries if their adult calls no longer exist.
8. Delete retired tests; do not weaken surviving ones.
9. Remove the parity-only RNG state API if unused.

## Verification

- `cargo build --workspace`, `cargo clippy --workspace --all-targets -D
  warnings`, `cargo test --workspace` green.
- `rg 'th-replay|th-net|th-steam|th-discord|th-menu|th-additions|th-parity'`
  in `Cargo.toml`/sources returns no dependency references.
- `HexagonGame::new` has no Steam/Discord/profile/replay arguments.
- No `Profiles`, `ReplayStore`, `ActiveReplay`, `fingerprint` references in
  `th-game`/`th-app`.
- The remaining engine tests pass; the deleted tests are gone, not ignored.
- `th-app` builds as a library; its temporary bin (if any) exits cleanly.

## Risks

- Replay is threaded through the engine; deleting it can silently change the
  input/update order. Remove the record/playback branches only; do not
  reorder the surviving simulation.
- The profile/PB path touches particles and scores; remove it without
  disturbing the RNG stream used by the simulation. If a stream change is
  unavoidable, record it in `FEED-FORWARD.md` (there is no replay parity to
  preserve any more, but determinism/consistency still matters).
- `windowed.rs` contains both adult UI and the reusable `GlDrawTarget`; extract
  the target before deleting the file.
- Web/Android stubs must compile on their targets; `cargo check` them in
  Issue 08, not only on host.

## Feed-forward

Record: the final `HexagonGame::new` signature, the removed `GameApi` methods,
the extracted `draw_target.rs` API, any Lua globals removed/no-oped, and any
RNG-order changes. Issues 04/07/08 depend on these.
