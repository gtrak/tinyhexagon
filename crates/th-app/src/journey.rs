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
//! and reaches 1.55× at the plateau, then holds (the 09 operator
//! passes slowed the cursor: the ramp and the level `speedMult` both
//! came down).

/// Seconds per stage (the contract: 30 s × 20 transitions = 10 minutes to
/// the plateau).
pub const STAGE_DURATION_S: f32 = 30.0;

/// The plateau stage (stages are 0..=20; 20 transitions).
pub const MAX_STAGE: u32 = 20;

/// The single level id (the 09-taste continuous journey). The arena's
/// **style and side count** now come from the upstream levels, in order
/// (the 09-taste-2 pass): the 9 `cube` levels by menu priority, then the
/// first selectable `hypercube` level. The journey level only owns the
/// walls; the app swaps `StyleData`/sides without `new_game`.
pub const LEVEL: &str = "journey";

/// The upstream packs whose styles/shapes the journey borrows.
pub const CUBE_PACK: &str = "ohvrvanilla_vittorio_romeo_cube_1";
pub const HYPERCUBE_PACK: &str = "ohvrvanilla_vittorio_romeo_hypercube_1";

/// One upstream level borrowed for two stages: its pack + style id, and
/// the side count its `onInit` authors (`l_setSides`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    pub pack: &'static str,
    pub style: &'static str,
    pub sides: u32,
}

/// The 10 blocks, in the upstream menu order: the cube levels
/// `pointless → flatteringshape → seconddimension → apeirogon →
/// commando → euclideanpc → pi → labyrinth → goldenratio`, then the
/// hypercube's first selectable (`acceleradiant`). `pi` is the 24-sided
/// circle-like arena; `labyrinth` is the 3-sided one.
pub const BLOCKS: [Block; 10] = [
    Block {
        pack: CUBE_PACK,
        style: "pointless",
        sides: 6,
    },
    Block {
        pack: CUBE_PACK,
        style: "flatteringshape",
        sides: 6,
    },
    Block {
        pack: CUBE_PACK,
        style: "seconddimension",
        sides: 6,
    },
    Block {
        pack: CUBE_PACK,
        style: "apeirogon",
        sides: 6,
    },
    Block {
        pack: CUBE_PACK,
        style: "commando",
        sides: 4,
    },
    Block {
        pack: CUBE_PACK,
        style: "euclideanpc",
        sides: 8,
    },
    Block {
        pack: CUBE_PACK,
        style: "pi",
        sides: 24,
    },
    Block {
        pack: CUBE_PACK,
        style: "labyrinth",
        sides: 3,
    },
    Block {
        pack: CUBE_PACK,
        style: "goldenratio",
        sides: 6,
    },
    Block {
        pack: HYPERCUBE_PACK,
        style: "acceleradiant",
        sides: 6,
    },
];

/// The block a stage belongs to (two stages per upstream level; the
/// plateau holds the last block).
#[must_use]
pub fn block_for_stage(stage: u32) -> &'static Block {
    &BLOCKS[((stage / 2).min((BLOCKS.len() - 1) as u32)) as usize]
}

/// `difficulty = 0.45 + 0.055 * min(t, 20)` (the arc 0.45..=1.55; the
/// engine's `set_difficulty_mult` clamps 0.1..=4.0, so every value lands).
#[must_use]
pub fn difficulty_for_stage(stage: u32) -> f32 {
    0.45 + 0.055 * stage.min(MAX_STAGE) as f32
}

/// `bpm = 150 + 1.2 * min(stage, 20)` (cap 174 — the chiptune drum-and-bass
/// plateau; `th_synth`'s `director::stage_params` owns the live mix).
#[must_use]
pub fn bpm_for_stage(stage: u32) -> f32 {
    150.0 + 1.2 * stage.min(MAX_STAGE) as f32
}

/// The level id at `stage`: always the single journey level.
#[must_use]
pub fn level_id_for_stage(_stage: u32, pack: &str) -> String {
    format!("{pack}_{LEVEL}")
}

