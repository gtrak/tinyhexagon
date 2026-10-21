//! The emscripten (web) output backend — emscripten's **OpenAL** (`-lopenal`),
//! the SFML-parity stack: SFML's audio module *is* OpenAL, and emscripten's
//! OpenAL maps onto Web Audio.
//!
//! Why OpenAL and not `rodio`/`cpal`: `cpal`'s browser host is gated on
//! `target_os = "unknown"` (the wasm-bindgen path), so
//! `wasm32-unknown-emscripten` falls to cpal's **null host** — whose
//! `default_output_device()` is `None`, so `rodio::open_default_sink()` fails
//! with `NoDevice` and the web has no sound. Emscripten, however, ships a real
//! OpenAL implementation (`AL/al.h` + `libopenal.js`) that plays through Web
//! Audio, so an OpenAL sink behind the `AudioDevice` seam is the web's real
//! device — and it moves the port TOWARD the original (which uses OpenAL via
//! SFML), not away from it.
//!
//! # The autoplay policy (the "no sound" the operator heard, once the device
//! exists)
//!
//! A Web Audio `AudioContext` starts **suspended** and only runs after a user
//! gesture. Emscripten's `alcCreateContext` already installs a one-shot
//! `keydown`/`mousedown`/`touchstart` listener (its `autoResumeAudioContext`)
//! that resumes the context on the first input, so the OpenAL device unlocks
//! itself. This module ALSO keeps a tiny probe: on the first gesture it opens
//! a throwaway `AudioContext` and reads its state, so the diagnostic can tell
//! "device present but suspended" from "device present and running" (the
//! whole answer to the operator's "there appears to be no sound there"). The
//! probe context is separate from OpenAL's and is dropped after the read —
//! OpenAL's context (the one actually producing sound) is Emscripten's.
//!
//! # The SFML-unit volume mapping
//!
//! `volume / 100.0` (SFML `0..=100` → linear `0..=1`) is applied here exactly
//! as the rodio backend does (issue 039's device evidence: 100 SFML units →
//! 1.0), so the desktop and web paths agree on the gain the device plays at.
//!
//! # The one-mixer plan (plan 002 issue 09)
//!
//! This device's context is the web's ONE audio context: the SFX one-shots
//! AND the streamed music (the in-memory synth's rendered frames, the
//! `MusicSource` — a dedicated `AL_STREAMING` source fed with
//! `alBufferData` + `alSourceQueueBuffers`) play through it. One context,
//! one gesture unlock (emscripten's `autoResumeAudioContext` fires on the
//! same first input the SFX's probe listener records), the SFX and the
//! music mixed by the browser's Web Audio graph. The context is created at
//! `MUSIC_RATE` (48 kHz — the in-memory engine's render rate, so the music
//! stream needs no resample; a browser that rejects the rate option falls
//! back to the default-rate context, where Web Audio resamples the
//! 48 kHz-labeled buffers).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use wasm_bindgen::JsCast;

use crate::audio::{
    emit_diag, AudioOpenError, AudioPlayer, DeviceBackend, MusicSource, MusicSourceBackend,
    PlayerBackend,
};

