# Issue 11d — the audition editor (per-slot timbre) + the SFX mute

**Worktree:** `.agents/worktrees/slot-2` (branch `slot-2`), base **the landed
11c commit** (the orchestrator gives you the exact SHA — do NOT start from an
older base). Main worktree and slot-1 are OFF-LIMITS. **Never commit.**

**Read first:**
1. `.agents/plans/002-tinyhexagon/11c-fm-kit.md` — the kit, the schema rules.
2. `crates/th-synth/src/score.rs` — the FINAL schema after 11c. `SlotGain` has
   `gain`, `kind: Option<String>`, `muted`. `ALL_KIND_IDS` is now **14** long
   (the 8 originals + the 6 FM voices). Read `kind_id`/`kind_from_id`.
3. `crates/th-web/site/index.html` — the WHOLE file. You are EXTENDING the DAW
   (issue 11b's track headers/rack/rolls). Keep everything working: the
   `?scorelab=1` gate, `?score=<url>` hot reload, `copyText`, the JSON row,
   and the probe's existing checks.
4. `vendor/rust-synth-0.23.0/src/audio/track.rs` — `TrackParams`: the knobs an
   instrument has. The audition editor exposes: **`character`, `cutoff`,
   `resonance`, `detune`, `reverb_mix`, `supermass`, `pulse_depth`, `lfo_rate`,
   `lfo_depth`, `lfo_target`, plus the slot `gain`** (already present).
5. `crates/th-audio/src/lib.rs` — `Audio::set_sound_volume(f32)` (SFML 0..=100
   units, clamped) and `crates/th-app/src/tiny.rs` (~914) where the app sets it
   to 60.0 at boot.

## Part A — the timbre schema (Rust, small)

The audition editor must be able to SAVE what it dials, or it is useless. So
`SlotGain` gains an optional timbre block:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub timbre: Option<Timbre>,
```

with

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timbre {
    pub character: f32,
    pub cutoff: f32,
    pub resonance: f32,
    pub detune: f32,
    pub reverb_mix: f32,
    pub supermass: f32,
    pub pulse_depth: f32,
    pub lfo_rate: f32,
    pub lfo_depth: f32,
    pub lfo_target: u32,
}
```

- `None` (the default) means "the role's built-in values from `apply_slot`" —
  so every existing score file and the shipped bake stay valid, and the shipped
  JSON does NOT gain a `timbre` block unless the operator dials one (that is
  the point: a baked custom timbre is a deliberate act).
- `apply_slot` must apply a slot's `Timbre` (when `Some`) AFTER its per-kind
  block, overriding those knobs, so a baked timbre wins over the defaults.
- Validation: `lfo_target` must be `< LFO_TARGETS` (the vendor exports it) and
  the float fields must be finite (reject NaN/∞) → `Err`, never a panic.
- Note: `character` etc. currently live as literals inside `apply_slot`'s
  per-kind arms. Refactor so each arm computes a `Timbre` (its defaults), the
  slot's `Some(timbre)` overrides it field-by-field, and the result is applied
  — one path, no duplication. If the six FM arms land in 11c with their own
  values, they flow through the same shape.

## Part B — the audition editor UI (the page)

Add a per-slot **timbre panel**, opened from the track header (a small "edit"
button on each header row; the panel shows the values for that slot):

- Sliders for the ten knobs above, each showing its numeric value, plus the
  existing gain. Every change writes `score.slots[i].timbre = {...}` and
  pushes via the existing `pushScore()` — the engine hears it live (these are
  all `Shared` params, so NO graph rebuild: verify that claim in code, and say
  so in a comment).
- A **"copy from role defaults"** button: drops `timbre` back to `None` (the
  built-in values) — the escape hatch.
- A **kind picker** already exists in the header; the timbre panel is about
  the KNOBS, so a freshly-picked FM voice can be shaped.
- Show the panel for ONE slot at a time (a track-header click selects; only the
  selected slot's panel is open) so the UI stays usable.
- All of it behind `?scorelab=1`, inside the existing scorelab branch.

## Part C — the SFX mute (the operator's request)

The operator wants the game's SFX (the bump beep, level-up, reset, icon tap)
silenceable **from the editor**, so they can hear the music alone while
working.

- Add a **"SFX" toggle** in the transport row (label it clearly, e.g. a button
  `SFX: on` / `SFX: off`). It is a page/session control (NOT baked into the
  score — the score is music data; SFX volume is a runtime preference).
- Wiring: a new wasm export `setSfxVolume(volume: f32)` (or
  `setSfxEnabled(bool)`) in `crates/th-web/src/web.rs` delegating to a new
  `th_app::tiny::web_set_sfx_volume(f32)` which calls
  `audio.set_sound_volume(v)` on the runtime's `Arc<TinyAudio>` — the director
  is already reachable through the existing `web` thread-local; do the same for
  the audio handle (or reuse whatever handle the app already exposes; read
  `crates/th-app/src/tiny.rs` and pick the least-invasive path — the audio is
  in `TinyRuntime.audio: Option<Arc<TinyAudio>>`).
- Volume 0 = silent, and the toggle should restore the app's boot value (60.0)
  when re-enabled — read the boot constant rather than hardcoding 60 in JS (the
  Rust side can own the default).
- It must be inert without `?scorelab=1` in the sense that the page never
  touches SFX volume unless the operator uses the toggle (default path: the
  game's SFX behave exactly as before).

## Probe coverage (`OH_SCORELAB=1`)

Add checks that fail if the feature is absent:
- the timbre panel opens for a slot and has the ten knobs + gain;
- moving a knob sets `slots[i].timbre.character` (or another field) in
  `getScore()`;
- "copy from role defaults" removes the `timbre` key from `getScore()` (back to
  built-in);
- the SFX toggle flips its label AND an observable (assert the toggle's state
  and, if cheaply observable, that `setSfxVolume` was called — a page-side
  spy/flag is acceptable; say what you asserted);
- **leave the engine on the shipped score at the end** (capture the canonical
  score before mutating; the existing code does this — keep it).

## Verification you must run

- `bash tools/gate.sh > /tmp/g.log 2>&1; echo $?` + grep the GATE_EXIT lines
  (never pip the gate to tail).
- Fresh high port; BOTH probes: default PASS and `OH_SCORELAB=1` PASS.
- **Screenshot the DAW** with the timbre panel open, and confirm by eye that it
  is readable and the values match the shipped defaults.
- Since the operator plays on a plain-http LAN origin, verify the SFX toggle
  and the timbre panel there too (127.0.0.1 is a secure context and has hidden
  a real bug before — `navigator.clipboard`). State in your report whether you
  tested on the LAN origin or only locally.

## Report (`.agents/tasks/plan-002/issue-011d-report.md`, RELATIVE)

The panel's structure, every control → schema field, the SFX wiring path you
chose, the probe lines, the screenshot's findings, whether LAN-origin testing
happened, deviations.
