//! The offline WAV audition renderer (plan 002 issue 11c; extended by
//! 11g to the PHRASE).
//!
//! Renders the REAL director setup — `MusicDirector::new_fallback()`'s
//! 8 tracks: the shipped `Score::default()`'s kinds, the `apply_slot`
//! per-kind params AND the role-based `pattern_bits` — through the
//! SAME graph the engine builds (each slot's `Preset::build` summed
//! through `master_bus`), for the WHOLE PHRASE (the resolved
//! `score.bars.len()` bars; default 4 when the score carries no
//! phrase; `--bars N` overrides), 48 kHz stereo, 16-bit PCM. `hound`
//! is NOT available (th-synth has no audio-encoding dep), so the
//! 44-byte WAV header is written by hand (no new dependency).
//!
//! Issue 11g: the render is DIRECTOR-DRIVEN — the fallback director
//! ticks sample by sample while the offline graphs run, so its
//! `sequence` rewrites the tracks' `Shared` params (the SAME atomics
//! the live graph reads): the resolved per-bar bass/lead/comp
//! figures, the per-bar `pattern_bits` refresh and the absolute-bar
//! chord offsets all actually vary inside the WAV (the pre-11g
//! renderer rendered 4 bars as ONE static setup — a phrase would have
//! sounded like one bar).
//!
//! Prints the values it used (the director's real `TrackSnapshot`s —
//! the renderer reads them through `MusicDirector::tracks()`, it does
//! NOT mirror `apply_slot`'s numbers by hand), a per-bar marker line
//! naming the bar index and which figure each part used (so a
//! listener can follow the form), and a per-slot table: peak / RMS
//! with each slot SOLOED (each solo pass runs with every voice
//! UNLOCKED — stage = the max slot's unlock — so a stage-locked voice
//! still gets a measured row; the mix stays the shipped stage-0 mix),
//! a coarse brightness proxy, the ATTACK/TAIL ratio (issue 11e's
//! permanent guard — see `attack_tail`), and the mix's peak — so a
//! silent, clipped or indistinguishable voice is a NUMBER before it
//! ships:
//!
//!   cargo run -q -p th-synth --example audition_render
//!   cargo run -q -p th-synth --example audition_render -- /tmp/phrase.wav --solo-dir /tmp/phrase-solos
//!
//! The optional first argument sets the output path (default
//! `audition.wav`); `--solo-dir DIR` dumps each soloed slot beside
//! the mix; `--bars N` renders N bars (default: the phrase length,
//! minimum 4).

use std::env;
use std::fs;
use std::path::Path;

use fundsp::hacker::*;
use fundsp::MAX_BUFFER_SIZE;

use rust_synth::audio::preset::{master_bus, GlobalParams, Preset};
use rust_synth::audio::Track;
use th_synth::director::{scale_mode_for_stage, MusicDirector, MASTER_GAIN, SLOTS};
use th_synth::score::Score;

const SR: f32 = 48_000.0;

