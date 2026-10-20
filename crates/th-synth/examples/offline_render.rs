//! Offline render of the exact `MusicDirector` track setup — no audio
//! device needed: `Preset::build` + `master_bus` are public, and a fundsp
//! `Net` renders with plain `get_stereo()` calls. Prints per-track peaks
//! (each track soloed, then all together through the master bus) so the
//! level thief (preset internals vs gate vs master chain) is MEASURED,
//! not guessed.
//!
//!   cargo run -q -p th-synth --example offline_render
//!
//! NOTE: this mirrors `crates/th-synth/src/director.rs`'s SLOTS +
//! `apply_slot` values by hand (the director's own fields are private and
//! the values are simple constants — drift is possible, this example
//! prints the values it used so the mirror is visible).

use fundsp::hacker::*;

use rust_synth::audio::preset::{master_bus, GlobalParams, Preset, PresetKind};
use rust_synth::audio::track::TrackParams;
use rust_synth::math::rhythm::euclidean_bits;

fn track(kind: PresetKind, freq: f32, gain: f32, bits: u32) -> TrackParams {
    let p = TrackParams::default_for(freq);
    p.gain.set_value(gain);
    p.mute.set_value(0.0);
    p.arp.set_value(0.0);
    p.life_mod.set_value(1.0);
    p.pattern_bits
        .store(bits, std::sync::atomic::Ordering::Relaxed);
    p.pattern_hits.set_value(bits.count_ones() as f32);
    let _ = kind;
    p
}

fn render(net: &mut Net, secs: f64) -> (f32, f32) {
    let n = (48_000.0 * secs) as usize;
    let (mut peak, mut sum, mut count) = (0.0f32, 0.0f64, 0usize);
    for _ in 0..n {
        let (l, r) = net.get_stereo();
        peak = peak.max(l.abs()).max(r.abs());
        sum += (l * l + r * r) as f64;
        count += 2;
    }
    (
        peak,
        (sum / std::cmp::Ord::max(count, 1) as f64).sqrt() as f32,
    )
}

fn main() {
    use fundsp::hacker::*;
    let g = GlobalParams::default();
    g.bpm.set_value(150.0);
    g.brightness.set_value(0.45);
    g.scale_mode.set_value(0.0);
    g.master_gain.set_value(1.0);

    // The director's stage-1 slots (only unlock<=1 tracks matter; the pad
    // unlocks at 4, the bell at 2 — included muted/unlocked as labelled).
    let slots: [(PresetKind, f32, f32, u32, &str); 8] = [
        (
            PresetKind::Heartbeat,
            55.0,
            0.9,
            (1 << 0) | (1 << 7) | (1 << 10),
            "kick",
        ),
        (
            PresetKind::Heartbeat,
            176.0,
            0.6,
            (1 << 4) | (1 << 12),
            "snare",
        ),
        (
            PresetKind::Heartbeat,
            880.0,
            0.26,
            (1 << 2) | (1 << 6) | (1 << 10) | (1 << 14),
            "hat",
        ),
        (
            PresetKind::BassPulse,
            55.0,
            0.8,
            euclidean_bits(4, 0),
            "bass",
        ),
        (
            PresetKind::PluckSaw,
            220.0,
            0.5,
            euclidean_bits(4, 0),
            "lead",
        ),
        (PresetKind::PadZimmer, 110.0, 0.2, 0, "pad(locked)"),
        (PresetKind::Bell, 220.0, 0.2, 0, "bell(locked)"),
        (PresetKind::DroneSub, 27.5, 0.28, 0, "sub"),
    ];

    println!("== soloed tracks (10 s, mute 0, gain as configured) ==");
    for (kind, freq, gain, bits, name) in slots {
        let p = track(kind, freq, gain, bits);
        let mut net = Preset::build(kind, &p, &g);
        net.set_sample_rate(48_000.0);
        let (peak, rms) = render(&mut net, 10.0);
        println!("{name:>12}: gain {gain} peak {peak:.4} rms {rms:.4}");
    }

    println!("== all 8 summed through the master bus (10 s) ==");
    let mut sum: Option<Net> = None;
    for (kind, freq, gain, bits, name) in slots {
        let p = track(kind, freq, gain, bits);
        let node = Preset::build(kind, &p, &g);
        println!("   + {name}");
        sum = Some(match sum {
            Some(acc) => acc + node,
            None => node,
        });
    }
    let mut net = sum.expect("8 tracks") >> master_bus(g.brightness.clone());
    net.set_sample_rate(48_000.0);
    // the master gain is applied OUTSIDE the bus (the stream callback):
    let (peak, rms) = render(&mut net, 10.0);
    println!("bus: peak {peak:.4} rms {rms:.4} (x master_gain 1.0)");
}
