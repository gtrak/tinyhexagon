//! plan 002 issue 07 — the tiny journey: the 20-stage arc (20 × 30 s = a
//! 10-minute run) and its per-stage difficulty / BPM / hue / shape table.
//!
//! plan 002 issue 09-taste (operator feedback): the run is now **one
//! continuous level** (`journey`). Stage changes no longer call
//! `new_game` — the arena, walls, player and level script survive the
//! whole run; only the difficulty (every frame), the hue (every frame),
//! the music stage and the arena shape (at the stage mark) change. That
//! removes the abrupt wall-clearing transition and the 3 s empty restart.
//!
//! The difficulty is a **continuous** function of the journey clock
//! (`stage + elapsed / 30`), so the walls speed up smoothly instead of in
//! 30-second steps. It starts at 0.45× (much slower than the old 0.55×)
//! and reaches 2.15× at the plateau, then holds.

/// Seconds per stage (the contract: 30 s × 20 transitions = 10 minutes to
/// the plateau).
pub const STAGE_DURATION_S: f32 = 30.0;

/// The plateau stage (stages are 0..=20; 20 transitions).
pub const MAX_STAGE: u32 = 20;

/// The single level id (the 09-taste continuous journey). The pack keeps
/// the old `meadow`/`river`/`stars` levels as reference content, but the
/// run never swaps levels mid-flight.
pub const LEVEL: &str = "journey";

/// `difficulty = 0.45 + 0.085 * min(t, 20)` (the arc 0.45..=2.15; the
/// engine's `set_difficulty_mult` clamps 0.1..=4.0, so every value lands).
#[must_use]
pub fn difficulty_for_stage(stage: u32) -> f32 {
    0.45 + 0.085 * stage.min(MAX_STAGE) as f32
}

/// `bpm = 70 + 2.5 * min(stage, 20)` (cap 120 — the 09-taste faster
/// plateau; `th_synth`'s `director::stage_params` owns the live mix, this
/// is the journey's mirror for the table's tests / the report).
#[must_use]
pub fn bpm_for_stage(stage: u32) -> f32 {
    70.0 + 2.5 * stage.min(MAX_STAGE) as f32
}

/// The level id at `stage`: always the single journey level.
#[must_use]
pub fn level_id_for_stage(_stage: u32, pack: &str) -> String {
    format!("{pack}_{LEVEL}")
}

/// The base hue = `100° + 28° * t` (mod 360), capped at the plateau: a
/// full rainbow cycle roughly every 13 stages. The app feeds it to
/// `apply_music_beat`; the style's dynamic colours do the rest.
#[must_use]
pub fn base_hue_for_stage(stage: u32) -> f32 {
    let s = stage.min(MAX_STAGE) as f32;
    (100.0 + 28.0 * s).rem_euclid(360.0)
}

/// The arena's side count per stage (index 0..=20). Two-stage blocks, a
/// tour through 3..8 sides, and a return to the hexagon at the plateau.
const SIDES_BY_STAGE: [u32; 21] = [
    6, 6, 4, 4, 3, 3, 5, 5, 6, 6, 7, 7, 4, 4, 8, 8, 5, 5, 6, 6, 6,
];

/// The shape the stage plays (3..=8 sides).
#[must_use]
pub fn sides_for_stage(stage: u32) -> u32 {
    SIDES_BY_STAGE[stage.min(MAX_STAGE) as usize]
}

/// The journey clock (the plan 07 contract struct).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Journey {
    /// The current stage (0..=20 — the plateau caps it at 20).
    pub stage: u32,
    /// Seconds into the current stage (always `<` [`STAGE_DURATION_S`]).
    pub elapsed: f32,
}

impl Default for Journey {
    fn default() -> Self {
        Self::new()
    }
}