fn main() {
    // argv: [out.wav] [--solo-dir DIR] [--bars N]
    let args: Vec<String> = env::args().skip(1).collect();
    let mut out: Option<String> = None;
    let mut solo_dir: Option<String> = None;
    let mut bars_override: Option<usize> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--solo-dir" => {
                i += 1;
                solo_dir = args.get(i).cloned();
            }
            "--bars" => {
                i += 1;
                match args
                    .get(i)
                    .and_then(|v| v.parse::<usize>().ok())
                    .filter(|&n| n >= 1)
                {
                    Some(n) => bars_override = Some(n),
                    None => eprintln!(
                        "audition: --bars needs a count >= 1 (got {:?}); using the default",
                        args.get(i)
                    ),
                }
            }
            other => {
                if out.is_none() {
                    out = Some(other.to_string());
                }
            }
        }
        i += 1;
    }
    let out = out.unwrap_or_else(|| "audition.wav".to_string());

    // The REAL director setup (stage 0, the shipped score) — and its
    // PHRASE: `score.bars` (empty = today's one bar whose figures are
    // the top-level lines — the backwards-compat contract).
    let probe = MusicDirector::new_fallback();
    let score = probe.score().clone();
    let phrase_len = score.phrase_len();
    let bars = bars_override.unwrap_or(std::cmp::max(phrase_len, 4));
    let p = score.stage_params(0);
    if let Some(dir) = &solo_dir {
        if let Err(err) = fs::create_dir_all(dir) {
            eprintln!("audition: create solo dir {dir}: {err}");
        }
    }

    let g = GlobalParams::default();
    g.bpm.set_value(p.bpm);
    g.brightness.set_value(p.brightness);
    g.scale_mode.set_value(scale_mode_for_stage(0));
    g.master_gain.set_value(MASTER_GAIN);

    let secs = bars as f64 * 4.0 * 60.0 / p.bpm as f64;
    // The frequency closures' `follow(0.08)` means a FRESH graph takes
    // ~0.3 s (≈3× the response time) for the carriers to spin up from
    // 0 Hz. Live, the graph runs continuously so this never happens;
    // offline, render the flush and measure/write the settled tail.
    const SPINUP_S: f64 = 0.3;
    let step_s = 60.0 / p.bpm as f64 / 4.0;
    let flush_steps = (SPINUP_S / step_s).round() as usize;

    println!(
        "audition: {} slots, {bars} bars @ {} bpm ({} s @ 48 kHz, +{SPINUP_S:.1} s spin-up flush ≈ {flush_steps} steps of bar 0) -> {out}",
        probe.tracks().len(),
        p.bpm,
        secs
    );
    println!(
        "phrase: {phrase_len} bar(s); rendering {bars} (markers: default = the top-level figure; bar[i] = phrase bar i's override; internal = the off-beat comp fallback):"
    );
    for b in 0..bars {
        println!("[bar {b}] {}", figure_markers(&score, b));
    }
    println!("values used (the director's REAL tracks at t=0 — the DEFAULT bar's figures; the per-bar markers above are what vary across the phrase):");
    for t in probe.tracks() {
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
    // fresh graph per pass keeps the reverb tails from cross-
    // contaminating the measurement). Two director-driven details:
    // (1) the silence lives in the SCORE (every other slot's `muted`
    // = true) — the director's per-tick mute fold rewrites every
    // slot's `mute` from the score's user state EVERY tick, so a raw
    // `params.mute` write (the pre-11g renderer's approach) would be
    // clobbered by the first tick; (2) each pass runs with every
    // voice UNLOCKED (stage = the max slot's unlock — the table's
    // job is each voice's CHARACTER, and a stage-locked row reading
    // 0.0 forever could never expose a broken voice; below stage 6
    // the drum tiers, the root and the chord table are
    // stage-0-identical, so only the bpm/brightness globals differ —
    // the pass gets its own). The MIX pass below stays the shipped
    // stage-0 mix.
    let unlock_stage = SLOTS.iter().map(|s| s.unlock).max().unwrap_or(0);
    let g_solo = {
        let ps = score.stage_params(unlock_stage);
        let g_solo = GlobalParams::default();
        g_solo.bpm.set_value(ps.bpm);
        g_solo.brightness.set_value(ps.brightness);
        g_solo
            .scale_mode
            .set_value(scale_mode_for_stage(unlock_stage));
        g_solo.master_gain.set_value(MASTER_GAIN);
        g_solo
    };
    let mut solo: Vec<(String, f32, f32, f64, f64, usize)> = Vec::new();
    for i in 0..probe.tracks().len() {
        let mut d = MusicDirector::new_fallback();
        let mut s = score.clone();
        for j in 0..s.slots.len() {
            s.slots[j].muted = j != i;
        }
        d.set_score(s);
        d.set_stage(unlock_stage); // the table's unlock (see above)
        let name = d.tracks()[i].name.clone();
        let mut net = build_sum(d.tracks(), &g_solo);
        let (left_full, right_full) =
            render_phrase(&mut d, &mut [&mut net], SPINUP_S + secs).remove(0);
        let drop = (SPINUP_S * SR as f64) as usize;
        let left: Vec<f32> = left_full[drop..].to_vec();
        let right: Vec<f32> = right_full[drop..].to_vec();
        let m = metrics(&left, &right);
        let (atktail, onsets) = attack_tail(&left, &right, SR as f64);
        solo.push((name.clone(), m.peak, m.rms, m.bright, atktail, onsets));
        // Also dump each soloed slot beside the mix WAV when asked
        // (`--solo-dir DIR`): the per-slot table is a SUMMARY, and a
        // summary cannot settle "is this voice actually FM?" — the
        // spectrum of the soloed signal can. Naming is index-prefixed so
        // the files sort in engine order.
        if let Some(dir) = &solo_dir {
            let fname = format!("{dir}/{i}-{}.wav", name.replace(' ', "_"));
            if let Err(err) = fs::write(Path::new(&fname), wav_bytes(&left, &right)) {
                eprintln!("audition: solo dump {fname}: {err}");
            }
        }
    }

    // ── The mix (all 8 live) — this is the WAV's content. The raw
    // (no master bus) net is pulled in the SAME tick loop (the
    // headroom probe below).
    let mut d = MusicDirector::new_fallback(); // the shipped score, unmuted
    let mut net = build_sum(d.tracks(), &g);
    let mut raw = build_sum_raw(d.tracks(), &g);
    let mut r = render_phrase(&mut d, &mut [&mut net, &mut raw], SPINUP_S + secs);
    let (mix_full, raw_full) = (r.remove(0), r.remove(0));
    let drop = (SPINUP_S * SR as f64) as usize;
    let left: Vec<f32> = mix_full.0[drop..].to_vec();
    let right: Vec<f32> = mix_full.1[drop..].to_vec();
    let mix = metrics(&left, &right);

    // The headroom probe: the same sum through NO master bus, so the
    // limiter cannot hide how hard it is working. The master bus ends in
    // `limiter_stereo(0.001, 0.3)`, and a limiter is not a mixer — when
    // the sum exceeds its ceiling it squashes EVERYTHING, transients
    // included, which is exactly the "no punch" symptom the atk/tail
    // column measures. `headroom_db` is how much to trim the slot gains
    // (or `master_gain`) to leave the limiter alone.
    let raw_left: Vec<f32> = raw_full.0[drop..].to_vec();
    let raw_right: Vec<f32> = raw_full.1[drop..].to_vec();
    let raw_peak = raw_left
        .iter()
        .chain(raw_right.iter())
        .fold(0.0f32, |p, s| p.max(s.abs()));
    let limited_peak = left
        .iter()
        .chain(right.iter())
        .fold(0.0f32, |p, s| p.max(s.abs()));
    let headroom_db = if raw_peak > 0.0 {
        20.0 * (1.0 / raw_peak).log10()
    } else {
        f32::INFINITY
    };

    println!(
        "\n== per-slot (soloed, all voices unlocked @ stage {unlock_stage}, {secs:.2} s) + mix (the shipped stage-0 mix) =="
    );
    println!(
        "{:<10} {:>10} {:>10} {:>12} {:>9} {:>7}",
        "slot", "peak", "rms", "bright (4k+)", "atk/tail", "onsets"
    );
    for (name, peak_s, rms_s, bright_s, atk_s, onsets_s) in &solo {
        println!(
            "{name:<10} {peak_s:>10.4} {rms_s:>10.4} {bright_s:>12.4} {atk_s:>9.2} {onsets_s:>7}"
        );
    }
    println!(
        "{:<10} {:>10.4} {:>10.4} {:>12.4} {:>9} {:>7}",
        "mix", mix.peak, mix.rms, mix.bright, "—", "—"
    );
    println!(
        "\nheadroom: pre-limiter peak {raw_peak:.4}, post {limited_peak:.4} -> trim slot gains by {headroom_db:+.1} dB to leave the limiter alone"
    );
    if raw_peak > 1.0 {
        println!(
            "  WARNING: the sum is {:.1}x over full scale; the limiter is squashing transients (the punch this table measures).",
            raw_peak
        );
    }

    fs::write(Path::new(&out), wav_bytes(&left, &right))
        .unwrap_or_else(|err| panic!("write {out}: {err}"));
    println!(
        "\nwrote {out} ({} bytes, 16-bit stereo 48 kHz)",
        44 + left.len() * 4
    );
}

