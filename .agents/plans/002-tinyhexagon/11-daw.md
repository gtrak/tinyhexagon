# Issue 11 — the DAW: instruments, mute/solo, piano rolls

**Status:** IN PROGRESS
**Depends:** 10 (the score lab), 05 (th-synth)
**Operator directive (verbatim):** "I need a more fully featured editor,
fruity-loops/fl-studio style. Each instrument should have a piano roll, I should
be able to solo and mute." → then: "ok do it, drums are a step grid, but I want
to see drums grouped together like a piano roll."

## What the engine actually allows (measured, not assumed)

These three facts decide the whole design; do not fight them.

1. **Only TWO lanes are step-sequenced melodically.** `apply_step` writes slot
   3 (bass) and slot 4 (lead) from `score.bass`/`score.lead`; slots 5/6/7
   follow the bar's chord (`apply_bar`, `Slot.chord`); slots 0/1/2 are gated by
   `pattern_bits`. The arithmetic is hardcoded per index.
2. **Drums are a 16-step bitmask** (`TrackParams::pattern_bits: Arc<AtomicU32>`,
   read per sample by the `Heartbeat` preset via `rhythm::step_is_active`). A
   hit has no pitch. So the drum editor is a **step grid**, not a pitch roll —
   the operator accepted exactly this ("drums are a step grid").
3. **Changing an instrument requires `EngineHandle::rebuild_graph()`** (the
   vendor's own documented path: "Call this after mutating any `track.kind`").
   It rebuilds the whole DSP net, discarding reverb tails — a small audible
   discontinuity, acceptable for an explicit user action but NOT click-free
   like a score edit.

### The role/timbre split (the core design decision)

A slot has a **role** (fixed by its index) and a **timbre** (selectable):

| Slot | Role | Sequenced by |
|------|------|--------------|
| 0,1,2 | drum | `pattern_bits` (kick/snare/hat tiers) |
| 3 | bass | `score.bass` per step |
| 4 | lead | `score.lead` per step |
| 5,6,7 | harmony | the bar's chord (`Slot.chord`) |

Picking a different instrument for a slot changes its **timbre only** — the
role, and therefore what drives its notes, stays. Choosing `Bell` for slot 3
gives a bell bassline; choosing `Heartbeat` for slot 5 gives a chord-following
drum (its `pattern_bits` falls back to the 4-hit Euclidean default). This is
honest, explainable, and needs no new routing. It is documented in the UI
(each lane's header names its role).

## The schema additions (frozen for issue 11a)

`slots` entries gain `kind` and `muted`; `Score` gains `solo`:

```json
"slots": [
  { "gain": 0.9, "kind": "heartbeat", "muted": false },
  ...
],
"solo": null
```

- `kind`: a stable snake_case id, NOT the display label. The mapping lives in
  th-synth (`kind_id` / `kind_from_id` / `ALL_KIND_IDS`), covering the vendor's
  8 `PresetKind`s: `pad_zimmer`, `drone_sub`, `shimmer`, `heartbeat`,
  `bass_pulse`, `bell`, `super_saw`, `pluck_saw`.
- Both fields are `#[serde(default)]` so a score JSON from issue 10 (which has
  neither) still parses — `kind` defaults to the slot's built-in kind, `muted`
  to false, `solo` to null. **This backwards compatibility is required**: the
  operator has score files already saved.
- `muted` + `solo` are BAKED state (like an FL project's track mutes). The
  shipped `assets/score.json` MUST have every `muted: false` and `solo: null`
  (a stray baked solo would silence the game for a child — the panel gets a
  "clear mix" button and this is called out).

## Mute/solo semantics (the fold, and why it must be per-tick)

Today `mute` is written from three places: `apply_slot` (the stage unlock),
`apply_step` (note present), and `sequence()`'s `gate_voice` (the staccato
window). The last one runs EVERY tick. So a user mute written once would be
overwritten by the gate on the next tick.

The fix: **`sequence()` computes and writes the effective mute for ALL 8 slots
every tick**, folding the program state with the user state:

```
effective[i] = program[i] || muted[i] || (solo.is_some() && solo != Some(i))
```

where `program[i]` is the stage-unlock state for drums/harmony and the
gate state (`!(note && in_window)`) for bass/lead. This makes any mix change
audible within one tick with no extra plumbing, and removes the write-once
paths (`apply_slot`/`apply_step` no longer write `mute`; they record the
program state the fold reads).

## Issues

| # | Issue | Owner | Slot | Depends |
|---|-------|-------|------|---------|
| 11a | the Rust core: kinds, mute/solo, the tick fold, exports | worker | slot-1 | — |
| 11b | the DAW UI: track headers, grouped drum rack, piano rolls | worker | slot-2 | 11a landed |

**Strictly sequential.** 11b must be built against a LANDED 11a: serde ignores
unknown fields, so a page sending `kind`/`muted` to a pre-11a engine would
look correct in the DOM while silently doing nothing — the exact vacuous-success
trap this project keeps hitting. 11b starts only after 11a is on master and the
wasm is rebuilt.

## Verification (the orchestrator's, not the worker's)

1. `bash tools/gate.sh` with the exit code captured honestly —
   `bash tools/gate.sh > /tmp/g.log 2>&1; echo $?` plus a grep of the
   `GATE_EXIT` lines. **Never** `| tail` (it reports tail's status; that bug
   hid a red clippy stage for most of the audio arc).
2. `cargo check -p th-synth --target wasm32-unknown-emscripten`.
3. `bash tools/package-web.sh`, then the probe on a **fresh high port**:
   default PASS and `OH_SCORELAB=1` PASS.
4. Orchestrator mutation-checks the new tests (mute actually silences, solo
   actually isolates, a kind change actually reaches the engine) before landing.
5. Verify on the INSECURE LAN origin (`192.168.10.31`), not just 127.0.0.1 —
   the clipboard bug proved those environments differ.
