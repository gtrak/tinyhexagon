//! cpal output stream wiring + the in-memory engine (the one-mixer plan's
//! `start_in_memory` — plan 002 issue 09).
//!
//! 8 pre-allocated track slots. Audio callback pulls stereo samples from
//! a FunDSP `Net` built once from all 8. Dormant slots are simply muted
//! — no reallocation, no graph hot-swap. Sample ring buffer captures
//! output for the TUI oscilloscope (producer-side only lock).
//!
//! Two construction halves share the graph/params/peaks/scope/phase-clock
//! and the per-frame render body (`render_buffer`):
//! * `start` — the cpal device stream (desktop/Android; the audio callback
//!   drives the phase clock);
//! * `start_in_memory` — NO stream: the caller renders the graph through
//!   `EngineHandle::render_stereo` and delivers the frames itself (the web:
//!   the SFX OpenAL context's streaming source — ONE context for every
//!   sound; the render pass drives the phase clock).

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use fundsp::hacker::*;
use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

use super::preset::{master_bus, GlobalParams, Preset, PresetKind};
use super::track::Track;
use crate::math::harmony::golden_freq;
// 05b feature gate (see Cargo.toml provenance block): the recorder is
// part of the `recording` feature; without it the audio path is
// byte-identical except for the per-frame `push_frame` (a no-op sink).
#[cfg(feature = "recording")]
use crate::recording::RecorderState;

/// Max tracks pre-allocated. Raise = more CPU, more slots.
pub const MAX_TRACKS: usize = 8;
/// The in-memory engine's fixed render rate (`start_in_memory`, the
/// one-mixer plan's web half): the graph renders at this rate and the
/// delivery labels its buffers with it; the browser resamples to the Web
/// Audio context's rate only when the two differ.
pub const IN_MEMORY_SAMPLE_RATE: f32 = 48_000.0;

/// Ring buffer of stereo samples for the oscilloscope (decimated).
pub const SCOPE_CAPACITY: usize = 512;
/// Keep one sample per N audio samples.
pub const SCOPE_DECIMATION: usize = 32;

pub type ScopeBuffer = Arc<Mutex<VecDeque<(f32, f32)>>>;
pub type SharedGraph = Arc<Mutex<Net>>;

/// Handle the TUI keeps alive for the lifetime of the app.
pub struct EngineHandle {
    pub tracks: Arc<Mutex<Vec<Track>>>,
    pub global: GlobalParams,
    pub peak_l: Shared,
    pub peak_r: Shared,
    pub sample_rate: f32,
    pub scope: ScopeBuffer,
    pub phase_clock: Shared,
    #[cfg(feature = "recording")]
    pub recorder: Arc<RecorderState>,
    /// Master DSP graph. Exposed behind a mutex so the TUI can swap it
    /// wholesale when the user changes a track's preset kind at runtime.
    /// Lock contention is tiny: audio callback holds it for one buffer
    /// (~10 ms at 48 kHz / 512-sample buffer) and UI rebuilds take ~5 ms.
    graph: SharedGraph,
    /// The cpal output stream (`start`). `None` for the in-memory engine
    /// (`start_in_memory` — the one-mixer plan: the frames go to the
    /// caller's delivery and the phase clock advances in `render_stereo`
    /// instead of the callback).
    _stream: Option<Stream>,
    /// The render state the cpal callback and `render_stereo` share (the
    /// envelope followers, the phase-clock time, the scope decimation).
    render_state: Arc<Mutex<RenderState>>,
}

pub struct AudioEngine;

impl AudioEngine {
    pub fn start(initial_tracks: Vec<Track>) -> Result<EngineHandle> {
        assert!(
            initial_tracks.len() == MAX_TRACKS,
            "expected exactly {MAX_TRACKS} pre-allocated tracks, got {}",
            initial_tracks.len()
        );

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no default output audio device")?;
        let config: StreamConfig = device.default_output_config()?.into();
        let sample_rate = config.sample_rate.0 as f32;
        let channels = config.channels as usize;

        let global = GlobalParams::default();
        let peak_l = shared(0.0);
        let peak_r = shared(0.0);
        let phase_clock = shared(0.0);
        let scope: ScopeBuffer = Arc::new(Mutex::new(VecDeque::with_capacity(SCOPE_CAPACITY)));
        let tracks = Arc::new(Mutex::new(initial_tracks));
        #[cfg(feature = "recording")]
        let recorder = RecorderState::new(sample_rate as u32);

        let mut graph = build_master(&tracks.lock(), &global);
        graph.set_sample_rate(sample_rate as f64);
        let graph: SharedGraph = Arc::new(Mutex::new(graph));

        let render_state = Arc::new(Mutex::new(RenderState::new(sample_rate)));
        let stream = start_stream(
            device,
            config,
            channels,
            graph.clone(),
            global.master_gain.clone(),
            peak_l.clone(),
            peak_r.clone(),
            scope.clone(),
            phase_clock.clone(),
            render_state.clone(),
            #[cfg(feature = "recording")]
            recorder.clone(),
        )?;

        Ok(EngineHandle {
            tracks,
            global,
            peak_l,
            peak_r,
            sample_rate,
            scope,
            phase_clock,
            #[cfg(feature = "recording")]
            recorder,
            graph,
            _stream: Some(stream),
            render_state,
        })
    }