/// The per-bar marker (issue 11g — the section header that lets a
/// listener follow the form): which figure each part (bass/lead/comp)
/// resolves to on bar `b` — the phrase bar's override (`bar[i]`,
/// `i = b % bars.len()` — the resolution's modulo), the top-level
/// figure (`default`), or, for the comp, the internal off-beat default
/// (`internal`) when the score itself carries no `comp` field.
fn figure_markers(score: &Score, b: usize) -> String {
    if score.bars.is_empty() {
        return "bass=default lead=default comp=default".to_string();
    }
    let i = b % score.bars.len();
    let bar = &score.bars[i];
    let part = |present: bool| -> String {
        if present {
            format!("bar[{i}]")
        } else {
            "default".to_string()
        }
    };
    let comp = if bar.comp.is_some() {
        format!("bar[{i}]")
    } else if score.comp.is_some() {
        "default".to_string()
    } else {
        "internal".to_string()
    };
    format!(
        "bass={} lead={} comp={}",
        part(bar.bass.is_some()),
        part(bar.lead.is_some()),
        comp
    )
}

/// Render the PHRASE DIRECTOR-DRIVEN (issue 11g): the fallback
/// director ticks SAMPLE by SAMPLE while the nets run — its
/// `sequence` rewrites the tracks' `Shared` params (the SAME atomics
/// the live graph reads: `apply_bar`'s per-bar `pattern_bits`
/// refresh, `apply_step`'s resolved note frequencies, the absolute-
/// bar chord offsets) — so the resolved per-bar figures actually vary
/// inside the render (the pre-11g "render ONE static setup with
/// `Wave::render`" path would have played a phrase as one bar). The
/// nets are pulled through fundsp's BLOCK `process` in the same
/// 64-sample chunks `Wave::render` uses (a per-sample `get_stereo()`
/// tick would be 10× slower for these long-reverb graphs — the
/// director's ticks are cheap; the graph's work is not).
fn render_phrase(
    d: &mut MusicDirector,
    nets: &mut [&mut Net],
    secs: f64,
) -> Vec<(Vec<f32>, Vec<f32>)> {
    let total = (secs * SR as f64).round() as usize;
    let dt = 1.0 / SR as f64;
    let mut channels: Vec<(Vec<f32>, Vec<f32>)> = nets
        .iter_mut()
        .map(|net| {
            net.set_sample_rate(SR as f64);
            (Vec::with_capacity(total), Vec::with_capacity(total))
        })
        .collect();
    let mut i = 0;
    while i < total {
        let n = std::cmp::min(total - i, MAX_BUFFER_SIZE);
        for _ in 0..n {
            d.tick(dt as f32);
        }
        for k in 0..nets.len() {
            // A FRESH local buffer per net per chunk (a `BufferMut`
            // into a Vec element of `channels` would not coexist with
            // the next `channels[k]` borrow — the borrows must be
            // disjoint).
            let mut buf = BufferVec::new(2);
            let mut out = buf.buffer_mut();
            nets[k].process(n, &BufferRef::new(&[]), &mut out);
            for j in 0..n {
                channels[k].0.push(out.at_f32(0, j));
                channels[k].1.push(out.at_f32(1, j));
            }
        }
        i += n;
    }
    channels
}

