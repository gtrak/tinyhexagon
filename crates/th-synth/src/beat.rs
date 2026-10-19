//! The `BeatView` the visuals consume (05-synth.md "Beat view"), plus the
//! pure phase/energy math it is built from.
//!
//! The phase functions delegate to `rust_synth::math::pulse` (f64 — the
//! crate keeps its time in f64 so multi-hour playback stays phase-stable);
//! the f32 wrappers here exist only for the `BeatView` surface.

/// One frame of beat data for the visuals (05-synth.md "Beat view").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeatView {
    /// 0..1 within the current beat (`pulse::beat_phase`).
    pub beat_phase: f32,
    /// 0..1 within the current 4-beat bar (`pulse::phrase_phase(t, bpm, 4)`).
    pub bar_phase: f32,
    /// 0..1 — `clamp(max(peak_l, peak_r) * 3.0)`, the live stream's master
    /// envelope follower. 0.0 in fallback mode (no audio to measure).
    pub energy: f32,
    /// The last stage set via `MusicDirector::set_stage`.
    pub stage: u32,
    /// True when the audio stream is live; false on the fallback clock.
    pub playing: bool,
}

/// `rust_synth::math::pulse::beat_phase(t, bpm)` as f32 (0..1 within the beat).
#[inline]
pub fn beat_phase(t: f32, bpm: f32) -> f32 {
    rust_synth::math::pulse::beat_phase(t as f64, bpm as f64) as f32
}

/// `rust_synth::math::pulse::phrase_phase(t, bpm, 4)` as f32
/// (0..1 within the 4-beat bar).
#[inline]
pub fn bar_phase(t: f32, bpm: f32) -> f32 {
    rust_synth::math::pulse::phrase_phase(t as f64, bpm as f64, 4.0) as f32
}

/// The contract energy expression: `clamp(max(peak_l, peak_r) * 3.0, 0, 1)`.
#[inline]
pub fn energy(peak_l: f32, peak_r: f32) -> f32 {
    (peak_l.max(peak_r) * 3.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_is_the_contract_expression() {
        assert!((energy(0.1, 0.2) - 0.6).abs() < 1e-6);
        assert_eq!(energy(0.4, 0.1), 1.0); // clamped
        assert_eq!(energy(0.0, 0.0), 0.0);
    }

    #[test]
    fn beat_phase_wraps_per_beat() {
        // 120 bpm → 0.5 s beats.
        assert!(beat_phase(0.0, 120.0).abs() < 1e-9);
        assert!((beat_phase(0.25, 120.0) - 0.5).abs() < 1e-6);
        assert!(beat_phase(0.5, 120.0).abs() < 1e-6);
    }

    #[test]
    fn bar_phase_is_four_beats() {
        // 120 bpm → bar = 2 s.
        assert!(bar_phase(0.0, 120.0).abs() < 1e-9);
        assert!((bar_phase(1.0, 120.0) - 0.5).abs() < 1e-6);
        assert!(bar_phase(2.0, 120.0).abs() < 1e-6);
    }
}