/// The arena's side count at `stage` (borrowed from the upstream block).
#[must_use]
pub fn sides_for_stage(stage: u32) -> u32 {
    block_for_stage(stage).sides
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
        0.45 + 0.055 * self.t()
    }

    /// The level id the run plays (the single journey level).
    #[must_use]
    pub fn level_id(&self, pack: &str) -> String {
        level_id_for_stage(self.stage, pack)
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
        assert!((difficulty_for_stage(MAX_STAGE) - 1.55).abs() < 1e-6);
        assert!((bpm_for_stage(0) - 150.0).abs() < 1e-6);
        assert!((bpm_for_stage(MAX_STAGE) - 174.0).abs() < 1e-6);
        for beyond in (MAX_STAGE + 1)..=64 {
            assert_eq!(
                difficulty_for_stage(beyond),
                difficulty_for_stage(MAX_STAGE)
            );
            assert_eq!(bpm_for_stage(beyond), bpm_for_stage(MAX_STAGE));
            assert_eq!(sides_for_stage(beyond), sides_for_stage(MAX_STAGE));
        }
        assert!(difficulty_for_stage(MAX_STAGE) / difficulty_for_stage(0) > 3.0);
        assert!(bpm_for_stage(MAX_STAGE) - bpm_for_stage(0) >= 24.0);
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
    /// stage too (the per-frame `set_difficulty_mult`).
    #[test]
    fn the_ramp_is_continuous_within_a_stage() {
        let start = Journey::new();
        let mut mid = start;
        mid.advance(15.0);
        assert_eq!(mid.stage, 0);
        assert!(mid.difficulty() > start.difficulty());
        assert!((mid.difficulty() - (0.45 + 0.055 * 0.5)).abs() < 1e-6);
        let late = Journey {
            stage: 2,
            elapsed: 5.0,
        };
        assert!((late.t() - 2.0 - 5.0 / 30.0).abs() < 1e-5);
        assert!((late.difficulty() - (0.45 + 0.055 * (2.0 + 5.0 / 30.0))).abs() < 1e-5);
        let plateau = Journey {
            stage: MAX_STAGE,
            elapsed: 29.0,
        };
        assert!((plateau.t() - 20.0).abs() < 1e-6);
        assert!((plateau.difficulty() - 1.55).abs() < 1e-6);
    }

    /// The arena style/shape table: the upstream cube levels in menu order,
    /// then the hypercube's first selectable — with their authored sides.
    #[test]
    fn the_shape_schedule_borrows_the_upstream_levels_in_order() {
        assert_eq!(BLOCKS.len(), 10);
        assert_eq!(block_for_stage(0).style, "pointless");
        assert_eq!(block_for_stage(2).style, "flatteringshape");
        assert_eq!(block_for_stage(4).style, "seconddimension");
        assert_eq!(block_for_stage(6).style, "apeirogon");
        assert_eq!(block_for_stage(8).style, "commando");
        assert_eq!(block_for_stage(10).style, "euclideanpc");
        assert_eq!(block_for_stage(12).style, "pi");
        assert_eq!(block_for_stage(14).style, "labyrinth");
        assert_eq!(block_for_stage(16).style, "goldenratio");
        assert_eq!(block_for_stage(18).style, "acceleradiant");
        assert_eq!(block_for_stage(MAX_STAGE).style, "acceleradiant");
        assert_eq!(block_for_stage(12).pack, CUBE_PACK);
        assert_eq!(block_for_stage(18).pack, HYPERCUBE_PACK);
        // The authored side counts: 6,6,6,6,4,8,24,3,6,6.
        assert_eq!(
            BLOCKS.iter().map(|b| b.sides).collect::<Vec<_>>(),
            vec![6, 6, 6, 6, 4, 8, 24, 3, 6, 6]
        );
        assert_eq!(sides_for_stage(0), 6);
        assert_eq!(sides_for_stage(1), 6);
        assert_eq!(sides_for_stage(8), 4);
        assert_eq!(sides_for_stage(10), 8);
        assert_eq!(sides_for_stage(12), 24);
        assert_eq!(sides_for_stage(14), 3);
        assert_eq!(sides_for_stage(20), 6);
        for stage in 0..=MAX_STAGE {
            assert!((3..=32).contains(&sides_for_stage(stage)), "stage {stage}");
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
