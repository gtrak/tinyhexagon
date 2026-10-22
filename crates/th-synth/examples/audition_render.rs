//! The offline WAV audition renderer (plan 002 issue 11c — the
//! deliverable that prevents another un-auditioned kit).
//!
//! Renders the REAL director setup — `MusicDirector::new_fallback()`'s
//! 8 tracks: the shipped `Score::default()`'s kinds, the `apply_slot`
//! per-kind params AND the role-based `pattern_bits` — through the SAME
//! graph the engine builds (each slot's `Preset::build` summed through
//! `master_bus`), for 4 bars at the shipped bpm, 48 kHz stereo, 16-bit
//! PCM. `hound` is NOT available (th-synth has no audio-encoding dep),
//! so the 44-byte WAV header is written by hand (no new dependency).
//!
//! Prints the values it used (the director's real `TrackSnapshot`s —
//! the renderer reads them through `MusicDirector::tracks()`, it does
//! NOT mirror `apply_slot`'s numbers by hand) and a per-slot table:
//! peak / RMS with each slot SOLOED, a coarse brightness proxy, and the
//! mix's peak — so a silent, clipped or indistinguishable voice is a
//! NUMBER before it ships:
//!
//!   cargo run -q -p th-synth --example audition_render
//!   cargo run -q -p th-synth --example audition_render -- /tmp/audition.wav
//!
//! The optional first argument sets the output path (default
//! `audition.wav`).

use std::env;
use std::fs;
use std::path::Path;

use fundsp::hacker::*;

use rust_synth::audio::preset::{master_bus, GlobalParams, Preset};
use rust_synth::audio::Track;
use th_synth::director::{scale_mode_for_stage, MusicDirector, MASTER_GAIN};

const SR: f32 = 48_000.0;

fn main() {
    // argv: [out.wav] [--solo-dir DIR]
    let args: Vec<String> = env::args().skip(1).collect();
    let solo_dir: Option<String> = args
        .iter()
        .position(|a| a == "--solo-dir")
        .and_then(|i| args.get(i + 1).cloned());
    let out = args
        .iter()
        .find(|a| !a.starts_with("--") && Some(*a) != solo_dir.as_ref())
        .cloned()
        .unwrap_or_else(|| "audition.wav".to_string());

    // The REAL director setup (stage 0, the shipped score).
    let d = MusicDirector::new_fallback();
    let tracks = d.tracks();
    let score = d.score();
    let p = score.stage_params(0);

    let g = GlobalParams::default();
    g.bpm.set_value(p.bpm);
    g.brightness.set_value(p.brightness);
    g.scale_mode.set_value(scale_mode_for_stage(0));
    g.master_gain.set_value(MASTER_GAIN);

    let bars = 4.0;
    let secs = bars * 4.0 * 60.0 / p.bpm as f64;
    // The frequency closures' `follow(0.08)` means a FRESH graph takes
    // ~0.3 s (≈3× the response time) for the carriers to spin up from
    // 0 Hz. Live, the graph runs continuously so this never happens;
    // offline, render the flush and measure/write the settled tail.
    const SPINUP_S: f64 = 0.3;

    println!(
        "audition: {} slots, {} bars @ {} bpm ({} s @ 48 kHz, +{SPINUP_S:.1} s spin-up flush) -> {out}",
        tracks.len(),
        bars,
        p.bpm,
        secs
    );
    println!("values used (the director's REAL tracks — read, not mirrored):");
    for t in tracks {
        let s = t.params.snapshot();
        println!(
            "  {:<10} {:<10} freq {:7.2} gain {:4.2} char {:4.2} res {:4.2} cut {:6.0} det {:4.0} rev {:4.2} bits {:#018b}",
            t.name,
            t.kind.label(),
            s.freq,
            s.gain,
            s.character,
            s.resonance,
            s.cutoff,
            s.detune,
            s.reverb_mix,
            s.pattern_bits
        );
    }

    // ── The per-slot table: each slot soloed (the others muted — the
    // fresh graph per pass keeps the reverb tails from cross-contaminating
    // the measurement). `Wave::render` is fundsp's block (SIMD) offline
    // renderer — the per-sample `get_stereo()` tick would be 10×+ slower
    // for these long-reverb graphs.
    let mut solo: Vec<(String, f32, f32, f64)> = Vec::new();
    for i in 0..tracks.len() {
        for (j, t) in tracks.iter().enumerate() {
            t.params.mute.set_value(if j == i { 0.0 } else { 1.0 });
        }
        let mut net = build_sum(tracks, &g);
        let wave = Wave::render(SR as f64, SPINUP_S + secs, &mut net);
        let drop = (SPINUP_S * SR as f64) as usize;
        let left: Vec<f32> = wave.channel(0)[drop..].to_vec();
        let right: Vec<f32> = wave.channel(1)[drop..].to_vec();
        let m = metrics(&left, &right);
        solo.push((tracks[i].name.clone(), m.peak, m.rms, m.bright));
        // Also dump each soloed slot beside the mix WAV when asked
        // (`--solo-dir DIR`): the per-slot table is a SUMMARY, and a
        // summary cannot settle "is this voice actually FM?" — the
        // spectrum of the soloed signal can. Naming is index-prefixed so
        // the files sort in engine order.
        if let Some(dir) = &solo_dir {
            let name = format!("{dir}/{i}-{}.wav", tracks[i].name.replace(' ', "_"));
            if let Err(err) = fs::write(Path::new(&name), wav_bytes(&left, &right)) {
                eprintln!("audition: solo dump {name}: {err}");
            }
        }
    }

    // ── The mix (all 8 live) — this is the WAV's content.
    for t in tracks {
        t.params.mute.set_value(0.0);
    }
    let mut net = build_sum(tracks, &g);
    let wave = Wave::render(SR as f64, SPINUP_S + secs, &mut net);
    let drop = (SPINUP_S * SR as f64) as usize;
    let left: Vec<f32> = wave.channel(0)[drop..].to_vec();
    let right: Vec<f32> = wave.channel(1)[drop..].to_vec();
    let mix = metrics(&left, &right);

    println!("\n== per-slot (soloed, {secs:.2} s) + mix ==");
    println!(
        "{:<10} {:>10} {:>10} {:>12}",
        "slot", "peak", "rms", "bright (4k+)"
    );
    for (name, peak_s, rms_s, bright_s) in &solo {
        println!("{name:<10} {peak_s:>10.4} {rms_s:>10.4} {bright_s:>12.4}");
    }
    println!(
        "{:<10} {:>10.4} {:>10.4} {:>12.4}",
        "mix", mix.peak, mix.rms, mix.bright
    );

    fs::write(Path::new(&out), wav_bytes(&left, &right))
        .unwrap_or_else(|err| panic!("write {out}: {err}"));
    println!(
        "\nwrote {out} ({} bytes, 16-bit stereo 48 kHz)",
        44 + left.len() * 4
    );
}

