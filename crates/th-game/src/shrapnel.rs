//! plan 002 issue 09 (rework) — the kid's collision shrapnel, from
//! scratch: a soft collision shatters the hit wall into GLASS SHARDS —
//! irregular convex pieces cut from the wall's actual 4-vertex quad (a
//! jittered fan cut, 6..10 pieces) — which burst under a real physics
//! sim: radial impulse from the impact point + tangential jitter,
//! mass-scaled momentum, drag, tumbling (decaying angular velocity), and
//! elastic bounces off the breathing core (plus a light pairwise
//! shard-shard bounce).
//!
//! Shards are engine-side (`EngineState::shrapnel`): the update advances
//! them with the frame delta (so a bump's slow-motion slows them too),
//! and the draw pushes each convex piece as a triangle fan (its 3+
//! vertices around the piece's own centroid) into the SAME
//! `FastVertexVectorTris` wall batch — one draw call, the wall pass's
//! SFML Alpha blend. Per-vertex color gives the glass look: the centroid
//! vertex carries the wall's own color, the perimeter vertices carry it
//! lightened toward white, and every vertex's alpha fades with the
//! shard's remaining life.
//!
//! Everything time-like is in ENGINE FT units (the update loop's `m_ft`
//! scale: 60 units = 1 real second, one 240 Hz tick = 0.25) — same
//! discipline the previous wedge implementation's seconds-based bug
//! taught us.
//!
//! Propagation is PHYSICAL, not queued, and it is EMERGENT: the hit
//! knocks the glass inward ([`SHARD_INWARD_KNOCK`]) — pieces fly across
//! the arena, tumble, and bounce off the core. A piece that later
//! reaches a LIVE wall shatters that wall at the piece's position (the
//! engine's impact pass, `update_shrapnel`): the wall dies, the hitter
//! is consumed, and the new shards carry the blast's `burst` id with
//! `generation + 1`. A piece with `generation >= [`MAX_PROPAGATION`]
//! crumbles on impact instead — the wall survives — so one player hit
//! shatters a bounded set of walls. No dedicated carrier projectiles,
//! no timers, no extra bumps: `bump_serial` increments only on the
//! player's physical hit (the flash rides per bump).
//!
//! Reach: a piece flies along a CHORD (no centripetal force) at the
//! mass-scaled impulse speeds ([`SHARD_IMPULSE_MIN..MAX`] / mass,
//! clamped to [`SHARD_SPEED_MAX`]) with per-FT drag, over its
//! [`SHARD_LIFE_FT`] life. Pieces knocked inward cross the arena in
//! well under a second; a piece deflected outward (the blast's radial
//! component still spreads) can sweep the ring band in its in-band
//! dwell and reach a neighbour span — propagation happens when the
//! physics happens to put a piece on a wall, not on a schedule.
//! The generation cap, the lifetimes and the [`MAX_LIVE_SHARDS`] cap
//! bound every blast in all cases.
//!
//! Lineage: every shard carries its BLAST's `burst` id (the engine's
//! `next_burst` counter, reset by `new_game`) and its `generation`
//! (0 = the pieces of the player's physical hit; +1 per shard-impact
//! shatter). Same-blast shards skip the pairwise bounce (the original
//! "fragments of one shatter" rule, now blast-wide — the pieces of one
//! explosion are one visual unit).
//!
//! Performance: [`MAX_LIVE_SHARDS`] caps total live shards (the oldest
//! die first on spawn, via [`make_room`]); the pairwise bounce is O(n²)
//! in shard count but each check is a handful of flops, so the 64 cap
//! keeps it comfortably within a frame. The impact pass is O(shards ×
//! walls) four-vertex point tests (≤ 64 × ~30) — negligible. No new
//! textures, no new render passes, no DrawTarget changes.

use th_math::hg::color::Color;
use th_math::ssvs::vector2::get_mag;
use th_math::ssvs::{get_vec_from_rad, Vec2};

use crate::polygon::point_in_four_vertex_polygon;
use crate::seams::CosmeticRng;
use crate::vertex_batch::FastVertexVectorTris;

// ----------------------------------------------------------------------
// Tunables (all time values in FT units: 60 = 1 real second)
// ----------------------------------------------------------------------

/// Shards spawned per shattered wall (a jittered fan cut of the wall's
/// quad: 6..10 pieces, chosen per wall).
pub const SHARDS_PER_WALL_MIN: usize = 6;
pub const SHARDS_PER_WALL_MAX: usize = 10;

/// The propagation depth cap: a piece with `generation >= MAX_PROPAGATION`
/// crumbles on a wall impact (the wall survives) instead of shattering
/// it. 2 = the player's hit (gen 0) can propagate through two more
/// shatters (gen 1 → gen 2); gen 2 never shatters.
pub const MAX_PROPAGATION: u32 = 2;

/// How strongly the hit KNOCKS the glass inward, into the arena (0 =
/// the pure radial blast from the impact point — which points mostly
/// OFF-SCREEN, because the impact sits at the walls' inner edge; 1 =
/// straight at the core). The blend keeps a radial component so the
/// burst still spreads from the impact, while most pieces visibly fly
/// across the arena, tumble, and bounce off the core.
pub const SHARD_INWARD_KNOCK: f32 = 0.55;

/// A shard's lifetime, in FT units (110 ≈ 1.8 s real time) — long
/// enough to SEE the glass fly, tumble and bounce.
pub const SHARD_LIFE_FT: f32 = 110.0;

