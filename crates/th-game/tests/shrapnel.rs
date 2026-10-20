//! plan 002 issue 09 (rework) — the collision shrapnel: a soft hit
//! shatters the wall into GLASS SHARDS — irregular pieces cut from the
//! wall's actual quad — which burst under a real physics sim (mass, the
//! dual-mode spawn aim: ~half radial spray + ~half tangential carriers,
//! drag, tumble, elastic core bounces). Propagation is PHYSICAL
//! (plan 002 issue 09 shards-3): the carriers sweep the ring and
//! shatter whatever live wall they overlap (the engine's impact pass),
//! generation-capped (`generation >= MAX_PROPAGATION` crumbles) — no
//! queued chain hops, no extra bumps. Same deterministic fixture
//! discipline as `soft_collisions.rs` (the 04a twin); the propagation
//! tests pin their cosmetic rng seed.

mod common;

use std::sync::Arc;

use th_config::TIME_STEP;
use th_game::polygon::point_in_polygon;
use th_game::seams::{NoJoystick, PcgCosmeticRng};
use th_game::shrapnel::{CARRIER_LIFE_FT, SHARD_LIFE_FT};
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
/// The seeded fixtures' cosmetic rng seed (deterministic carrier split
/// and directions for the propagation pins — probed: the gen-0 carriers
/// lean CCW, the gen-1 burst follows the sweep, and the pattern walls
/// stay out of the 500-radius band for the whole 300-tick window).
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
    let mut saw_spray = 0;
    let mut saw_carrier = 0;
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
        // The dual-mode spawn aim: the RADIAL SPRAY pieces carry the
        // radial impulse from the impact point (the jitter only shaves
        // it, never inverts it); the TANGENTIAL CARRIERS sweep the ring
        // (their tangential component dominates the small outward kick).
        let to_shard = s.position - impact;
        let dot = s.velocity.x * to_shard.x + s.velocity.y * to_shard.y;
        if s.max_life == SHARD_LIFE_FT {
            saw_spray += 1;
            assert!(
                dot >= -1.0e-4,
                "the radial impulse from the impact point ({} vs dir {:?})",
                dot,
                to_shard
            );
        } else {
            saw_carrier += 1;
            assert_eq!(s.max_life, CARRIER_LIFE_FT);
            let r_mag = get_mag(&s.position).max(1.0e-4);
            let tangent = Vec2::new(-s.position.y / r_mag, s.position.x / r_mag);
            let v_tan = s.velocity.x * tangent.x + s.velocity.y * tangent.y;
            let radial = s.position * (1.0 / r_mag);
            let v_rad = s.velocity.x * radial.x + s.velocity.y * radial.y;
            assert!(
                v_tan.abs() > v_rad.abs(),
                "the carrier's tangential sweep dominates its radial kick"
            );
        }
    }
    assert!(
        saw_spray > 0 && saw_carrier > 0,
        "both spawn modes are present (spray {} carriers {})",
        saw_spray,
        saw_carrier
    );
    // Exactly ONE bump for the whole explosion (the shard-impact
    // shatters add none — the impact pass adds no bumps).
    assert_eq!(c.game.bump_serial(), 1);
}