/// The SAME graph the director builds: each slot's `Preset::build`
/// (the slot's resolved kind + its `TrackParams` — the `Shared`
/// atomics the live graph reads) summed through the master bus.
fn build_sum(tracks: &[Track], g: &GlobalParams) -> Net {
    build_sum_raw(tracks, g) >> master_bus(g.brightness.clone())
}

/// The same sum WITHOUT the master bus (so no limiter): the headroom probe.
/// The master bus ends in `limiter_stereo(0.001, 0.3)`, and a limiter
/// is not a mixer: when the sum exceeds its ceiling it squashes
/// EVERYTHING, transients included. Since the issue-11e dry+wet fix
/// multiplied every voice's energy ~8x, the sum now hits the limiter
/// hard — and a squashed kick is exactly the "no punch" symptom we just
/// fixed at the voice level. This render exists to keep that honest:
/// `headroom` below is the gain the operator should trim from the
/// slots' gains, not a secret the limiter hides.
fn build_sum_raw(tracks: &[Track], g: &GlobalParams) -> Net {
    let mut sum: Option<Net> = None;
    for t in tracks {
        let node = Preset::build(t.kind, &t.params, g);
        sum = Some(match sum {
            Some(acc) => acc + node,
            None => node,
        });
    }
    sum.expect("8 slots")
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

/// The ATTACK/TAIL guard (issue 11e — the metric that should have
/// caught the wet-only reverb bug): per detected onset, the peak in the
/// first ~10 ms after the onset (the hit's ATTACK) over the peak in the
/// 50–300 ms window after it (the reverb TAIL).
///
/// WHY THIS EXISTS: peak, RMS and brightness all look HEALTHY when a
/// signal is 100% reverb — the pre-11e kit measured fine on every one
/// of those and was still nothing but reverb tail (every tail louder
/// than the hit that caused it; median 0.58 across onsets where a
/// punchy drum wants 5–20×). The attack/tail ratio is the table number
/// that exposes a wet-only signal.
///
/// How to read it: a percussive voice below ~2 is a failure worth
/// eyeballing; a sustained voice (pad/drone/sub) may legitimately sit
/// near or below 1 — its energy is continuous, not transient — so the
/// number is printed and the reader judges. A tail window never reaches
/// past the NEXT onset (the next hit's attack is not this hit's tail).
/// When no onset is detected (a purely sustained voice) the
/// steady-state reading is returned instead (the render head's first
/// 10 ms over its 50–300 ms window — ≈ 1.0 for a flat sustained
/// signal) and the onset count reads 0. Ratios are capped at 99.90
/// (a dry voice with no audible tail is "punchy past the cap").
fn attack_tail(left: &[f32], right: &[f32], sr: f64) -> (f64, usize) {
    // 5 ms envelope frames (240 samples @ 48 kHz).
    let n = left.len();
    let frame = (5.0e-3 * sr) as usize;
    let mut env: Vec<f32> = Vec::with_capacity(n / frame + 1);
    let mut i = 0;
    while i < n {
        let end = std::cmp::min(i + frame, n);
        let mut m = 0.0f32;
        for k in i..end {
            m = m.max(left[k].abs()).max(right[k].abs());
        }
        env.push(m);
        i = end;
    }
    let max_e = env.iter().copied().fold(0.0f32, f32::max);
    if max_e < 1.0e-7 || env.len() < 3 {
        return (0.0, 0);
    }
    // Onset candidates: a frame that jumps >20% over its predecessor
    // (a hit arriving — a sustained voice's envelope wobbles far more
    // slowly per 5 ms frame) and peaks locally above 25% of the
    // render's own peak. A decaying reverb tail is never such a peak;
    // the first rise out of a reverb's predelay is.
    const MIN_SPACING_FRAMES: usize = 6; // 30 ms — one hit, one onset
    let thr = 0.25 * max_e;
    let mut kept: Vec<usize> = Vec::new();
    for i in 1..env.len() - 1 {
        if env[i] > thr && env[i] >= env[i + 1] && env[i] > 1.2 * env[i - 1] {
            match kept.last() {
                Some(&last) if i - last < MIN_SPACING_FRAMES => {
                    // Close to the last kept onset: keep the louder of
                    // the two (the same hit's double peak). `last` is a
                    // FRAME index — the element slot is the Vec's own.
                    if env[i] > env[last] {
                        let slot = kept.len() - 1;
                        kept[slot] = i;
                    }
                }
                _ => kept.push(i),
            }
        }
    }
    let atk = (10.0e-3 * sr) as usize; // ~480 samples (the attack window)
    let t0 = (50.0e-3 * sr) as usize; // 2400 (tail window start)
    let t1 = (300.0e-3 * sr) as usize; // 14400 (tail window end)
    let mut ratios: Vec<f64> = Vec::new();
    for (oi, &f) in kept.iter().enumerate() {
        let onset = f * frame;
        let a_end = std::cmp::min(onset + atk, n);
        if a_end <= onset {
            continue;
        }
        let mut atk_peak = 0.0f32;
        for k in onset..a_end {
            atk_peak = atk_peak.max(left[k].abs()).max(right[k].abs());
        }
        // The tail window never reaches past the next onset (the next
        // hit's attack is not this hit's reverb tail).
        let mut t_end = std::cmp::min(onset + t1, n);
        if oi + 1 < kept.len() {
            t_end = std::cmp::min(t_end, kept[oi + 1] * frame);
        }
        let t_start = onset + t0;
        if t_start >= t_end {
            continue; // no tail room before the next onset
        }
        let mut tail_peak = 0.0f32;
        for k in t_start..t_end {
            tail_peak = tail_peak.max(left[k].abs()).max(right[k].abs());
        }
        let r = if tail_peak < 1.0e-7 {
            99.9
        } else {
            (atk_peak as f64 / tail_peak as f64).min(99.9)
        };
        ratios.push(r);
    }
    if ratios.is_empty() {
        // No onset detected — a purely sustained voice (or every onset
        // sat inside another's tail window). Steady-state reading:
        // the render head's first 10 ms over its 50–300 ms window.
        let mut atk_peak = 0.0f32;
        for k in 0..std::cmp::min(atk, n) {
            atk_peak = atk_peak.max(left[k].abs()).max(right[k].abs());
        }
        let mut tail_peak = 0.0f32;
        for k in std::cmp::min(t0, n)..std::cmp::min(t1, n) {
            tail_peak = tail_peak.max(left[k].abs()).max(right[k].abs());
        }
        let r = if tail_peak < 1.0e-7 {
            99.9
        } else {
            (atk_peak as f64 / tail_peak as f64).min(99.9)
        };
        return (r, 0);
    }
    // The MEDIAN across onsets (one value per hit — the "median across
    // onsets" the 11e before/after tables quote).
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    (ratios[ratios.len() / 2], ratios.len())
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