    /// The one-mixer plan's in-memory half (plan 002 issue 09): the SAME
    /// graph build, `GlobalParams`, peaks, scope and phase clock as
    /// [`Self::start`], but NO cpal stream — the caller renders the graph
    /// itself through [`EngineHandle::render_stereo`] and delivers the
    /// frames (on the web: the SFX OpenAL context's streaming source —
    /// ONE context for every sound). `sample_rate` is the fixed
    /// [`IN_MEMORY_SAMPLE_RATE`]; the delivery resamples to the context's
    /// rate only when the two differ. Never touches a device (the
    /// autoplay policy's first-gesture half and the CI's no-device tests
    /// can both use it).
    pub fn start_in_memory(initial_tracks: Vec<Track>) -> Result<EngineHandle> {
        assert!(
            initial_tracks.len() == MAX_TRACKS,
            "expected exactly {MAX_TRACKS} pre-allocated tracks, got {}",
            initial_tracks.len()
        );

        let sample_rate = IN_MEMORY_SAMPLE_RATE;
        let global = GlobalParams::default();
        let peak_l = shared(0.0);
        let peak_r = shared(0.0);
        let phase_clock = shared(0.0);
        let scope: ScopeBuffer = Arc::new(Mutex::new(VecDeque::with_capacity(SCOPE_CAPACITY)));
        let tracks = Arc::new(Mutex::new(initial_tracks));
        #[cfg(feature = "recording")]
        let recorder = RecorderState::new(sample_rate as u32);

        let mut graph = build_master(&tracks.lock(), &global);
        graph.set_sample_rate(sample_rate as f64);
        let graph: SharedGraph = Arc::new(Mutex::new(graph));

        Ok(EngineHandle {
            tracks,
            global,
            peak_l,
            peak_r,
            sample_rate,
            scope,
            phase_clock,
            #[cfg(feature = "recording")]
            recorder,
            graph,
            _stream: None,
            render_state: Arc::new(Mutex::new(RenderState::new(sample_rate))),
        })
    }
}

impl EngineHandle {
    /// 09 instrumentation — the emscripten WebAudio context's live state
    /// (the tinyhexagon status probe's `ctx=` field); `None`-equivalent
    /// (an empty string) on other hosts AND for the in-memory engine
    /// (`start_in_memory` — its delivery, the web's OpenAL context, is the
    /// SFX device's, so the probe reads THAT context's state instead). The
    /// setTimeout-driven graph renders even while the context is
    /// suspended — only this proves the browser is actually playing.
    pub fn audio_context_state(&self) -> String {
        #[cfg(target_os = "emscripten")]
        {
            return self
                ._stream
                .as_ref()
                .map_or_else(String::new, |s| s.audio_context_state());
        }
        #[cfg(not(target_os = "emscripten"))]
        {
            String::new()
        }
    }

    /// 09 fix (tinyhexagon) — re-resume a suspended WebAudio context from
    /// a NEW user gesture (a no-op on other hosts / while running / for
    /// the in-memory engine — its shared OpenAL context is the SFX
    /// device's, unlocked by that device's own gesture listeners). The
    /// page's gesture handler calls this every gesture while the status
    /// says the music isn't playing.
    pub fn replay_context(&self) {
        #[cfg(target_os = "emscripten")]
        {
            if let Some(s) = &self._stream {
                s.replay();
            }
        }
        #[cfg(not(target_os = "emscripten"))]
        {
            let _ = self;
        }
    }

