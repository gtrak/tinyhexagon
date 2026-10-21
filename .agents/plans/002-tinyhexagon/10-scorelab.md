# Issue 10 — the score lab: edit the music live, bake it in

**Status:** IN PROGRESS
**Depends:** 05 (th-synth), 09 (one-path audio landed)
**Operator directive (verbatim):** "I want to improve the music situation. How
can I edit the sequencer in realtime and preview it, then bake it in when
satisfied?" → chose option **(c): both** — a live step-grid lab AND a JSON
edit + hot-reload loop, over ONE shared representation.

## Objective

Make the chiptune score **data** instead of Rust `const`s, so the operator can
(a) hear an edit within a second while the game runs, and (b) "bake" the
satisfying version into the shipped build with no copy-paste drift.

The hard part is already free: `MusicDirector` writes every sequenced value
into `Shared` atomics (`apply_slot` / `apply_step` / `gate_voice` /
`apply_bar`) and never calls `rebuild_graph` — a score change is live-audible
within one frame, no click. What is missing is that the CONTENT lives in Rust
constants (`BASS_LINE`, `LEAD_LINE`, `kick_bits`/`snare_bits`/`hat_bits`,
`CHORD_MAJOR`/`CHORD_MINOR`, `SLOTS` gains, `stage_params` ramps).

## The architecture (one representation, three surfaces)

```
                     Score (serde JSON)  ← the single source of truth
                            │
        ┌───────────────────┼────────────────────┐
        │                   │                    │
  include_str!        setScoreJson()        the lab UI
  assets/score.json   wasm export           ?scorelab=1
  (the SHIPPED bake)  (the LIVE path)       (the grid overlay)
```

- **Bake** = the file at `crates/th-synth/assets/score.json`, `include_str!`-ed
  as the default. The lab's "Copy JSON" button emits exactly that file's text.
- **Live** = `MusicDirector::set_score(Score)` — swap in place, re-apply, no
  graph rebuild. The wasm export takes JSON so the page never needs a Rust
  rebuild to change the music.
- **Grid** = a page-side overlay (`?scorelab=1`) that renders the score it
  reads from the wasm, toggles/edits it, and pushes changes straight back.

## The score schema (frozen by this issue)

Tracker-style pattern strings for drums (a human edits `x......x..x.....`, not
a bitmask), `null` = rest for the melodic lines. Every existing constant maps
1:1; `Score::default()` MUST reproduce today's music exactly (asserted).

```json
{
  "steps_per_bar": 16,
  "gate_open": 0.7,
  "swing": 0.15,
  "bpm": { "base": 150.0, "per_stage": 1.2, "cap": 20 },
  "brightness": { "base": 0.45, "per_stage": 0.025 },
  "root_hz": 110.0,
  "root_shift": 1.3333334,
  "bars_per_chord": 1,
  "minor_from": 8,
  "minor_until": 16,
  "chords": { "major": [0, 9, 5, 7], "minor": [0, 8, 3, 10] },
  "kick":  { "tiers": [ { "stage": 0,  "pattern": "x......x..x....." },
                        { "stage": 10, "pattern": "x......x..x..x.." },
                        { "stage": 16, "pattern": "x......x..x..x.x" } ] },
  "snare": { "tiers": [ { "stage": 0,  "pattern": "....x.......x..." },
                        { "stage": 12, "pattern": "....x.......x.x." } ] },
  "hat":   { "tiers": [ { "stage": 0,  "pattern": "..x...x...x...x." },
                        { "stage": 6,  "pattern": ".x.x.x.x.x.x.x.x" },
                        { "stage": 14, "pattern": "xxxxxxxxxxxxxxxx", "exclude": [4, 12] } ] },
  "bass":  { "notes": [0,null,null,12,0,null,7,null,0,null,10,12,0,10,3,null],
             "octave_shift": -1 },
  "lead":  { "notes": [12,null,15,null,19,17,null,15,12,null,10,null,7,10,null,12] },
  "slots": [ { "gain": 0.9 }, { "gain": 0.6 }, { "gain": 0.26 }, { "gain": 0.8 },
             { "gain": 0.5 }, { "gain": 0.20 }, { "gain": 0.20 }, { "gain": 0.28 } ]
}
```

