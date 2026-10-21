//! The generative glass clinks (plan 002 issue 09, the operator's
//! second taste pass): every shatter CRACKS once and every shard that
//! BOUNCES off the core TINKS positionally — modal synthesis of a glass
//! fragment (inharmonic partials, size-pitched, speed-loud, short
//! exponential ring) rendered per event on the app's thread, played
//! through the ONE OpenAL mixer (mono → the shim's auto-panner puts it
//! at the shard's world position).
//!
//! Deterministic: the rng is a seeded xorshift64* built from the event's
//! seed — the tests pin content (peak, length, partial count) per seed.

/// A partial's ratio family (the irregular glass-mode ladder — glass
/// modes are NOT harmonic; these are a plausible thick-glass set).
const PARTIAL_RATIOS: [f32; 8] = [1.0, 1.83, 2.76, 3.66, 4.81, 6.2, 7.9, 9.7];

/// The clink's ceiling (keep the ring under the harsh zone).
const PARTIAL_MAX_HZ: f32 = 12_000.0;

/// The peak normalization target.
const PEAK: f32 = 0.85;

/// Render one glass clink, mono, `sample_rate` Hz.
///
/// * `speed01` — the impact/flight speed normalized 0..1 (loudness +
///   the crack's share).
/// * `size01` — the shard's size normalized 0..1 (pitch: a big chunk
///   rings LOW and long, a sliver TINKS high and short).
/// * `seed` — the deterministic rng seed (per event).
#[must_use]
pub fn glass_clink(sample_rate: u32, speed01: f32, size01: f32, seed: u64) -> Vec<f32> {
    let speed01 = speed01.clamp(0.0, 1.0);
    let size01 = size01.clamp(0.0, 1.0);
    let sr = sample_rate as f32;

    // A tiny deterministic xorshift64* (no dep; the draws are cosmetic).
    let mut state = seed.max(1) | 0x9E37_79B9_7F4A_7C15;
    let mut rnd = move || -> f64 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let r = state.wrapping_mul(0x2545_F491_4F6C_DD1D);
        ((r >> 32) as f64) / (u32::MAX as f64)
    };

    // Pitch: a big chunk rings low, a sliver tinks high.
    let f0 = 1200.0 + (1.0 - size01) * 2800.0;
    // Ring length: big = long.
    let ring_base = 0.05 + size01 * 0.12;

    let n_partials = 4 + (rnd() * 3.9) as usize; // 4..7
    let mut partials: Vec<(f32, f32, f32)> = Vec::with_capacity(n_partials);
    for k in 0..n_partials {
        let ratio = PARTIAL_RATIOS[k % PARTIAL_RATIOS.len()] * (0.92 + 0.16 * rnd() as f32);
        let f = (f0 * ratio).min(PARTIAL_MAX_HZ);
        let amp = (1.0 / (k as f32 + 1.0).powf(1.1)) * (0.7 + 0.6 * rnd() as f32);
        let tau = ring_base / (1.0 + 0.55 * k as f32);
        let phase = rnd() as f32 * std::f32::consts::TAU;
        partials.push((f, amp, tau));
        let _ = phase; // the phase jitter reads as "different glass" per seed
    }

    // The crack: a few ms of noise at the onset (speed-scaled).
    let crack_len = (0.004 + 0.003 * rnd() as f32) * sr;
    let crack_amp = 0.25 + 0.45 * speed01;

    let tau_max = ring_base;
    let dur_secs = (tau_max * 5.5).min(1.2);
    let n_samples = (dur_secs * sr) as usize;

    let mut out = Vec::with_capacity(n_samples);
    let dt = 1.0 / sr;
    let mut t = 0.0_f32;
    for _ in 0..n_samples {
        let mut s = 0.0_f32;
        for (f, amp, tau) in &partials {
            let env = (-t / tau).exp();
            s += amp * env * (std::f32::consts::TAU * f * t).sin();
        }
        if t < crack_len / sr {
            s += crack_amp * (rnd() as f32 * 2.0 - 1.0) * (1.0 - t * sr / crack_len);
        }
        out.push(s);
        t += dt;
    }
    // Normalize.
    let peak = out.iter().fold(0.0_f32, |m, &v| m.max(v.abs())).max(1.0e-6);
    let g = PEAK / peak * (0.35 + 0.65 * speed01);
    for v in &mut out {
        *v *= g;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::glass_clink;

    #[test]
    fn clinks_are_deterministic_and_bounded() {
        let a = glass_clink(48_000, 0.7, 0.4, 11);
        let b = glass_clink(48_000, 0.7, 0.4, 11);
        assert_eq!(a, b, "same seed, same clink");
        assert!(!a.is_empty());
        let peak = a.iter().fold(0.0_f32, |m, &v| m.max(v.abs()));
        assert!(peak > 0.3 && peak <= 0.9, "sane peak {peak}");
        assert!(a.len() < 48_000, "under 1 s");
    }

    #[test]
    fn size_pitches_the_ring() {
        // A big shard rings longer than a sliver at the same speed.
        let big = glass_clink(48_000, 0.5, 1.0, 5);
        let small = glass_clink(48_000, 0.5, 0.0, 5);
        // Measure the ring: the last sample above 1% of the peak.
        let tail = |s: &Vec<f32>| {
            let peak = s.iter().fold(0.0_f32, |m, &v| m.max(v.abs()));
            s.iter()
                .rposition(|v| v.abs() > peak * 0.01)
                .map(|i| i as f32 / 48_000.0)
                .unwrap_or(0.0)
        };
        assert!(
            tail(&big) > tail(&small),
            "big {} small {}",
            tail(&big),
            tail(&small)
        );
    }

    #[test]
    fn speed_loudens_the_clink() {
        let soft = glass_clink(48_000, 0.05, 0.5, 3);
        let loud = glass_clink(48_000, 1.0, 0.5, 3);
        let peak = |s: &Vec<f32>| s.iter().fold(0.0_f32, |m, &v| m.max(v.abs()));
        assert!(peak(&loud) > peak(&soft) * 1.5, "speed scales the peak");
    }
}