/// The cap on total live shards (all bursts combined); the oldest are
/// evicted first when a new burst would exceed it.
pub const MAX_LIVE_SHARDS: usize = 64;

/// The blast impulse range, in momentum units (px*mass/FT-unit): the
/// shard's radial speed is `impulse / mass`, so light slivers fly fast
/// and heavy chunks lag — the radial component of the burst from the
/// impact point.
pub const SHARD_IMPULSE_MIN: f32 = 2.0;
pub const SHARD_IMPULSE_MAX: f32 = 5.5;

/// The tangential jitter, as a FRACTION of the piece's own speed (±0.35
/// = ±35%): breaks the perfectly-radial symmetry of the burst while
/// staying mass-scaled (the jitter rides the same speed the impulse
/// produced — a heavy chunk's sideways kick is as gentle as its flight).
pub const SHARD_TANGENT_MAX: f32 = 0.35;

/// The speed clamp (px/FT-unit): the blast never throws a piece faster
/// than this (≈ 540 px/s).
pub const SHARD_SPEED_MAX: f32 = 9.0;

/// The spin range (rad/FT-unit): ±(1..5 rad/s → × 1/60).
pub const SHARD_SPIN_MAX: f32 = 5.0 / 60.0;

/// Mass scale: `mass = area_px² / SHARD_MASS_SCALE`, clamped to
/// `[SHARD_MASS_MIN, SHARD_MASS_MAX]` (a typical wall shard, ~600 px²,
/// lands near 1.2).
pub const SHARD_MASS_SCALE: f32 = 500.0;
pub const SHARD_MASS_MIN: f32 = 0.2;
pub const SHARD_MASS_MAX: f32 = 4.0;

/// Drag per FT-unit (0.5/s → the velocity halves in ~1.4 s): the
/// shards settle instead of flying off the screen, but keep flying
/// long enough to read as glass.
pub const SHARD_DRAG_PER_FT: f32 = 0.5_f32 / 60.0;

/// Spin decay per FT-unit (1.2/s): the tumble winds down.
pub const SHARD_SPIN_DECAY_PER_FT: f32 = 0.02;

/// The core bounce: the radial reflection keeps this fraction of the
/// speed (energetic glass — the pieces skip and spin across the arena).
pub const BOUNCE_KEEP: f32 = 0.72;

/// The shard-shard bounce restitution (light elastic collisions).
pub const SHARD_BOUNCE_KEEP: f32 = 0.7;

/// How far the shard's perimeter-vertex color is lightened toward white
/// (the "glass edge" tint): `channel + (255 - channel) * TINT`.
pub const GLASS_EDGE_TINT: f32 = 0.55;

/// The perimeter-vertex alpha relative to the centroid-vertex alpha
/// (glass reads as more transparent at the edges).
pub const GLASS_EDGE_ALPHA: f32 = 0.75;

// ----------------------------------------------------------------------
// The shard
// ----------------------------------------------------------------------

/// One glass shard: an irregular triangular piece of a shattered wall.
///
/// `local` holds the three vertices in the SPAWN frame (offsets from
/// `position`, the piece's centroid); `angle`/`spin` tumble the piece
/// around that centroid. `mass` (from the piece's area) scales the blast
/// momentum; `radius` (the piece's circumradius) is the pairwise
/// collision radius; the two colors drive the glass look at draw time.
/// One auditable glass event, for the app's generative clink pass (the
/// sound is the APP's business — the engine only records what physically
/// happened, where, how fast, how big).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShardEvent {
    /// The wall's material CRACKED (the shatter instant — one loud,
    /// low, noisy event; the pieces' sizes scale the partial spread).
    Crack { position: Vec2, speed01: f32 },
    /// A piece BOUNCED off the core (the glass TINKS: pitched by the
    /// piece's size, loud by the bounce speed, at the bounce position).
    Tink {
        position: Vec2,
        speed01: f32,
        size01: f32,
    },
}

/// The pending events' cap (per engine; the drain drops the excess).
pub const MAX_PENDING_EVENTS: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shrapnel {
    pub position: Vec2,
    pub velocity: Vec2,
    pub local: [Vec2; 3],
    /// The tumble (radians from the spawn orientation).
    pub angle: f32,
    /// The angular velocity (rad/FT-unit; decays).
    pub spin: f32,
    pub mass: f32,
    /// The circumradius (the max vertex distance from the centroid) —
    /// the pairwise-collision radius.
    pub radius: f32,
    pub life: f32,
    pub max_life: f32,
    /// The BLAST this piece belongs to (the engine's `next_burst`
    /// sequence): the pieces of one blast — the player's hit and every
    /// shard-impact shatter it physically caused — share the id, and
    /// same-blast pieces never collide with each other (they are one
    /// visual explosion; the pieces of one shatter tile one wall).
    pub burst: u32,
    /// The propagation generation: 0 = the pieces of the player's
    /// physical hit, +1 per shard-impact shatter. A piece with
    /// `generation >= MAX_PROPAGATION` crumbles on impact (the wall
    /// survives) instead of shattering it.
    pub generation: u32,
    /// The centroid-vertex color: the wall's own color (alpha applied at
    /// draw time, fading with life).
    pub color_in: Color,
    /// The perimeter-vertex color: the wall's color lightened toward
    /// white (the glass edge).
    pub color_out: Color,
}