impl Journey {
    /// Stage 0, not elapsed.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            stage: 0,
            elapsed: 0.0,
        }
    }

    /// The continuous journey position `t = stage + elapsed / 30`, capped
    /// at the plateau.
    #[must_use]
    pub fn t(&self) -> f32 {
        (self.stage as f32 + self.elapsed / STAGE_DURATION_S).min(MAX_STAGE as f32)
    }

    /// The multiplier the current frame plays with (the engine's
    /// `new_game` argument / the per-frame `set_difficulty_mult` value).
    #[must_use]
    pub fn difficulty(&self) -> f32 {
        0.45 + 0.085 * self.t()
    }

    /// The level id the run plays (the single journey level).
    #[must_use]
    pub fn level_id(&self, pack: &str) -> String {
        level_id_for_stage(self.stage, pack)
    }

    /// The frame's base hue (the `apply_music_beat` argument), continuous
    /// across stage boundaries.
    #[must_use]
    pub fn base_hue(&self) -> f32 {
        (100.0 + 28.0 * self.t()).rem_euclid(360.0)
    }

    /// The stage's arena side count.
    #[must_use]
    pub fn sides(&self) -> u32 {
        sides_for_stage(self.stage)
    }

    /// Advance the clock by `dt` seconds (the frame's real time — the
    /// bump slow-motion does NOT delay the journey). Returns `Some(stage)`
    /// when a stage boundary crossed (at most one per call — the frame's
    /// clamped dt can never span two 30-s stages); the caller performs the
    /// stage-up work (music stage, shape, hue / SFX).
    pub fn advance(&mut self, dt: f32) -> Option<u32> {
        self.elapsed += dt;
        let mut advanced = None;
        while self.elapsed >= STAGE_DURATION_S {
            if self.stage < MAX_STAGE {
                self.stage += 1;
                advanced = Some(self.stage);
            }
            self.elapsed -= STAGE_DURATION_S;
        }
        advanced
    }

    /// A manual reset: restart the current stage (no stage change).
    pub fn reset_elapsed(&mut self) {
        self.elapsed = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The contract ramps are monotonic and capped at stage 20.
    #[test]
    fn the_ramps_are_monotonic_and_capped_at_the_plateau() {
        for stage in 1..=MAX_STAGE {
            assert!(
                difficulty_for_stage(stage) > difficulty_for_stage(stage - 1),
                "difficulty must rise up to the plateau"
            );
            assert!(
                bpm_for_stage(stage) > bpm_for_stage(stage - 1),
                "bpm must rise up to the plateau"
            );
        }
        assert!((difficulty_for_stage(0) - 0.45).abs() < 1e-6);
        assert!((difficulty_for_stage(MAX_STAGE) - 2.15).abs() < 1e-6);
        assert!((bpm_for_stage(0) - 70.0).abs() < 1e-6);
        assert!((bpm_for_stage(MAX_STAGE) - 120.0).abs() < 1e-6);
        for beyond in (MAX_STAGE + 1)..=64 {
            assert_eq!(
                difficulty_for_stage(beyond),
                difficulty_for_stage(MAX_STAGE)
            );
            assert_eq!(bpm_for_stage(beyond), bpm_for_stage(MAX_STAGE));
            assert_eq!(base_hue_for_stage(beyond), base_hue_for_stage(MAX_STAGE));
            assert_eq!(sides_for_stage(beyond), sides_for_stage(MAX_STAGE));
        }
        // The 09-taste arc is much wider than the old 0.55..1.15 / 70..100.
        assert!(difficulty_for_stage(MAX_STAGE) / difficulty_for_stage(0) > 4.0);
        assert!(bpm_for_stage(MAX_STAGE) - bpm_for_stage(0) >= 50.0);
    }

    /// The single continuous journey level: every stage plays `journey`.
    #[test]
    fn every_stage_plays_the_journey_level() {
        for stage in 0..=MAX_STAGE {
            assert_eq!(
                level_id_for_stage(stage, "pack"),
                "pack_journey",
                "stage {stage}"
            );
        }
        assert_eq!(level_id_for_stage(MAX_STAGE, "pack"), "pack_journey");
    }

    /// The continuous ramp: the journey's `difficulty` rises *within* a
    /// stage too (the per-frame `set_difficulty_mult`), and the hue drifts
    /// continuously.
    #[test]
    fn the_ramp_is_continuous_within_a_stage() {
        let start = Journey::new();
        let mut mid = start;
        mid.advance(15.0);
        assert_eq!(mid.stage, 0);
        assert!(mid.difficulty() > start.difficulty());
        assert!((mid.difficulty() - (0.45 + 0.085 * 0.5)).abs() < 1e-6);
        // The stage-2 mark + 5 s: t = 2 + 5/30.
        let late = Journey {
            stage: 2,
            elapsed: 5.0,
        };
        assert!((late.t() - 2.0 - 5.0 / 30.0).abs() < 1e-5);
        assert!((late.difficulty() - (0.45 + 0.085 * (2.0 + 5.0 / 30.0))).abs() < 1e-5);
        // The hue is continuous and wraps instead of clamping.
        assert!((start.base_hue() - 100.0).abs() < 1e-6);
        let expected_hue: f32 =
            (100.0_f32 + 28.0_f32 * (2.0_f32 + 5.0_f32 / 30.0_f32)).rem_euclid(360.0_f32);
        assert!((late.base_hue() - expected_hue).abs() < 1e-5);
        let wrap = Journey {
            stage: 10,
            elapsed: 0.0,
        };
        assert!((wrap.base_hue() - 20.0).abs() < 1e-6);
        // The plateau never exceeds t = 20.
        let plateau = Journey {
            stage: MAX_STAGE,
            elapsed: 29.0,
        };
        assert!((plateau.t() - 20.0).abs() < 1e-6);
        assert!((plateau.difficulty() - 2.15).abs() < 1e-6);
    }

    /// The base hue: `(100 + 28 * stage) mod 360`.
    #[test]
    fn the_base_hue_walks_a_rainbow() {
        assert!((base_hue_for_stage(0) - 100.0).abs() < 1e-6);
        assert!((base_hue_for_stage(5) - 240.0).abs() < 1e-6);
        assert!((base_hue_for_stage(10) - 20.0).abs() < 1e-6);
        assert!((base_hue_for_stage(20) - 300.0).abs() < 1e-6);
    }

    /// The shape schedule: 6 → 4 → 3 → 5 → 6 → 7 → 4 → 8 → 5 → 6, with a
    /// two-stage dwell and the hexagon at both ends.
    #[test]
    fn the_shape_schedule_tours_three_to_eight_sides() {
        assert_eq!(sides_for_stage(0), 6);
        assert_eq!(sides_for_stage(1), 6);
        assert_eq!(sides_for_stage(2), 4);
        assert_eq!(sides_for_stage(4), 3);
        assert_eq!(sides_for_stage(6), 5);
        assert_eq!(sides_for_stage(8), 6);
        assert_eq!(sides_for_stage(10), 7);
        assert_eq!(sides_for_stage(12), 4);
        assert_eq!(sides_for_stage(14), 8);
        assert_eq!(sides_for_stage(16), 5);
        assert_eq!(sides_for_stage(18), 6);
        assert_eq!(sides_for_stage(MAX_STAGE), 6);
        for stage in 0..=MAX_STAGE {
            assert!((3..=8).contains(&sides_for_stage(stage)), "stage {stage}");
        }
        // The shape changes every two stages (the dwell), not every stage.
        for stage in (2..=MAX_STAGE).step_by(2) {
            assert_eq!(
                sides_for_stage(stage),
                sides_for_stage(stage + 1),
                "stage {stage} and {} share a shape",
                stage + 1
            );
        }
    }

    /// `advance` fires the stage boundary exactly at 30 s and holds the
    /// plateau at stage 20 (no further transitions; `elapsed` stays
    /// bounded).
    #[test]
    fn advance_crosses_the_stage_boundary_and_plateaus() {
        let mut j = Journey::new();
        assert!(j.advance(STAGE_DURATION_S - 1.0 / 64.0).is_none());
        assert_eq!(j.stage, 0);
        assert_eq!(j.advance(1.0 / 64.0), Some(1));
        assert_eq!(j.stage, 1);
        assert!((j.elapsed - 0.0).abs() < 1e-6);

        let mut j = Journey::new();
        assert_eq!(j.advance(STAGE_DURATION_S), Some(1));

        let mut j = Journey::new();
        for _ in 0..20 {
            j.advance(STAGE_DURATION_S).unwrap_or_default();
        }
        assert_eq!(j.stage, MAX_STAGE);
        assert!(j.advance(STAGE_DURATION_S).is_none());
        assert_eq!(j.stage, MAX_STAGE);
        assert!(j.elapsed < STAGE_DURATION_S);
        j.advance(STAGE_DURATION_S * 100.0);
        assert_eq!(j.stage, MAX_STAGE);
        assert!(j.elapsed < STAGE_DURATION_S);

        j.reset_elapsed();
        assert_eq!(j.elapsed, 0.0);
        assert_eq!(j.stage, MAX_STAGE);
    }
}
