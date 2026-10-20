//! plan 002 issue 09 — the kid's collision shrapnel: a soft-collision
//! shatters the hit wall into little wedge pieces that spin away and
//! fade, and the explosion chains into up to [`CHAIN_WALLS`] neighbouring
//! walls (a short hop apart, so it reads as the blast spreading).
//!
//! The pieces are engine-side (`EngineState::shrapnel`): the update
//! advances them with the frame delta (so the bump's slow-motion slows
//! them too — the kid sees the burst), and the draw pushes each wedge as
//! three vertices into the SAME `FastVertexVectorTris` batch the walls
//! draw into (one draw call, the wall's own color, alpha-faded by the
//! remaining life — the SFML Alpha blend is the wall pass's blend).
//!
//! The chain is a plain data queue (`PendingChain`: the victim's
//! mid-angle + the remaining delay), not a closure — the victim is
//! re-looked-up by angular distance at fire time, so the wall vector's
//! `retain` (the killed wall's removal) can never invalidate it. The
//! chain fires do NOT re-queue (bounded: one hit → at most
//! [`CHAIN_WALLS`] extra walls).

use th_math::hg::color::Color;
use th_math::ssvs::vector2::get_mag;
use th_math::ssvs::Vec2;
use th_math::ssvu::math::TAU;

use crate::vertex_batch::FastVertexVectorTris;

/// Shrapnel pieces spawned per shattered wall.
pub const PIECES_PER_WALL: usize = 12;
/// The neighbouring walls a hit takes out (the chain).
pub const CHAIN_WALLS: usize = 2;
/// The seconds between the chain hops — in ENGINE FT units (the update
/// loop's `m_ft` scale: 60 units = 1 real second, one 240 Hz tick =
/// 0.25): 5.4 units = 90 ms — the two victims go ~90 ms and ~180 ms
/// after the hit, one readable blast.
pub const CHAIN_HOP_FT: f32 = 5.4;
/// A piece's lifetime, in FT units (48 = 0.8 s real time).
pub const PIECE_LIFE_FT: f32 = 48.0;
/// The piece speed range, in px per FT-unit (× 60 = px/s: 60..170).
pub const PIECE_SPEED_MIN: f32 = 1.0;
pub const PIECE_SPEED_MAX: f32 = 2.8;
/// The piece size range (px — the wedge's tip distance from its center).
pub const PIECE_SIZE_MIN: f32 = 4.0;
pub const PIECE_SIZE_MAX: f32 = 9.0;
/// The half-spread of the burst cone around the outward direction.
pub const PIECE_SPREAD_RAD: f32 = 0.9;
/// The drag per FT-unit (2.0/s → 1/30 per unit): the pieces settle.
pub const PIECE_DRAG_PER_FT: f32 = 1.0 / 30.0;
/// The spin range, rad per FT-unit (±300 deg/s → ±5/60).
pub const PIECE_SPIN_MAX: f32 = 5.0 / 60.0;
/// The bounce: an elastic reflection off the core keeps this fraction of
/// the speed (a real bounce eats energy).
pub const BOUNCE_KEEP: f32 = 0.55;

/// One flying piece: a small wedge (the draw's three points) that spins,
/// drifts outward and fades over its life.
#[derive(Debug, Clone, PartialEq)]
pub struct Shrapnel {
    pub position: Vec2,
    pub velocity: Vec2,
    /// The wedge's pointing direction (radians; the tip side).
    pub angle: f32,
    /// The spin (radians/s; the cosmetic rng's sign).
    pub spin: f32,
    pub size: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
}