// ---------------------------------------------------------------------------
// OpenAL constants (the emscripten `AL/al.h` values — the source of truth for
// this build's `-lopenal`).
// ---------------------------------------------------------------------------
const AL_FALSE: u32 = 0;
const AL_TRUE: u32 = 1;
const AL_FORMAT_MONO16: u32 = 0x1101;
const AL_FORMAT_STEREO16: u32 = 0x1103;
const AL_PITCH: u32 = 0x1003;
const AL_LOOPING: u32 = 0x1007;
const AL_BUFFER: u32 = 0x1009;
const AL_GAIN: u32 = 0x100A;
const AL_SOURCE_STATE: u32 = 0x1010;
const AL_SOURCE_RELATIVE: u32 = 0x202;
const AL_PLAYING: u32 = 0x1012;
const AL_STOPPED: u32 = 0x1014;
const AL_PAUSED: u32 = 0x1013;
const AL_INITIAL: u32 = 0x1011;
const AL_BUFFERS_PROCESSED: u32 = 0x1016;
/// The ALC context attribute: the Web Audio context's sample rate (the
/// one-mixer plan: the context is created at the in-memory engine's render
/// rate so the music stream needs no resample).
const ALC_FREQUENCY: i32 = 0x1007;
/// The music stream's render rate (48 kHz — the in-memory engine's fixed
/// rate; the buffer labels + the context's `ALC_FREQUENCY` attribute).
const MUSIC_RATE: u32 = 48_000;
/// One music-stream buffer's frames (21.3 ms at 48 kHz).
const MUSIC_BUFFER_FRAMES: usize = 1024;
/// The queue depth the feed keeps topped up (~128 ms — ahead of the
/// shim's 100 ms scheduling lookahead, so the 25 ms scheduler tick never
/// underruns a topped-up queue).
const MUSIC_TARGET_QUEUED: usize = 6; // ~128 ms of look-ahead: the DEPTH-3 experiment (the operator's latency test) chopped in the user's Chrome — the main-thread hiccup tolerance is the depth, so this is the floor; the visual-lead fix is the clock compensation, not the depth
/// The per-feed burst cap (the rAF catch-up after a hidden tab).
const MUSIC_MAX_BURST: usize = 4;
/// The buffer pool (target + the in-flight margin; `feed` never queues
/// past the pool, so a free id is always available while topping up).
const MUSIC_POOL: usize = 8;

const AL_NONE: u32 = 0;
const AL_INVALID_NAME: u32 = 0x4001;
const AL_INVALID_VALUE: u32 = 0x4002;
const AL_INVALID_ENUM: u32 = 0x4003;
const AL_INVALID_OPERATION: u32 = 0x4004;

fn al_error_str(code: u32) -> &'static str {
    match code {
        AL_NONE => "AL_NONE",
        AL_INVALID_NAME => "AL_INVALID_NAME",
        AL_INVALID_VALUE => "AL_INVALID_VALUE",
        AL_INVALID_ENUM => "AL_INVALID_ENUM",
        AL_INVALID_OPERATION => "AL_INVALID_OPERATION",
        _ => "AL_(unknown)",
    }
}

fn source_state_str(state: u32) -> &'static str {
    match state {
        AL_PLAYING => "AL_PLAYING",
        AL_STOPPED => "AL_STOPPED",
        AL_PAUSED => "AL_PAUSED",
        AL_INITIAL => "AL_INITIAL",
        _ => "AL_(unknown)",
    }
}

// ---------------------------------------------------------------------------
// The OpenAL C ABI. Emscripten's `AL*` opaque handles are plain
// pointer-sized values; they are stored as `usize` (below) so the backend
// stays `Send + Sync` without `unsafe impl` on raw pointers.
// ---------------------------------------------------------------------------
extern "C" {
    fn alcOpenDevice(devicename: *const i8) -> *mut core::ffi::c_void;
    fn alcCloseDevice(device: *mut core::ffi::c_void) -> u32;
    fn alcCreateContext(
        device: *mut core::ffi::c_void,
        attrlist: *const i32,
    ) -> *mut core::ffi::c_void;
    fn alcMakeContextCurrent(context: *mut core::ffi::c_void) -> u32;
    fn alcDestroyContext(context: *mut core::ffi::c_void);
    fn alcGetIntegerv(device: *mut core::ffi::c_void, param: i32, size: i32, values: *mut i32);

    fn alGenBuffers(n: i32, buffers: *mut u32);
    fn alBufferData(buffer: u32, format: u32, data: *const core::ffi::c_void, size: i32, freq: i32);
    fn alDeleteBuffers(n: i32, buffers: *const u32);
    fn alGenSources(n: i32, sources: *mut u32);
    fn alDeleteSources(n: i32, sources: *const u32);
    fn alSourcei(source: u32, param: u32, value: i32);
    fn alSourcef(source: u32, param: u32, value: f32);
    fn alSource3f(source: u32, param: u32, v1: f32, v2: f32, v3: f32);
    fn alSourcePlay(source: u32);
    fn alSourceStop(source: u32);
    // The REAL OpenAL signatures: (source, n, buffers) — a wrong
    // (n, buffer_ids) declaration misaligns the FFI (the source id lands
    // in the count slot, the count gets the data pointer, the data slot
    // gets undefined): the queue silently never fills (the probe's
    // buffers=4 was the kick SFX — the music never played).
    fn alSourceQueueBuffers(source: u32, n: i32, buffer_ids: *const u32);
    fn alSourceUnqueueBuffers(source: u32, n: i32, buffer_ids: *mut u32);
    fn alGetSourcei(source: u32, param: u32, value: *mut i32);
    fn alGetError() -> u32;
}

