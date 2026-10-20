use js_sys::Float32Array;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::AudioContext;

use crate::traits::{DeviceTrait, HostTrait, StreamTrait};
use crate::{
    BufferSize, BuildStreamError, Data, DefaultStreamConfigError, DeviceNameError, DevicesError,
    InputCallbackInfo, OutputCallbackInfo, PauseStreamError, PlayStreamError, SampleFormat,
    SampleRate, StreamConfig, StreamError, SupportedBufferSize, SupportedStreamConfig,
    SupportedStreamConfigRange, SupportedStreamConfigsError,
};

// The emscripten backend currently works by instantiating an `AudioContext` object per `Stream`.
// Creating a stream creates a new `AudioContext`. Destroying a stream destroys it. Creation of a

/// 09 (tinyhexagon) — the console's `log` binding (a manual extern: the
/// `console` web-sys feature is not enabled in cpal's dep spec, and the
/// rejection path must be VISIBLE — the old panic killed the game).
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}
// `Host` instance initializes the `stdweb` context.

/// The default emscripten host type.
#[derive(Debug)]
pub struct Host;

/// Content is false if the iterator is empty.
pub struct Devices(bool);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device;

#[wasm_bindgen]
#[derive(Clone)]
pub struct Stream {
    // A reference to an `AudioContext` object.
    audio_ctxt: AudioContext,
}

impl Stream {
    /// 09 instrumentation — the WebAudio context's live state
    /// ("running"/"suspended"/"closed"): the tinyhexagon status probe's
    /// `ctx=` field. The setTimeout-driven graph keeps rendering (the
    /// peaks/phase clock advance) even while the context is SUSPENDED —
    /// only this field proves the browser is actually PLAYING.
    pub fn audio_context_state(&self) -> &'static str {
        use web_sys::AudioContextState;
        match self.audio_ctxt.state() {
            AudioContextState::Running => "running",
            AudioContextState::Suspended => "suspended",
            AudioContextState::Closed => "closed",
            _ => "unknown",
        }
    }

    /// 09 fix (tinyhexagon) — re-resume a suspended context from a NEW
    /// user gesture (the page calls this on every gesture while the
    /// music isn't running). Idempotent while running (a running
    /// context's resume() resolves immediately); a rejected resume logs
    /// instead of panicking.
    pub fn replay(&self) {
        if self.audio_ctxt.state() == web_sys::AudioContextState::Running {
            return;
        }
        let future = JsFuture::from(
            self.audio_ctxt
                .resume()
                .expect("Could not resume the stream"),
        );
        spawn_local(async {
            match future.await {
                Ok(value) => assert!(value.is_undefined()),
                Err(value) => console_log(&format!(
                    "[cpal] AudioContext.resume() rejected again: {:?}",
                    value
                )),
            }
        });
    }
}

// Index within the `streams` array of the events loop.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamId(usize);

pub type SupportedInputConfigs = ::std::vec::IntoIter<SupportedStreamConfigRange>;
pub type SupportedOutputConfigs = ::std::vec::IntoIter<SupportedStreamConfigRange>;

const MIN_CHANNELS: u16 = 1;
const MAX_CHANNELS: u16 = 32;
const MIN_SAMPLE_RATE: SampleRate = SampleRate(8_000);
const MAX_SAMPLE_RATE: SampleRate = SampleRate(96_000);
const DEFAULT_SAMPLE_RATE: SampleRate = SampleRate(44_100);
const MIN_BUFFER_SIZE: u32 = 1;
const MAX_BUFFER_SIZE: u32 = u32::MAX;
const DEFAULT_BUFFER_SIZE: usize = 4096; // ~93 ms @44.1k: the look-ahead absorbs the main-thread render hiccups
const SUPPORTED_SAMPLE_FORMAT: SampleFormat = SampleFormat::F32;

impl Host {
    pub fn new() -> Result<Self, crate::HostUnavailable> {
        Ok(Host)
    }
}

impl Devices {
    fn new() -> Result<Self, DevicesError> {
        Ok(Self::default())
    }
}

impl Device {
    #[inline]
    fn name(&self) -> Result<String, DeviceNameError> {
        Ok("Default Device".to_owned())
    }