Semantics that MUST hold (they are today's behaviour):

- **tiers**: a drum's pattern for a stage is the last tier whose `stage <=
  stage`. `exclude` lists step indices forced OFF in that tier (the late-hat
  case: all 16 minus the snares at 4 and 12).
- **pattern strings**: exactly `steps_per_bar` chars; `x`/`X` = hit, anything
  else = rest. Parsed by index (never by byte position arithmetic).
- **chords**: `major`/`minor` tables; the minor table is used when
  `minor_from <= stage < minor_until`; the bar's chord is
  `table[bar % table.len()]` (i.e. `bars_per_chord` semantics stay one table
  entry per bar unless the schema's `bars_per_chord` says otherwise — if
  implementing the field is more than a trivial index divide, keep the current
  behaviour and note it in the report).
- **bass** `octave_shift`: today the bass is `root * 0.5 * semitone(offset +
  note)` — the `0.5` IS `octave_shift: -1`; keep the multiplication explicit
  and the value data-driven. **The lead has no octave shift** (it is `root *
  2.0 * semitone(note)`, stage/root only, NOT chord-following) — represent it
  as `"octave_shift": 1` and note that the lead ignores `offset` (it is
  deliberately on the key root, asserted by the existing test
  `the_bar_chord_moves_the_harmony_only`).
- **slots**: gains only. Unlock stages, `ratio`, `chord`, `kind` and the
  per-preset character/LFO/reverb values stay in Rust (`SLOTS` + `apply_slot`)
  — they are synthesis design, not score. The `slots` array MUST have exactly
  8 entries (assert).

## Non-negotiables

- `Score::default()` (i.e. the shipped `assets/score.json`) is byte-for-byte
  today's music. Keep the existing director tests meaningful: they must pass
  unchanged against the default score, and ideally gain a
  `score_default_matches_the_legacy_constants` test comparing the parsed
  default against the old constant expressions.
- **Never `rebuild_graph`** on a score change; `set_score` only writes
  `Shared` params + `pattern_bits` and re-arms `last_bar`/`last_step`.
- All existing tests keep passing. `cargo clippy --all-targets -- -D warnings`
  is clean. `cargo fmt` clean.
- The web lab is **query-gated** (`?scorelab=1`) — a kid never sees it. The
  page stays text-free by default.
- The JSON must be parseable by `serde_json` (already a workspace dep) with no
  new deps in `th-synth` beyond adding `serde`/`serde_json` (workspace
  versions; `serde` with `derive`).
- wasm target is first-class: `cargo check -p th-synth --target
  wasm32-unknown-emscripten` must be clean (the JSON path uses no std
  filesystem — the default comes from `include_str!`; live scores arrive as
  strings).

## Issues

| # | Issue | Owner | Slot | Base |
|---|-------|-------|------|------|
| 10a | `Score` type + default + `set_score` + exports + `assets/score.json` + tests | worker | slot-1 | master |
| 10b | the lab page: `?scorelab=1` grid overlay + `?score=` hot reload + probe coverage | worker | slot-2 | master |

10a and 10b touch disjoint files (`crates/th-synth/**`, `crates/th-app/src/tiny.rs`,
`crates/th-web/src/web.rs` vs `crates/th-web/site/index.html`,
`tools/web-tiny-probe.mjs`) and may run in parallel. **10b depends on 10a's
export names** — the interface is frozen below, so 10b implements against the
frozen names, not against 10a's tree.

## The frozen interface (both issues code against this)

Rust (`th-synth`):

```rust
// crates/th-synth/src/score.rs
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Score { /* the schema above, snake_case field names */ }

impl Default for Score { fn default() -> Self { /* parses SCORE_JSON */ } }

/// The shipped score: `include_str!("../assets/score.json")`.
pub const SCORE_JSON: &str = include_str!("../assets/score.json");

impl Score {
    /// The canonical JSON text (pretty — this is the BAKE artifact).
    pub fn to_json_pretty(&self) -> String;
    /// Parse; `Err(message)` is human-facing (surfaced in the lab's status).
    pub fn from_json(text: &str) -> Result<Self, String>;
    /// The tier resolution the director uses.
    pub fn drum_pattern(&self, tier: &DrumTiered, stage: u32) -> u32; // bitmask
    /// A stage's bpm/brightness/root — replaces `stage_params`' internals.
    pub fn stage_params(&self, stage: u32) -> StageParams;
    pub fn chord_offset(&self, stage: u32, bar: u64) -> i32;
    pub fn minor_for_stage(&self, stage: u32) -> bool;
}

// on MusicDirector:
pub fn set_score(&mut self, score: Score);
#[must_use] pub fn score(&self) -> &Score;
```

wasm (`th-web`) — the frozen export names:

```rust
#[wasm_bindgen(js_name = "getScore")]  pub fn get_score() -> String;   // canonical pretty JSON
#[wasm_bindgen(js_name = "setScore")]  pub fn set_score(json: &str) -> String; // "" = ok, else the error
```

`th-app::tiny` gets the plumbing (a `thread_local` director handle already
exists in `web`; reuse it):

```rust
pub fn web_get_score() -> String;
pub fn web_set_score(json: &str) -> String;  // "" ok; error text on parse failure
```

Page (10b) contract:

- `?scorelab=1` → the overlay renders the 8-track × 16-step grid (drums as
  toggle cells from the tier's pattern string, bass/lead as note cells) + gain
  sliders + a BPM readout. Every edit calls `mod.setScore(json)`; the status
  line shows `""` or the parse error. A "Copy JSON" button writes the exact
  canonical text to the clipboard (the bake step).
- `?score=<url>` → poll `fetch(url, {cache:"no-store"})` every 500 ms; when the
  text changes, `mod.setScore(text)`; log parse errors to the console AND the
  overlay. This is the editor-driven live loop.
- The probe (`tools/web-tiny-probe.mjs`) gains coverage: with `?scorelab=1`,
  assert the overlay exists, that a `setScore` round-trip of the canonical JSON
  returns `""`, and that a deliberately broken JSON returns a non-empty error.

## Verification (the orchestrator re-runs these, not the workers)

1. `bash tools/gate.sh` → `GATE_EXIT=0` (fmt/build/clippy/tests).
2. `cargo check -p th-synth --target wasm32-unknown-emscripten`.
3. `bash tools/package-web.sh` (the emsdk container) then the probe:
   `node tools/web-tiny-probe.mjs http://127.0.0.1:<fresh port>/` → PASS, plus
   the `?scorelab=1` assertions.
4. Orchestrator's own ear/eyes check via the probe's screenshot where useful.
