//! plan 002 issue 07 — the tiny journey: the 20-stage arc (20 × 30 s = a
//! 10-minute run) and its per-stage difficulty / BPM / level / hue / music
//! layer table.
//!
//! The table drives the engine's `new_game` / `set_difficulty_mult` /
//! `apply_music_beat` and `th_synth`'s `MusicDirector::set_stage` — the
//! engine knows nothing about the journey (04a: "the app owns the
//! 0.55..=1.15 arc"). After stage 20 the journey holds the plateau: no
//! further difficulty / tempo / hue / layer increase (the C++'s
//! `min(stage, 20)` cap on every ramp).

/// Seconds per stage (the contract: 30 s × 20 transitions = 10 minutes to
/// the plateau).
pub const STAGE_DURATION_S: f32 = 30.0;

/// The plateau stage (stages are 0..=20; 20 transitions).
pub const MAX_STAGE: u32 = 20;

/// The three level ids in stage order (06's pack), one level per 2
/// stages, cycling every 6: `meadow`, `river`, `stars`, `meadow`, …
pub const LEVELS: [&str; 3] = ["meadow", "river", "stars"];

/// The style's `hue_min` per level (06's hue families: meadow 100–160,
/// river 190–250, stars 270–330).
pub const HUE_MIN: [f32; 3] = [100.0, 190.0, 270.0];

/// `difficulty = 0.55 + 0.03 * min(stage, 20)` (the arc 0.55..=1.15; the
/// engine's `set_difficulty_mult` clamps 0.1..=4.0, so every value lands).
#[must_use]
pub fn difficulty_for_stage(stage: u32) -> f32 {
    0.55 + 0.03 * stage.min(MAX_STAGE) as f32
}

/// `bpm = 70 + 1.5 * min(stage, 20)` (cap 100 — `th_synth`'s
/// `director::stage_params` owns the live mix; this is the journey's
/// mirror for the table's tests / the report).
#[must_use]
pub fn bpm_for_stage(stage: u32) -> f32 {
    70.0 + 1.5 * stage.min(MAX_STAGE) as f32
}

/// The level index at `stage` (0 = meadow, 1 = river, 2 = stars): one
/// level per 2 stages, cycling every 6.
#[must_use]
pub fn level_index_for_stage(stage: u32) -> usize {
    ((stage / 2) % 3) as usize
}

/// The level id at `stage` (`{pack}_{level}`).
#[must_use]
pub fn level_id_for_stage(stage: u32, pack: &str) -> String {
    format!("{pack}_{}", LEVELS[level_index_for_stage(stage)])
}