impl Shrapnel {
    /// A synthetic piece for the integration tests' deterministic
    /// impact-pass drives (the real spawn path is `split_wall_quad`'s;
    /// this one is a small, heavy, full-life triangle the tests place
    /// exactly where they need a hit).
    #[must_use]
    pub fn test_piece(position: Vec2, velocity: Vec2, generation: u32) -> Shrapnel {
        let local = [
            Vec2::new(-6.0, -4.0),
            Vec2::new(7.0, -2.0),
            Vec2::new(-1.0, 6.0),
        ];
        let radius = local.iter().fold(0.0_f32, |m, p| m.max(get_mag(p)));
        Shrapnel {
            position,
            velocity,
            local,
            angle: 0.0,
            spin: 0.0,
            mass: 1.2,
            radius,
            life: SHARD_LIFE_FT,
            max_life: SHARD_LIFE_FT,
            burst: 0,
            generation,
            color_in: Color::new(10, 20, 30, 255),
            color_out: Color::new(10, 20, 30, 255),
        }
    }

    /// Advance by the frame delta `ft` (FT units — the update loop's
    /// scale, 60 = 1 real second): the position rides the velocity
    /// (explicit Euler), a light drag settles it, the tumble integrates
    /// the (decaying) spin, the life burns — and a shard that CROSSES
    /// the core edge from outside to INSIDE within the tick BOUNCES:
    /// the radial velocity component reflects (elastic, ×
    /// [`BOUNCE_KEEP`]), the piece is pushed back onto the core edge,
    /// and the tumble direction flips, damped. Crossing semantics —
    /// not "inside → reflect": a piece BORN inside the breathing orbit
    /// (a wall whose inner edge sits below the core edge, a burst's
    /// inner slivers) must fly free, not be trapped and re-reflected
    /// by the old position-only check.
    /// `true` when this step BOUNCED off the core (the caller's glass
    /// Tink event — the caller reads the post-bounce state).
    #[must_use]
    pub fn update(&mut self, ft: f32, core_radius: f32) -> bool {
        // The pre-step radius: the crossing test's "from" side.
        let prev_mag = get_mag(&self.position);
        self.position += self.velocity * ft;
        let drag = (1.0_f32 - SHARD_DRAG_PER_FT * ft).max(0.0);
        self.velocity *= drag;
        self.angle += self.spin * ft;
        self.spin *= (1.0_f32 - SHARD_SPIN_DECAY_PER_FT * ft).max(0.0);
        self.life -= ft;

        let mag = get_mag(&self.position);
        if prev_mag >= core_radius && mag < core_radius && mag > 1.0e-4 {
            let nx = self.position.x / mag;
            let ny = self.position.y / mag;
            let v_dot_n = self.velocity.x * nx + self.velocity.y * ny;
            if v_dot_n < 0.0 {
                // v' = v - 2 (v·n) n, then the energy loss.
                self.velocity.x -= 2.0 * v_dot_n * nx;
                self.velocity.y -= 2.0 * v_dot_n * ny;
                self.velocity.x *= BOUNCE_KEEP;
                self.velocity.y *= BOUNCE_KEEP;
                self.position.x = nx * core_radius;
                self.position.y = ny * core_radius;
                self.spin = -self.spin * BOUNCE_KEEP;
                return true;
            }
        }
        false
    }

    /// `true` while the shard still shows (life > 0).
    #[must_use]
    pub fn is_alive(&self) -> bool {
        self.life > 0.0
    }

    /// Draw the piece into the wall batch: the convex polygon as a
    /// triangle fan from its (tumbled) centroid — one triangle per
    /// vertex pair — with per-vertex color: the centroid vertex carries
    /// the wall color, the perimeter vertices carry the lightened glass
    /// edge, and every vertex's alpha fades with the remaining life.
    pub fn draw(&self, wall_quads: &mut FastVertexVectorTris) {
        let alpha = (self.life / self.max_life).clamp(0.0, 1.0);
        let (sin, cos) = (self.angle.sin(), self.angle.cos());
        // The spawn-frame local vertices rotated by the tumble.
        let v = |p: Vec2| Vec2::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos);
        let (a, b, c) = (
            self.position + v(self.local[0]),
            self.position + v(self.local[1]),
            self.position + v(self.local[2]),
        );
        let mut in_color = self.color_in;
        in_color.a = (in_color.a as f32 * alpha) as u8;
        let mut out_color = self.color_out;
        out_color.a = (out_color.a as f32 * alpha * GLASS_EDGE_ALPHA) as u8;

        // Fan: (center, v0, v1), (center, v1, v2), (center, v2, v0).
        wall_quads.batch_unsafe_emplace_back(in_color, &[self.position]);
        wall_quads.batch_unsafe_emplace_back(out_color, &[a, b]);
        wall_quads.batch_unsafe_emplace_back(in_color, &[self.position]);
        wall_quads.batch_unsafe_emplace_back(out_color, &[b, c]);
        wall_quads.batch_unsafe_emplace_back(in_color, &[self.position]);
        wall_quads.batch_unsafe_emplace_back(out_color, &[c, a]);
    }
}

// ----------------------------------------------------------------------
// The shatter: a jittered fan cut of the wall's quad
// ----------------------------------------------------------------------

