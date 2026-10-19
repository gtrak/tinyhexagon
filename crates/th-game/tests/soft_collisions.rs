//! plan 002 issue 04a — the kid engine mechanics: the soft (no-death)
//! collision branch, the bump serial, the live difficulty setter.
//!
//! The fixture is deterministic: a FRESH composed engine, one level
//! (`apeirogon`), walls injected directly into the shared store at a
//! known distance, and NO player input (the player sits at angle 0). The
//! level's own pattern-spawned walls are all at the 1600 spawn distance
//! when the tests observe, so they cannot collide in the test windows.
//!
//! The apeirogon pulse + beat pulse keep the player's radius in the band
//! `[72, 118.5]` (radius = 72 * pulse/64 + beat_pulse; pulse in [64, 84],
//! beat_pulse in [0, 24]), starting at 107.7 on tick 1. At the player's
//! angle (the side-0 bisector) a 6-sided wall's radial extent is its
//! chord `dist * cos(30°)`, so the static walls below (inner 80,
//! thickness 60) span `69.3..121.2` at the bisector and cover the whole
//! band.

mod common;

use std::sync::Arc;

use th_config::TIME_STEP;
use th_game::{CWall, SpeedData};
use th_math::ssvs::Vec2;

use common::{compose, release_dir};

const PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
const LEVEL: &str = "ohvrvanilla_vittorio_romeo_cube_1_apeirogon";