    #[inline]
    fn supported_input_configs(
        &self,
    ) -> Result<SupportedInputConfigs, SupportedStreamConfigsError> {
        unimplemented!();
    }

    #[inline]
    fn supported_output_configs(
        &self,
    ) -> Result<SupportedOutputConfigs, SupportedStreamConfigsError> {
        let buffer_size = SupportedBufferSize::Range {
            min: MIN_BUFFER_SIZE,
            max: MAX_BUFFER_SIZE,
        };
        let configs: Vec<_> = (MIN_CHANNELS..=MAX_CHANNELS)
            .map(|channels| SupportedStreamConfigRange {
                channels,
                min_sample_rate: MIN_SAMPLE_RATE,
                max_sample_rate: MAX_SAMPLE_RATE,
                buffer_size: buffer_size.clone(),
                sample_format: SUPPORTED_SAMPLE_FORMAT,
            })
            .collect();
        Ok(configs.into_iter())
    }

    fn default_input_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError> {
        unimplemented!();
    }

    fn default_output_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError> {
        const EXPECT: &str = "expected at least one valid webaudio stream config";
        let config = self
            .supported_output_configs()
            .expect(EXPECT)
            .max_by(|a, b| a.cmp_default_heuristics(b))
            .unwrap()
            .with_sample_rate(DEFAULT_SAMPLE_RATE);

        Ok(config)
    }
}

impl HostTrait for Host {
    type Devices = Devices;
    type Device = Device;

    fn is_available() -> bool {
        // Assume this host is always available on emscripten.
        true
    }

    fn devices(&self) -> Result<Self::Devices, DevicesError> {
        Devices::new()
    }

    fn default_input_device(&self) -> Option<Self::Device> {
        default_input_device()
    }

    fn default_output_device(&self) -> Option<Self::Device> {
        default_output_device()
    }
}

impl DeviceTrait for Device {
    type SupportedInputConfigs = SupportedInputConfigs;
    type SupportedOutputConfigs = SupportedOutputConfigs;
    type Stream = Stream;

    fn name(&self) -> Result<String, DeviceNameError> {
        Device::name(self)
    }

    fn supported_input_configs(
        &self,
    ) -> Result<Self::SupportedInputConfigs, SupportedStreamConfigsError> {
        Device::supported_input_configs(self)
    }

    fn supported_output_configs(
        &self,
    ) -> Result<Self::SupportedOutputConfigs, SupportedStreamConfigsError> {
        Device::supported_output_configs(self)
    }

    fn default_input_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError> {
        Device::default_input_config(self)
    }

    fn default_output_config(&self) -> Result<SupportedStreamConfig, DefaultStreamConfigError> {
        Device::default_output_config(self)
    }