    /// Render `out` (interleaved stereo `f32`; an odd trailing channel
    /// value is ignored) from the graph and update the peaks / scope /
    /// phase clock exactly the cpal callback does. The one-mixer plan's
    /// in-memory half: on the web the frame timer (th-app's loop) calls
    /// this per rAF tick and queues the frames into the SFX OpenAL
    /// context's streaming source — the phase clock the `BeatView`
    /// consumes is thus driven by the render pass (the same render-driven
    /// clock as today's audio callback, not wall time).
    ///
    /// Call this on a [`Self::start_in_memory`] handle only: on a
    /// streaming handle it would render a second, out-of-sync copy of the
    /// mix and corrupt the callback's clock.
    pub fn render_stereo(&self, out: &mut [f32]) {
        debug_assert!(
            self._stream.is_none(),
            "render_stereo is for the in-memory engine (start_in_memory)"
        );
        if out.len() < 2 {
            return;
        }
        let master = self.global.master_gain.value();
        // Lock the shared graph for the whole buffer — a few µs longer
        // than per-sample locking, but no extra cost (the callback's note).
        let mut graph = self.graph.lock();
        let mut st = self.render_state.lock();
        render_buffer(
            &mut graph,
            master,
            out,
            2,
            &mut st,
            &self.scope,
            &self.peak_l,
            &self.peak_r,
            &self.phase_clock,
            #[cfg(feature = "recording")]
            &self.recorder,
        );
    }

    /// Rebuild the master DSP graph from the current track list. Call
    /// this after mutating any `track.kind`. Cheap enough for live use —
    /// the audio callback sees the new graph on its next buffer.
    ///
    /// NB: the old graph's internal state (reverb tails, oscillator
    /// phases) is discarded. There's a small audible discontinuity
    /// during preset swaps; acceptable given how rarely kind changes.
    pub fn rebuild_graph(&self) {
        let tracks = self.tracks.lock();
        let mut new_graph = build_master(&tracks, &self.global);
        drop(tracks);
        new_graph.set_sample_rate(self.sample_rate as f64);
        *self.graph.lock() = new_graph;
    }
}

fn build_master(tracks: &[Track], g: &GlobalParams) -> Net {
    let mut summed: Option<Net> = None;
    for t in tracks {
        let node = Preset::build(t.kind, &t.params, g);
        summed = Some(match summed {
            Some(acc) => acc + node,
            None => node,
        });
    }
    let summed = summed.unwrap_or_else(|| Net::wrap(Box::new(zero() | zero())));

    // Pipe the stereo sum through the master bus (variable lowpass +
    // soft limiter). This is where "highs punch" gets tamed before the
    // signal reaches cpal.
    summed >> master_bus(g.brightness.clone())
}

/// The stateful half of one output-buffer render (the cpal callback's
/// former locals): the envelope followers' state, the phase-clock time
/// and the scope decimation counter. Shared between the `start` stream
/// callback and the in-memory `render_stereo` so BOTH update the peaks /
/// scope / phase clock byte-for-byte the same way (the one-mixer plan:
/// the in-memory engine's clock must track the delivered audio exactly
/// the stream's clock does).
struct RenderState {
    env_l: f32,
    env_r: f32,
    decim: usize,
    /// The phase-clock time (f64 — multi-hour playback stays stable).
    t: f64,
    /// One frame's time step (`1 / sample_rate`, f64).
    dt: f64,
}

impl RenderState {
    fn new(sample_rate: f32) -> Self {
        Self {
            env_l: 0.0,
            env_r: 0.0,
            decim: 0,
            t: 0.0,
            dt: 1.0 / sample_rate as f64,
        }
    }
}

/// One output-buffer render (the shared body): `out` is `channels`-
/// interleaved frames. The graph's master output (× the master gain)
/// fills it; the envelope followers (peaks), the scope decimation and
/// the phase clock update once per frame — the callback's semantics
/// verbatim.
fn render_buffer(
    graph: &mut Net,
    master: f32,
    out: &mut [f32],
    channels: usize,
    st: &mut RenderState,
    scope: &ScopeBuffer,
    peak_l: &Shared,
    peak_r: &Shared,
    phase_clock: &Shared,
    #[cfg(feature = "recording")]
    recorder: &Arc<RecorderState>,
) {
    let fall = 0.9995f32;
    let mut pending: [(f32, f32); 32] = [(0.0, 0.0); 32];
    let mut pending_n = 0usize;

    for frame in out.chunks_mut(channels) {
        let (lo, ro) = graph.get_stereo();
        let l = lo * master;
        let r = ro * master;
        st.env_l = (st.env_l * fall).max(l.abs());
        st.env_r = (st.env_r * fall).max(r.abs());

        for (ch, slot) in frame.iter_mut().enumerate() {
            *slot = if ch & 1 == 0 { l } else { r };
        }

        // Record master output if active. Lock is held for a
        // handful of ns per frame — safe at 48 kHz.
        #[cfg(feature = "recording")]
        recorder.push_frame(l, r);

        st.decim = st.decim.wrapping_add(1);
        if st.decim.is_multiple_of(SCOPE_DECIMATION) && pending_n < pending.len() {
            pending[pending_n] = (l, r);
            pending_n += 1;
        }

        st.t += st.dt;
    }

    // Single lock per callback, not per sample.
    if pending_n > 0 {
        let mut scope = scope.lock();
        for &s in &pending[..pending_n] {
            if scope.len() == SCOPE_CAPACITY {
                scope.pop_front();
            }
            scope.push_back(s);
        }
    }

    peak_l.set_value(st.env_l);
    peak_r.set_value(st.env_r);
    phase_clock.set_value(st.t as f32);
}

