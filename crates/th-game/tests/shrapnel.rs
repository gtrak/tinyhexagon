//! plan 002 issue 09 (rework) — the collision shrapnel: a soft hit
//! shatters the wall into GLASS SHARDS — irregular pieces cut from the
//! wall's actual quad — which burst under a real physics sim (mass, the
//! dual-mode spawn aim: ~half radial spray + ~half tangential carriers,
//! drag, tumble, elastic core bounces, knocked inward into the arena
//! by the hit — the pure radial blast would point off-screen). The
//! propagation is PHYSICAL and EMERGENT (plan 002 issue 09 shards-4):
//! a piece that reaches a live wall shatters it (the engine's impact
//! pass), generation-capped (`generation >= MAX_PROPAGATION` crumbles)
//! — no carrier projectiles, no queued chain hops, no extra bumps. The
//! deterministic hop tests inject synthetic pieces
//! (`HexagonGame::test_inject_shard`). Same deterministic fixture
//! discipline as `soft_collisions.rs` (the 04a twin); the propagation
//! tests pin their cosmetic rng seed.

mod common;

use std::sync::Arc;

use th_config::TIME_STEP;
use th_game::polygon::point_in_polygon;
use th_game::seams::{NoJoystick, PcgCosmeticRng};
use th_game::shrapnel::SHARD_LIFE_FT;
use th_game::{CWall, SpeedData};
use th_math::ssvs::vector2::{get_mag, get_rad};
use th_math::ssvs::Vec2;
use th_math::ssvu::math::TAU;

use common::{compose, compose_with, release_dir};

const PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
const LEVEL: &str = "ohvrvanilla_vittorio_romeo_cube_1_apeirogon";

/// The shrapnel constants (the `shrapnel.rs` contract).
const SHARDS_PER_WALL_MIN: usize = 6;
const SHARDS_PER_WALL_MAX: usize = 10;
const MAX_LIVE_SHARDS: usize = 64;
/// The propagation depth cap (a gen-2 piece crumbles on impact).
const MAX_PROPAGATION: u32 = 2;
/// One 240 Hz tick in FT units (the engine's `m_ft` scale: 60 = 1 s).
const TICK_FT: f32 = 0.25;
/// The seeded fixtures' cosmetic rng seed (deterministic fan cut +
/// blast jitter for the physics pins).
const SEED: u64 = 42;

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

/// [`fixture`] with a SEEDED cosmetic rng: the propagation tests pin
/// their rng draws (the carrier coin, the carrier direction, the fan
/// jitter — all cosmetic, all seeded).
fn fixture_seeded(assets: &Arc<th_assets::Assets>, seed: u64) -> common::Composed {
    let c = compose_with(
        assets.clone(),
        release_dir(),
        false,
        Arc::new(NoJoystick),
        Arc::new(PcgCosmeticRng::with_seed(seed)),
    );
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

/// A wall's mid-angle from its `angle_span` (the span's bisector,
/// seam-normalized) — test-local (the engine's mid-angle helper retired
/// with the queued chain; the tests only need it to NAME the injected
/// walls).
fn mid_angle(start: f32, end: f32) -> f32 {
    (start + (end - start).rem_euclid(TAU) / 2.0).rem_euclid(TAU)
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
            (r < 500.0).then(|| ((mid_angle(s, e) * 1000.0).round() as u32, !w.is_dead()))
        })
        .collect()
}

/// The injected walls' side indices (this 6-sided level: side k sits at
/// TAU/6·k) — `injected()`'s mids back-solved to side numbers.
fn injected_sides(c: &common::Composed) -> Vec<u32> {
    injected(c)
        .into_iter()
        .map(|(m, _)| ((m as f32) / (TAU / 6.0) / 1000.0).round() as u32 % 6)
        .collect()
}

#[test]
fn a_soft_hit_shatters_the_wall_into_glass_shards_cut_from_its_quad() {
    let assets = load_release();
    let c = fixture_seeded(&assets, SEED);
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
    // shape (the burst is AT the wall, not the center).
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
        // The physical hit's burst: generation 0.
        assert_eq!(s.generation, 0);
        // The glass look: the perimeter color is lightened toward white.
        assert!(s.color_out.r >= s.color_in.r);
        assert!(s.color_out.g >= s.color_in.g);
        assert!(s.color_out.b >= s.color_in.b);
        // The spawn aim: ONE mode — the mass-scaled impulse KNOCKED
        // INWARD (see `SHARD_INWARD_KNOCK`): the perp jitter never
        // touches the radial axis, so every piece's radial velocity
        // component is net inward (the glass visibly flies into the
        // arena and bounces off the core).
        let r_mag = get_mag(&s.position).max(1.0e-4);
        let inward = s.position * (-1.0 / r_mag);
        let v_in = s.velocity.x * inward.x + s.velocity.y * inward.y;
        assert!(v_in > 0.0, "every piece is knocked inward ({})", v_in);
        assert_eq!(s.max_life, SHARD_LIFE_FT, "one life for all pieces");
    }
    // Exactly ONE bump for the whole explosion (the shard-impact
    // shatters add none — the impact pass adds no bumps).
    assert_eq!(c.game.bump_serial(), 1);
}

