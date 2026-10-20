//! The audio ladder's cpal-only wasm rung (issue 09 isolation): a 440 Hz
//! sine through the vendored cpal emscripten host's output path —
//! `default_host` → `default_output_device` → `build_output_stream` (the
//! setTimeout + AudioBufferSourceNode loop) → `play`. NO rust-synth, NO
//! game, NO director: if this is silent on a device, the cpal emscripten
//! host itself is broken there (and the fix is in the host, not the
//! synth/game stack).
//!
//! The page (tools/webtone.html) starts it from a real gesture via the
//! exported `startTone()` (the autoplay policy) and reports the host's
//! every step to the browser console.

#[cfg(target_os = "emscripten")]
use wasm_bindgen::prelude::*;

#[cfg(target_os = "emscripten")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}

/// Build + play a 440 Hz sine (F32 stereo, cpal's hardcoded 44100 config)
/// through the emscripten host's own output path. Called from the page's
/// gesture (the autoplay policy).
#[cfg(target_os = "emscripten")]
#[wasm_bindgen(js_name = startTone)]
pub fn start_tone() -> String {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{SampleRate, StreamConfig};

    console_log(&format!(
        "webtone: ua={}",
        web_sys::window().map(|w| w.navigator().user_agent().unwrap_or_default()).unwrap_or_default()
    ));
    // Surface the constructor's REAL exception (the host's availability
    // probe discards it): create one directly and log the DOMException.
    match js_sys::eval(
        "(() => { try { return String(new (window.AudioContext||window.webkitAudioContext)().state) } catch (e) { return 'THREW: ' + (e && e.name) + ': ' + (e && e.message) } })()",
    ) {
        Ok(v) => console_log(&format!("webtone: direct ctx probe: {:?}", v)),
        Err(e) => console_log(&format!("webtone: direct ctx probe eval failed: {:?}", e)),
    }

    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else {
        let m = "webtone: no default output device".to_string();
        console_log(&m);
        return m;
    };
    let config: StreamConfig = StreamConfig {
        channels: 2,
        sample_rate: SampleRate(44_100),
        buffer_size: cpal::BufferSize::Default,
    };
    console_log(&format!("webtone: building stream ({config:?})"));

    let mut phase: f32 = 0.0;
    let mut ticks: u32 = 0;
    let built = device.build_output_stream(
        &config,
        move |data: &mut [f32], _| {
            // One line per ~0.5 s of callbacks: the console must show
            // these CONTINUING (a stalled log = the timer loop died;
            // continuing = the loop renders but the playback wiring is
            // the thief).
            ticks += 1;
            for frame in data.chunks_mut(2) {
                for s in frame.iter_mut() {
                    *s = 0.4 * phase.sin();
                }
                phase += 2.0 * std::f32::consts::PI * 440.0 / 44_100.0;
                if phase > 2.0 * std::f32::consts::PI {
                    phase -= 2.0 * std::f32::consts::PI;
                }
            }
            // Measure AFTER the fill (the pre-fill zeros fooled the last
            // instrument): 0.4 expected; 0.0 = the write itself is lost.
            let fill_max = data.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
            if ticks % 10 == 1 {
                console_log(&format!(
                    "webtone: callback tick {ticks} ({} frames) fill_max={fill_max}",
                    data.len() / 2
                ));
            }
        },
        |err| console_log(&format!("webtone: stream error: {err}")),
        None,
    );
    console_log("webtone: stream built; play()");
    match built {
        Ok(s) => match s.play() {
            Ok(()) => {
                console_log("webtone: play() ok — the sine should be audible");
                // Keep the stream alive (a dropped stream stops it).
                std::mem::forget(s);
                "webtone: playing".to_string()
            }
            Err(e) => {
                let m = format!("webtone: play() failed: {e}");
                console_log(&m);
                m
            }
        },
        Err(e) => {
            let m = format!("webtone: build failed: {e}");
            console_log(&m);
            m
        }
    }
}

/// The emscripten module's entry: an EMPTY main (the exports are the
/// surface; the page calls `startTone()` from its gesture).
#[cfg(target_os = "emscripten")]
fn main() {}

#[cfg(not(target_os = "emscripten"))]
pub fn main() {
    println!("the webtone rung builds for wasm32-unknown-emscripten only");
}