/// The mutable per-device diagnostic / state (shared with the gesture closure
/// via `Arc` so the closure does not capture the backend by reference).
struct State {
    /// The Web Audio context state: "suspended" until the first gesture, then
    /// "running" (a successful unlock) or "suspended" (the resume was blocked,
    /// e.g. a synthetic/CDP input that grants no user activation).
    context_state: String,
    /// The AL source state after the last `alSourcePlay` (the audibility
    /// observable: `AL_PLAYING` means the source is scheduled).
    last_source_state: Option<String>,
    /// The `alGetError()` after the last play (`AL_NONE` = clean).
    last_al_error: Option<String>,
    /// Whether any play has happened yet (gates the "first play" diagnostic).
    has_played: bool,
}

/// The emscripten OpenAL output device (the `AudioDevice`'s web backend).
pub struct OpenalBackend {
    /// `ALCdevice*` (as `usize`; `Send` without `unsafe impl`).
    device: usize,
    /// `ALCcontext*` (as `usize`).
    context: usize,
    state: Arc<Mutex<State>>,
    /// The SFX stream's state (the ONE-PATH endgame — the same MusicState
    /// machinery as the music's stream, its own source + pool).
    sfx: Arc<Mutex<MusicState>>,
    /// The music streaming source's state (the one-mixer plan's music half
    /// — the dedicated `AL_STREAMING` source + its buffer pool, shared
    /// with the cloned [`MusicSource`] handles).
    music: Arc<Mutex<MusicState>>,
    /// Set once, on the first user gesture (the unlock).
    unlocked: Arc<AtomicBool>,
    /// The one-shot gesture listeners — kept alive for the device's life
    /// (a dropped `Closure` disables its listener).
    listeners: Vec<wasm_bindgen::closure::Closure<dyn FnMut(web_sys::Event)>>,
}

// `DeviceBackend: Send + Sync` (the seam bound, so `th-audio` can hold the
// device behind its `Mutex`) requires `OpenalBackend: Send + Sync`. Its only
// `!Send + !Sync` field is the `listeners` `Closure` vec (a wasm-bindgen
// closure is not thread-shareable). That is sound on the web: the build is
// single-threaded, so the closures are never actually shared across OS
// threads — the bound is satisfied vacuously here (the rodio backend, the
// genuinely multi-threaded path, needs no such impl).
unsafe impl Send for OpenalBackend {}
unsafe impl Sync for OpenalBackend {}

