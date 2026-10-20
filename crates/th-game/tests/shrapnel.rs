//! plan 002 issue 09 (rework) — the collision shrapnel: a soft hit
//! shatters the wall into GLASS SHARDS — irregular pieces cut from the
//! wall's actual quad — which burst under a real physics sim (mass,
//! radial impulse from the impact point + tangential jitter, drag,
//! tumble, elastic core bounces) and chain into the (up to) two nearest
//! neighbours a short hop apart. Same deterministic fixture discipline
//! as `soft_collisions.rs` (the 04a twin).

mod common;

use std::sync::Arc;

use th_config::TIME_STEP;
use th_game::polygon::point_in_polygon;
use th_game::shrapnel::wall_mid_angle;
use th_game::{CWall, SpeedData};
use th_math::ssvs::vector2::get_mag;
use th_math::ssvs::Vec2;

use common::{compose, release_dir};

const PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
const LEVEL: &str = "ohvrvanilla_vittorio_romeo_cube_1_apeirogon";

/// The shrapnel constants (the `shrapnel.rs` contract).
const CHAIN_WALLS: usize = 2;
const SHARDS_PER_WALL_MIN: usize = 6;
const SHARDS_PER_WALL_MAX: usize = 10;
const MAX_LIVE_SHARDS: usize = 64;
/// One 240 Hz tick in FT units (the engine's `m_ft` scale: 60 = 1 s).
const TICK_FT: f32 = 0.25;
/// The chain hops in FT units: 5.4 = 90 ms → fires 22 ticks after the
/// hit (21 ticks leave 0.15 FT of delay; the 22nd crosses zero); 10.8
/// = 180 ms → 44 ticks.
const HOP1_TICKS: u32 = 22;
const HOP2_TICKS: u32 = 44;

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

/// The injected walls' state, `(mid_angle×1000 rounded, alive)` — the
/// level's own pattern walls (spawned at the 1600 distance) are excluded
/// by the radius filter.
fn injected(c: &common::Composed) -> Vec<(u32, bool)> {
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
}