#[allow(clippy::too_many_arguments)]
fn start_stream(
    device: Device,
    config: StreamConfig,
    channels: usize,
    graph: SharedGraph,
    master: Shared,
    peak_l: Shared,
    peak_r: Shared,
    scope: ScopeBuffer,
    phase_clock: Shared,
    render_state: Arc<Mutex<RenderState>>,
    #[cfg(feature = "recording")]
    recorder: Arc<RecorderState>,
) -> Result<Stream> {
    let err_fn = |err| tracing::error!("audio stream error: {err}");

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _| {
            let m = master.value();
            // Lock the shared graph for the whole buffer — a few µs
            // longer than per-sample locking, but no extra cost.
            let mut graph = graph.lock();
            let mut st = render_state.lock();
            render_buffer(
                &mut graph,
                m,
                data,
                channels,
                &mut st,
                &scope,
                &peak_l,
                &peak_r,
                &phase_clock,
                #[cfg(feature = "recording")]
                &recorder,
            );
        },
        err_fn,
        None,
    )?;
    stream.play()?;
    Ok(stream)
}

/// Default 8-track set: 3 active + 5 dormant, rooted on golden-ratio frequencies.
pub fn default_track_set() -> Vec<Track> {
    let root = 55.0f32; // A1
    let mut tracks = Vec::with_capacity(MAX_TRACKS);

    // Four active voices — full ambient layout by default.
    tracks.push(Track::new(0, "Pad",       PresetKind::PadZimmer, golden_freq(root, 0)));
    tracks.push(Track::new(1, "Bass",      PresetKind::BassPulse, golden_freq(root, 0)));
    tracks.push(Track::new(2, "Heartbeat", PresetKind::Heartbeat, golden_freq(root, 0)));
    tracks.push(Track::new(3, "Drone",     PresetKind::DroneSub,  golden_freq(root, -1)));
    tracks[3].params.gain.set_value(0.32);
    tracks[3].params.reverb_mix.set_value(0.7);

    // Dormant slots — one per remaining preset kind so `a` shows every
    // available voice. Names mirror the kind label; when activated the
    // user immediately knows what they just turned on.
    tracks.push(Track::dormant(4, "Shimmer",  PresetKind::Shimmer,  golden_freq(root, 1)));
    tracks.push(Track::dormant(5, "Bell",     PresetKind::Bell,     golden_freq(root, 2)));
    tracks.push(Track::dormant(6, "SuperSaw", PresetKind::SuperSaw, golden_freq(root, -2)));
    tracks.push(Track::dormant(7, "Pluck",    PresetKind::PluckSaw, golden_freq(root, 1)));

    tracks
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one-mixer plan's in-memory half: the graph renders with NO
    /// audio device, the output is not silent, and the peaks / scope /
    /// phase clock advance with the rendered frames (the render pass
    /// drives the clock — the web's frame timer's contract).
    #[test]
    fn start_in_memory_renders_nonzero_output() {
        let handle = AudioEngine::start_in_memory(default_track_set()).expect("in-memory start");
        assert_eq!(handle.sample_rate, IN_MEMORY_SAMPLE_RATE);
        assert!(handle._stream.is_none());

        let mut out = vec![0.0f32; (IN_MEMORY_SAMPLE_RATE as usize) * 2]; // one second
        handle.render_stereo(&mut out);

        let peak = out.iter().fold(0.0f32, |p, &s| p.max(s.abs()));
        assert!(peak > 0.0, "the default set must render audible output, got peak {peak}");
        // The render pass drove the clock: ~1 s of frames → ~1 s of time.
        assert!(
            (handle.phase_clock.value() - 1.0).abs() < 1e-3,
            "phase clock advanced by the render: {}",
            handle.phase_clock.value()
        );
        // The envelope followers saw the output, the scope decimated frames.
        assert!(handle.peak_l.value() > 0.0 || handle.peak_r.value() > 0.0);
        assert!(!handle.scope.lock().is_empty());
    }
}