impl OpenalBackend {
    /// Open the default OpenAL device + context. Fails (→ world (a), "no
    /// device") when the browser has no Web Audio `AudioContext`.
    pub fn open() -> Result<Self, AudioOpenError> {
        // `alcOpenDevice(NULL)` = the default device. Emscripten returns NULL
        // when `globalThis.AudioContext` is undefined (no Web Audio).
        let device = unsafe { alcOpenDevice(core::ptr::null()) };
        if device.is_null() {
            return Err(AudioOpenError {
                detail: "no Web Audio AudioContext (alcOpenDevice returned NULL)".to_string(),
            });
        }
        // The ONE-PATH endgame (the operator's "one sample rate, convert
        // on our end"): the context is created at the DEVICE'S NATURAL
        // rate (no `ALC_FREQUENCY` demand) — the 48 kHz demand produced
        // "The AudioContext encountered an error from the audio device"
        // on the user's Chrome 155 (the renderer froze at the first
        // resume; everything scheduled on it stayed silent). OUR side
        // converts: the feeds resample the 48 kHz engine/mixer frames to
        // the context's actual rate (read below) and label the buffers
        // with it.
        let context = unsafe { alcCreateContext(device, core::ptr::null()) };
        if context.is_null() {
            unsafe {
                alcCloseDevice(device);
            }
            return Err(AudioOpenError {
                detail: "alcCreateContext returned NULL".to_string(),
            });
        }
        unsafe {
            alcMakeContextCurrent(context);
        }
        // The context's ACTUAL rate (the device's natural one): both
        // streams' feeds convert to it.
        let mut context_rate: i32 = 48_000;
        unsafe {
            alcGetIntegerv(device, ALC_FREQUENCY, 1, &mut context_rate);
        }
        let context_rate = if (8_000..=96_000).contains(&context_rate) {
            context_rate as u32
        } else {
            48_000
        };
        // The music streaming source (the one-mixer plan's music half):
        // ONE dedicated `AL_STREAMING` source on this context (the SFX and
        // the music share the context + its gesture unlock). Created
        // stopped — the web frame timer queues the in-memory engine's
        // frames into it and plays it (post-gesture, when the director
        // goes live).
        let mut music_source_id: u32 = 0;
        unsafe {
            alGenSources(1, &mut music_source_id);
            alSourcei(music_source_id, AL_SOURCE_RELATIVE, AL_TRUE as i32);
            alSourcef(music_source_id, AL_GAIN, 1.0);
        }
        let mut pool = Vec::with_capacity(MUSIC_POOL);
        for _ in 0..MUSIC_POOL {
            let mut id: u32 = 0;
            unsafe {
                alGenBuffers(1, &mut id);
            }
            if id != 0 {
                pool.push(id);
            }
        }
        // The free list starts as the whole pool (`pop_back` hands out the
        // ids, most-recently-allocated first).
        let free: VecDeque<u32> = pool.iter().copied().collect();
        let music = Arc::new(Mutex::new(MusicState {
            source: music_source_id,
            pool,
            free,
            queued: 0,
            rate: context_rate,
        }));
        // The SFX stream (the ONE-PATH endgame, the operator's call): every
        // SFX + generated clink is OUR 48 kHz stereo PCM, mixed by
        // th-audio and streamed through THIS source with the same
        // queue mechanics the music uses — the only delivery the user's
        // Chrome 155 has never failed to sound. The one-shot
        // alGenSources+play path (the shim's per-sound sources) is
        // retired on the web.
        let mut sfx_source_id: u32 = 0;
        unsafe {
            alGenSources(1, &mut sfx_source_id);
            alSourcei(sfx_source_id, AL_SOURCE_RELATIVE, AL_TRUE as i32);
            alSourcef(sfx_source_id, AL_GAIN, 1.0);
        }
        let mut sfx_pool = Vec::with_capacity(MUSIC_POOL);
        for _ in 0..MUSIC_POOL {
            let mut id: u32 = 0;
            unsafe {
                alGenBuffers(1, &mut id);
            }
            if id != 0 {
                sfx_pool.push(id);
            }
        }
        let sfx = Arc::new(Mutex::new(MusicState {
            source: sfx_source_id,
            free: sfx_pool.iter().copied().collect(),
            pool: sfx_pool,
            queued: 0,
            rate: context_rate,
        }));
        let state = Arc::new(Mutex::new(State {
            context_state: "suspended".to_string(), // the autoplay-policy default
            last_source_state: None,
            last_al_error: None,
            has_played: false,
        }));
        let unlocked = Arc::new(AtomicBool::new(false));

        let mut backend = Self {
            device: device as usize,
            context: context as usize,
            listeners: Vec::new(),
            state: state.clone(),
            unlocked: unlocked.clone(),
            music: music.clone(),
            sfx: sfx.clone(),
        };

        // The world + initial-context diagnostic (visible on the web via
        // console.log — the `eprintln!` stdio path is swallowed).
        emit_diag(
            "[th-audio] device present (OpenAL/Web Audio); context suspended (awaits first input)",
        );

        // The gesture-unlock probe: on the FIRST key/pointer input, open a
        // throwaway AudioContext (this call happens inside the gesture, so it
        // carries user activation) and read its state — the "suspended vs
        // running" observable. Emscripten's own auto-resume unlocks the OpenAL
        // context on the same event.
        install_gesture_probe(&mut backend, &state, &unlocked);

        Ok(backend)
    }