#[test]
fn a_soft_hit_shatters_the_wall_into_glass_shards_cut_from_its_quad() {
    let assets = load_release();
    let c = fixture(&assets);
    // The hit wall: static, spanning the player's radial band at angle 0
    // (the player sits at angle 0 — the impact point is near it).
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    let quad = {
        let g = c.game.state_handle().lock();
        g.walls.last().unwrap().get_vertex_positions()
    };

    assert!(c.game.shrapnel().is_empty());
    let p0 = c.game.get_player_position();
    c.game.update(TIME_STEP, 1.0);

    let shards = c.game.shrapnel();
    assert!(
        (SHARDS_PER_WALL_MIN..=SHARDS_PER_WALL_MAX).contains(&shards.len()),
        "one burst of 6..10 pieces per shattered wall (got {})",
        shards.len()
    );
    // The impact point: the wall vertex nearest the player (the engine's
    // blast origin).
    let impact = quad
        .iter()
        .copied()
        .min_by(|a, b| {
            let da = (*a - p0).x * (*a - p0).x + (*a - p0).y * (*a - p0).y;
            let db = (*b - p0).x * (*b - p0).x + (*b - p0).y * (*b - p0).y;
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();
    // Every piece is cut from the wall's ACTUAL quad: all of its
    // vertices sit inside (or on the edges of) the wall's 4-vertex
    // shape (the burst is AT the wall, not the center) — and each
    // piece's velocity carries the RADIAL impulse from the impact
    // point (the tangential jitter only shaves it, never inverts it).
    // (Containment is tested with the even-odd ray cast after a tiny
    // inward nudge: the cross-product test is sign-ambiguous for
    // vertices that lie exactly on the quad's edges in f32.)
    let center = (quad[0] + quad[1] + quad[2] + quad[3]) * 0.25;
    for s in &shards {
        for p in s.local {
            let world = s.position + p;
            let nudged = world + (center - world) * 1.0e-4;
            assert!(
                point_in_polygon(&quad, nudged.x, nudged.y),
                "shard vertex {:?} is inside the wall quad",
                world
            );
        }
        assert!(s.is_alive());
        let to_shard = s.position - impact;
        let dot = s.velocity.x * to_shard.x + s.velocity.y * to_shard.y;
        assert!(
            dot >= -1.0e-4,
            "the radial impulse from the impact point ({} vs dir {:?})",
            dot,
            to_shard
        );
        // The glass look: the perimeter color is lightened toward white.
        assert!(s.color_out.r >= s.color_in.r);
        assert!(s.color_out.g >= s.color_in.g);
        assert!(s.color_out.b >= s.color_in.b);
    }
    // Exactly ONE bump for the whole explosion (the chain walls die
    // "for free").
    assert_eq!(c.game.bump_serial(), 1);
}

#[test]
fn the_chain_takes_out_the_two_nearest_walls_a_hop_apart_with_no_extra_bumps() {
    let assets = load_release();
    let c = fixture(&assets);
    // The hit wall at angle 0 + victims at the neighbour sides of a
    // 6-sided arena (side k sits at TAU/6*k): side 1 (60°) and side 5
    // (300°) are the two nearest (tied at one hop of angular distance);
    // side 2 (120°) is farther. Only the hit wall overlaps the player
    // (inner 80 spans the radial band; the victims sit farther out at
    // 300 so they cannot collide directly).
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    inject_wall(&c, 1, 300.0, 60.0, 0.0);
    inject_wall(&c, 5, 300.0, 60.0, 0.0);
    inject_wall(&c, 2, 300.0, 60.0, 0.0);

    c.game.update(TIME_STEP, 1.0); // the hit shatters; the chains arm
    assert_eq!(c.game.bump_serial(), 1, "one bump total");
    let state = injected(&c);
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
    let burst1 = c.game.shrapnel().len();
    assert!((SHARDS_PER_WALL_MIN..=SHARDS_PER_WALL_MAX).contains(&burst1));

    // The first hop is DUE at 5.4 FT = 21.6 ticks: after 21 ticks the
    // delay is still 0.15 FT (no fire), the 22nd tick crosses zero.
    for _ in 0..HOP1_TICKS - 1 {
        c.game.update(TIME_STEP, 1.0);
    }
    assert_eq!(
        injected(&c).iter().filter(|(_, a)| *a).count(),
        3,
        "no hop has fired yet (5.4 FT not reached)"
    );
    c.game.update(TIME_STEP, 1.0);
    let after_hop1 = injected(&c).iter().filter(|(_, a)| *a).count();
    assert_eq!(
        after_hop1, 2,
        "the first hop shattered exactly one neighbour"
    );
    assert!(
        injected(&c).iter().any(|(m, a)| m == &2094 && *a),
        "side 2 (120°) is NOT a chain victim (the far neighbour)"
    );
    assert_eq!(c.game.bump_serial(), 1, "the chain adds no bumps");
    assert!(
        c.game.shrapnel().len() >= burst1 + SHARDS_PER_WALL_MIN,
        "the victim shattered into its own shard set"
    );

    // The second hop is due at 10.8 FT = 42.75 ticks: the 43rd tick
    // after the hit fires it; the 42nd does not.
    for _ in 0..HOP2_TICKS - HOP1_TICKS - 1 {
        c.game.update(TIME_STEP, 1.0);
    }
    assert!(
        injected(&c).iter().filter(|(_, a)| *a).count() >= 2,
        "the second hop has not fired yet (10.8 FT not reached)"
    );
    c.game.update(TIME_STEP, 1.0);
    let alive: Vec<u32> = injected(&c)
        .iter()
        .filter(|(_, a)| *a)
        .map(|(m, _)| *m)
        .collect();
    assert_eq!(
        alive,
        vec![2094],
        "the second hop shattered the other neighbour; only the far neighbour (side 2 at 120°) survives"
    );
    assert!(
        c.game.shrapnel_chains().is_empty(),
        "the chain is exhausted (it does not re-chain)"
    );
    assert!(
        c.game.shrapnel().len() <= MAX_LIVE_SHARDS,
        "the shard cap holds"
    );

    // More ticks: no further kills from this hit (bounded), and the
    // shards all melt away within their 48-unit life (192 ticks).
    for _ in 0..200 {
        c.game.update(TIME_STEP, 1.0);
    }
    assert_eq!(
        injected(&c)
            .iter()
            .filter(|(_, a)| *a)
            .map(|(m, _)| *m)
            .collect::<Vec<_>>(),
        vec![2094],
        "nothing else died from this hit (side 2 survives)"
    );
    assert!(c.game.shrapnel().is_empty(), "the shards melted away");
    assert_eq!(c.game.bump_serial(), 1);
    assert!(!c.game.get_status().has_died, "never a death in soft mode");
}

#[test]
fn shard_physics_ride_momentum_drag_tumble_and_cull() {
    // Direct shard mechanics through the engine's update (the unit tests
    // in shrapnel.rs pin the exact math; this pins the LOOP's wiring).
    // A hit wall (inner 80, spanning the player's radial band; a thick
    // wall so the outer fan pieces sit well past the breathing core):
    // some shards spawn inside the core and bounce (that is the
    // core-bounce wiring — pinned exactly in shrapnel.rs's unit tests),
    // so the exact-Euler assertions apply to the shards that stayed
    // outside the core (same-burst pairs skip the pairwise bounce, so
    // nothing else perturbs them).
    let assets = load_release();
    let c = fixture(&assets);
    inject_wall(&c, 0, 80.0, 120.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    let before = c.game.shrapnel();
    assert!(!before.is_empty());

    // One more tick: the life burns and the speed decays for EVERY
    // shard (drag, or the bounce's damping), the tumble integrates.
    // Shards that did NOT bounce off the core integrate EXACTLY (the
    // bounce fires only for a shard that ends its step INSIDE the core
    // edge moving INWARD — predicted here with the same math, so the
    // exactness check is robust to where the shards happen to land).
    // Same-burst pairs skip the pairwise bounce, so nothing else
    // perturbs them.
    let core_radius = c.game.get_radius() * 1.05;
    c.game.update(TIME_STEP, 1.0);
    let after = c.game.shrapnel();
    assert_eq!(after.len(), before.len());
    let drag = 1.0_f32 - (0.9_f32 / 60.0_f32) * TICK_FT;
    let spin_decay = 1.0_f32 - 0.02_f32 * TICK_FT;
    let mut exact = 0;
    for (b, a) in before.iter().zip(after.iter()) {
        assert!((a.life - (b.life - TICK_FT)).abs() < 1e-6, "the life burns");
        assert!(
            get_mag(&a.velocity) < get_mag(&b.velocity),
            "the speed decays (drag, or the bounce's energy loss)"
        );
        assert!(
            (a.angle - (b.angle + b.spin * TICK_FT)).abs() < 1e-6,
            "the tumble integrates the spin"
        );
        // Predict the bounce with the engine's own order: step first
        // (Euler on the pre-drag velocity), drag, then the radial
        // check on the post-step position / post-drag velocity.
        let pos_after = b.position + b.velocity * TICK_FT;
        let vel_after = b.velocity * drag;
        let mag_after = get_mag(&pos_after);
        let bounced = mag_after > 1.0e-4 && mag_after < core_radius && {
            let n = pos_after * (1.0 / mag_after);
            vel_after.x * n.x + vel_after.y * n.y < 0.0
        };
        if !bounced {
            exact += 1;
            assert!(
                (a.spin - b.spin * spin_decay).abs() < 1e-6,
                "the spin decays"
            );
            assert!(
                (a.position.x - pos_after.x).abs() < 1e-4,
                "the shard moved (Euler)"
            );
            assert!((a.position.y - pos_after.y).abs() < 1e-4);
            assert!(
                (a.velocity.x - vel_after.x).abs() < 1e-5,
                "the drag slows it"
            );
            assert!((a.velocity.y - vel_after.y).abs() < 1e-5);
        }
    }
    assert!(
        exact > 0,
        "the non-bounced shards integrated exactly through the loop ({} of {})",
        exact,
        before.len()
    );
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
    assert!(c.game.shrapnel().is_empty(), "the reset clears the shards");
    assert!(
        c.game.shrapnel_chains().is_empty(),
        "the reset clears the chain queue"
    );
}

#[test]
fn the_shards_reach_the_draw_batches_as_glass_fans() {
    // The draw half: draw_scene_batches must emit each shard as a
    // triangle fan (9 vertices — 3 fan triangles) into the wall batch,
    // on top of the walls', with the per-vertex glass colors (the edge
    // vertices lighter and more transparent than the centroid vertex).
    let assets = load_release();
    let c = fixture(&assets);
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    let shards = c.game.shrapnel();
    assert!(!shards.is_empty());

    // One more tick so the life has burned (alpha < 255).
    c.game.update(TIME_STEP, 1.0);
    let (wall_quads, _, _, _) = c.game.draw_scene_batches();
    // The shard vertices are the LAST shards*9 in the batch (pushed
    // after the walls).
    let total = wall_quads.size();
    assert!(
        total >= shards.len() * 9,
        "the shard fans are in the wall batch ({} vertices for {} shards)",
        total,
        shards.len()
    );
    let fan = wall_quads.as_slice()[total - shards.len() * 9..].to_vec();
    assert_eq!(fan.len(), shards.len() * 9);
    // Every 3-vertex group is one fan triangle: (centroid, edge, edge).
    // The centroid carries the wall color at full (life-faded) alpha; the
    // edges carry the lightened glass color at a LOWER alpha (the glass
    // edge).
    for (i, chunk) in fan.chunks_exact(9).enumerate() {
        let (center, edge_a, edge_b) = (chunk[0], chunk[1], chunk[2]);
        assert!(
            center.color.a < 255,
            "the centroid alpha faded with the life (shard {})",
            i
        );
        // The edge vertices carry the lightened color…
        assert!(edge_a.color.r >= shards[i].color_in.r);
        assert!(edge_b.color.r >= shards[i].color_in.r);
        // …and a lower-or-equal alpha than the centroid (the glass edge).
        assert!(
            edge_a.color.a <= center.color.a,
            "edge alpha ({} ) <= centroid alpha ({})",
            edge_a.color.a,
            center.color.a
        );
        // The fan's three triangles share the centroid vertex position,
        // which is the shard's tumbled position.
        assert_eq!(chunk[0].position, chunk[3].position);
        assert_eq!(chunk[3].position, chunk[6].position);
        assert_eq!(chunk[0].position, c.game.shrapnel()[i].position);
    }
}