    fn build_input_stream_raw<D, E>(
        &self,
        _config: &StreamConfig,
        _sample_format: SampleFormat,
        _data_callback: D,
        _error_callback: E,
        _timeout: Option<Duration>,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        D: FnMut(&Data, &InputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static,
    {
        unimplemented!()
    }

    fn build_output_stream_raw<D, E>(
        &self,
        config: &StreamConfig,
        sample_format: SampleFormat,
        data_callback: D,
        _error_callback: E,
        _timeout: Option<Duration>,
    ) -> Result<Self::Stream, BuildStreamError>
    where
        D: FnMut(&mut Data, &OutputCallbackInfo) + Send + 'static,
        E: FnMut(StreamError) + Send + 'static,
    {
        if !valid_config(config, sample_format) {
            return Err(BuildStreamError::StreamConfigNotSupported);
        }

        let buffer_size_frames = match config.buffer_size {
            BufferSize::Fixed(v) => {
                if v == 0 {
                    return Err(BuildStreamError::StreamConfigNotSupported);
                } else {
                    v as usize
                }
            }
            BufferSize::Default => DEFAULT_BUFFER_SIZE,
        };

        // Create the stream.
        let audio_ctxt = AudioContext::new().expect("webaudio is not present on this system");
        let stream = Stream { audio_ctxt };

        // Use `set_timeout` to invoke a Rust callback repeatedly.
        //
        // The job of this callback is to fill the content of the audio buffers.
        //
        // See also: The call to `set_timeout` at the end of the `audio_callback_fn` which creates
        // the loop.
        set_timeout(
            10,
            stream.clone(),
            data_callback,
            config,
            sample_format,
            buffer_size_frames as u32,
        );

        Ok(stream)
    }
}

impl StreamTrait for Stream {
    fn play(&self) -> Result<(), PlayStreamError> {
        let future = JsFuture::from(
            self.audio_ctxt
                .resume()
                .expect("Could not resume the stream"),
        );
        // 09 fix (tinyhexagon) — the resume rejection is NOT fatal: the
        // C++-era panic here killed the whole game on a browser that
        // rejects a resume outside a user activation. Log it; the page's
        // next gesture re-resumes (`Stream::replay`).
        spawn_local(async {
            match future.await {
                Ok(value) => assert!(value.is_undefined()),
                Err(value) => console_log(&format!(
                    "[cpal] AudioContext.resume() rejected (no activation?); retrying on the next gesture: {:?}",
                    value
                )),
            }
        });
        Ok(())
    }

    fn pause(&self) -> Result<(), PauseStreamError> {
        let future = JsFuture::from(
            self.audio_ctxt
                .suspend()
                .expect("Could not suspend the stream"),
        );
        spawn_local(async {
            match future.await {
                Ok(value) => assert!(value.is_undefined()),
                Err(value) => panic!("AudioContext.suspend() promise was rejected: {:?}", value),
            }
        });
        Ok(())
    }
}

// 08a fix 2 (wasm-bindgen 0.2.129 runtime ABI): `sample_format` is
// captured by value (it is `Copy`) instead of crossing the JS timer as a
// `Closure<FnOnce() -> SampleFormat>` argument — the 0.2.129 timer
// invocation converts a JS function argument to the `SampleFormat` u32
// ABI, where a JS function coerces to `0` (`I8`), so the callback's
// `Data` carried the wrong sample format and the typed wrapper in
// `traits.rs` panicked ("host supplied incorrect sample type").
fn audio_callback_fn<D>(
    mut data_callback: D,
    sample_format: SampleFormat,
    config: StreamConfig,
    buffer_size_frames: u32,
    stream: Stream,
) -> impl FnOnce()
where
    D: FnMut(&mut Data, &OutputCallbackInfo) + Send + 'static,
{
    // 09 fix (tinyhexagon) — the host's playback wiring had two defects:
    // (a) every `AudioBufferSourceNode` was dropped right after
    //     `start()` — a GC'd node can stop mid-playback: silence on real
    //     browsers while the context still reads "running";
    // (b) `node.start()` played AT CALL TIME on a main-thread timer —
    //     overlap/gaps under jitter.
    // The fix: hold the live sources (the GC cannot reap them) and
    // schedule each buffer on the AUDIO THREAD's clock with gapless
    // look-ahead (`start_with_when`), so timer drift only changes the
    // look-ahead depth, never the playback continuity.
    thread_local! {
        static LIVE_SOURCES: std::cell::RefCell<Vec<web_sys::AudioBufferSourceNode>> =
            const { std::cell::RefCell::new(Vec::new()) };
        static NEXT_START: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
    }
    move || {
        let sample_rate = config.sample_rate.0;
        let buffer_size_samples = buffer_size_frames as usize * config.channels as usize;
        let audio_ctxt = &stream.audio_ctxt;

        // TODO: We should be re-using a buffer.
        let mut temporary_buffer = vec![0f32; buffer_size_samples];

        {
            let len = temporary_buffer.len();
            let data = temporary_buffer.as_mut_ptr() as *mut ();
            let mut data = unsafe { Data::from_parts(data, len, sample_format) };
            let now_secs: f64 = audio_ctxt.current_time();
            let callback = crate::StreamInstant::from_secs_f64(now_secs);
            // TODO: Use proper latency instead. Currently, unsupported on most browsers though, so
            // we estimate based on buffer size instead. Probably should use this, but it's only
            // supported by firefox (2020-04-28).
            // let latency_secs: f64 = audio_ctxt.outputLatency.try_into().unwrap();
            let buffer_duration = frames_to_duration(len, sample_rate as usize);
            let playback = callback
                .add(buffer_duration)
                .expect("`playback` occurs beyond representation supported by `StreamInstant`");
            let timestamp = crate::OutputStreamTimestamp { callback, playback };
            let info = OutputCallbackInfo { timestamp };
            data_callback(&mut data, &info);
        }

        let typed_array: Float32Array = temporary_buffer.as_slice().into();

        // 09 instrumentation (tinyhexagon) — read the VIEW's own values
        // (it is a view over the wasm linear memory): sane values here +
        // a silent buffer at start() = the getChannelData write half is
        // broken; zeros here = the view/glue is broken (the same glue
        // family that dropped lAudioContext's const).
        {
            use std::cell::Cell;
            thread_local! { static DBG: Cell<u32> = const { Cell::new(0) }; }
            let n = DBG.get() + 1;
            DBG.set(n);
            if n % 10 == 1 {
                console_log(&format!(
                    "[cpal] copy: view.len={} view[0]={:.4} view[100]={:.4} view[1000]={:.4} temp[0]={:.4} temp[1000]={:.4}",
                    typed_array.length(),
                    typed_array.get_index(0),
                    typed_array.get_index(100),
                    typed_array.get_index(1000),
                    temporary_buffer.first().copied().unwrap_or(f32::NAN),
                    temporary_buffer.get(1000).copied().unwrap_or(f32::NAN),
                ));
            }
        }

        debug_assert_eq!(temporary_buffer.len() % config.channels as usize, 0);

        let context = audio_ctxt;
        // 09 fix — label the buffer with the CONTEXT's actual rate (the
        // old cpal-rate label resampled: a 48000 context played the
        // 44100-labeled buffer ~8% fast — a per-buffer gap under the
        // scheduler's cursor = choppy).
        let context_rate = context.sample_rate() as f64;
        let buffer = context
            .create_buffer(
                config.channels as u32,
                buffer_size_frames,
                context_rate as f32,
            )
            .expect("Buffer could not be created");
        // 09 fix (tinyhexagon) — THE WEB SILENCE: web-sys's
        // `get_channel_data` returns a `Vec<f32>` — a COPY — so the old
        // de-interleave loop wrote the samples into the copy and dropped
        // it: the AudioBuffer stayed at its zero-initialized content
        // (silence on every browser, with a perfectly healthy graph,
        // clock and loop). `copy_to_channel` writes THROUGH the live
        // AudioBuffer: de-interleave into per-channel Vecs first, then
        // copy each channel in.
        let channels = config.channels as usize;
        let frames = buffer_size_frames as usize;
        let mut chans: Vec<Vec<f32>> = (0..channels).map(|_| Vec::with_capacity(frames)).collect();
        for (i, sample) in temporary_buffer.iter().enumerate() {
            chans[i % channels].push(*sample);
        }
        for (channel, data) in chans.iter().enumerate() {
            buffer
                .copy_to_channel(data, channel as i32)
                .map_err(|e| {
                    console_log(&format!(
                        "[cpal] copy_to_channel(ch {channel}) failed: {:?}",
                        e
                    ))
                })
                .ok();
        }

        let node = context
            .create_buffer_source()
            .expect("The buffer source node could not be created");
        node.set_buffer(Some(&buffer));
        // 08a fix 3 — connect the source node TO the destination:
        // `AudioDestinationNode` has 0 outputs, so the registry source's
        // `context.destination().connect_with_audio_node(&node)` panics at
        // runtime (IndexSizeError: output index (0) exceeds number of
        // outputs (0)); `node.connect_with_audio_node(&destination)` is the
        // WebAudio direction the webaudio host uses (webaudio/mod.rs:363).
        node
            .connect_with_audio_node(&context.destination())
            .expect("Could not connect the audio node to the destination");
        // 09 fix (tinyhexagon) — gapless audio-clock scheduling + held
        // sources (see the closure-head note): the buffer plays at
        // `NEXT_START` (>= now + 20 ms), and the cursor advances by the
        // buffer's real duration; the main-thread timer only re-arms the
        // NEXT buffer's render, so its jitter cannot overlap or gap the
        // playback. The node is held until eviction (~1.6 s of history)
        // so the GC cannot reap a playing source.
        let now = audio_ctxt.current_time();
        // 09 fix — gapless: the cursor is authoritative while it is in
        // the future (the old now+0.02 clamp reset the schedule whenever
        // it came near the clock — a fresh gap per buffer); reset only
        // after a REAL stall. The cursor's step is the buffer's duration
        // AT THE CONTEXT'S RATE (the resample mismatch above).
        let buffer_duration_secs = buffer_size_frames as f64 / context_rate;
        let when = {
            let scheduled = NEXT_START.get();
            if scheduled >= now {
                scheduled
            } else {
                now + 0.05
            }
        };
        node.start_with_when(when)
            .expect("Could not start the audio node");
        NEXT_START.set(when + buffer_duration_secs);
        LIVE_SOURCES.with(|live| {
            let mut live = live.borrow_mut();
            live.push(node);
            let keep = (buffer_duration_secs.recip() * 1.6).max(4.0) as usize;
            if live.len() > keep {
                let excess = live.len() - keep;
                live.drain(0..excess);
            }
        });

        // TODO: handle latency better ; right now we just use setInterval with the amount of sound
        // data that is in each buffer ; this is obviously bad, and also the schedule is too tight
        // and there may be underflows
        set_timeout(
            (1000.0 * buffer_size_frames as f64 / context_rate) as i32,
            stream,
            data_callback,
            &config,
            sample_format,
            buffer_size_frames,
        );
    }
}

fn set_timeout<D>(
    time: i32,
    stream: Stream,
    data_callback: D,
    config: &StreamConfig,
    sample_format: SampleFormat,
    buffer_size_frames: u32,
) where
    D: FnMut(&mut Data, &OutputCallbackInfo) + Send + 'static,
{
    let window = web_sys::window().expect("Not in a window somehow?");
    // 09 fix (tinyhexagon) — ZERO JS ARGUMENTS: the timer chain used to
    // pass (stream, config, buffer_size_frames) as setTimeout arguments,
    // and the emscripten glue's argument delivery can corrupt them (the
    // user's Chrome 155 measured a garbage buffer_size_frames →
    // create_buffer(2, 0, …) → zero-length buffers → silence with a
    // perfectly healthy graph). Everything is now captured IN the
    // closure; the timer callback takes no arguments.
    window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            &Closure::once_into_js(audio_callback_fn(
                data_callback,
                sample_format,
                (*config).clone(),
                buffer_size_frames,
                stream,
            ))
            .dyn_ref::<js_sys::Function>()
            .expect("The function was somehow not a function"),
            time,
        )
        .expect("The timeout could not be set");
}

impl Default for Devices {
    fn default() -> Devices {
        // We produce an empty iterator if the WebAudio API isn't available.
        Devices(is_webaudio_available())
    }
}
impl Iterator for Devices {
    type Item = Device;
    #[inline]
    fn next(&mut self) -> Option<Device> {
        if self.0 {
            self.0 = false;
            Some(Device)
        } else {
            None
        }
    }
}

#[inline]
fn default_input_device() -> Option<Device> {
    unimplemented!();
}

#[inline]
fn default_output_device() -> Option<Device> {
    if is_webaudio_available() {
        Some(Device)
    } else {
        None
    }
}

// Detects whether the `AudioContext` global variable is available.
fn is_webaudio_available() -> bool {
    AudioContext::new().is_ok()
}

// Whether or not the given stream configuration is valid for building a stream.
fn valid_config(conf: &StreamConfig, sample_format: SampleFormat) -> bool {
    conf.channels <= MAX_CHANNELS
        && conf.channels >= MIN_CHANNELS
        && conf.sample_rate <= MAX_SAMPLE_RATE
        && conf.sample_rate >= MIN_SAMPLE_RATE
        && sample_format == SUPPORTED_SAMPLE_FORMAT
}

// Convert the given duration in frames at the given sample rate to a `std::time::Duration`.
fn frames_to_duration(frames: usize, rate: usize) -> std::time::Duration {
    let secsf = frames as f64 / rate as f64;
    let secs = secsf as u64;
    let nanos = ((secsf - secs as f64) * 1_000_000_000.0) as u32;
    std::time::Duration::new(secs, nanos)
}