    /// The shared play path (one-shot vs looped).
    fn play_inner(
        &self,
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
        volume: f32,
        pitch: f32,
        looping: bool,
        position: Option<(f32, f32, f32)>,
    ) -> Option<AudioPlayer> {
        let channels = std::num::NonZero::new(channels)?;
        let sample_rate = std::num::NonZero::new(sample_rate)?;
        if samples.is_empty() {
            return None;
        }

        // f32 → i16 (the standard 16-bit OpenAL format; the SFX are short and
        // 16-bit is the CD-quality ceiling). Interleaved in, interleaved out.
        let i16: Vec<i16> = samples
            .iter()
            .map(|s| {
                let v = (s.clamp(-1.0, 1.0) * 32767.0).round();
                v as i16
            })
            .collect();

        // Upload the buffer (alBufferData copies synchronously into a Web
        // Audio `AudioBuffer`, so the `i16` vec can be dropped after).
        let mut buffer_id: u32 = 0;
        let format = if channels.get() == 1 {
            AL_FORMAT_MONO16
        } else {
            AL_FORMAT_STEREO16
        };
        // `size` is in BYTES (emscripten's alBufferData: frames = size >> 1 for
        // mono16, size >> 2 for stereo16).
        let size_bytes = (i16.len() * std::mem::size_of::<i16>()) as i32;
        unsafe {
            alGenBuffers(1, &mut buffer_id);
            alBufferData(
                buffer_id,
                format,
                i16.as_ptr() as *const core::ffi::c_void,
                size_bytes,
                sample_rate.get() as i32,
            );
        }

        // A 2D (relative) source at full gain — the game's SFX are not
        // positional. `AL_SOURCE_RELATIVE` keeps the listener's 3D position
        // from attenuating it.
        let mut source_id: u32 = 0;
        unsafe {
            alGenSources(1, &mut source_id);
            alSourcei(source_id, AL_BUFFER, buffer_id as i32);
            alSourcei(source_id, AL_SOURCE_RELATIVE, AL_TRUE as i32);
            alSourcei(
                source_id,
                AL_LOOPING,
                if looping { AL_TRUE } else { AL_FALSE } as i32,
            );
            alSourcef(source_id, AL_PITCH, pitch);
            alSourcef(source_id, AL_GAIN, (volume / 100.0).clamp(0.0, 1.0));
            alSourcePlay(source_id);
        }

        // The audibility observables: the source state + the AL error read
        // AFTER the play (AL_NONE = the play was accepted cleanly).
        let mut src_state: i32 = 0;
        let al_err_code: u32;
        unsafe {
            alGetSourcei(source_id, AL_SOURCE_STATE, &mut src_state);
            al_err_code = alGetError();
        }
        let src_state = source_state_str(src_state as u32).to_string();
        let al_err = al_error_str(al_err_code).to_string();

        // The "first play" diagnostic (the audibility evidence line): the
        // source is scheduled (AL_PLAYING) and the AL call was clean (AL_NONE).
        {
            let mut g = self.state.lock().expect("audio state poisoned");
            g.last_source_state = Some(src_state.clone());
            g.last_al_error = Some(al_err.clone());
            if !g.has_played {
                g.has_played = true;
                emit_diag(&format!(
                    "[th-audio] first play: source {src_state}, al error {al_err}, {} samples @ {}Hz {}ch, gain {:.2}",
                    samples.len(),
                    sample_rate.get(),
                    channels.get(),
                    (volume / 100.0).clamp(0.0, 1.0)
                ));
            }
        }

        Some(AudioPlayer {
            backend: Box::new(OpenalPlayer {
                source_id,
                buffer_id,
            }),
        })
    }
}

impl DeviceBackend for OpenalBackend {
    fn is_ready(&self) -> bool {
        true
    }

    fn play_one_shot(
        &self,
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
        volume: f32,
        pitch: f32,
    ) -> Option<AudioPlayer> {
        self.play_inner(sample_rate, channels, samples, volume, pitch, false, None)
    }

    fn play_music(
        &self,
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
        volume: f32,
        pitch: f32,
        _offset_secs: f32,
    ) -> Option<AudioPlayer> {
        // Emscripten's OpenAL `al.h` has no settable `AL_OFFSET` /
        // `AL_SEC_OFFSET`, so a looped source starts at 0 (the shipped build
        // has no music content, so this is not observable in practice).
        self.play_inner(sample_rate, channels, samples, volume, pitch, true, None)
    }

