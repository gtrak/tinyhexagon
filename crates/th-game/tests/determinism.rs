//! The C++-derived level-script / `run_lua_file` pins and the D-019
//! regression guard.
//!
//! This file used to be the ported `test/ReplayExecution.t.cpp`
//! (2 pairs, 25 iterations — D-018) + `test/ReplayExecutionBenchmark.t.cpp`
//! (4 pairs, 100 iterations) + the replay serialization pin
//! (`tools/oracle/replay_probe.cpp` fixture 2). All of those
//! replay self-consistency / recording tests were retired by plan 002
//! issue 02c2 with the replay removal. What survives:
//!
//! * **`run_lua_file_open_failure_message`** — the C++ `Utils::runLuaFile`
//!   open-failure message (C++-derived).
//! * **`run_lua_file_cached_path_executes_real_level_scripts`** — the
//!   headless `newGame` takes the C++ CACHED path and executes the REAL
//!   shipped level scripts: their `onInit` values land in the shared
//!   level status (C++-source-derived expectations, the shipped `.lua`
//!   `onInit` bodies).
//! * **`engine_reuse_after_second_new_game_is_deterministic`** — the D-019
//!   regression guard. The C++'s replay-to-identical-score assertion is
//!   replaced (post-replay-removal) by a PINNED-stream comparison: the
//!   second `new_game` on the same engine, seeded from the same fixed
//!   stream as a fresh engine's first game, must die at the same played
//!   time. A stale timeline runner (D-019) shifts the second game's spawn
//!   schedule and the equality breaks.

mod common;

use std::sync::Arc;

use th_config::Config;
use th_lua::api::state::SharedLua;
use th_lua::LuaContext;
use th_rng::RandomNumberGenerator;

use common::{compose, execute_game_until_death, release_dir, LuaScriptHost};

/// The exact pack/level ids the C++ test names (re-derived via `buildPackId`
/// against the shipped `pack.json`s — D-018; level id = `packId + "_" +
/// levelId`, `MenuGame.cpp:4840`).
const PACKS: [&str; 2] = [
    "ohvrvanilla_vittorio_romeo_cube_1",
    "ohvrvanilla_vittorio_romeo_experimental_1",
];
const LEVELS: [&str; 2] = [
    "ohvrvanilla_vittorio_romeo_cube_1_apeirogon",
    "ohvrvanilla_vittorio_romeo_experimental_1_autotest0",
];