/// Shatter a wall quad into 6..10 irregular glass shards: a jittered
/// point INTERIOR to the quad fans out to every corner plus jittered
/// points on the edges; each consecutive perimeter pair with the fan
/// point is one triangular piece (all pieces are convex, all pieces are
/// cut from — and tile — the wall's actual shape).
///
/// `corners`: the wall's 4 vertices in cyclic order (the CWall nw/sw/se
/// /ne); `impact`: the blast origin (the player hit's nearest wall
/// vertex, or the impacting piece's position for shard-impact shatters
/// — the fan cut is biased toward it: the fan point is pulled halfway
/// from the quad's centroid toward `impact` when it lies inside the
/// quad); `outward`: the radial angle at `impact` (the fallback burst
/// direction); `color`: the wall's current color; `burst`: the blast id
/// the pieces inherit (the engine's `next_burst`); `generation`: the
/// propagation generation (0 = the player's physical hit, +1 per
/// shard-impact shatter); `rng`: the cosmetic rng (non-simulation).
/// The pieces carry the spawn aim: the mass-scaled impulse from
/// `impact` knocked inward (see [`SHARD_INWARD_KNOCK`]) + tangential
/// jitter, with decaying spin, the glass colors, and their lineage
/// (`burst`, `generation`).
// 8 args: the flat spawn contract (geometry + color + lineage + rng)
// stays more readable than a builder struct at a single call site
// family (the engine's shatters + the unit tests).
#[allow(clippy::too_many_arguments)]
pub fn split_wall_quad(
    corners: [Vec2; 4],
    impact: Vec2,
    outward: f32,
    color: Color,
    burst: u32,
    generation: u32,
    rng: &dyn CosmeticRng,
) -> Vec<Shrapnel> {
    // 2..6 extra perimeter points → 6..10 pieces.
    let extra = rng.get_rnd_i(
        (SHARDS_PER_WALL_MIN - 4) as i32,
        (SHARDS_PER_WALL_MAX - 4 + 1) as i32,
    ) as usize;

    // Jittered points on the quad's edges: (edge-index, t in (0,1)).
    let mut edge_points: Vec<(usize, f32)> = Vec::with_capacity(extra);
    for _ in 0..extra {
        edge_points.push((rng.get_rnd_i(0, 4) as usize, rng.get_rnd_r(0.15, 0.85)));
    }

    // The perimeter in cyclic order: walk the corners, dropping each
    // edge's points (sorted by t) before the edge's end corner.
    let mut perimeter: Vec<Vec2> = Vec::with_capacity(4 + extra);
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        let mut ts: Vec<f32> = edge_points
            .iter()
            .filter(|(e, _)| *e == k)
            .map(|(_, t)| *t)
            .collect();
        ts.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        for t in ts {
            perimeter.push(a + (b - a) * t);
        }
        perimeter.push(corners[(k + 1) % 4]);
    }

    // The fan point: the quad centroid pulled halfway toward the blast
    // origin when the origin lies inside the quad (the shard-impact
    // case — the cut biases to where the glass actually broke), then
    // jittered a little; the quad is convex and the jitter is bounded
    // by a quarter of its span, so if the jitter ever lands outside
    // (rounding), fall back to the plain centroid.
    let centroid = (corners[0] + corners[1] + corners[2] + corners[3]) * 0.25;
    let fan_base = if point_in_four_vertex_polygon(
        &corners[0],
        &corners[1],
        &corners[2],
        &corners[3],
        &impact,
    ) {
        (centroid + impact) * 0.5
    } else {
        centroid
    };
    let span = corners
        .iter()
        .map(|c| get_mag(&(*c - centroid)))
        .fold(0.0_f32, f32::max);
    let jitter = Vec2::new(
        rng.get_rnd_r(-0.25, 0.25) * span,
        rng.get_rnd_r(-0.25, 0.25) * span,
    );
    let fan = if point_in_four_vertex_polygon(
        &corners[0],
        &corners[1],
        &corners[2],
        &corners[3],
        &(fan_base + jitter),
    ) {
        fan_base + jitter
    } else {
        fan_base
    };

    // Each consecutive perimeter pair + the fan point is one piece.
    let mut out = Vec::with_capacity(perimeter.len());
    let n = perimeter.len();
    for i in 0..n {
        let a = perimeter[i];
        let b = perimeter[(i + 1) % n];
        if a == b {
            continue; // a degenerate sliver (coincident perimeter points)
        }
        out.push(make_shard(
            &fan, a, b, impact, outward, color, burst, generation, rng,
        ));
    }
    out
}

