//! plan 002 issue 04a — wall-gap math: `CWall::angle_span`, the pure
//! `compute_wall_gaps`, and the engine's `wall_gaps` / `nearest_gap_angle`.
//!
//! The synthetic walls are 6-sided (each span = 60° = TAU/6, half =
//! TAU/12) at a fixed 1600 distance, so the expected gap centres /
//! widths are the missing sides' angles.

mod common;

use std::sync::Arc;

use th_game::wall::compute_wall_gaps;
use th_game::{CWall, SpeedData};
use th_math::ssvs::Vec2;
use th_math::ssvu::math::TAU;

use common::{compose, release_dir};

const CENTER: Vec2 = Vec2::new(0.0, 0.0);
const DIST: f32 = 1600.0;

fn wall(side: i32) -> CWall {
    let s = SpeedData::new(0.0, 0.0, 0.0, 0.0, false);
    let cr = SpeedData::new(0.0, 0.0, 0.0, 0.0, false);
    CWall::new(
        6, 0.0, 0.0, 0.0, 0.0, &CENTER, side, 40.0, DIST, &s, &cr, 0.0,
    )
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

// --- angle_span ------------------------------------------------------------

#[test]
fn angle_span_of_a_plain_side() {
    // side 1 of a 6-sided wall: from 30° to 90° (no seam wrap).
    let w = wall(1);
    let (s, e, r) = w.angle_span().expect("span");
    assert!(approx(s, TAU / 12.0), "start (30°) {s}");
    assert!(approx(e, TAU / 4.0), "end (90°) {e}");
    assert!((r - DIST).abs() < 1e-2, "radius {r}");
}

#[test]
fn angle_span_wraps_the_seam() {
    // side 0: from -30° (normalized to TAU - 30°) to +30° — the span
    // crosses the 0/TAU seam.
    let w = wall(0);
    let (s, e, _) = w.angle_span().expect("span");
    assert!(approx(s, TAU - TAU / 12.0), "start (330°) {s}");
    assert!(approx(e, TAU / 12.0), "end (30°) {e}");
}

// --- compute_wall_gaps ------------------------------------------------------

#[test]
fn five_walls_leave_one_gap_centered_on_the_missing_side() {
    // Sides 0, 1, 2, 4, 5: the gap is side 3's span (150°..210°),
    // centred on 180°.
    let walls = [0, 1, 2, 4, 5]
        .iter()
        .copied()
        .map(wall)
        .collect::<Vec<_>>();
    let gaps = compute_wall_gaps(&walls);
    assert_eq!(gaps.len(), 1);
    let g = &gaps[0];
    assert!(
        approx(g.center_angle, TAU / 2.0),
        "center (180°) {}",
        g.center_angle
    );
    assert!(
        approx(g.half_width_angle, TAU / 12.0),
        "half width (30°) {}",
        g.half_width_angle
    );
    assert!((g.radius - DIST).abs() < 1e-2, "radius {}", g.radius);
}

#[test]
fn the_gap_across_the_seam_is_centered_on_zero() {
    // Sides 1..5 (the side-0 wall missing): the gap runs 330°..30°
    // across the 0/TAU seam, centred on 0.
    let walls = [1, 2, 3, 4, 5]
        .iter()
        .copied()
        .map(wall)
        .collect::<Vec<_>>();
    let gaps = compute_wall_gaps(&walls);
    assert_eq!(gaps.len(), 1);
    let g = &gaps[0];
    // Centred on 0 mod TAU: within vertex-angle noise of the seam itself
    // (the atan2-recovered bounds push the centre to the TAU side).
    let seam_dist = g.center_angle.min(TAU - g.center_angle);
    assert!(seam_dist < 1e-4, "center (0, not TAU) {}", g.center_angle);
    assert!(
        approx(g.half_width_angle, TAU / 12.0),
        "half width (30°) {}",
        g.half_width_angle
    );
}

#[test]
fn two_gaps_are_reported_for_a_sparse_ring() {
    // Walls on sides 0, 1, 3, 4 → gaps on side 2 (90°..150°, centre
    // 120°) and side 5 (270°..330°, centre 300°).
    let walls = [0, 1, 3, 4].iter().copied().map(wall).collect::<Vec<_>>();
    let gaps = compute_wall_gaps(&walls);
    assert_eq!(gaps.len(), 2);
    // Ordered by start angle: 120° then 300°.
    assert!(
        approx(gaps[0].center_angle, TAU / 3.0),
        "first centre (120°) {}",
        gaps[0].center_angle
    );
    assert!(
        approx(gaps[1].center_angle, 5.0 * TAU / 6.0),
        "second centre (300°) {}",
        gaps[1].center_angle
    );
    assert!(gaps.iter().all(|g| approx(g.half_width_angle, TAU / 12.0)));
}

#[test]
fn a_full_ring_leaves_no_gaps() {
    let walls = (0..6i32).map(wall).collect::<Vec<_>>();
    assert!(
        compute_wall_gaps(&walls).is_empty(),
        "a fully covered ring has no gaps"
    );
}

#[test]
fn no_walls_no_gaps() {
    assert!(compute_wall_gaps(&[]).is_empty());
}

// --- the engine delegation --------------------------------------------------

const PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
const LEVEL: &str = "ohvrvanilla_vittorio_romeo_cube_1_apeirogon";

#[test]
fn engine_wall_gaps_and_nearest_gap_angle() {
    let assets = Arc::new(
        th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"),
    );
    let c = compose(assets, release_dir(), false);
    c.game.new_game(PACK, LEVEL, true, 1.0);
    // Inject a sparse ring (sides 0, 1, 3, 4) directly into the shared
    // store (new_game cleared it).
    let walls = [0, 1, 3, 4].iter().copied().map(wall).collect::<Vec<_>>();
    c.game.state_handle().lock().walls.clone_from(&walls);

    let gaps = c.game.wall_gaps();
    assert_eq!(gaps.len(), 2, "delegates over st().walls");
    assert_eq!(gaps, compute_wall_gaps(&walls));

    // From 0°: the 300° gap is 60° "behind" (normalized to -60°), the
    // 120° gap is 120° ahead → the hand wants the 300° gap (the wrap
    // half).
    let want = c.game.nearest_gap_angle(0.0).expect("a gap");
    assert!(
        approx(want, 5.0 * TAU / 6.0),
        "nearest to 0° is the 300° gap, got {want}"
    );
    // From 180°: the 120° gap is 60° away, the 300° gap 120° away.
    let want = c.game.nearest_gap_angle(TAU / 2.0).expect("a gap");
    assert!(
        approx(want, TAU / 3.0),
        "nearest to 180° is the 120° gap, got {want}"
    );
}

#[test]
fn nearest_gap_angle_is_none_without_gaps() {
    let assets = Arc::new(
        th_assets::Assets::new(release_dir(), true, false).expect("_RELEASE loads headless"),
    );
    let c = compose(assets, release_dir(), false);
    c.game.new_game(PACK, LEVEL, true, 1.0);
    // No walls → no gaps.
    assert!(c.game.wall_gaps().is_empty());
    assert!(c.game.nearest_gap_angle(0.0).is_none());
    // A full ring → no gaps either.
    let walls = (0..6i32).map(wall).collect::<Vec<_>>();
    c.game.state_handle().lock().walls.clone_from(&walls);
    assert!(c.game.nearest_gap_angle(0.0).is_none());
}