#[test]
fn a_shard_impact_shatters_a_neighbouring_wall_with_no_extra_bump() {
    let assets = load_release();
    let c = fixture_seeded(&assets, SEED);
    // The hit wall at angle 0 (side 0) + the two neighbour walls of a
    // 6-sided arena at the SAME radial band (sides 1 and 5).
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    inject_wall(&c, 1, 80.0, 60.0, 0.0);
    inject_wall(&c, 5, 80.0, 60.0, 0.0);

    c.game.update(TIME_STEP, 1.0); // tick 1: the hit shatters side 0
    assert_eq!(c.game.bump_serial(), 1, "one bump for the physical hit");
    let burst = c.game.shrapnel().first().map(|s| s.burst);
    assert!(
        c.game
            .shrapnel()
            .iter()
            .all(|s| s.generation == 0 && s.burst == burst.unwrap()),
        "the hit's burst: gen 0, one blast id"
    );
    let alive = injected_sides(&c);
    assert_eq!(
        alive,
        vec![1, 5],
        "the hit wall is gone; both neighbours wait"
    );

    // Deterministic hop (shards-4: the propagation is emergent physics,
    // so the seeded real pieces cannot be relied on to reach a
    // neighbour — the test drives the impact pass with a synthetic
    // gen-0 piece placed ON the neighbour's quad, the same way the
    // walls themselves are injected).
    let q1 = {
        let g = c.game.state_handle().lock();
        // The injected walls: side 0 was consumed by the hit's retain;
        // the remaining injected walls are sides 1 and 5 (push order).
        let w = g
            .walls
            .iter()
            .find(|w| {
                !w.is_dead() && {
                    let (sp, _ep, r) = w.angle_span().unwrap();
                    r < 500.0 && (mid_angle(sp, _ep) / (TAU / 6.0)).round() as u32 % 6 == 1
                }
            })
            .unwrap()
            .get_vertex_positions();
        (w[0] + w[1] + w[2] + w[3]) * 0.25
    };
    c.game.test_inject_shard(q1, Vec2::new(0.0, 0.0), 0);
    c.game.update(TIME_STEP, 1.0); // the impact pass shatters side 1
    let tick = 2;

    // The impact shattered the neighbour physically: the wall is dead,
    // the bump serial did NOT move (the bump is the player's physical
    // hit alone — the impact pass adds no bumps)...
    assert_eq!(
        c.game.bump_serial(),
        1,
        "the shard impact at tick {} adds no bump",
        tick
    );
    // ...and it spawned the gen-1 burst: the new shards inherit the
    // blast's burst id, step up to generation 1, and are cut from the
    // victim's ring sector.
    let shards = c.game.shrapnel();
    let gen1: Vec<_> = shards.iter().filter(|s| s.generation == 1).collect();
    assert!(!gen1.is_empty(), "the gen-1 burst spawned at tick {}", tick);
    assert!(
        gen1.iter().all(|s| s.burst == burst.unwrap()),
        "the gen-1 shards carry the blast's burst id"
    );
    assert!(
        c.game.shrapnel().len() <= MAX_LIVE_SHARDS,
        "the shard cap holds"
    );
    // The gen-1 shards sit in the victim's sector (side 1: 30..90°) at
    // the walls' radial band (80..140, ± margin), knocked inward.
    for s in &gen1 {
        let r = get_mag(&s.position);
        assert!(
            (70.0..150.0).contains(&r),
            "gen-1 shard at radius {} sits in the walls' band",
            r
        );
        let a = get_rad(&s.position).rem_euclid(TAU);
        let side1_span = (TAU / 12.0)..(3.0 * TAU / 12.0); // 30..90°
        assert!(
            side1_span.contains(&a),
            "gen-1 shard at angle {} is in the victim's sector",
            a
        );
        let inward = s.position * (-1.0 / r);
        assert!(
            s.velocity.x * inward.x + s.velocity.y * inward.y > -0.4,
            "the gen-1 pieces lean inward (the jitter may shave the weak radial component)"
        );
    }
    // The burst's MEAN radial component is inward (the knock dominates
    // the jitter across the burst even where a single piece's jitter
    // flips its weak radial component).
    let mean_in = gen1
        .iter()
        .map(|s| {
            let r = get_mag(&s.position).max(1.0e-4);
            let inward = s.position * (-1.0 / r);
            s.velocity.x * inward.x + s.velocity.y * inward.y
        })
        .sum::<f32>()
        / gen1.len() as f32;
    assert!(mean_in > 0.0, "the gen-1 burst leans inward ({})", mean_in);

    // The bounded blast's invariants, over a settling window: no extra
    // bumps, the generation cap, the shard cap.
    for tick in (tick + 1)..=120 {
        c.game.update(TIME_STEP, 1.0);
        assert_eq!(
            c.game.bump_serial(),
            1,
            "tick {}: no bump is ever added by a shard impact",
            tick
        );
        assert!(
            c.game
                .shrapnel()
                .iter()
                .all(|s| s.generation <= MAX_PROPAGATION),
            "tick {}: a shard past gen {} exists",
            tick,
            MAX_PROPAGATION
        );
        assert!(
            c.game.shrapnel().len() <= MAX_LIVE_SHARDS,
            "tick {}: the shard cap holds",
            tick
        );
    }
    assert!(!c.game.get_status().has_died, "never a death in soft mode");
}

