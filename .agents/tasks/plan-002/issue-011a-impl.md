# Issue 11a — the DAW's Rust core: instruments, mute/solo, the tick fold

**Worktree:** `.agents/worktrees/slot-1` (branch `slot-1`), base **`6ce698ba`**
(master). Main worktree and slot-2 are OFF-LIMITS. **Never commit.**

**Read first, in this order:**
1. `.agents/plans/002-tinyhexagon/11-daw.md` — the schema, the role/timbre
   split, the mute/solo fold. THE SOURCE OF TRUTH.
2. `crates/th-synth/src/score.rs` — the whole file (the `Score` model,
   validation, the `mod legacy` fidelity tests you must not break).
3. `crates/th-synth/src/director.rs` — the whole file. Note especially:
   `SLOTS`, `apply_slot`, `apply_step`, `apply_bar`, `sequence`, `gate_voice`,
   `set_score`, `set_stage`, `build_tracks`, `try_start_live`.
4. `vendor/rust-synth-0.23.0/src/audio/preset.rs` — the `PresetKind` enum
   (line ~17), `ALL_KINDS` (~29), `label()` (~41) and `Preset::build` (~157).
5. `vendor/rust-synth-0.23.0/src/audio/engine.rs` — `EngineHandle` (~52, note
   `pub tracks` and `pub fn rebuild_graph` ~276) and its doc warning that a
   rebuild discards reverb tails.

Write-first: add the `kind`/`muted`/`solo` schema fields and the kind-id
mapping FIRST, compile, then do the fold.

## What to build

### 1. The kind-id mapping (in `score.rs`)

The vendor's `PresetKind` has no serde and must stay vendored/unmodified. Add
to `score.rs`:

```rust
/// The 8 instrument ids, in ALL_KINDS order (stable JSON ids).
pub const ALL_KIND_IDS: [&str; 8] = [
    "pad_zimmer", "bass_pulse", "heartbeat", "drone_sub",
    "shimmer", "bell", "super_saw", "pluck_saw",
];
#[must_use] pub fn kind_id(kind: PresetKind) -> &'static str;
#[must_use] pub fn kind_from_id(id: &str) -> Option<PresetKind>;
```

Match the ids to the vendor's `ALL_KINDS` order EXACTLY (read it — do not
guess), so a UI cycling kinds walks a stable order. `kind_from_id` is
case-sensitive, returns `Option` (never panics).

### 2. Schema: `kind`, `muted`, `solo`

- `SlotGain` gains:
  ```rust
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub kind: Option<String>,
  #[serde(default)]
  pub muted: bool,
  ```
  `kind: None` means "the slot's built-in kind from `SLOTS`". This is what
  keeps issue-10 score files valid AND keeps the shipped JSON diff minimal.
- `Score` gains:
  ```rust
  #[serde(default)]
  pub solo: Option<u32>,
  ```
- **Validation**: `kind` (when `Some`) must be a known id → else `Err` naming
  the slot index and the bad id. `solo` (when `Some`) must be `< slots.len()`
  → else `Err`.
- Keep `to_json_pretty`/`from_json` round-tripping. The existing
  `the_baked_score_is_the_canonical_pretty_text` test must still pass: add
  `kind: None` and `muted: false` to the shipped `assets/score.json` **only if
  the serializer emits them** (it does not for `None`/`skip_serializing_if`;
  `muted: false` WILL be emitted since it has no skip — so update the asset
  file to carry `"muted": false` on each slot, and DO NOT add `"kind"`).

### 3. The effective-mute fold (in `director.rs`) — the heart of this issue

Today `mute` is written by `apply_slot` (stage unlock), `apply_step` (note
present) and `sequence`'s `gate_voice` (the staccato window, every tick). A
user mute written once is overwritten by the gate on the next tick.

Restructure to a **per-tick fold**:

- Track the program state instead of writing `mute` in the various passes:
  - add a field `program_mute: [bool; 8]` to `MusicDirector` (or a small
    struct). `apply_slot` records the stage-unlock state (muted while
    `stage < unlock`), `apply_step` records the note-present state, and
    `sequence`'s window logic supplies the gate half.
- In `sequence()`, after the window is computed, write ALL 8 mutes every tick:

  ```
  for i in 0..8 {
      let program = /* bass/lead: !(note && in_window); drums/harmony: the stage unlock */;
      let user = self.score.slots[i].muted
          || self.score.solo.is_some_and(|s| s != i as u32);
      self.tracks[i].params.mute.set_value(if program || user { 1.0 } else { 0.0 });
  }
  ```

  Keep `gate_voice`'s behaviour for bass (3) and lead (4) — it IS the program
  half for those slots — but route every write through the fold so a user mute
  cannot be clobbered.