#[test]
fn a_shard_impact_shatters_a_neighbouring_wall_with_no_extra_bump() {
    let assets = load_release();
    let c = fixture_seeded(&assets, SEED);
    // The hit wall at angle 0 (side 0) + the two neighbour walls of a
    // 6-sided arena at the SAME radial band (sides 1 and 5): the
    // carriers sweep the ring at the walls' radius, so they can reach
    // them. The level's own pattern walls sit at the 1600 spawn
    // distance — unreachable in the test window.
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

    // The carriers sweep the ring (1.5..5.25 px/tick at the wall band):
    // the first neighbour impact is due well inside the 240-tick window
    // (a 105 px arc at the walls' mid-radius, even for the slowest
    // carrier).
    let mut first_hop_tick = None;
    for tick in 2..=240 {
        c.game.update(TIME_STEP, 1.0);
        let alive = injected_sides(&c);
        if alive.len() < 2 {
            first_hop_tick = Some(tick);
            break;
        }
    }
    let tick = first_hop_tick.expect("a carrier reached a neighbour within 240 ticks");
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
    // victim's ring sector (its annular band, not the hit wall's).
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
    // The gen-1 shards sit in the victim's sector: one of the two
    // neighbour spans (side 1: 30..90°, side 5: 270..330°), at the
    // walls' radial band (80..140, ± margin).
    for s in &gen1 {
        let r = get_mag(&s.position);
        assert!(
            (70.0..150.0).contains(&r),
            "gen-1 shard at radius {} sits in the walls' band",
            r
        );
        let a = get_rad(&s.position).rem_euclid(TAU);
        let side1_span = (TAU / 12.0)..(3.0 * TAU / 12.0); // 30..90°
        let side5_span = (9.0 * TAU / 12.0)..(11.0 * TAU / 12.0); // 270..330°
        assert!(
            side1_span.contains(&a) || side5_span.contains(&a),
            "gen-1 shard at angle {} is in a neighbour's sector",
            a
        );
    }

    // The blast is bounded in the walls' own band: in this fixture the
    // gen-1 carriers sweep the EMPTY arcs (sides 2/4 are not injected),
    // so no further injected wall dies and nothing in the band ever
    // bumps again.
    //
    // Far OUTSIDE the band the story continues: the shards fly straight
    // (tangent chords), so a sweeping piece leaves the ring, and by
    // ~tick 200-250 the fastest gen-1 pieces reach the level's PATTERN
    // walls (r ~800-1000 there), spawning bounded gen-2 debris far out
    // (gen 2 crumbles on the next hit; see the reach note in the
    // module docs). That debris is pinned per tick below (gen cap,
    // piece cap) — not by a window-end "everything is empty", because
    // the level's own spawn cadence is game-RNG-driven and outside
    // this test's contract.
    for tick in (tick + 1)..=300 {
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
    }
    let alive = injected_sides(&c);
    assert!(
        alive.len() <= 2 && alive.iter().all(|s| *s == 1 || *s == 5),
        "only the two injected neighbours died (alive: {:?})",
        alive
    );
    // The blast's near field has settled: the gen-0 burst (max life
    // 60 FT = 240 ticks, born at tick 1) is entirely gone, and no
    // blast piece lingers in the walls' band (r < 250) — the ring
    // pieces all died by their lifetimes (gen-1 ring pieces: born at
    // the hops, 192/240-tick lives, gone by tick ~250), and the
    // far-field debris (born out at r >= ~800) cannot fly back inside
    // r < 250 within its own lifetime (max inward travel ~411 px).
    let shards = c.game.shrapnel();
    assert!(
        shards.iter().all(|s| !(s.burst == 0 && s.generation == 0)),
        "the gen-0 burst has melted away ({} left)",
        shards
            .iter()
            .filter(|s| s.burst == 0 && s.generation == 0)
            .count()
    );
    assert!(
        shards
            .iter()
            .all(|s| s.burst != 0 || get_mag(&s.position) >= 250.0),
        "no blast shard lingers in the walls' band (r < 250)"
    );
    assert!(!c.game.get_status().has_died, "never a death in soft mode");
}