/// Build one shard from the fan triangle (fan, a, b): centroid + local
/// offsets, area-based mass, the blast velocity (the mass-scaled
/// impulse from the impact point + tangential jitter, speed-clamped,
/// KNOCKED INWARD by [`SHARD_INWARD_KNOCK`] — the walls live along the
/// ring and the impact sits at their inner edge, so the pure radial
/// blast mostly points off-screen), the tumble, the glass colors, and
/// the lineage.
#[allow(clippy::too_many_arguments)]
fn make_shard(
    fan: &Vec2,
    a: Vec2,
    b: Vec2,
    impact: Vec2,
    outward: f32,
    color: Color,
    burst: u32,
    generation: u32,
    rng: &dyn CosmeticRng,
) -> Shrapnel {
    let position = (*fan + a + b) * (1.0_f32 / 3.0);
    let local = [*fan - position, a - position, b - position];

    // Mass ∝ the piece's area (heavy chunks lag, light slivers fly).
    let e1 = a - *fan;
    let e2 = b - *fan;
    let area = 0.5_f32 * (e1.x * e2.y - e1.y * e2.x).abs();
    let mass = (area / SHARD_MASS_SCALE).clamp(SHARD_MASS_MIN, SHARD_MASS_MAX);

    // The blast direction from the impact point (fallback: `outward`).
    let mut dir = position - impact;
    let dir_mag = get_mag(&dir);
    if dir_mag < 1.0e-3 {
        dir = get_vec_from_rad(outward, 1.0);
    } else {
        dir *= 1.0 / dir_mag;
    }

    // The spawn aim: the mass-scaled impulse from the impact point,
    // KNOCKED INWARD (see [`SHARD_INWARD_KNOCK`] — the impact sits at
    // the walls' inner edge, so a pure radial blast flies off-screen;
    // the knock throws the glass into the arena where the physics is
    // visible), plus the symmetry-breaking tangential jitter.
    {
        // Blend the blast direction with the inward unit vector.
        let r_mag = get_mag(&position).max(1.0e-4);
        let inward = position * (-1.0 / r_mag);
        let dir_blend = dir * (1.0 - SHARD_INWARD_KNOCK) + inward * SHARD_INWARD_KNOCK;
        dir = dir_blend * (1.0 / get_mag(&dir_blend).max(1.0e-4));
    }
    let speed = rng.get_rnd_r(SHARD_IMPULSE_MIN, SHARD_IMPULSE_MAX) / mass;
    let tangential_jitter = rng.get_rnd_r(-SHARD_TANGENT_MAX, SHARD_TANGENT_MAX) * speed;
    let perp = Vec2::new(-dir.y, dir.x);
    let mut velocity = dir * speed + perp * tangential_jitter;
    let v_mag = get_mag(&velocity);
    if v_mag > SHARD_SPEED_MAX {
        velocity *= SHARD_SPEED_MAX / v_mag;
    }
    let life = SHARD_LIFE_FT;

    let radius = local.iter().fold(0.0_f32, |m, p| m.max(get_mag(p)));

    Shrapnel {
        position,
        velocity,
        local,
        angle: 0.0,
        spin: rng.get_rnd_r(-SHARD_SPIN_MAX, SHARD_SPIN_MAX),
        mass,
        radius,
        life,
        max_life: life,
        burst,
        generation,
        color_in: color,
        color_out: lighten_toward_white(color, GLASS_EDGE_TINT),
    }
}

/// Mix a color toward white by `t` (the glass-edge tint): each channel
/// `c + (255 - c) * t` (alpha untouched — it is applied at draw time).
fn lighten_toward_white(c: Color, t: f32) -> Color {
    let mix = |x: u8| (x as f32 + (255.0_f32 - x as f32) * t).round() as u8;
    Color::new(mix(c.r), mix(c.g), mix(c.b), c.a)
}

/// Evict the OLDEST shards (the vector's front) so that `shards` plus
/// `incoming` fits within [`MAX_LIVE_SHARDS`]. Spawn order is the age
/// order, so the front is the oldest.
pub fn make_room(shards: &mut Vec<Shrapnel>, incoming: usize) {
    let excess = shards
        .len()
        .saturating_add(incoming)
        .saturating_sub(MAX_LIVE_SHARDS);
    let evict = excess.min(shards.len());
    if evict > 0 {
        shards.drain(0..evict);
    }
}

/// The light pairwise shard-shard bounce: circle-approximation
/// (circumradius) elastic collisions between live pairs from DIFFERENT
/// blasts (same-`burst` pieces are one visual explosion — the pieces of
/// one shatter tile one wall exactly and don't collide with each other,
/// and the blast's later-generation pieces ride the same explosion). The
/// positions are separated mass-weighted (half the overlap each, split
/// by inverse mass); the velocities get the standard equal-impulse
/// bounce along the pair's normal, damped by
/// [`SHARD_BOUNCE_KEEP`]. Deterministic (index order), O(n²) — the
/// [`MAX_LIVE_SHARDS`] cap keeps it cheap.
pub fn bounce_pairs(shards: &mut [Shrapnel]) {
    let n = shards.len();
    for i in 0..n {
        for j in i + 1..n {
            // Copy out the pair (Shrapnel is Copy) before mutating.
            let (si, sj) = (shards[i], shards[j]);
            if si.burst == sj.burst {
                continue; // one explosion's pieces (same blast)
            }
            let d = sj.position - si.position;
            let dist = get_mag(&d);
            let min = si.radius + sj.radius;
            if dist <= min && dist > 1.0e-4 {
                let nx = d.x / dist;
                let ny = d.y / dist;
                let inv_i = 1.0 / si.mass;
                let inv_j = 1.0 / sj.mass;
                let inv_sum = inv_i + inv_j;

                // Positional separation (half the overlap, mass-weighted).
                let sep = (min - dist) * 0.5;
                shards[i].position.x -= nx * sep * (inv_i / inv_sum);
                shards[i].position.y -= ny * sep * (inv_i / inv_sum);
                shards[j].position.x += nx * sep * (inv_j / inv_sum);
                shards[j].position.y += ny * sep * (inv_j / inv_sum);

                // The elastic impulse (only when the pair is closing).
                let rvx = sj.velocity.x - si.velocity.x;
                let rvy = sj.velocity.y - si.velocity.y;
                let v_n = rvx * nx + rvy * ny;
                if v_n < 0.0 {
                    let j_imp = (-(1.0 + SHARD_BOUNCE_KEEP) * v_n) / inv_sum;
                    shards[i].velocity.x -= nx * j_imp * inv_i;
                    shards[i].velocity.y -= ny * j_imp * inv_i;
                    shards[j].velocity.x += nx * j_imp * inv_j;
                    shards[j].velocity.y += ny * j_imp * inv_j;
                }
            }
        }
    }
}