fn load_release() -> Arc<th_assets::Assets> {
    Arc::new(th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"))
}

/// A fresh engine with the game started and (optionally) soft collisions
/// on.
fn fixture(assets: &Arc<th_assets::Assets>, soft: bool) -> common::Composed {
    let c = compose(assets.clone(), release_dir(), false);
    c.game.new_game(PACK, LEVEL, true, 1.0);
    c.game.start();
    c.game.set_soft_collisions(soft);
    c
}

/// Injects a static or moving 6-sided wall at `side` with the given inner
/// distance / thickness / speed into the shared store's wall list.
fn inject_wall(c: &common::Composed, side: i32, inner: f32, thickness: f32, speed: f32) {
    let center = Vec2::new(0.0, 0.0);
    let s = SpeedData::new(speed, 0.0, 0.0, 0.0, false);
    let cr = SpeedData::new(0.0, 0.0, 0.0, 0.0, false);
    c.game.state_handle().lock().walls.push(CWall::new(
        6, 0.0, 0.0, 0.0, 0.0, &center, side, thickness, inner, &s, &cr, 0.0,
    ));
}

// --- soft vs non-soft, same collision ------------------------------------

#[test]
fn soft_collision_kills_the_wall_not_the_player() {
    let assets = load_release();
    let c = fixture(&assets, true);
    // A static wall spanning the player's whole radial band at the
    // bisector (chord 69.3..121.2 covers [72, 118.5]) at angle 0, where
    // the player sits.
    inject_wall(&c, 0, 80.0, 60.0, 0.0);

    c.game.update(TIME_STEP, 1.0);

    let st = c.game.get_status();
    assert!(!st.has_died, "soft collision must not kill the player");
    assert_eq!(c.game.bump_serial(), 1, "one bump per killed wall");
    assert_eq!(st.flash_effect, 120.0, "the bump flash is 120");
    assert!(
        c.game.state_handle().lock().walls.is_empty(),
        "the killed wall is retained out"
    );
}

#[test]
fn non_soft_collision_kills_the_player_not_the_wall() {
    let assets = load_release();
    let c = fixture(&assets, false);
    inject_wall(&c, 0, 80.0, 60.0, 0.0);

    c.game.update(TIME_STEP, 1.0);

    let st = c.game.get_status();
    assert!(st.has_died, "the classic path must still die");
    assert_eq!(c.game.bump_serial(), 0, "no bump on the death path");
    assert_eq!(
        st.flash_effect, 255.0,
        "the death flash (255), not the bump flash"
    );
    let walls = {
        let g = c.game.state_handle().lock();
        g.walls.clone()
    };
    assert_eq!(walls.len(), 1, "the wall survives (only the player dies)");
    assert!(!walls[0].is_dead());
}

// --- a wall driven in: soft bumps, non-soft dies, on the same tick -------

#[test]
fn soft_run_driven_into_a_wall_bumps_never_dies() {
    let assets = load_release();
    let c = fixture(&assets, true);
    // Inner edge 160 (chord 138.6 at the bisector > the 118.5 band top,
    // so no overlap at spawn) -> approaches at 0.625 px/tick (0.5 * 5 *
    // TIME_STEP) and reaches the band around tick 80, when the beat
    // pulse re-fires.
    inject_wall(&c, 0, 160.0, 40.0, 0.5);

    let mut bump_tick = None;
    for tick in 1..=150u32 {
        c.game.update(TIME_STEP, 1.0);
        if c.game.bump_serial() >= 1 {
            bump_tick = Some(tick);
            break;
        }
    }
    let bump_tick = bump_tick.expect("the approaching wall must reach the player");
    assert!(bump_tick > 1, "no pre-existing overlap at spawn");
    assert!(
        !c.game.get_status().has_died,
        "a soft run driven into a wall must never die"
    );
}

#[test]
fn non_soft_run_driven_into_the_same_wall_dies_on_the_same_tick() {
    // Both modes over the SAME injected wall: the classic death tick must
    // equal the soft bump tick (the first tick the wall overlaps the
    // player) — the non-soft path is behaviourally unchanged.
    let assets = load_release();

    let soft_c = fixture(&assets, true);
    inject_wall(&soft_c, 0, 160.0, 40.0, 0.5);
    let mut soft_tick = None;
    for tick in 1..=150u32 {
        soft_c.game.update(TIME_STEP, 1.0);
        if soft_c.game.bump_serial() >= 1 {
            soft_tick = Some(tick);
            break;
        }
    }
    let soft_tick = soft_tick.expect("soft bump");

    let hard_c = fixture(&assets, false);
    inject_wall(&hard_c, 0, 160.0, 40.0, 0.5);
    let mut hard_tick = None;
    for tick in 1..=150u32 {
        hard_c.game.update(TIME_STEP, 1.0);
        if hard_c.game.get_status().has_died {
            hard_tick = Some(tick);
            break;
        }
    }
    let hard_tick = hard_tick.expect("the classic path must die");

    assert_eq!(hard_tick, soft_tick, "same collision, same tick");
}

// --- bump serial semantics ------------------------------------------------

#[test]
fn bump_serial_advances_once_per_killed_wall() {
    let assets = load_release();
    let c = fixture(&assets, true);

    assert_eq!(c.game.bump_serial(), 0);

    // First collision.
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    assert_eq!(c.game.bump_serial(), 1, "first collision");

    // Repeated updates: the killed wall was retained out, so no
    // double-count.
    c.game.update(TIME_STEP, 1.0);
    c.game.update(TIME_STEP, 1.0);
    assert_eq!(
        c.game.bump_serial(),
        1,
        "a killed wall must not double-bump"
    );

    // A fresh collision bumps again.
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    assert_eq!(c.game.bump_serial(), 2, "second collision");

    // Two overlapping walls in one frame: exactly one bump each.
    inject_wall(&c, 0, 80.0, 60.0, 0.0); // chord 69.3..121.2
    inject_wall(&c, 0, 90.0, 50.0, 0.0); // chord 77.9..121.2 (both overlap the player)
    c.game.update(TIME_STEP, 1.0);
    assert_eq!(c.game.bump_serial(), 4, "one bump per killed wall");
    assert!(!c.game.get_status().has_died);
}

// --- custom walls ---------------------------------------------------------

/// Injects a custom quad around the player (the player sits at
/// `(radius, 0)` with radius in [72, 118.5]).
fn inject_custom_quad(c: &common::Composed) -> i32 {
    c.game.state_handle().lock().cw_manager.create(|cw| {
        cw.set_vertex_pos(0, Vec2::new(60.0, -20.0));
        cw.set_vertex_pos(1, Vec2::new(125.0, -20.0));
        cw.set_vertex_pos(2, Vec2::new(125.0, 20.0));
        cw.set_vertex_pos(3, Vec2::new(60.0, 20.0));
    })
}

#[test]
fn soft_custom_wall_collision_kills_the_wall_not_the_player() {
    let assets = load_release();
    let c = fixture(&assets, true);
    let h = inject_custom_quad(&c);

    c.game.update(TIME_STEP, 1.0);

    assert!(
        !c.game.get_status().has_died,
        "soft: a custom-wall hit must not kill the player"
    );
    assert_eq!(
        c.game.bump_serial(),
        1,
        "one bump for the killed custom wall"
    );
    let cw_count = c.game.state_handle().lock().cw_manager.count();
    assert_eq!(cw_count, 0, "the custom wall was killed");
    // The handle was destroyed cleanly (reused by the next create).
    let h2 = c
        .game
        .state_handle()
        .lock()
        .cw_manager
        .create(|cw| cw.reset());
    assert_eq!(h2, h, "the killed handle is back on the free list");
}

#[test]
fn non_soft_custom_wall_collision_kills_the_player() {
    let assets = load_release();
    let c = fixture(&assets, false);
    inject_custom_quad(&c);

    c.game.update(TIME_STEP, 1.0);

    assert!(
        c.game.get_status().has_died,
        "the classic path must still die on a custom-wall hit"
    );
    assert_eq!(c.game.bump_serial(), 0, "no bump on the death path");
    assert_eq!(
        c.game.state_handle().lock().cw_manager.count(),
        1,
        "the custom wall survives (only the player dies)"
    );
}

// --- difficulty setter ----------------------------------------------------

#[test]
fn set_difficulty_mult_writes_the_shared_mult() {
    let assets = load_release();
    let c = fixture(&assets, false);
    assert_eq!(c.game.difficulty_mult(), 1.0);

    c.game.set_difficulty_mult(3.0);
    assert_eq!(c.game.difficulty_mult(), 3.0);
    // The existing speed/delay paths read it:
    assert_eq!(
        c.game.get_music_dm_sync_factor(),
        3.0_f32.powf(0.12),
        "getMusicDMSyncFactor = dm^0.12"
    );
    // getSpeedMultDM = min(speedMax, speedMult * dm^0.65) (apeirogon caps
    // the speed at 3.6).
    let ls = c.game.get_level_status();
    let expected_speed = if ls.has_speed_max_limit() {
        (ls.speed_mult * 3.0_f32.powf(0.65)).min(ls.speed_max)
    } else {
        ls.speed_mult * 3.0_f32.powf(0.65)
    };
    assert_eq!(
        c.game.state_handle().lock().get_speed_mult_dm(),
        expected_speed,
        "getSpeedMultDM uses the new mult"
    );
}

#[test]
fn set_difficulty_mult_clamps_out_of_range() {
    let assets = load_release();
    let c = fixture(&assets, false);

    c.game.set_difficulty_mult(99.0);
    assert_eq!(c.game.difficulty_mult(), 4.0, "clamped to the top");
    c.game.set_difficulty_mult(1e9);
    assert_eq!(c.game.difficulty_mult(), 4.0, "clamped to the top");
    c.game.set_difficulty_mult(0.001);
    assert_eq!(c.game.difficulty_mult(), 0.1, "clamped to the floor");
    c.game.set_difficulty_mult(0.0);
    assert_eq!(c.game.difficulty_mult(), 0.1, "clamped to the floor");
    c.game.set_difficulty_mult(1.5);
    assert_eq!(
        c.game.difficulty_mult(),
        1.5,
        "in-range values pass through"
    );
}
