# Issue 05a report — `th-synth` host spike + the alsa-sys coexistence patch

Host-only implementation of plan 002 issue 05 (`.agents/plans/002-tinyhexagon/05-synth.md`)
per the task spec `.agents/tasks/plan-002/issue-005a-impl.md`. Base: `9130c969`
(branch `slot-1`). **Committed by the orchestrator after the forked PASS** (one squashed commit).
Android/wasm target checks are handed to 05b.

## The alsa-sys coexistence patch (the real blocker)

The blocker was exactly as the task spec stated: `rust-synth 0.23` →
cpal 0.15.3 → alsa 0.9 → `alsa-sys ^0.3.1`, while rodio 0.23 (th-render/
th-audio) → cpal 0.18.2 → alsa 0.11 → `alsa-sys 0.4.0`; both declare
`links = "alsa"` and Cargo refused the graph. No second conflict appeared.

Sanctioned fix, executed:

1. **Vendored** the cached registry source into `vendor/alsa-sys-0.3.1/`
   (copied from `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/
   alsa-sys-0.3.1`, LICENSE included; only the registry-extraction
   artifacts `.cargo-ok` / `.cargo_vcs_info.json` dropped).
2. **One change** in `vendor/alsa-sys-0.3.1/Cargo.toml`:
   `links = "alsa"` → `links = "alsa_0_3"` (+ a provenance comment block).
   `diff` against the registry copy shows nothing else (verified:
   `diff <registry>.../Cargo.toml vendor/.../Cargo.toml` → comment + links
   only; the whole `src/` is byte-identical).
3. **Root `Cargo.toml`** gains:

   ```toml
   [patch.crates-io]
   alsa-sys = { path = "vendor/alsa-sys-0.3.1", version = "0.3.1" }
   ```

   (patched with a comment citing plan 002 issue 05a; the 0.4.0 registry
   crate keeps `links = "alsa"` — only the 0.3.x line is patched).

Resolved lock versions (`Cargo.lock`):

| crate | version | source |
|---|---|---|
| alsa-sys | 0.3.1 | **path** `vendor/alsa-sys-0.3.1` (no checksum) |
| alsa-sys | 0.4.0 | registry (checksum kept) |
| alsa | 0.9.1 + 0.11.0 | registry (both) |
| cpal | 0.15.3 + 0.18.2 | registry (both) |
| rust-synth | 0.23.0 | registry |
| fundsp | 0.20.0 | registry |
| oboe | 0.6.1 | registry (android half of cpal 0.15) |
| rodio | 0.23.0 | registry (untouched) |

`rodio` / `th-audio` were **not** removed; nothing was substituted. The
whole-workspace gate (below) proves the two alsa-sys crates coexist with
rodio/th-render.

## `th-synth` implementation (host)

New files: `crates/th-synth/src/{lib,beat,director}.rs` (the crate + member
were already scaffolded into `crates/th-synth/` on the branch).

**`beat.rs`** — `BeatView` (contract shape: `beat_phase`, `bar_phase`,
`energy`, `stage`, `playing`) + pure f32 wrappers over
`rust_synth::math::pulse` (`beat_phase(t, bpm)`, `phrase_phase(t, bpm, 4)`
→ the bar) + `energy = clamp(max(peak_l, peak_r) * 3.0, 0, 1)`.

**`director.rs`** — the stage map + `MusicDirector`:

- **8 fixed slots** (`SLOTS`, pub const): PadZimmer@0 (×1.0), DroneSub@0
  (×0.5), PluckSaw@2 (×1.5), Bell@5 (×2.0), BassPulse@8 (×0.5), Shimmer@11
  (×2.0), Heartbeat@14 (×0.5, half-time pattern = the "soft kick" role),
  SuperSaw@17 (×1.0) — the ratios are the documented "simple ratios"
  choice; always exactly `MAX_TRACKS = 8` (the engine asserts it).
- **Stage ramp** (`stage_params`, pure + pub): `bpm = 70 + 1.5·min(stage,20)`
  (cap 100), `brightness = 0.45 + 0.02·min(stage,20)` (cap 0.85), root =
  A2 110 Hz × (4/3)^⌊min(stage,20)/3⌋ (a fourth every 3 stages, capped at
  the stage plan), `scale_mode = 0` (major pentatonic).
- **`MusicDirector` API** (as contracted):
  - `start() -> Self` — desktop/Android: attempts the live engine at boot,
    `Err` → silent fallback (eprintln + fallback clock); emscripten:
    **fallback only** (autoplay policy — `start_on_gesture`).
  - `set_stage(stage)` — mutates `Shared` params only (bpm/brightness on
    the engine's `GlobalParams`, per-slot mute + root-×-ratio freq on the
    shared track atomics); **no `rebuild_graph`** → click-free stage
    changes; idempotent.
  - `bump()` — duck: master 0.55 → 0.22, linear recovery over 0.6 s
    (advanced in `tick`, applied live via the per-buffer
    `master_gain` `Shared` read in the engine callback).
  - `tick(dt) -> BeatView` — live: `t = phase_clock`, energy from
    `peak_l/peak_r`, `playing = true`; fallback: `fallback_t += dt`,
    energy 0, `playing = false`.
  - `stop()` — drops the engine (stream stops); state preserved; back on
    the fallback clock.
  - `start_on_gesture()` — **cfg-gated to emscripten** (the web boot hook
    from the first canvas pointer/key event; idempotent).
  - `resume()` — (re)start after `stop()`/suspension on any target
    (idempotent; cpal 0.15's emscripten host owns one `AudioContext` per
    stream, so a fresh engine IS the resume).
