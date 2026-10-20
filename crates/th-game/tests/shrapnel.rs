//! plan 002 issue 09 — the collision shrapnel: a soft hit shatters the
//! wall into wedge pieces and chains into the (up to) two nearest
//! neighbours a short hop apart. Same deterministic fixture discipline
//! as `soft_collisions.rs` (the 04a twin).

mod common;

use std::sync::Arc;

use th_config::TIME_STEP;
use th_game::shrapnel::wall_mid_angle;
use th_game::{CWall, SpeedData};
use th_math::ssvs::Vec2;

use common::{compose, release_dir};

const PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
const LEVEL: &str = "ohvrvanilla_vittorio_romeo_cube_1_apeirogon";

/// The shrapnel constants (the `shrapnel.rs` contract).
const PIECES_PER_WALL: usize = 12;
const CHAIN_WALLS: usize = 2;

fn load_release() -> Arc<th_assets::Assets> {
    Arc::new(th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"))
}

fn fixture(assets: &Arc<th_assets::Assets>) -> common::Composed {
    let c = compose(assets.clone(), release_dir(), false);
    c.game.new_game(PACK, LEVEL, true, 1.0);
    c.game.start();
    c.game.set_soft_collisions(true);
    c
}

fn inject_wall(c: &common::Composed, side: i32, inner: f32, thickness: f32, speed: f32) {
    let center = Vec2::new(0.0, 0.0);
    let s = SpeedData::new(speed, 0.0, 0.0, 0.0, false);
    let cr = SpeedData::new(0.0, 0.0, 0.0, 0.0, false);
    c.game.state_handle().lock().walls.push(CWall::new(
        6, 0.0, 0.0, 0.0, 0.0, &center, side, thickness, inner, &s, &cr, 0.0,
    ));
}

#[test]
fn a_soft_hit_spawns_the_wedge_burst_at_the_impact_point() {
    let assets = load_release();
    let c = fixture(&assets);
    // The hit wall: static, spanning the player's radial band at angle 0
    // (the player sits at angle 0 — the impact point is near it).
    inject_wall(&c, 0, 80.0, 60.0, 0.0);

    assert!(c.game.shrapnel().is_empty());
    c.game.update(TIME_STEP, 1.0);

    let pieces = c.game.shrapnel();
    assert_eq!(
        pieces.len(),
        PIECES_PER_WALL,
        "one burst of wedge pieces per shattered wall"
    );
    // The pieces spawn AT the wall's near vertex (the impact point): the
    // wall's inner edge is 80 away, so every piece starts within ~40 px
    // of it (never at the center — the blast is AT the wall, not the
    // player's own radius... which is the same thing here; the point is
    // they start on the wall, moving outward).
    for p in &pieces {
        assert!(
            p.position.x > 40.0,
            "pieces start at the wall, not the center"
        );
        assert!(p.is_alive());
        assert!(
            p.velocity.x > 0.0,
            "the burst is outward (the wall sat at angle 0)"
        );
    }
    // Exactly ONE bump for the whole explosion (the chain walls die
    // "for free").
    assert_eq!(c.game.bump_serial(), 1);
}

#[test]
fn the_chain_takes_out_the_two_nearest_walls_a_hop_apart() {
    let assets = load_release();
    let c = fixture(&assets);
    // The hit wall at angle 0 + victims at the neighbour sides of a
    // 6-sided arena (side k sits at TAU/6*k): side 1 (60°) and side 5
    // (300°) are the nearest; side 2 (120°) is farther. Only the hit
    // wall overlaps the player (inner 80 spans the radial band; the
    // victims sit farther out at 300 so they cannot collide directly).
    // The level's own pattern walls (spawned at the 1600 distance) are
    // excluded from every assertion via the radius filter below.
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    inject_wall(&c, 1, 300.0, 60.0, 0.0);
    inject_wall(&c, 5, 300.0, 60.0, 0.0);
    inject_wall(&c, 2, 300.0, 60.0, 0.0);

    // The injected victims are the radius < 500 walls (the pattern
    // walls sit at the 1600 spawn distance): `(mid_angle, alive)`.
    let injected = |c: &common::Composed| -> Vec<(u32, bool)> {
        c.game
            .state_handle()
            .lock()
            .walls
            .iter()
            .filter_map(|w| {
                let (s, e, r) = w.angle_span()?;
                (r < 500.0).then(|| ((wall_mid_angle(s, e) * 1000.0).round() as u32, !w.is_dead()))
            })
            .collect()
    };

    c.game.update(TIME_STEP, 1.0); // the hit shatters; the chains arm
    assert_eq!(c.game.bump_serial(), 1, "one bump total");
    let mut state = injected(&c);
    assert_eq!(
        state.iter().filter(|(_, a)| *a).count(),
        3,
        "the hit wall is gone; the three victims wait"
    );
    assert_eq!(
        c.game.shrapnel_chains().len(),
        CHAIN_WALLS,
        "two chain victims queued"
    );
    assert_eq!(c.game.shrapnel().len(), PIECES_PER_WALL);

    // Advance until the first hop fires (5.4 FT units / 0.25 = 22
    // ticks) + a margin.
    for _ in 0..24 {
        c.game.update(TIME_STEP, 1.0);
    }
    state = injected(&c);
    let alive: Vec<u32> = state.iter().filter(|(_, a)| *a).map(|(m, _)| *m).collect();
    assert_eq!(
        alive.len(),
        2,
        "the first hop shattered a neighbour (side 1 at 1.047)"
    );
    assert!(
        !alive.contains(&1048),
        "side 1 (60°) was the first hop's victim"
    );
    assert_eq!(c.game.bump_serial(), 1, "the chain adds no bumps");
    assert_eq!(c.game.shrapnel().len(), 2 * PIECES_PER_WALL);

    // The second hop (another 5.4 units).
    for _ in 0..24 {
        c.game.update(TIME_STEP, 1.0);
    }
    let alive: Vec<u32> = injected(&c)
        .iter()
        .filter(|(_, a)| *a)
        .map(|(m, _)| *m)
        .collect();
    assert_eq!(
        alive.len(),
        1,
        "the second hop shattered the other neighbour (side 5 at 5.236)"
    );
    assert!(
        !alive.contains(&5236),
        "side 5 (300°) was the second hop's victim"
    );
    assert!(
        alive.contains(&2094),
        "side 2 (120°) is NOT a chain victim (it is the far neighbour)"
    );
    assert!(
        c.game.shrapnel_chains().is_empty(),
        "the chain is exhausted (it does not re-chain)"
    );

    // More ticks: no further kills from this hit (bounded), and the
    // pieces all fade within their 48-unit life (192 ticks).
    for _ in 0..200 {
        c.game.update(TIME_STEP, 1.0);
    }
    let alive: Vec<u32> = injected(&c)
        .iter()
        .filter(|(_, a)| *a)
        .map(|(m, _)| *m)
        .collect();
    assert_eq!(
        alive,
        vec![2094],
        "nothing else died from this hit (side 2 survives)"
    );
    assert!(c.game.shrapnel().is_empty(), "the pieces melted away");
    assert_eq!(c.game.bump_serial(), 1);
    assert!(!c.game.get_status().has_died, "never a death in soft mode");
}

#[test]
fn the_pieces_update_ride_drag_spin_and_cull() {
    // Direct piece mechanics through the engine's update (the unit tests
    // in shrapnel.rs pin the math; this pins the LOOP's wiring).
    let assets = load_release();
    let c = fixture(&assets);
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    let pieces = c.game.shrapnel();
    let mut worst_speed = 0.0_f32;
    for p in &pieces {
        worst_speed = worst_speed.max(p.velocity.x);
    }
    assert!(worst_speed > 0.0);

    // One more tick: every piece moved (the loop advances them).
    c.game.update(TIME_STEP, 1.0);
    let moved = c.game.shrapnel();
    assert_eq!(moved.len(), pieces.len());
    for (before, after) in pieces.iter().zip(moved.iter()) {
        assert_ne!(before.position, after.position, "the piece moved");
        assert!(after.velocity.x < before.velocity.x, "the drag slows it");
        assert!(after.life < before.life, "the life burns");
    }
}

#[test]
fn a_new_game_clears_the_shrapnel() {
    let assets = load_release();
    let c = fixture(&assets);
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    inject_wall(&c, 1, 300.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    assert!(!c.game.shrapnel().is_empty());

    c.game.new_game(PACK, LEVEL, true, 1.0);
    assert!(c.game.shrapnel().is_empty(), "the reset clears the pieces");
    assert!(
        c.game.shrapnel_chains().is_empty(),
        "the reset clears the chain queue"
    );
}

#[test]
fn the_wedges_reach_the_draw_batches() {
    // The draw half: draw_scene_batches must emit the wedges into the
    // wall batch (3 vertices per piece, on top of the walls').
    let assets = load_release();
    let c = fixture(&assets);
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    let pieces = c.game.shrapnel().len();

    let (wall_quads, _, _, _) = c.game.draw_scene_batches();
    // 1 killed wall (retained out, 0 wall quads) + the level's own
    // pattern walls… but the WEDGES must be there: at least
    // pieces * 3 vertices beyond any walls. The level walls at the 1600
    // spawn distance contribute their own quads — assert the total is
    // at least the wedges'.
    assert!(
        wall_quads.size() >= pieces * 3,
        "the wedges are in the wall batch ({} vertices for {} pieces)",
        wall_quads.size(),
        pieces
    );
}
