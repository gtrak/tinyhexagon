# lib-cpal — cpal 0.15.3 (Rust)

Two cpals in this graph (plan 002 issue 05a):

| pin | pulled by | hosts we care about |
|---|---|---|
| **cpal 0.15.3** | `rust-synth 0.23` → `th-synth` | alsa (host), oboe (android), emscripten (web) |
| cpal 0.18.2 | rodio 0.23 → th-render/th-audio | (covered by `lib-render-stack.md`) |

Read from checked-out source:

`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cpal-0.15.3/src/`

Coexistence (the 05a blocker): both cpals' ALSA stacks declared
`links = "alsa"` (`alsa-sys 0.3.1` vs `0.4.0`). Fixed by the vendored
`vendor/alsa-sys-0.3.1` (`links = "alsa_0_3"`) + root `[patch.crates-io]` —
see `.agents/skills/lib-rust-synth.md` "ALSA coexistence patch".

## Host selection (`src/host/mod.rs`, 0.15.3)

cfg-gated modules: alsa (`linux/dragonfly/freebsd/netbsd`), coreaudio
(`macos/ios`), wasapi (+ optional asio) (`windows`), oboe (`android`),
emscripten (`emscripten`), webaudio (`wasm32` + `wasm-bindgen` feature),
null, jack (linux + `jack` feature). `default_host()` dispatches to the
platform one.

## Emscripten host (`src/host/emscripten/mod.rs`) — the web/autoplay core

- **One `AudioContext` per `Stream`**: `build_output_stream` does
  `AudioContext::new().expect("webaudio is not present on this system")` —
  creating the context is what the autoplay policy gates.
- Pumping: `window.setTimeout` loop (10 ms first, then per buffer duration)
  — each tick fills a `Float32Array`, `createBuffer`, `createBufferSource`,
  connect to destination, `start`. Latency is estimated, not measured.
- `Stream::play()` = `audio_ctxt.resume()` as a `JsFuture` awaited in a
  `spawn_local`; **a rejected promise PANICS** (`"AudioContext.resume()
  promise was rejected"`) — outside a user gesture the resume can reject,
  which is why `th-synth` never starts the engine at boot on emscripten
  (`start_on_gesture` is cfg-gated to emscripten; `start()` there stays on
  the fallback clock).
- `Stream::pause()` = `suspend()` (same rejection semantics). cpal 0.15.3
  exposes **no** suspend-state query on `Stream` — a hidden tab's suspended
  context simply stops callbacks (frozen `phase_clock`).
- Defaults/limits: sample rate 44 100 (range 8 000..96 000), 1..32
  channels, F32 only, default buffer 2048 frames; `is_available()` always
  `true`; `default_output_device()` = `Some(Device)` iff the `AudioContext`
  global constructs; input is `unimplemented!()`.
- No target flags needed beyond the `wasm32-unknown-emscripten` target;
  deps: `js-sys`, `wasm-bindgen`, `wasm-bindgen-futures`, `web-sys`
  (AudioContext/BufferSourceNode/...).

## Oboe host (`src/host/oboe/mod.rs`) — Android

- `android` cfg only; deps: `oboe 0.6` (feature `java-interface`),
  `jni 0.21`, `ndk 0.8`, `ndk-context 0.1`.
- Config space: **mono or stereo only** (`CHANNEL_MASKS` =
  `CHANNEL_OUT_MONO`/`CHANNEL_OUT_STEREO`), formats I16 + F32, sample rates
  {5512, 8000, 11025, 16000, 22050, 32000, 44100, 48000, 64000, 88200,
  96000, 176400, 192000}; min buffer from the Android
  `getMinBufferSize` (audio_track / audio_record, via `ndk` JNI).
- `default_output_device()` always returns `Some(Device(None))` (no
  device enumeration is required to open the default output);
  `is_available()` always `true` — so "no device" on Android surfaces as a
  stream build/play error, not a `None` device.
- **`Stream` is `!Send + !Sync` on ALL platforms** (0.15.3 wraps every
  stream in `NotSendSyncAcrossAllPlatforms` — upstream comment: "Streams
  cannot be `Send` or `Sync` if we plan to support Android's AAudio API.
  This is because the stream API is not thread-safe, and the API prohibits
  calling certain functions within the callback"). Hence
  `th-synth::MusicDirector` is a main-thread object.
- `TODO(05b)`: whether oboe 0.6 picks AAudio or OpenSL on a real device,
  and the boot-time open under the tiny runtime's activity — the android
  target check lands in issue 05b.

## Coexistence notes (both cpals in one graph)

- The two cpals are independent crates (0.15.3 + 0.18.2) — no type overlap;
  `th-synth` talks cpal 0.15 exclusively through rust-synth, th-render/
  th-audio talk cpal 0.18 exclusively through rodio. Two output streams can
  run at once (the synth + rodio SFX on the same device).
- The only graph-level conflict was the `links = "alsa"` duplication
  (solved by the vendor patch above). Watch build size/link in the 05b
  target checks (plan 05 "Risks").
