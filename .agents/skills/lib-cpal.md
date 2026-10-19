# lib-cpal — cpal 0.15.3 (Rust)

Two cpals in this graph (plan 002 issue 05a):

| pin | pulled by | hosts we care about |
|---|---|---|
| **cpal 0.15.3** | `rust-synth 0.23` → `th-synth` | alsa (host), oboe (android), emscripten (web) — **vendored + the 05b emscripten ABI patch** (see below; 0.18.2 stays registry) |
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
  (AudioContext/BufferSourceNode/...). 05b: the whole host compiles green
  on `wasm32-unknown-emscripten` once the two `lib.rs` ABI fixes above are
  in (wasm-bindgen 0.2.129).

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
  stream build/play error, not a `None` device (05b: `cargo check -p
  th-synth --target aarch64-linux-android` green — the oboe half compiles;
  the runtime open is issue 08's device smoke).
- **`Stream` is `!Send + !Sync` on ALL platforms** (0.15.3 wraps every
  stream in `NotSendSyncAcrossAllPlatforms` — upstream comment: "Streams
  cannot be `Send` or `Sync` if we plan to support Android's AAudio API.
  This is because the stream API is not thread-safe, and the API prohibits
  calling certain functions within the callback"). Hence
  `th-synth::MusicDirector` is a main-thread object.
- **AAudio-vs-OpenSL (the 05a `TODO(05b)`, answered 05b from source):**
  cpal 0.15.3 builds every oboe stream from
  `AudioStreamBuilder::default()` — `AudioApi::Unspecified`, no
  `set_audio_api` call anywhere in `src/host/oboe/mod.rs`. The oboe 0.6.1
  Rust binding then does `will_use_aaudio` =
  `AAudio && is_aaudio_supported()` OR
  `Unspecified && is_aaudio_recommended()` (`oboe-0.6.1/src/
  audio_stream_builder.rs:463-466`), and the C++ (bundled in
  `oboe-sys-0.6.1/oboe/src/common/AudioStreamBuilder.cpp:64`) defines
  `isAAudioRecommended = (getSdkVersion() >= __ANDROID_API_O_MR1__) &&
  isAAudioSupported()` — i.e. **AAudio on API ≥ 27, OpenSL ES below**.
  Our APK's minSdk is 28 (feed-forward §02a), so every supported device
  takes AAudio at boot; OpenSL ES remains only the <27 fallback (outside
  our minSdk). This is compile-time/source evidence — the on-device
  confirmation is issue 08.
- **oboe C++ provenance (05b):** `oboe-sys 0.6.1`'s `build.rs` compiles the
  bundled oboe C++ (55 `src/` files + 3 `oboe-ext` wrappers, `-std=c++17
  -fno-rtti -fno-exceptions`, static `c++_static`) via `cc` using the NDK
  `aarch64-linux-android28-clang++` from `tools/android-env.sh` — verified
  in the 05b check (C++ warnings from `oboe/src/...` in the build log,
  `CXX_aarch64_linux_android` set). The prebuilt-fetch path (`fetch-`
  `prebuilt` feature → GitHub release tarballs) is OFF by default, so no
  network fetch; link line: `static=oboe-ext` + `log` + `OpenSLES`.

## 05b emscripten ABI patch (the vendored cpal)

`cargo check -p th-synth --target wasm32-unknown-emscripten` (05b) exposed
a SECOND blocker behind the rust-synth TUI/recording deps: cpal **0.15.3
is the last 0.15.x release** and its emscripten host does not compile
against the workspace's **wasm-bindgen 0.2.129** (the pin `th-web` carries
that matches the installed wasm-bindgen CLI — the 03c web recipe documents
version drift as build-breaking, so downgrading is off the table). Three
errors, all in `cpal/src/lib.rs` around `BufferSize`:

1. `BufferSize: FromWasmAbi` unsatisfied — the `#[wasm_bindgen]` derive on
   `StreamConfig` (field `buffer_size`) now requires BOTH Abi directions on
   every pub field; the registry source only hand-wrote `WasmDescribe` +
   `IntoWasmAbi` for `BufferSize`.
2. the hand-written `IntoWasmAbi`'s `match { … }.into_abi()` (routing
   `Option<u32>` through `IntoWasmAbi` again) no longer type-checks under
   0.2.129.

Fix (supervisor ruling A, 05b — same sanctioned class as the 05a
alsa-sys vendor patch): `vendor/cpal-0.15.3/` (registry cache copy,
LICENSE in, registry-extraction artifacts dropped) with exactly two
changes, both `#[cfg(target_os = "emscripten")]`-gated in `src/lib.rs`:

- the missing `impl FromWasmAbi for BufferSize` (`type Abi = Option<u32>`;
  `None ⇔ Default`, `Some(fc) ⇔ Fixed(fc)` — the exact mirror of the
  existing `IntoWasmAbi` encoding), and
- `IntoWasmAbi::into_abi` drops the trailing `.into_abi()` — the match
  already yields `Option<u32>` (identical encoding).

Wired by the root `[patch.crates-io] cpal = { path = "vendor/cpal-0.15.3",
version = "0.15.3" }` — the `version` pin limits the patch to the 0.15.3
line; the 0.18.2 registry cpal (rodio) stays stock. No non-emscripten
host, and no API/DSP surface, is touched; `diff -r` against the registry
source shows only the `Cargo.toml` provenance header + those two
`lib.rs` blocks. Residual (upstream, harmless): two dead-code warnings
from the vendored copy's emscripten host (`StreamId` never constructed,
an unused `#[macro_use]`) — left in place per the surgical-patch ruling.

## Coexistence notes (both cpals in one graph)

- The two cpals are independent crates (0.15.3 + 0.18.2) — no type overlap;
  `th-synth` talks cpal 0.15 exclusively through rust-synth, th-render/
  th-audio talk cpal 0.18 exclusively through rodio. Two output streams can
  run at once (the synth + rodio SFX on the same device).
- The only graph-level conflict was the `links = "alsa"` duplication
  (solved by the vendor patch above). Watch build size/link in the 05b
  target checks (plan 05 "Risks").