/// The SAME graph the director builds: each slot's `Preset::build`
/// (the slot's resolved kind + its `TrackParams` — the `Shared`
/// atomics the live graph reads) summed through the master bus.
fn build_sum(tracks: &[Track], g: &GlobalParams) -> Net {
    let mut sum: Option<Net> = None;
    for t in tracks {
        let node = Preset::build(t.kind, &t.params, g);
        sum = Some(match sum {
            Some(acc) => acc + node,
            None => node,
        });
    }
    sum.expect("8 slots") >> master_bus(g.brightness.clone())
}

struct Metrics {
    peak: f32,
    rms: f32,
    bright: f64,
}

/// Peak / RMS + a coarse brightness proxy (the one-pole highpassed
/// 4 kHz energy fraction — the "indistinguishable from the sub it
/// should complement" detector: a voice that is loud but entirely dull
/// shows a ~0.0 bright).
fn metrics(left: &[f32], right: &[f32]) -> Metrics {
    let n = left.len();
    let mut peak = 0.0f32;
    let mut sum_sq = 0.0f64;
    let mut hf = Highpass::default();
    for (l, r) in left.iter().zip(right) {
        peak = peak.max(l.abs()).max(r.abs());
        sum_sq += (l * l + r * r) as f64;
        hf.step(0, *l);
        hf.step(1, *r);
    }
    Metrics {
        peak,
        rms: (sum_sq / n as f64 / 2.0).sqrt() as f32,
        bright: hf.energy / sum_sq.max(1e-12),
    }
}

/// A one-pole high-pass (4 kHz at 48 kHz) energy accumulator — the
/// brightness proxy (dependency-free, ~3 flops per sample per channel).
/// Both channels are filtered (L and R are independent filters) and the
/// energies summed.
const HIGHPASS_ALPHA: f32 = 0.4077; // 1 − exp(−2π·4 kHz / 48 kHz)

#[derive(Default)]
struct Highpass {
    prev: [f32; 2],
    state: [f32; 2],
    energy: f64,
}

impl Highpass {
    /// One sample through channel `ch`'s (0 = L, 1 = R) filter.
    fn step(&mut self, ch: usize, x: f32) {
        let y = x - self.prev[ch] + (1.0 - HIGHPASS_ALPHA) * self.state[ch];
        self.prev[ch] = x;
        self.state[ch] = y;
        self.energy += (y * y) as f64;
    }
}

/// The 44-byte WAV header + the interleaved 16-bit PCM (the `hound`-free
/// half of the renderer — no audio-encoding dependency).
fn wav_bytes(left: &[f32], right: &[f32]) -> Vec<u8> {
    let n = left.len();
    let data_len = (n * 2) as u32 * 2; // interleaved L R, 16-bit each
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36u32 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVE");
    b.extend_from_slice(b"fmt ");
    b.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&2u16.to_le_bytes()); // stereo
    b.extend_from_slice(&(SR as u32).to_le_bytes());
    b.extend_from_slice(&(SR as u32 * 4).to_le_bytes()); // byte rate
    b.extend_from_slice(&4u16.to_le_bytes()); // block align
    b.extend_from_slice(&16u16.to_le_bytes()); // bits
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..n {
        b.extend_from_slice(&clamp16(left[i]).to_le_bytes());
        b.extend_from_slice(&clamp16(right[i]).to_le_bytes());
    }
    b
}

fn clamp16(v: f32) -> i16 {
    (v.clamp(-1.0, 1.0) * 32_767.0) as i16
}