impl Shrapnel {
    /// Advance by the frame delta `ft` (FT units — the update loop's
    /// scale, 60 = 1 real second): the position rides the velocity (a
    /// light drag so the pieces settle instead of flying off the
    /// screen), the wedge spins, the life burns — and a piece that
    /// reaches the core BOUNCES: the radial velocity component reflects
    /// (elastic, energy-damped), the piece is pushed back out of the
    /// core, and the tumble direction flips with the impact. Physics,
    /// not a scripted fade.
    pub fn update(&mut self, ft: f32, core_radius: f32) {
        self.position += self.velocity * ft;
        let drag = (1.0_f32 - PIECE_DRAG_PER_FT * ft).max(0.0);
        self.velocity *= drag;
        self.angle += self.spin * ft;
        self.life -= ft;

        let mag = get_mag(&self.position);
        if mag > 1.0e-3 && mag < core_radius {
            let nx = self.position.x / mag;
            let ny = self.position.y / mag;
            let v_dot_n = self.velocity.x * nx + self.velocity.y * ny;
            if v_dot_n < 0.0 {
                // v' = v - 2 (v·n) n — the radial component flips; keep
                // BOUNCE_KEEP of the speed and step the piece back out.
                self.velocity.x -= 2.0 * v_dot_n * nx;
                self.velocity.y -= 2.0 * v_dot_n * ny;
                self.velocity.x *= BOUNCE_KEEP;
                self.velocity.y *= BOUNCE_KEEP;
                self.position.x = nx * core_radius;
                self.position.y = ny * core_radius;
                self.spin = -self.spin;
            }
        }
    }

    /// `true` while the piece still shows (life > 0).
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.life > 0.0
    }

    /// Draw the wedge into the wall batch: the tip points along `angle`,
    /// the base trails behind; the size shrinks with the life and the
    /// color's alpha fades — the pieces melt away.
    pub fn draw(&self, wall_quads: &mut FastVertexVectorTris) {
        let alpha = (self.life / self.max_life).clamp(0.0, 1.0);
        let mut color = self.color;
        color.a = ((color.a as f32) * alpha) as u8;
        let size = self.size * (0.35_f32 + 0.65_f32 * alpha);

        let dir = Vec2::new(self.angle.cos(), self.angle.sin());
        let tip = self.position + dir * size;
        let back = self.position - dir * size * 0.4;
        let perp = Vec2::new(-dir.y, dir.x);
        let base_a = back + perp * size * 0.35;
        let base_b = back - perp * size * 0.35;
        wall_quads.batch_unsafe_emplace_back(color, &[tip, base_a, base_b]);
    }
}

/// A queued chain victim: the wall's mid-angle (radians about the world
/// origin, `[0, TAU)`) + the seconds until its shatter fires.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PendingChain {
    pub mid_angle: f32,
    pub delay: f32,
}

/// A wall's mid-angle from its `angle_span` (the span's bisector,
/// seam-normalized). Shared by the queue and the fire-time lookup.
#[must_use]
pub fn wall_mid_angle(start: f32, end: f32) -> f32 {
    (start + (end - start).rem_euclid(TAU) / 2.0).rem_euclid(TAU)
}