#[test]
fn a_gen_2_piece_crumbles_instead_of_shattering() {
    let assets = load_release();
    let c = fixture_seeded(&assets, SEED);
    // The hit wall (side 0) + the row sides 1, 2, 3.
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    inject_wall(&c, 1, 80.0, 60.0, 0.0);
    inject_wall(&c, 2, 80.0, 60.0, 0.0);
    inject_wall(&c, 3, 80.0, 60.0, 0.0);

    c.game.update(TIME_STEP, 1.0); // tick 1: the hit shatters side 0
    assert_eq!(c.game.bump_serial(), 1);

    // Hop 0→1: a synthetic gen-0 piece on side 1's quad; the impact
    // pass shatters it and spawns the gen-1 burst.
    let q1 = {
        let g = c.game.state_handle().lock();
        let w = g
            .walls
            .iter()
            .find(|w| {
                !w.is_dead() && {
                    let (sp, ep, r) = w.angle_span().unwrap();
                    r < 500.0 && (mid_angle(sp, ep) / (TAU / 6.0)).round() as u32 % 6 == 1
                }
            })
            .unwrap()
            .get_vertex_positions();
        (w[0] + w[1] + w[2] + w[3]) * 0.25
    };
    c.game.test_inject_shard(q1, Vec2::new(0.0, 0.0), 0);
    c.game.update(TIME_STEP, 1.0);
    assert!(
        !injected_sides(&c).contains(&1),
        "hop 0→1: side 1 shattered by the injected gen-0 piece"
    );

    // Hop 1→2: a synthetic gen-1 piece on side 2's quad; side 2
    // shatters and the gen-2 burst spawns.
    let q2 = {
        let g = c.game.state_handle().lock();
        let w = g
            .walls
            .iter()
            .find(|w| {
                !w.is_dead() && {
                    let (sp, ep, r) = w.angle_span().unwrap();
                    r < 500.0 && (mid_angle(sp, ep) / (TAU / 6.0)).round() as u32 % 6 == 2
                }
            })
            .unwrap()
            .get_vertex_positions();
        (w[0] + w[1] + w[2] + w[3]) * 0.25
    };
    c.game.test_inject_shard(q2, Vec2::new(0.0, 0.0), 1);
    c.game.update(TIME_STEP, 1.0);
    assert!(
        !injected_sides(&c).contains(&2),
        "hop 1→2: side 2 shattered by the injected gen-1 piece"
    );
    assert!(
        c.game.shrapnel().iter().any(|s| s.generation == 2),
        "the gen-2 burst spawned"
    );

    // The last hop MUST NOT shatter: a synthetic gen-2 piece on side 3
    // CRUMBLES on contact (the piece dies, the wall survives).
    let q3 = {
        let g = c.game.state_handle().lock();
        let w = g
            .walls
            .iter()
            .find(|w| {
                !w.is_dead() && {
                    let (sp, ep, r) = w.angle_span().unwrap();
                    r < 500.0 && (mid_angle(sp, ep) / (TAU / 6.0)).round() as u32 % 6 == 3
                }
            })
            .unwrap()
            .get_vertex_positions();
        (w[0] + w[1] + w[2] + w[3]) * 0.25
    };
    c.game.test_inject_shard(q3, Vec2::new(0.0, 0.0), 2);
    c.game.update(TIME_STEP, 1.0);
    let alive = injected_sides(&c);
    assert_eq!(
        alive,
        vec![3],
        "hit + sides 1, 2 shattered; side 3 survived the gen-2 crumble"
    );
    assert!(
        !c.game.shrapnel().iter().any(|s| s.generation > 2),
        "no shard past the gen cap"
    );
    // No extra bumps, ever.
    assert_eq!(c.game.bump_serial(), 1);
    // The crumbled piece is consumed (gone) and side 3 lives on.
    assert!(!c.game.get_status().has_died, "never a death in soft mode");
}