    fn unlock(&self) {
        // The unlock is driven by the first-gesture probe listener (and by
        // Emscripten's own auto-resume on the same event). This explicit hook
        // is the no-op "I was asked to unlock" — the real resume happens in
        // the gesture listener / Emscripten. (Provided so the seam's contract
        // is uniform across backends.)
        let _ = self.unlocked;
    }

    fn sfx_source(&self) -> MusicSource {
        // The ONE-PATH endgame's SFX half: the same streaming mechanics,
        // th-audio mixes the one-shots into it (the one-shot
        // alGenSources path retired on the web).
        MusicSource {
            inner: Some(Arc::new(OpenalMusicSource {
                state: self.sfx.clone(),
            })),
        }
    }

    fn music_source(&self) -> MusicSource {
        // The one-mixer plan's music half: the dedicated streaming source
        // on THIS device's context (the SFX and the music share the one
        // context + the one gesture unlock). The handle is a cheap Arc
        // clone — the web frame timer feeds it per rAF.
        MusicSource {
            inner: Some(Arc::new(OpenalMusicSource {
                state: self.music.clone(),
            })),
        }
    }

    fn context_state(&self) -> String {
        // The status probe's `ctx=` field: the shim's own probe context's
        // recorded state ("suspended" until the first gesture, then the
        // throwaway AudioContext's read — the OpenAL context itself is not
        // reachable through the AL C API, and both contexts resume on the
        // same first input — emscripten's `autoResumeAudioContext`).
        self.state
            .lock()
            .expect("audio state poisoned")
            .context_state
            .clone()
    }

    fn describe(&self) -> String {
        let g = self.state.lock().expect("audio state poisoned");
        let src = g.last_source_state.as_deref().unwrap_or("(no play yet)");
        let err = g.last_al_error.as_deref().unwrap_or("(no play yet)");
        format!(
            "device present (OpenAL/Web Audio); context {}; last source {src}; al error {err}",
            g.context_state
        )
    }
}

impl Drop for OpenalBackend {
    fn drop(&mut self) {
        // Drop the gesture listeners, stop + free the music streaming
        // source (the buffers with it), then tear down the AL context /
        // device.
        self.listeners.clear();
        {
            let music = self.music.lock().expect("music state poisoned");
            unsafe {
                alSourceStop(music.source);
                alDeleteSources(1, &music.source);
                if !music.pool.is_empty() {
                    alDeleteBuffers(music.pool.len() as i32, music.pool.as_ptr());
                }
            }
        }
        let context = self.context as *mut core::ffi::c_void;
        let device = self.device as *mut core::ffi::c_void;
        unsafe {
            alcMakeContextCurrent(core::ptr::null_mut());
            if !context.is_null() {
                alcDestroyContext(context);
            }
            if !device.is_null() {
                alcCloseDevice(device);
            }
        }
    }
}

/// The music streaming source's state (the one-mixer plan's web half):
/// the dedicated `AL_STREAMING` source's queue bookkeeping. The queue
/// holds rendered chunks of the in-memory engine's output; the shim's
/// 25 ms scheduler tick schedules the queued buffers onto the context's
/// Web Audio clock up to its 100 ms lookahead, so a topped-up queue
/// (≥ the lookahead) plays gapless — the main-thread feed only ever
/// replenishes, never schedules.
struct MusicState {
    /// The dedicated source (created stopped at device open; the feed
    /// plays it once the queue has content).
    source: u32,
    /// The context's actual sample rate (the device's natural rate): the
    /// feeds convert the 48 kHz engine frames to this and label the
    /// buffers with it (the ONE-PATH "convert on our end").
    rate: u32,
    /// Every buffer id the pool owns (the free + the queued ones).
    pool: Vec<u32>,
    /// The ids ready to be (re)uploaded (hand-out order: `pop_back`).
    free: VecDeque<u32>,
    /// Buffers currently in the source's queue (our bookkeeping — the
    /// shim's zero-buffer sentinel is internal to it).
    queued: usize,
}