/// The seam-aware angular distance between two `[0, TAU)` angles.
#[must_use]
pub fn angle_distance(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(TAU);
    d.min(TAU - d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece() -> Shrapnel {
        Shrapnel {
            position: Vec2::new(100.0, 0.0),
            velocity: Vec2::new(2.0, 0.0),
            angle: 0.0,
            spin: 0.05,
            size: 6.0,
            life: PIECE_LIFE_FT,
            max_life: PIECE_LIFE_FT,
            color: Color::new(255, 0, 0, 255),
        }
    }

    #[test]
    fn update_rides_the_velocity_drags_and_burns_the_life() {
        let mut p = piece();
        let x0 = p.position.x;
        let v0 = p.velocity.x;
        p.update(0.25, 72.0);
        // moved outward…
        assert!(p.position.x > x0);
        // …slowed (drag)…
        assert!(p.velocity.x < v0);
        // …spun…
        assert!((p.angle - 0.05 * 0.25).abs() < 1e-6);
        // …and the life shrank.
        assert!((p.life - (PIECE_LIFE_FT - 0.25)).abs() < 1e-6);
        // A huge delta stalls the drag (never negative) and runs the
        // life out (48 units; 50 > 48).
        let mut p = piece();
        p.velocity = Vec2::new(2.0, 0.0);
        p.update(50.0, 72.0);
        assert_eq!(p.velocity.x, 0.0);
        assert!(!p.is_alive());
    }

    #[test]
    fn a_piece_bounces_elastically_off_the_core() {
        // A piece just inside the core boundary, moving INWARD (at the
        // center): the radial component reflects outward, damped; the
        // piece is stepped back outside the core.
        let mut p = Shrapnel {
            position: Vec2::new(50.0, 0.0), // inside the 72 core
            velocity: Vec2::new(-2.0, 0.0), // moving toward the center
            angle: 0.0,
            spin: 0.05,
            size: 6.0,
            life: PIECE_LIFE_FT,
            max_life: PIECE_LIFE_FT,
            color: Color::new(255, 0, 0, 255),
        };
        p.update(0.25, 72.0);
        assert!(p.position.x >= 72.0, "pushed back out of the core");
        assert!(p.velocity.x > 0.0, "the radial velocity reflected");
        // The drag ran first (one tick), then the reflection kept
        // BOUNCE_KEEP of the (already dragged) speed.
        let dragged = 2.0 * (1.0_f32 - (1.0_f32 / 30.0) * 0.25);
        assert!((p.velocity.x - dragged * 0.55).abs() < 1e-5);
        assert_eq!(p.spin, -0.05, "the tumble flips on impact");
        // A piece heading AWAY from the core from inside does not bounce.
        let mut p = Shrapnel {
            position: Vec2::new(50.0, 0.0),
            velocity: Vec2::new(2.0, 0.0),
            angle: 0.0,
            spin: 0.05,
            size: 6.0,
            life: PIECE_LIFE_FT,
            max_life: PIECE_LIFE_FT,
            color: Color::new(255, 0, 0, 255),
        };
        p.update(0.25, 72.0);
        assert!(p.position.x > 50.0);
        assert!(!p.is_alive() || p.position.x <= 72.0 || p.velocity.x > 0.0);
    }

    #[test]
    fn draw_emits_three_fading_shrinking_vertices() {
        let mut p = piece();
        let mut batch = FastVertexVectorTris::new();
        p.draw(&mut batch);
        assert_eq!(batch.size(), 3);
        // Full life: opaque, the full size.
        assert_eq!(batch.as_slice()[0].color.a, 255);
        // Half life: alpha halves (u8 floor) and the tip shrinks toward
        // the position.
        p.life = PIECE_LIFE_FT * 0.5;
        let mut half = FastVertexVectorTris::new();
        p.draw(&mut half);
        assert!(half.as_slice()[0].color.a < 255);
        let tip_full = batch.as_slice()[0].position.x;
        let tip_half = half.as_slice()[0].position.x;
        assert!(tip_half.abs() < tip_full.abs());
        // Dead: fully transparent (the cull happens in the update loop).
        p.life = 0.0;
        let mut dead = FastVertexVectorTris::new();
        p.draw(&mut dead);
        assert_eq!(dead.as_slice()[0].color.a, 0);
    }

    #[test]
    fn mid_angle_bisects_the_span_across_the_seam() {
        // A normal span.
        assert!((wall_mid_angle(0.0, 1.0) - 0.5).abs() < 1e-6);
        // A span wrapping the 0/TAU seam (start 5.5, end 0.5 → the arc
        // width is TAU-5; the bisector sits 5.5 + width/2).
        let expected = 5.5_f32 + ((0.5_f32 - 5.5_f32).rem_euclid(TAU)) / 2.0_f32;
        assert!((wall_mid_angle(5.5, 0.5) - expected).abs() < 1e-6);
    }

    #[test]
    fn angle_distance_is_seam_aware() {
        assert!((angle_distance(0.1, 0.2) - 0.1).abs() < 1e-6);
        assert!((angle_distance(0.0, TAU - 0.1) - 0.1).abs() < 1e-6);
    }
}
