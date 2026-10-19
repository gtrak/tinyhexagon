//! plan 002 issue 06 — the `tiny` content pack: `Assets` loads the pack
//! and all four levels (the three 06 reference levels + the 09-taste
//! continuous `journey`), and each runs 30 s headless with soft
//! collisions (no Lua error, no death) keeping the authored six sides
//! and the per-level speed/delay flavour.
//!
//! The composition shape mirrors `tests/bridge.rs` (`load_release` +
//! `common::compose` + a few ticks per `TIME_STEP`).

mod common;

use std::sync::Arc;

use th_config::TIME_STEP;

use common::{compose, release_dir};

/// `th_data::build_pack_id("tinyhexagon", "tinyhexagon", "tiny", 1)` —
/// the same id scheme the C++ test names use (bridge.rs's
/// `ohvrvanilla_vittorio_romeo_cube_1`).
const PACK: &str = "tinyhexagon_tinyhexagon_tiny_1";

/// (level id suffix, authored `l_setSpeedMult`, authored `l_setDelayMult`).
const LEVELS: &[(&str, f32, f32)] = &[
    ("meadow", 0.55, 1.75),
    ("river", 0.62, 1.70),
    ("stars", 0.70, 1.65),
    ("journey", 0.55, 1.05),
];

/// The bridge test's `_RELEASE` load idiom.
fn load_release() -> Arc<th_assets::Assets> {
    Arc::new(th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"))
}

#[test]
fn tiny_pack_loads_and_runs() {
    let assets = load_release();
    let root = release_dir();

    // The pack and every level id must be present in the loaded Assets
    // (the styles load with their levels via `styleId`).
    assert!(assets.is_valid_pack_id(PACK), "the tiny pack loads");
    for (level, _, _) in LEVELS {
        let level_id = format!("{PACK}_{level}");
        assert!(assets.is_valid_level_id(&level_id), "{level_id} loads");
    }

    for (level, expected_speed, expected_delay) in LEVELS {
        let level_id = format!("{PACK}_{level}");
        let c = compose(assets.clone(), root.clone(), false);
        c.game.set_soft_collisions(true);
        c.game.new_game(PACK, &level_id, true, 1.0);
        c.game.set_must_start(true);
        // 30 s at TIME_STEP (the spec's 30*240 update budget).
        for _ in 0..(30 * 240) {
            if c.game.get_status().has_died {
                break;
            }
            c.game.update(TIME_STEP, 1.0);
            c.game.post_update();
        }
        assert!(!c.game.get_status().has_died, "{level} died");
        assert_eq!(c.game.get_sides(), 6, "{level} sides");
        // The level script's `onInit` authored these — a Lua error in the
        // script (a missing `base` helper, a bad pattern call) would leave
        // the defaults (speed 1.0 / delay 1.0) in place.
        let st = c.game.get_level_status();
        assert_eq!(
            st.speed_mult, *expected_speed,
            "{level} speed_mult (authored by the level script)"
        );
        assert_eq!(
            st.delay_mult, *expected_delay,
            "{level} delay_mult (authored by the level script)"
        );
    }
}