/// Linear-resample interleaved stereo `f32` from `from` to `to` Hz (the
/// ONE-PATH conversion — the SFX mixer's stream_pcm does the same for
/// the decoded assets; this one lives with the feeds).
fn resample_stereo(frames: &[f32], from: u32, to: u32) -> Vec<f32> {
    let ratio = f64::from(to) / f64::from(from);
    let out_frames = ((frames.len() / 2) as f64 * ratio) as usize;
    let mut out = Vec::with_capacity(out_frames * 2);
    for i in 0..out_frames {
        let t = i as f64 / ratio;
        let i0 = (t as usize).min(frames.len() / 2 - 1);
        let i1 = (i0 + 1).min(frames.len() / 2 - 1);
        let f = (t - i0 as f64) as f32;
        let l0 = frames[i0 * 2];
        let r0 = frames[i0 * 2 + 1];
        let l1 = frames[i1 * 2];
        let r1 = frames[i1 * 2 + 1];
        out.push(l0 + (l1 - l0) * f);
        out.push(r0 + (r1 - r0) * f);
    }
    out
}

/// The OpenAL half of a [`MusicSource`] (the dedicated streaming source's
/// queue bookkeeping, behind the shared `MusicState`).
struct OpenalMusicSource {
    state: Arc<Mutex<MusicState>>,
}

impl MusicSourceBackend for OpenalMusicSource {
    fn desired_frames(&self) -> usize {
        let g = self.state.lock().expect("music state poisoned");
        let deficit = MUSIC_TARGET_QUEUED.saturating_sub(g.queued);
        deficit.min(MUSIC_MAX_BURST) * MUSIC_BUFFER_FRAMES
    }

    /// The reclaim pass: recycle the buffers the shim has played back
    /// into the pool. MUST run every pump — the shim auto-stops a
    /// drained (non-looping) queue, and the top-up's `want` is computed
    /// from OUR queued count, which only this decrements: a pump that
    /// skips this once the target is reached deadlocks the stream at
    /// the target depth (the probe's frozen buffers=10 — the one-mixer
    /// issue's second FFI-adjacent bug).
    fn reclaim(&self) {
        let mut g = self.state.lock().expect("music state poisoned");
        let mut processed: i32 = 0;
        unsafe {
            alGetSourcei(g.source, AL_BUFFERS_PROCESSED, &mut processed);
        }
        for _ in 0..processed.max(0) {
            let mut id: u32 = 0;
            unsafe {
                alSourceUnqueueBuffers(g.source, 1, &mut id);
            }
            if id != 0 {
                g.queued = g.queued.saturating_sub(1);
                g.free.push_back(id);
            }
        }
    }

    fn feed(&self, sample_rate: u32, frames: &[f32]) {
        self.reclaim();
        if frames.is_empty() {
            return;
        }
        let mut g = self.state.lock().expect("music state poisoned");
        let mut queued_here = 0usize;
        for chunk in frames.chunks(MUSIC_BUFFER_FRAMES * 2) {
            let Some(id) = g.free.pop_back() else {
                break; // the pool is exhausted (cannot happen: pool > target)
            };
            // f32 → i16 (the standard 16-bit OpenAL format — the SFX's
            // `play_inner`'s same conversion).
            let i16: Vec<i16> = chunk
                .iter()
                .map(|s| {
                    let v = (s.clamp(-1.0, 1.0) * 32767.0).round();
                    v as i16
                })
                .collect();
            let size_bytes = (i16.len() * std::mem::size_of::<i16>()) as i32;
            unsafe {
                alBufferData(
                    id,
                    AL_FORMAT_STEREO16,
                    i16.as_ptr() as *const core::ffi::c_void,
                    size_bytes,
                    g.rate as i32,
                );
                alSourceQueueBuffers(g.source, 1, &id);
            }
            g.queued += 1;
            queued_here += 1;
        }
        // Keep it playing: a stopped source (an underrun, or the
        //    pre-first-play state) needs the explicit start once the
        //    queue has content again.
        if queued_here > 0 {
            let mut src_state: i32 = AL_INITIAL as i32;
            unsafe {
                alGetSourcei(g.source, AL_SOURCE_STATE, &mut src_state);
            }
            if src_state != AL_PLAYING as i32 {
                unsafe {
                    alSourcePlay(g.source);
                }
            }
        }
    }

    fn is_streaming(&self) -> bool {
        let g = self.state.lock().expect("music state poisoned");
        let mut src_state: i32 = AL_INITIAL as i32;
        unsafe {
            alGetSourcei(g.source, AL_SOURCE_STATE, &mut src_state);
        }
        src_state == AL_PLAYING as i32
    }
}