fn load_release() -> Arc<th_assets::Assets> {
    // C++: `hg::HGAssets assets{nullptr, true /* headless */};`
    Arc::new(th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"))
}

// ---------------------------------------------------------------------------
// D-019 regression guard (the ported benchmark's surviving property)
// ---------------------------------------------------------------------------

/// D-019 regression guard: a SECOND `new_game` on the SAME engine must stay
/// deterministic.
///
/// C++ `setLevelData` resets each timeline **and its runner**:
/// `eventTimeline.clear(); eventTimelineRunner = {};` (`HexagonGame.cpp:668-669`),
/// `messageTimeline... + messageTimelineRunner = {};` (`:672-673`),
/// `timeline.clear(); timelineRunner = {};` (`:686-687`). The port
/// originally cleared the three timelines but kept `main_timeline_runner` /
/// `event_timeline_runner` (stale `current_idx` / `wait_start_tp`), so the
/// first `t_wait` / `e_wait` of the NEXT game measured against the previous
/// game's clock: the spawn schedule shifted and the death time moved (D-019).
/// Levels that wait early and heavily on the timelines expose it —
/// `orthoplex/bipolarity` does; `apeirogon` / `autotest0` happen not to (which
/// is why the 2-pair harness passed). `bipolarity.lua`'s `onInit` ends with
/// `t_wait(12 / getMusicDMSyncFactor())` and its `onStep` schedules
/// `e_wait(...)`s.
///
/// The assertion shape: a live run that follows a previous `newGame` on the
/// same engine must still die at the same played time as the SAME game on a
/// FRESH engine, both seeded from the SAME fixed stream (the replay removal
/// (plan 002 issue 02c2) retired the C++'s replay-to-identical-score
/// oracle, so the comparison stream is pinned DIRECTLY through
/// `state_handle()` — `st.rng = RandomNumberGenerator::new(SEED)` after
/// each `new_game`, the C++'s `rng = random_number_generator{seed}`
/// equivalent). **Self-consistent, NOT C++-verified.**
#[test]
fn engine_reuse_after_second_new_game_is_deterministic() {
    const SEED: u64 = 987_654_321;
    let assets = load_release();
    let root = release_dir();
    let pack = "ohvrvanilla_vittorio_romeo_orthoplex_1";
    let level = "ohvrvanilla_vittorio_romeo_orthoplex_1_bipolarity";

    // Baseline: one game on a FRESH engine, pinned to the fixed stream.
    let a = compose(assets.clone(), root.clone(), false);
    a.game.set_always_spin_right(true);
    a.game.new_game(pack, level, true, 1.0);
    a.game.state_handle().lock().rng = RandomNumberGenerator::new(SEED);
    let score_baseline = execute_game_until_death(&a.game).played_time_seconds;
    assert!(
        score_baseline > 0.0,
        "baseline run died with zero played time"
    );

    // D-019: game 1 on engine `c` (a normal first game), then game 2 — the
    // SAME level via a second `newGame` on the SAME engine (the restart
    // path in the real game too), re-pinned to the same stream. With the
    // D-019 bug this run's spawn schedule shifts (its first wait measured
    // against game 1's runner clock) and it dies at a different time.
    let c = compose(assets.clone(), root, false);
    c.game.set_always_spin_right(true);
    c.game.new_game(pack, level, true, 1.0);
    c.game.state_handle().lock().rng = RandomNumberGenerator::new(SEED);
    let score1 = execute_game_until_death(&c.game).played_time_seconds;
    assert!(score1 > 0.0, "game 1 died with zero played time");

    c.game.new_game(pack, level, true, 1.0);
    c.game.state_handle().lock().rng = RandomNumberGenerator::new(SEED);
    let score2 = execute_game_until_death(&c.game).played_time_seconds;
    assert!(score2 > 0.0, "game 2 died with zero played time");

    assert_eq!(
        score2, score_baseline,
        "D-019: the second newGame on the same engine must die at the identical \
         played time of the pinned fresh-engine baseline (exact ==)"
    );
}

// ---------------------------------------------------------------------------
// The C++-derived level-script and `run_lua_file` pins
// ---------------------------------------------------------------------------

/// C++ `Utils::runLuaFile` open failure: `runtime_error("Fatal Lua error\n
/// Could not open file: <name>\n")`. Exercised on a NON-headless host
/// (headless takes the cached path — `HexagonGame::runLuaFile`,
/// `HGScripting.cpp:1130-1141`).
#[test]
fn run_lua_file_open_failure_message() {
    let lua = LuaContext::new();
    let host = LuaScriptHost::new(
        &lua,
        false, /* windowed → the uncached path (use_lua_file_cache defaults false) */
        release_dir(),
        Arc::new(std::sync::RwLock::new(Config::default())),
    );
    let shared = SharedLua::new(&lua);
    let err = host
        .execute_file(&shared, "Packs/does-not-exist.lua")
        .expect_err("missing file must fail");
    assert_eq!(
        err,
        "Fatal Lua error\nCould not open file: Packs/does-not-exist.lua\n"
    );
}

/// The headless `newGame` took the C++ CACHED path (`runLuaFileCached` —
/// `HexagonGame::runLuaFile`: headless ⇒ cache) and executed the REAL shipped
/// level scripts: their `onInit` values land in the shared level status.
/// C++-source-derived expectations (the shipped `.lua` `onInit` bodies):
/// apeirogon (`cube`): `l_setSides(6)` + `l_setRotationSpeed(0.3)`;
/// autotest0 (`experimental`): `l_setSides(12)` + `l_setRotationSpeed(5.0)`
/// (both at `diffMult = 1.0`, i.e. the `u_getDifficultyMult() > 1.5`-false
/// branches).
#[test]
fn run_lua_file_cached_path_executes_real_level_scripts() {
    let assets = load_release();
    for (pack, level, sides, rotation_speed) in [
        (PACKS[0], LEVELS[0], 6_u32, 0.3_f32),
        (PACKS[1], LEVELS[1], 12_u32, 5.0_f32),
    ] {
        let c = compose(assets.clone(), release_dir(), false);
        c.game.new_game(pack, level, true, 1.0);
        assert_eq!(
            c.game.get_sides(),
            sides,
            "{level}: the level script's onInit l_setSides must land in the shared status"
        );
        assert_eq!(
            c.game.get_rotation_speed(),
            rotation_speed,
            "{level}: the level script's onInit l_setRotationSpeed must land in the shared status"
        );
    }
}