#[test]
fn the_full_0_1_2_propagation_crumbles_at_gen_2() {
    let assets = load_release();
    let c = fixture_seeded(&assets, SEED);
    // The hit wall (side 0) + a CCW row: sides 1, 2, 3 sit in a row
    // along the seeded burst's carrier sweep (the gen-0 carriers lean
    // CCW — probed). Sides 4/5 are NOT injected: any CW carrier sweeps
    // an empty arc. (Far-field note as in the neighbour test: by
    // ~tick 200 the fastest ring pieces' chord sweeps can reach the
    // level's pattern walls out at r ~800+; the near-field contract
    // below is unaffected.)
    inject_wall(&c, 0, 80.0, 60.0, 0.0);
    inject_wall(&c, 1, 80.0, 60.0, 0.0);
    inject_wall(&c, 2, 80.0, 60.0, 0.0);
    inject_wall(&c, 3, 80.0, 60.0, 0.0);

    c.game.update(TIME_STEP, 1.0); // tick 1: the hit shatters side 0
    assert_eq!(c.game.bump_serial(), 1);

    // Hop 0→1: a gen-0 carrier sweeps CCW into side 1; hop 1→2: a gen-1
    // carrier (biased by the hitter's tangential velocity — the same
    // sweep direction) sweeps into side 2; the gen-2 shards then sweep
    // on toward side 3 and CRUMBLE on contact (the wall survives).
    let mut side1_died_at = None;
    let mut side2_died_at = None;
    // Crumble evidence: track the gen-2 counts tick by tick. A gen-2
    // piece's life is 192/240 ticks from its birth (hop 2), so no gen-2
    // piece can MELT before tick ~194 — a gen-2 count that DROPS by
    // tick 150 is a piece CONSUMED by the impact pass (crumbled), not
    // a melt. And the near-field count (r < 250) dropping at the same
    // tick as the total says the consumed piece was in the near field,
    // where the only LIVE wall is side 3 (sides 0-2 are dead; dead
    // walls never re-consume a piece; the level's far walls are out at
    // r >= ~1000 all the way past tick 150). Both counts dropping
    // together, before any far-field gen-2 debris can exist, is a
    // near-field gen-2 crumble on side 3's quad.
    let mut prev_near2 = 0usize;
    let mut prev_total2 = 0usize;
    let mut saw_crumble = false;
    for tick in 2..=300 {
        c.game.update(TIME_STEP, 1.0);
        let alive = injected_sides(&c);
        if !alive.contains(&1) && side1_died_at.is_none() {
            side1_died_at = Some(tick);
        }
        if !alive.contains(&2) && side2_died_at.is_none() {
            side2_died_at = Some(tick);
        }
        // The propagation is generation-capped AT ANY MOMENT: no shard
        // past gen 2 ever exists (a gen-2 piece crumbles on impact).
        assert!(
            c.game
                .shrapnel()
                .iter()
                .all(|s| s.generation <= MAX_PROPAGATION),
            "tick {}: a shard past gen {} exists",
            tick,
            MAX_PROPAGATION
        );
        // The shard cap holds throughout the blast.
        assert!(
            c.game.shrapnel().len() <= MAX_LIVE_SHARDS,
            "tick {}: the shard cap holds",
            tick
        );
        // The third hop MUST NOT shatter: side 3 is the gen-2 sweep's
        // target and survives every tick of the run.
        assert!(
            alive.contains(&3),
            "tick {}: side 3 shattered — a gen-2 piece must crumble, not shatter",
            tick
        );
        // No extra bumps, ever.
        assert_eq!(
            c.game.bump_serial(),
            1,
            "tick {}: the impact pass adds no bumps",
            tick
        );

        let cur = c.game.shrapnel();
        let near2 = cur
            .iter()
            .filter(|s| s.generation == 2 && get_mag(&s.position) < 250.0)
            .count();
        let total2 = cur.iter().filter(|s| s.generation == 2).count();
        if tick <= 150 && near2 < prev_near2 && total2 < prev_total2 {
            saw_crumble = true;
        }
        prev_near2 = near2;
        prev_total2 = total2;
    }
    let t1 = side1_died_at.expect("a gen-0 carrier shattered side 1 (hop 0→1)");
    let t2 = side2_died_at.expect("a gen-1 carrier shattered side 2 (hop 1→2)");
    assert!(t1 < t2, "the hops resolve in order ({} then {})", t1, t2);

    // At tick 300 the blast's NEAR field is settled: the gen-0 burst
    // (max life 60 FT = 240 ticks, born at tick 1) is gone, the gen-1
    // ring pieces (born at the hops, 192/240-tick lives) are gone, and
    // the gen-2 pieces are gone too — crumbled on side 3 (the evidence
    // below) or melted out in the far field. Far out at r >= ~800 the
    // fastest ring pieces' chord sweeps can still be shattering the
    // level's PATTERN walls (bounded gen-2 debris — pinned per tick
    // above, outside this near-field contract).
    let shards = c.game.shrapnel();
    assert!(
        shards.iter().all(|s| !(s.burst == 0 && s.generation == 0)),
        "the gen-0 burst has melted away ({} left)",
        shards
            .iter()
            .filter(|s| s.burst == 0 && s.generation == 0)
            .count()
    );
    assert!(
        shards
            .iter()
            .all(|s| s.burst != 0 || get_mag(&s.position) >= 250.0),
        "no blast shard lingers in the walls' band (r < 250)"
    );
    assert!(
        saw_crumble,
        "a gen-2 piece crumbled in the near field (consumed before it could melt; side 3 stayed alive)"
    );
    // Exactly two propagation shatters happened (the hit wall + sides 1
    // and 2 — bounded: one shatter per consumed gen-0/gen-1 piece, and
    // side 3's gen-2 contact crumbled instead of shattering).
    let alive = injected_sides(&c);
    assert_eq!(
        alive,
        vec![3],
        "hit + sides 1, 2 shattered; side 3 survived the gen-2 crumble"
    );
    assert!(!c.game.get_status().has_died, "never a death in soft mode");
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