#[test]
fn the_glass_events_record_the_crack_and_the_bounces() {
    let assets = load_release();
    let c = fixture_seeded(&assets, SEED);
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0); // the hit: the wall shatters
    let events = c.game.drain_shard_events();
    assert!(
        matches!(
            events.first(),
            Some(th_game::shrapnel::ShardEvent::Crack { position, .. })
                if position.x.is_finite()
        ),
        "the shatter records a Crack at the impact"
    );
    // The bounce Tinks, deterministically: a synthetic piece flying at
    // the core crosses its edge within one tick (the fixture's own game
    // rng drives the LEVEL's spawn cadence, so waiting for a REAL piece
    // to reach the core is not deterministic).
    let core = c.game.get_radius() * 1.05;
    c.game
        .test_inject_shard(Vec2::new(0.0, core + 10.0), Vec2::new(0.0, -50.0), 0);
    c.game.update(TIME_STEP, 1.0);
    let events = c.game.drain_shard_events();
    assert!(
        matches!(events.iter().find(|e| matches!(e, th_game::shrapnel::ShardEvent::Tink { .. })),
            Some(th_game::shrapnel::ShardEvent::Tink { position, speed01, .. })
                if position.y.is_finite() && *speed01 > 0.0
        ),
        "a bounced piece records a Tink (events: {:?})",
        events
    );
    // The cap: the pending vector never exceeds it (indirectly: the
    // drain always returns <= the cap even mid-blast).
    assert!(events.len() <= 24);
}

#[test]
fn a_new_game_clears_the_shrapnel_and_the_bursts() {
    let assets = load_release();
    let c = fixture_seeded(&assets, SEED);
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    inject_wall(&c, 1, 80.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    assert!(!c.game.shrapnel().is_empty());
    // The running blast's id is non-zero-ish; after the reset the NEXT
    // blast must start at burst 0 again (the counter is part of the
    // reset state).
    c.game.new_game(PACK, LEVEL, true, 1.0);
    c.game.start();
    assert!(c.game.shrapnel().is_empty(), "the reset clears the shards");

    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    let shards = c.game.shrapnel();
    assert!(!shards.is_empty(), "the post-reset hit shatters again");
    assert!(
        shards.iter().all(|s| s.burst == 0 && s.generation == 0),
        "the post-reset blast starts at burst 0, generation 0 (got {:?})",
        shards
            .iter()
            .map(|s| (s.burst, s.generation))
            .collect::<Vec<_>>()
    );
}

#[test]
fn shard_physics_ride_momentum_drag_tumble_and_cull() {
    // Direct shard mechanics through the engine's update (the unit tests
    // in shrapnel.rs pin the exact math; this pins the LOOP's wiring).
    // A hit wall (inner 80, spanning the player's radial band; a thick
    // wall so the outer fan pieces sit well past the breathing core):
    // some shards spawn near the core edge and can cross it inward
    // (that is the core-bounce wiring — pinned exactly in shrapnel.rs's
    // unit tests), so the exact-Euler assertions apply to the shards
    // that did not cross (same-blast pairs skip the pairwise bounce, so
    // nothing else perturbs them).
    let assets = load_release();
    let c = fixture_seeded(&assets, SEED);
    inject_wall(&c, 0, 80.0, 120.0, 0.0);
    c.game.update(TIME_STEP, 1.0);
    let before = c.game.shrapnel();
    assert!(!before.is_empty());

    // One more tick: the life burns and the speed decays for EVERY
    // shard (drag, or the bounce's damping), the tumble integrates.
    // Shards that did NOT bounce off the core integrate EXACTLY (the
    // crossing bounce fires only for a shard that crossed from OUTSIDE
    // the core edge to inside within the tick — predicted here with the
    // same math, so the exactness check is robust to where the shards
    // happen to land). Same-blast pairs skip the pairwise bounce, so
    // nothing else perturbs them.
    let core_radius = c.game.get_radius() * 1.05;
    c.game.update(TIME_STEP, 1.0);
    let after = c.game.shrapnel();
    assert_eq!(after.len(), before.len());
    let drag = 1.0_f32 - (0.5_f32 / 60.0_f32) * TICK_FT;
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
        // Predict the crossing bounce with the engine's own order: step
        // first (Euler on the pre-drag velocity), drag, then the
        // OUTSIDE→INSIDE crossing check on the pre/post-step radii and
        // the post-drag velocity.
        let pos_after = b.position + b.velocity * TICK_FT;
        let vel_after = b.velocity * drag;
        let mag_after = get_mag(&pos_after);
        let prev_mag = get_mag(&b.position);
        let bounced = prev_mag >= core_radius && mag_after > 1.0e-4 && mag_after < core_radius && {
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