/// The base hue = `style.hue_min + 47° * (stage / 6)`, capped at the
/// plateau (the contract): the app's per-stage base hue that the engine's
/// `apply_music_beat` shimmers ±1.5° over and clamps into the style's hue
/// range (04b).
#[must_use]
pub fn base_hue_for_stage(stage: u32) -> f32 {
    let s = stage.min(MAX_STAGE);
    HUE_MIN[level_index_for_stage(s)] + 47.0 * (s / 6) as f32
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

    /// The multiplier the stage plays with (the engine's `new_game`
    /// argument / `set_difficulty_mult` value).
    #[must_use]
    pub fn difficulty(&self) -> f32 {
        difficulty_for_stage(self.stage)
    }

    /// The level id the stage plays (06's pack).
    #[must_use]
    pub fn level_id(&self, pack: &str) -> String {
        level_id_for_stage(self.stage, pack)
    }

    /// The stage's base hue (the `apply_music_beat` argument).
    #[must_use]
    pub fn base_hue(&self) -> f32 {
        base_hue_for_stage(self.stage)
    }

    /// Advance the clock by `dt` seconds (the frame's real time — the
    /// bump slow-motion does NOT delay the journey). Returns `Some(stage)`
    /// when a stage boundary crossed (at most one per call — the frame's
    /// clamped dt can never span two 30-s stages); the caller performs the
    /// stage-up work (`new_game` / `set_difficulty_mult`, the music stage,
    /// the SFX).
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

    /// The contract ramps (difficulty / bpm) are monotonic and capped at
    /// stage 20 (the plateau: nothing beyond 20 shifts).
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
        assert!((difficulty_for_stage(0) - 0.55).abs() < 1e-6);
        assert!((difficulty_for_stage(MAX_STAGE) - 1.15).abs() < 1e-6);
        assert!((bpm_for_stage(0) - 70.0).abs() < 1e-6);
        assert!((bpm_for_stage(MAX_STAGE) - 100.0).abs() < 1e-6);
        for beyond in (MAX_STAGE + 1)..=64 {
            assert_eq!(
                difficulty_for_stage(beyond),
                difficulty_for_stage(MAX_STAGE)
            );
            assert_eq!(bpm_for_stage(beyond), bpm_for_stage(MAX_STAGE));
            assert_eq!(base_hue_for_stage(beyond), base_hue_for_stage(MAX_STAGE));
        }
    }

    /// The level cycle: one level per 2 stages, `meadow` / `river` /
    /// `stars`, cycling every 6 stages (the 06 pack ids).
    #[test]
    fn the_level_sequence_cycles_meadow_river_stars() {
        let expected = [
            "meadow", "meadow", "river", "river", "stars", "stars", "meadow", "meadow", "river",
            "river", "stars", "stars", "meadow",
        ];
        for (stage, level) in expected.into_iter().enumerate() {
            assert_eq!(
                level_id_for_stage(stage as u32, "pack"),
                format!("pack_{level}"),
                "stage {stage}"
            );
        }
        // Past the plateau the level keeps its stage-20 identity (the
        // plateau: no further level change).
        assert_eq!(
            level_index_for_stage(MAX_STAGE),
            level_index_for_stage(20_000)
        );
    }

    /// The base hue: `hue_min + 47° * (stage / 6)` on the stage's own
    /// level's hue family (06's `hue_min` values).
    #[test]
    fn the_base_hue_walks_47_degrees_per_six_stages() {
        // Stage 0 = meadow @ hue_min 100.
        assert!((base_hue_for_stage(0) - 100.0).abs() < 1e-6);
        // Stage 5 is still meadow (stages 0-1 meadow, 2-3 river, 4-5
        // stars — 5/2 = 2 → stars @ 270).
        assert!((base_hue_for_stage(5) - 270.0).abs() < 1e-6);
        // Stage 6 = meadow again, +47° (6/6 = 1).
        assert!((base_hue_for_stage(6) - 147.0).abs() < 1e-6);
        // Stage 18 = meadow, +47° × 3 (18/6 = 3).
        assert!((base_hue_for_stage(18) - 241.0).abs() < 1e-6);
        // Stage 20 = river (20/2 % 3 = 1), +47° × 3 (20/6 = 3).
        assert!((base_hue_for_stage(20) - 331.0).abs() < 1e-6);
    }

    /// `advance` fires the stage boundary exactly at 30 s and holds the
    /// plateau at stage 20 (no further transitions; `elapsed` stays
    /// bounded).
    #[test]
    fn advance_crosses_the_stage_boundary_and_plateaus() {
        let mut j = Journey::new();
        // Just short of the boundary (f32-exact steps — 1/64 s): no
        // transition.
        assert!(j.advance(STAGE_DURATION_S - 1.0 / 64.0).is_none());
        assert_eq!(j.stage, 0);
        // The last 1/64 s crosses it exactly (the sums are f32-exact).
        assert_eq!(j.advance(1.0 / 64.0), Some(1));
        assert_eq!(j.stage, 1);
        assert!((j.elapsed - 0.0).abs() < 1e-6);

        // A whole stage in one call (the clamped frame dt can do this).
        let mut j = Journey::new();
        assert_eq!(j.advance(STAGE_DURATION_S), Some(1));

        // Run to the plateau: 20 transitions of 30 s each.
        let mut j = Journey::new();
        for _ in 0..20 {
            j.advance(STAGE_DURATION_S).unwrap_or_default();
        }
        assert_eq!(j.stage, MAX_STAGE);
        // The plateau: further time elapses, no transition, `elapsed`
        // stays bounded.
        assert!(j.advance(STAGE_DURATION_S).is_none());
        assert_eq!(j.stage, MAX_STAGE);
        assert!(j.elapsed < STAGE_DURATION_S);
        j.advance(STAGE_DURATION_S * 100.0);
        assert_eq!(j.stage, MAX_STAGE);
        assert!(j.elapsed < STAGE_DURATION_S);

        // A manual reset restarts the stage clock, not the stage.
        j.reset_elapsed();
        assert_eq!(j.elapsed, 0.0);
        assert_eq!(j.stage, MAX_STAGE);
    }
}