// ----------------------------------------------------------------------
// The unit tests
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lua_state::ReentrantMutex;
    use crate::polygon::point_in_polygon;
    use th_math::ssvu::math::TAU;
    use th_rng::{uniform_int, uniform_real, Pcg32Fast};

    /// A deterministic cosmetic rng for the unit tests: the production
    /// `PcgCosmeticRng`'s exact distributions (libstdc++-matching
    /// `uniform_real`/`uniform_int` on `pcg32_fast`), but seeded.
    #[derive(Debug)]
    struct SeededCosmeticRng {
        rng: ReentrantMutex<Pcg32Fast>,
    }

    impl SeededCosmeticRng {
        fn new(seed: u64) -> Self {
            Self {
                rng: ReentrantMutex::new(Pcg32Fast::from_seed(seed)),
            }
        }
    }

    impl CosmeticRng for SeededCosmeticRng {
        fn get_rnd_r(&self, min: f32, max: f32) -> f32 {
            uniform_real(&mut self.rng.lock(), min, max)
        }
        fn get_rnd_i(&self, min: i32, max: i32) -> i32 {
            uniform_int(&mut self.rng.lock(), min, max - 1)
        }
    }

    fn shard() -> Shrapnel {
        Shrapnel {
            position: Vec2::new(100.0, 0.0),
            velocity: Vec2::new(2.0, 0.0),
            local: [
                Vec2::new(-6.0, 0.0),
                Vec2::new(6.0, 4.0),
                Vec2::new(0.0, -4.0),
            ],
            angle: 0.0,
            spin: 0.05,
            mass: 1.0,
            radius: 7.21,
            life: SHARD_LIFE_FT,
            max_life: SHARD_LIFE_FT,
            burst: 1,
            generation: 0,
            color_in: Color::new(255, 0, 0, 255),
            color_out: Color::new(255, 140, 140, 255),
        }
    }

    fn wall_quad() -> [Vec2; 4] {
        // A plain wall quad at angle 0: inner edge at r=80 spanning ±30°,
        // outer edge at r=140 (the CWall nw/sw/se/ne order).
        let orbit = |rad: f32, r: f32| get_vec_from_rad(rad, r);
        let a = TAU / 6.0; // 60° = the half-span for a 6-sided wall
        [
            orbit(-a, 80.0),
            orbit(a, 80.0),
            orbit(a, 140.0),
            orbit(-a, 140.0),
        ]
    }

    /// Containment test tolerant of vertices that sit exactly ON the
    /// quad's edges (the cross-product test in
    /// `point_in_four_vertex_polygon` is sign-ambiguous for boundary
    /// points in f32): nudge the point a hair toward the interior and
    /// use the even-odd ray cast.
    fn in_quad(corners: &[Vec2; 4], p: Vec2) -> bool {
        let c = (corners[0] + corners[1] + corners[2] + corners[3]) * 0.25;
        let nudged = p + (c - p) * 1.0e-4;
        point_in_polygon(corners, nudged.x, nudged.y)
    }

    #[test]
    fn split_cut_the_quad_into_6_to_10_pieces_inside_it() {
        let corners = wall_quad();
        let impact = corners[0];
        let outward = 0.0;
        for seed in [1u64, 7, 99] {
            let rng = SeededCosmeticRng::new(seed);
            let shards = split_wall_quad(
                corners,
                impact,
                outward,
                Color::new(10, 20, 30, 255),
                7,
                0,
                &rng,
            );
            assert!(
                (SHARDS_PER_WALL_MIN..=SHARDS_PER_WALL_MAX).contains(&shards.len()),
                "expected 6..10 pieces, got {}",
                shards.len()
            );
            // Every vertex of every piece is cut from the wall's quad.
            for s in &shards {
                for p in s.local {
                    let world = s.position + p;
                    assert!(
                        in_quad(&corners, world),
                        "shard vertex {:?} is inside the wall quad",
                        world
                    );
                }
                assert!(s.is_alive());
                assert!((SHARD_MASS_MIN..=SHARD_MASS_MAX).contains(&s.mass));
                assert!(s.radius > 0.0);
                // The lineage rides the piece.
                assert_eq!(s.burst, 7);
                assert_eq!(s.generation, 0);
                // The glass tint lightens every channel.
                assert!(s.color_out.r >= s.color_in.r);
                assert!(s.color_out.g >= s.color_in.g);
                assert!(s.color_out.b >= s.color_in.b);
            }
        }
    }

    #[test]
    fn split_knocks_the_glass_inward_into_the_arena() {
        let corners = wall_quad();
        let impact = corners[0]; // angle -30°, r=80
        let shards = split_wall_quad(
            corners,
            impact,
            0.0,
            Color::new(10, 20, 30, 255),
            7,
            0,
            &SeededCosmeticRng::new(42),
        );
        // The impact sits at the wall's inner corner, so the pure radial
        // blast would point OFF-SCREEN: the knock (SHARD_INWARD_KNOCK)
        // blends every piece's blast direction with the inward unit
        // vector, and the perp jitter never touches the radial axis —
        // so EVERY piece's radial velocity component is net inward
        // (the glass visibly flies into the arena and bounces off the
        // core instead of spraying off-screen).
        for s in &shards {
            let r_mag = get_mag(&s.position).max(1.0e-4);
            let inward = s.position * (-1.0 / r_mag);
            let v_in = s.velocity.x * inward.x + s.velocity.y * inward.y;
            assert!(
                v_in > 0.0,
                "piece velocity {:?} is knocked inward (inward component {})",
                s.velocity,
                v_in
            );
            assert_eq!(s.max_life, SHARD_LIFE_FT, "one life for all pieces");
            assert!(
                get_mag(&s.velocity) <= SHARD_SPEED_MAX + 1.0e-4,
                "the blast is speed-clamped"
            );
        }
    }

    #[test]
    fn split_pieces_all_carry_the_single_inward_knock_aim() {
        // The spawn aim is ONE mode now (the mass-scaled impulse knocked
        // inward): every piece carries the full shard life, is
        // speed-clamped, and has a positive inward radial component.
        let corners = wall_quad();
        let impact = corners[0];
        let rng = SeededCosmeticRng::new(1234);
        let shards = split_wall_quad(
            corners,
            impact,
            0.0,
            Color::new(10, 20, 30, 255),
            9,
            1,
            &rng,
        );
        for s in &shards {
            assert_eq!(s.max_life, SHARD_LIFE_FT);
            let r_mag = get_mag(&s.position).max(1.0e-4);
            let inward = s.position * (-1.0 / r_mag);
            assert!(
                s.velocity.x * inward.x + s.velocity.y * inward.y > 0.0,
                "every piece is knocked inward"
            );
            assert!(get_mag(&s.velocity) <= SHARD_SPEED_MAX + 1.0e-4);
        }
    }

    #[test]
    fn update_rides_velocity_drags_tumbles_and_burns_life() {
        let mut s = shard();
        let (p0, v0, spin0) = (s.position, s.velocity, s.spin);
        let _ = s.update(0.25, 72.0);
        // Position: explicit Euler with the pre-drag velocity.
        let expected = p0 + v0 * 0.25;
        assert!((s.position.x - expected.x).abs() < 1e-6);
        assert!((s.position.y - expected.y).abs() < 1e-6);
        // Drag slowed it…
        let drag = 1.0 - SHARD_DRAG_PER_FT * 0.25;
        assert!((s.velocity.x - v0.x * drag).abs() < 1e-6);
        // …the tumble integrated…
        assert!((s.angle - spin0 * 0.25).abs() < 1e-6);
        // …and the spin decayed; the life burned.
        assert!((s.spin - spin0 * (1.0 - SHARD_SPIN_DECAY_PER_FT * 0.25)).abs() < 1e-6);
        assert!((s.life - (SHARD_LIFE_FT - 0.25)).abs() < 1e-6);

        // A huge delta stalls drag/spin-decay (never negative) and runs
        // the life out (110 units; 120 > 110).
        let mut s = shard();
        let _ = s.update(120.0, 72.0);
        assert!(s.velocity.x >= 0.0);
        assert!(s.spin >= 0.0);
        assert!(!s.is_alive());
    }

    #[test]
    fn the_core_bounce_uses_crossing_semantics() {
        // Outside→INSIDE crossing within the tick: the radial component
        // reflects outward (× BOUNCE_KEEP, after the tick's drag), the
        // piece is stepped back onto the core edge, the tumble flips,
        // damped.
        let mut s = shard();
        s.position = Vec2::new(75.0, 0.0); // outside the 72 core edge
        s.velocity = Vec2::new(-20.0, 0.0); // 75 - 5 = 70 < 72: crosses
        s.spin = 0.05;
        let _ = s.update(0.25, 72.0);
        assert!(
            (s.position.x - 72.0).abs() < 1e-6,
            "pushed back onto the core edge"
        );
        assert!(s.velocity.x > 0.0, "the radial velocity reflected");
        // The drag ran first (one tick), then the reflection kept
        // BOUNCE_KEEP of the (already dragged) speed.
        let dragged = 20.0 * (1.0 - SHARD_DRAG_PER_FT * 0.25);
        assert!((s.velocity.x - dragged * BOUNCE_KEEP).abs() < 1e-5);
        assert!(
            (s.spin - (-(0.05 * (1.0 - SHARD_SPIN_DECAY_PER_FT * 0.25)) * BOUNCE_KEEP)).abs()
                < 1e-6,
            "the tumble flipped, damped (the spin decay ran before the impact)"
        );

        // A piece BORN inside the orbit, moving INWARD: it flies free —
        // no reflection, no clamp (the old inside→reflect check trapped
        // exactly this piece: a carrier spawned below the breathing core
        // edge would lose half its sweep speed to a phantom bounce).
        let mut s = shard();
        s.position = Vec2::new(50.0, 0.0);
        s.velocity = Vec2::new(-2.0, 0.0);
        let _ = s.update(0.25, 72.0);
        assert!(
            (s.position.x - (50.0 - 2.0 * 0.25)).abs() < 1e-6,
            "no clamp to the core edge"
        );
        assert!(
            (s.velocity.x - (-2.0 * (1.0 - SHARD_DRAG_PER_FT * 0.25))).abs() < 1e-6,
            "no reflection"
        );

        // Inside→OUTSIDE crossing: no bounce (only outside→inside
        // counts — the piece already passed the edge outward).
        let mut s = shard();
        s.position = Vec2::new(70.0, 0.0);
        s.velocity = Vec2::new(20.0, 0.0); // 70 + 5 = 75 > 72
        let _ = s.update(0.25, 72.0);
        assert!((s.position.x - 75.0).abs() < 1e-6);
        assert!((s.velocity.x - 20.0 * (1.0 - SHARD_DRAG_PER_FT * 0.25)).abs() < 1e-5);

        // Stays outside: no bounce (the default shard at 100, +2).
        let mut s = shard();
        let _ = s.update(0.25, 72.0);
        assert!((s.position.x - (100.0 + 2.0 * 0.25)).abs() < 1e-6);
        assert!((s.velocity.x - 2.0 * (1.0 - SHARD_DRAG_PER_FT * 0.25)).abs() < 1e-6);
    }

    #[test]
    fn bounce_pairs_separate_and_exchange_momentum() {
        // Two equal shards overlapping head-on: they separate and the
        // radial velocities swap (restitution 0.5, equal mass → each
        // keeps half its own speed and gains half the other's).
        let mut a = shard();
        a.position = Vec2::new(0.0, 0.0);
        a.velocity = Vec2::new(1.0, 0.0);
        a.mass = 1.0;
        a.radius = 5.0;
        let mut b = shard();
        b.position = Vec2::new(8.0, 0.0);
        b.velocity = Vec2::new(-1.0, 0.0);
        b.mass = 1.0;
        b.radius = 5.0;
        b.burst = 2; // a different shatter (same-burst pairs skip)
        let mut shards = [a, b];
        bounce_pairs(&mut shards);
        // Overlap 2, half each, mass-weighted (equal masses → 0.5 each):
        // a at -0.5, b at 8.5 (positions apart again).
        assert!((shards[0].position.x - -0.5).abs() < 1e-6);
        assert!((shards[1].position.x - 8.5).abs() < 1e-6);
        // vn = -2, j = (1.7 * 2) / 2 = 1.7 (restitution 0.7), equal
        // masses: each Δv = 1.7 → a at −0.7, b at +0.7 (they separate).
        assert!((shards[0].velocity.x - -0.7).abs() < 1e-6);
        assert!((shards[1].velocity.x - 0.7).abs() < 1e-6);

        // Non-overlapping pairs are untouched.
        let mut far = shard();
        far.position = Vec2::new(100.0, 0.0);
        far.velocity = Vec2::new(-1.0, 0.0);
        let mut near = shard();
        near.position = Vec2::new(0.0, 0.0);
        near.velocity = Vec2::new(1.0, 0.0);
        let mut pair = [near, far];
        let before = pair.map(|s| (s.position, s.velocity));
        bounce_pairs(&mut pair);
        assert_eq!((pair[0].position, pair[0].velocity), before[0]);
        assert_eq!((pair[1].position, pair[1].velocity), before[1]);

        // Same-burst pairs (fragments of one wall) are untouched even
        // when overlapping.
        let mut s1 = shard();
        s1.position = Vec2::new(0.0, 0.0);
        s1.velocity = Vec2::new(1.0, 0.0);
        s1.radius = 5.0;
        let mut s2 = shard();
        s2.position = Vec2::new(8.0, 0.0);
        s2.velocity = Vec2::new(-1.0, 0.0);
        s2.radius = 5.0; // same burst as s1 (both from `shard()`)
        let mut same = [s1, s2];
        let same_before = same.map(|s| (s.position, s.velocity));
        bounce_pairs(&mut same);
        assert_eq!((same[0].position, same[0].velocity), same_before[0]);
        assert_eq!((same[1].position, same[1].velocity), same_before[1]);
    }

    #[test]
    fn make_room_evicts_the_oldest_first() {
        let mut shards: Vec<Shrapnel> = (0..70)
            .map(|i| {
                let mut s = shard();
                s.position = Vec2::new(i as f32, 0.0);
                s
            })
            .collect();
        make_room(&mut shards, 10);
        assert_eq!(shards.len(), 54, "70 + 10 incoming - 16 evicted = 54");
        assert_eq!(
            shards.first().map(|s| s.position.x),
            Some(16.0),
            "the 16 oldest (front) died"
        );

        // No eviction under the cap.
        let mut few: Vec<Shrapnel> = (0..10)
            .map(|i| {
                let mut s = shard();
                s.position = Vec2::new(i as f32, 0.0);
                s
            })
            .collect();
        make_room(&mut few, 10);
        assert_eq!(few.len(), 10);
    }

    #[test]
    fn draw_emits_the_fan_with_per_vertex_glass_colors() {
        let s = shard();
        let mut batch = FastVertexVectorTris::new();
        s.draw(&mut batch);
        // 3 fan triangles × 3 vertices.
        assert_eq!(batch.size(), 9);
        let v = batch.as_slice();
        // The centroid vertex (the wall color) is the odd… — the fan is
        // (center, a, b)(center, b, c)(center, c, a): centers at 0, 3, 6.
        assert_eq!(
            v[0].color,
            Color::new(255, 0, 0, 255),
            "full life: opaque wall color"
        );
        assert_eq!(
            v[1].color,
            Color::new(255, 140, 140, (255_f32 * GLASS_EDGE_ALPHA) as u8),
            "the edge vertex carries the tint + edge alpha"
        );
        assert_eq!(v[3].color, v[0].color);
        assert_eq!(v[6].color, v[0].color);

        // Half life: every alpha halves (u8 floor) and the tumbled
        // vertices moved off the spawn orientation.
        let mut half = s;
        half.life = SHARD_LIFE_FT * 0.5;
        half.angle = 0.3;
        let mut hb = FastVertexVectorTris::new();
        half.draw(&mut hb);
        let hv = hb.as_slice();
        assert!(hv[0].color.a < 255);
        assert!(hv[0].color.a as f32 - 255.0 * 0.5 < 1.0); // u8 rounding
        assert_ne!(
            v[1].position, hv[1].position,
            "the tumble rotated the piece"
        );

        // Dead: fully transparent.
        let mut dead = s;
        dead.life = 0.0;
        let mut db = FastVertexVectorTris::new();
        dead.draw(&mut db);
        assert!(db.as_slice().iter().all(|x| x.color.a == 0));
    }
}