- **State-sharing mechanics**: `Track` is *not* `Clone` upstream (only
  `TrackParams` is, and its `Shared` fields are `Arc<AtomicU32>`), so
  `try_start_live` rebuilds the `Vec<Track>` with cloned `params` that
  share the director's atomics — the live graph reads exactly what
  `set_stage`/`bump`/`tick` write. No drift possible.

## Tests (host; all 12 pass)

`crates/th-synth/src/{beat,director}.rs` (unit, no audio device needed):

1. `stage_map_monotonic_and_capped` — bpm/brightness strictly rise to the
   caps (100 / 0.85 @ stage 20), root steps up a fourth exactly at
   stages 3/6/… and holds between, nothing shifts past the cap (stages
   21..64 == stage 20).
2. `track_table_matches_contract` — the (kind, unlock) table matches the
   plan; `SLOTS.len() == MAX_TRACKS == 8`.
3. `slots_unlock_per_stage` — every slot muted exactly below its unlock
   (all stages 0..18).
4. `root_shift_retunes_tracks` — freq = root × ratio, re-tuned on the
   shift; `set_stage` idempotent.
5. `beat_and_bar_phases_at_known_times` — at 70 bpm: half-beat ⇒
   0.5/0.125, 2 beats ⇒ 0/0.5, 4 beats ⇒ 0/0, 3 beats ⇒ bar 0.75;
   fallback view: energy 0, `playing == false`.
6. `duck_lowers_and_recovers_over_0_6s` — 0.55 → 0.22 on `bump`, linear
   midpoint at 0.3 s, 0.55 at 0.6 s, no overshoot past.
7. `bump_during_recovery_resets_the_ramp`.
8. `start_without_a_device_does_not_panic` — `MusicDirector::start()` on a
   device-less host (the CI/this-host case: ALSA `Unknown PCM default`)
   takes the fallback; `tick` advances it.
9. `stop_drops_to_the_fallback_and_resume_is_safe`.
10–12. `beat.rs`: the energy expression, beat-phase wrap, bar = 4 beats.

## Skills

- `.agents/skills/lib-rust-synth.md` — new, from the checked-out 0.23.0
  source: engine/`EngineHandle` surface, the 8 presets, `TrackParams`
  defaults, `Track`-not-`Clone` / `Stream`-`!Send` traps, pulse/rhythm
  math, the `master_gain` duck approach, the alsa-sys patch notes.
  **Host sections verified in 05a; the android (oboe) and wasm
  (emscripten) target notes are marked `TODO(05b)`.**
- `.agents/skills/lib-cpal.md` — new: both cpal pins, the 0.15.3 host
  selection cfgs, the emscripten host (one `AudioContext` per stream;
  `Stream::play()` = `resume()` whose rejected promise **panics** — the
  autoplay gate; no suspend query), the oboe host (mono/stereo, I16/F32,
  rate list, always-`Some` default device, `Stream !Send+!Sync` marker,
  oboe 0.6/jni 0.21/ndk 0.8 deps; the AAudio-vs-OpenSL question is
  `TODO(05b)`), coexistence notes.
- `.agents/skills/README.md` — both indexed; the "pending" plan-002 note
  updated to "landed with 05a (host), 05b pending".

## Verification

```
cargo check -p th-synth   → green (the patched graph resolves; 51 crates)
cargo test -p th-synth    → 12 passed / 0 failed (no device: fallback)
cargo clippy -p th-synth --all-targets → clean
cargo fmt -p th-synth -- --check → clean
bash tools/gate.sh → (whole workspace; result in "Gate" below)
```

Gate:

```
== cargo fmt --all -- --check        GATE_EXIT=0
== cargo build --workspace          GATE_EXIT=0
== cargo clippy … -D warnings       GATE_EXIT=0
== cargo test --workspace           GATE_EXIT=0
```

`cargo test --workspace`: **551 passed / 0 failed** (includes th-synth's
12; the device-less host takes the fallback path in every audio test —
the expected `th-synth: live audio start failed …` line on stderr is the
fallback log, not a failure).

(no desktop listen recorded — no audio device on this host; the fallback
path was exercised by every test and by `start()` itself, which logged the
expected `no default output audio device` fallback. A 20 s listen is a
05b/07 item on a device-bearing machine.)

## Hand-off to 05b

- `cargo check -p th-synth --target aarch64-linux-android` (the oboe host;
  AAudio-vs-OpenSL confirmation lives in `lib-cpal.md` as `TODO(05b)`).
- `cargo check -p th-synth --target wasm32-unknown-emscripten` (the
  emscripten host; the TUI/recording deps — ratatui, crossterm, flacenc,
  vorbis_rs — compiling on wasm is the plan-05 risk; if it fails, vendor a
  feature-gated rust-synth per plan 05's spike-failure path).
- Re-run the gate afterwards; fill the `TODO(05b)` markers in the two
  skills with the real target evidence.

## Changed files

- `vendor/alsa-sys-0.3.1/` (new, vendored + the one `links` change)
- `Cargo.toml` (root: `[patch.crates-io]` + member already present)
- `Cargo.lock` (both alsa-sys lines + the rust-synth tree)
- `crates/th-synth/src/{lib,beat,director}.rs`
- `.agents/skills/{lib-rust-synth.md, lib-cpal.md, README.md}`
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` (§05a)
- `.agents/tasks/plan-002/issue-005a-report.md` (this file)

Committed by the orchestrator after the forked PASS. No app/runtime wiring (07), no SFX, no graph rebuild on stage
changes.