- `apply_slot` must NOT write `mute` any more (it records the unlock state); a
  test must prove a user-muted slot STAYS muted across many ticks INCLUDING
  when its step/tier is active (that is the regression this restructure
  exists to prevent).
- **Solo**: `solo = Some(i)` silences every other slot. A test must prove
  soloing the kick leaves only the kick's pattern audible (assert the other
  slots' `mute` is 1.0 and the soloed one follows its own program).

### 4. Instrument selection reaches the engine

- `set_score` must apply a changed `slots[i].kind` by mutating the live
  engine's track kind and rebuilding the graph:
  - when `self.live` is `Some`, for each slot whose RESOLVED kind differs from
    the engine's current track kind: mutate the handle's `tracks` entry
    (`handle.tracks` is `Arc<Mutex<Vec<Track>>>`, `Track.kind` is a public
    field) and then call `handle.rebuild_graph()` ONCE after all changes (not
    per slot).
  - **Guard it**: only rebuild when a kind actually CHANGED (never on a plain
    gain/mute edit) — a needless rebuild costs an audible discontinuity.
  - Keep a `Vec<PresetKind>` (or read from `SLOTS` + score) tracking what the
    engine currently has, so the diff is correct.
  - `apply_slot`'s per-preset parameter block is keyed on `slot.kind`; it must
    now key on the slot's RESOLVED kind (`score.slots[i].kind` if `Some`, else
    the built-in `SLOTS[i].kind`) — otherwise a swapped instrument gets the
    wrong character/LFO/reverb/sweep values.
  - **Careful with `Self::build_tracks`**: it creates `Track::new(i, kind.label(),
    kind, 0.0)` — a swapped kind needs the track rebuilt with the new
    `kind`/`label` at CONSTRUCTION for a fresh start, but for a live swap
    mutate the existing `Track.kind` + `.name` then rebuild the graph.
    Choose one consistent path and say which in your report.

### 5. Exports

- `th-app/src/tiny.rs` + `th-web/src/web.rs`: no NEW exports are needed
  (`getScore`/`setScore` already carry the whole `Score`, and the new fields
  ride along). Verify this by reading them; if a change IS needed, say so.

## Tests (all must be non-vacuous — a test that passes with the feature deleted is a bug)

- `user_mute_survives_the_gate`: mute slot 0, tick through several steps where
  its pattern has hits, assert `mute == 1.0` every tick. **Prove it fails if
  the fold is removed.**
- `solo_isolates_one_slot`: `solo = Some(0)`, assert slots 1..7 are muted and
  slot 0 follows its pattern; then `solo = None` and assert the others come
  back.
- `kind_change_reaches_the_engine`: in-memory engine (`AudioEngine::
  start_in_memory`, the CI-safe path — see the existing
  `in_memory_engine_renders_the_score` test), `set_score` with a changed
  `slots[3].kind`, assert the handle's track kind actually changed AND that
  no rebuild happened when only a gain changed (count rebuilds — e.g. a flag
  or by asserting the kind vector is unchanged; do not weaken this to a no-op).
- `slots_without_kind_use_the_builtin`: a score parsed from issue-10 JSON
  (no `kind` fields) resolves each slot to `SLOTS[i].kind`.
- `unknown_kind_is_an_error`: `from_json` with `"kind": "nope"` → `Err` naming
  the slot; `"solo": 99` → `Err`.
- **Keep every existing test passing**, including the `mod legacy` fidelity
  tests and `score_default_matches_the_legacy_constants`. The default score
  MUST still render identical music (that is the whole point of the bake).

## Gates you must run (capture the exit code HONESTLY)

```
cargo fmt --all -- --check
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p th-synth --target wasm32-unknown-emscripten
```
Do NOT pipe the gate to `tail` (it reports tail's status and hides a failing
stage). Do not run `tools/package-web.sh` (the orchestrator does).

## Report (write to `.agents/tasks/plan-002/issue-011a-report.md` RELATIVE — it
must land in YOUR worktree)

File-by-file changes; the exact gate output lines; the kind-swap path you
chose in item 4 and why; the mutation checks you ran for the mute/solo tests;
deviations and why; any skill-file correction.