/// Install the one-shot first-gesture probe listeners (keydown + pointer events
/// on `window`). The closures capture the shared `state` / `unlocked` (NOT the
/// backend, avoiding a self-referential borrow) and are stored on the backend
/// to stay alive.
fn install_gesture_probe(
    backend: &mut OpenalBackend,
    state: &Arc<Mutex<State>>,
    unlocked: &Arc<AtomicBool>,
) {
    let Some(window) = web_sys::window() else {
        return; // no DOM (should not happen on the web)
    };
    for ty in ["keydown", "pointerdown", "mousedown", "touchstart"] {
        let state = state.clone();
        let unlocked = unlocked.clone();
        let cb = wasm_bindgen::closure::Closure::new(move |_: web_sys::Event| {
            // Fire-once guard: the FIRST gesture unlocks; later ones no-op.
            if unlocked.swap(true, Ordering::SeqCst) {
                return;
            }
            // Open a throwaway AudioContext inside the gesture (user
            // activation present → it starts "running" when the browser
            // allows; "suspended" when a synthetic input grants none).
            let context_state = match web_sys::AudioContext::new() {
                Ok(ctx) => match ctx.state() {
                    web_sys::AudioContextState::Running => "running".to_string(),
                    web_sys::AudioContextState::Suspended => "suspended".to_string(),
                    web_sys::AudioContextState::Closed => "closed".to_string(),
                    _ => "invalid".to_string(), // the `__Invalid` (unknown) arm
                },
                Err(_) => "unavailable".to_string(),
            };
            {
                let mut g = state.lock().expect("audio state poisoned");
                g.context_state = context_state.clone();
            }
            emit_diag(&format!(
                "[th-audio] unlocked on first input; context {context_state}"
            ));
        });
        if window
            .add_event_listener_with_callback(ty, cb.as_ref().unchecked_ref::<js_sys::Function>())
            .is_ok()
        {
            backend.listeners.push(cb); // keep the closure (listener) alive
        }
    }
}

/// The OpenAL half of an [`AudioPlayer`] (an AL source + its buffer).
struct OpenalPlayer {
    source_id: u32,
    buffer_id: u32,
}

impl PlayerBackend for OpenalPlayer {
    fn stop(&self) {
        unsafe {
            alSourceStop(self.source_id);
        }
    }

    fn pause(&self) {
        // OpenAL has no per-source pause that preserves position the way the
        // SFML `Sound::pause` does on all backends; emscripten's OpenAL maps
        // pause to stop-with-offset-restore. The one-shot SFX never pause (the
        // game calls `stop`, not `pause`, on them), so this is a stop.
        unsafe {
            alSourceStop(self.source_id);
        }
    }

    fn play(&self) {
        unsafe {
            alSourcePlay(self.source_id);
        }
    }

    fn set_volume(&self, volume: f32) {
        // SFML-style `0..=100` → OpenAL `AL_GAIN`'s native linear `0..=1`
        // (the same SFML-unit gain the rodio backend uses).
        unsafe {
            alSourcef(self.source_id, AL_GAIN, (volume / 100.0).clamp(0.0, 1.0));
        }
    }

    fn set_speed(&self, pitch: f32) {
        unsafe {
            alSourcef(self.source_id, AL_PITCH, pitch);
        }
    }

    fn get_pos(&self) -> std::time::Duration {
        // Emscripten's `al.h` has no `AL_OFFSET` / `AL_SEC_OFFSET` to read, so
        // the live position is not available — the one-shot SFX's position is
        // never read by the game (only the music offset is, and the shipped
        // build has no music content).
        let _ = (self.source_id, self.buffer_id);
        std::time::Duration::ZERO
    }

    fn describe(&self) -> String {
        format!("openal source {}", self.source_id)
    }
}

impl Drop for OpenalPlayer {
    fn drop(&mut self) {
        // Free the source + buffer (the AudioBuffer is released with the
        // buffer). The context is current for the app's life (single-threaded
        // web; `OpenalBackend` outlives every player).
        unsafe {
            alSourceStop(self.source_id);
            alDeleteSources(1, &self.source_id);
            alDeleteBuffers(1, &self.buffer_id);
        }
    }
}
