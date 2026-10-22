# Issue 11a report — the DAW's Rust core: instruments, mute/solo, the tick fold

Worktree: `.agents/worktrees/slot-1` (branch `slot-1`, base `52c2926d`).
Nothing committed. All changes are uncommitted in this worktree.

## File-by-file changes

### `crates/th-synth/src/score.rs` (schema + kind mapping)
- **Kind-id mapping (item 1)**: `ALL_KIND_IDS: [&str; 8]` in the vendor's
  `ALL_KINDS` cycle order (verified against
  `vendor/rust-synth-0.23.0/src/audio/preset.rs` — `PadZimmer, BassPulse,
  Heartbeat, DroneSub, Shimmer, Bell, SuperSaw, PluckSaw`), plus
  `kind_id(PresetKind) -> &'static str` and `kind_from_id(&str) ->
  Option<PresetKind>` (case-sensitive, `None` for unknown, never panics).
  The vendor's `PresetKind` stays untouched.
- **Schema (item 2)**: `SlotGain` gains `kind: Option<String>`
  (`#[serde(default, skip_serializing_if = "Option::is_none")]` — `None`
  means the slot's built-in `SLOTS[i].kind`) and `muted: bool`
  (`#[serde(default)]` — always serialized, `false` in the bake). `Score`
  gains `solo: Option<u32>` (`#[serde(default)]`, serialized as `null`
  when `None`), placed after `slots` per the plan's JSON example.
- **Validation**: `slots[i].kind` (when `Some`) must map through
  `kind_from_id` → else `Err` naming the slot index AND the bad id;
  `solo` (when `Some`) must be `< slots.len()` → else `Err` naming the
  field and the value.
- **Tests added**: `kind_ids_follow_the_vendor_cycle_order` (pins the
  id↔`ALL_KINDS` pairing, case-sensitivity, unknown-id → `None`) and
  `unknown_kind_is_an_error` (`"kind": "nope"` → `Err` naming
  `slots[3]` + `nope`; `"solo": 99` → `Err` naming `solo`).
- Existing `the_baked_score_is_the_canonical_pretty_text` and
  `score_default_matches_the_legacy_constants` (bake fidelity) pass
  unchanged.

### `crates/th-synth/assets/score.json` (the bake)
- Each slot gains `"muted": false` (the serializer emits it — no skip
  attribute), and the score gains `"solo": null` after `slots`. NO
  `"kind"` added (`skip_serializing_if = Option::is_none` suppresses it).
  `the_baked_score_is_the_canonical_pretty_text` confirms the file is
  exactly `Score::default().to_json_pretty()` + trailing newline.

### `crates/th-synth/src/director.rs` (the fold + the swap)
- **The effective-mute fold (item 3 — the heart)**: new
  `MusicDirector.program_mute: [bool; 8]` (the program half).
  `fold_mutes()` writes ALL 8 slots' `mute` as
  `program_mute[i] || (score.slots[i].muted || score.solo.is_some_and(|s| s != i))`
  — called at the end of `sequence()` **every tick**, and at the end of
  `new_fallback`/`set_stage`/`set_score` so a mix change is current
  before the next tick too.
  - `apply_slot` no longer writes `mute`; it records the stage-unlock
    program state (`program_mute[index] = stage < slot.unlock`).
  - `gate_voice` no longer writes `mute`; it records the gate's program
    state (`program_mute[slot] = !on`) — same `on` expression as before
    (`note present && in window`), so bass/lead gate behaviour is
    preserved.
  - `apply_step` writes freqs only (its `bass.mute`/`lead.mute` writes
    removed — rests gate through the fold's program half).
- **Instrument selection reaches the engine (item 4)**:
  - `resolved_kind(score, i)` = `score.slots[i].kind` (validated by
    `from_json`) or `SLOTS[i].kind` when `None`.
  - `apply_kinds()` (called from `set_score`): computes the 8 resolved
    kinds, diffs against the tracks' current kinds (the director's
    tracks mirror the engine's kinds — both updated here, and
    `try_start_live` clones from them, so the mirror IS "what the
    engine currently has"), and only when something actually changed:
    mutates `self.tracks[i].kind`/`.name`, then under one lock mutates
    every live `handle.tracks` entry's `kind`/`.name` and calls
    `handle.rebuild_graph()` **once** (not per slot). Plain
    gain/mute/solo edits hit the early return — no mutation, no rebuild.
  - `apply_slot`'s per-preset parameter block now keys on
    `Self::resolved_kind(score, index)` (was `slot.kind`) — a swapped
    instrument gets the new preset's character/LFO/reverb values.
  - `build_tracks` constructs each `Track::new` with the RESOLVED
    kind/label (the fresh-start path: a swapped kind is in the track at
    construction; `new_fallback`'s `Score::default()` resolves to the
    built-ins, so the bake is unchanged).
  - Role-aware Heartbeat: a Heartbeat swapped INTO a non-drum slot (the
    plan's "choosing Heartbeat for slot 5 gives a chord-following drum")
    keeps the 4-hit Euclidean default `pattern_bits` (never overwritten)
    and a neutral 0.5 character, instead of inheriting the hat tier's
    pattern. Drum-role slots (0..3) are unchanged.
- **Module docs updated**: the "never touch the graph" line now carries
  the documented kind-swap exception; a new "plan 11 effective-mute
  fold" section documents the role/timbre split.

### `crates/th-synth/src/director.rs` — tests
- **New (the spec's five)**:
  - `user_mute_survives_the_gate` — drum half: kick (slot 0, tier hits
    at 0/7/10) user-muted stays `mute == 1.0` on every tick of a full
    bar (and lands immediately post-`set_score`); snare proves the fold
    is per-slot. Gate half: bass (slot 3) user-muted stays `1.0` while
    its window is OPEN on a note step (in-steps 0.125..0.625 of step 0,
    where the pre-fold gate wrote 0.0 on every tick).
  - `solo_isolates_one_slot` — `solo = Some(0)` silences slots 1..7
    while the soloed kick follows its program, across ticks; clearing
    the solo brings the unlocked drums/harmony + the gated voices back
    (through their gate) and leaves the stage-locked pad/bell muted
    (proves the fold is program OR solo, not just solo's absence).
  - `kind_change_reaches_the_engine` — CI-safe in-memory engine
    (`AudioEngine::start_in_memory`, same path as the existing
    `in_memory_engine_renders_the_score`): (1) gain-only edit → engine
    kind vector UNCHANGED (the guard's observable: no mutation → no
    rebuild) while the gain itself applied; (2) `slots[3].kind =
    "super_saw"` → the engine's track kind AND label actually become
    `SuperSaw`/`"SuperSaw"` (and the director's tracks mirror it);
    (3) second swap (`"pluck_saw"`) diffs against the engine's CURRENT
    kind; (4) re-applying the same kind is a no-op diff.
  - `slots_without_kind_use_the_builtin` — an issue-10-shaped JSON
    (the 11a fields stripped from the canonical text) parses, and every
    slot resolves to `SLOTS[i].kind` via `resolved_kind`.
  - `unknown_kind_is_an_error` (in `score.rs` — see above).
- **One mechanical fix to an existing test**: `set_score_rewrites_the_
  slot_gains`'s `SlotGain { gain: 0.3 }` literal now carries the two new
  fields (`kind: None, muted: false`) — a compile-forced consequence of
  the new schema, no semantic change.

### `crates/th-app/src/tiny.rs`, `crates/th-web/src/web.rs` (item 5)
- **No changes — verified by reading.** `getScore` returns
  `score().to_json_pretty()` (the whole `Score`), `setScore` parses
  `th_synth::Score::from_json` and hot-swaps via `director.set_score`;
  the new `kind`/`muted`/`solo` fields ride along in both directions.
  No new exports needed.

### `.agents/skills/lib-rust-synth.md` (skill-file correction)
- The `rebuild_graph()` bullet claimed "th-synth NEVER calls it" — now
  stale: it documents the 11a exception (guarded one-shot rebuild on a
  kind swap; fresh engines get the resolved kind at construction).

## The kind-swap path I chose (item 4) and why

Two complementary paths, exactly as the spec's `build_tracks` note
prescribes:
- **Fresh start** (`new_fallback` → `build_tracks`, and every
  `try_start_live`): the tracks are CONSTRUCTED with the resolved
  kind/label (`Track::new(i, resolved.label(), resolved, 0.0)`), so no
  live mutation is ever needed on a fresh engine.
- **Live swap** (`set_score` while `live` is `Some`): mutate the
  existing `handle.tracks[i].kind`/`.name` (and the director's mirror)
  under one lock, then `handle.rebuild_graph()` ONCE for all changed
  slots — the vendor's documented path. I do NOT rebuild the whole
  `Vec<Track>` live: the engine's `TrackParams` are `Arc`-shared
  atomics with the director's (the module-docs' "same atomics"
  contract), so only the copied `kind`/`name` need the live mutation —
  rebuilding the Vec would orphan the shared params and re-introduce
  the very drift the one-mixer design forbids.
The rebuild guard (the spec's trap): `apply_kinds` diffs resolved kinds
against the tracks' CURRENT kinds and early-returns when nothing
changed — a plain gain/mute/solo edit never touches the engine, so the
reverb-tail discontinuity cost is paid only for an explicit instrument
swap.

## Mutation checks I ran (non-vacuous proof)

1. **Fold removed** (restored the pre-11a write-once semantics: old
   `gate_voice` direct `mute` write, old `apply_slot` unlock write, all
   four `fold_mutes()` call sites removed; file backed up and restored
   afterward):
   - `user_mute_survives_the_gate` **FAILED** at its first assert —
     "the user mute lands immediately": `left: 0.0, right: 1.0` (the
     `set_score`/`apply_slot` clobber).
   - `solo_isolates_one_slot` **FAILED** at tick 0 — "soloing the kick
     silences slot 1": `left: 0.0, right: 1.0` (the gate's every-tick
     clobber).
   - All other 32 tests still passed (the regression is isolated to the
     user-state half, as designed).
2. **Kind-swap mechanism disabled** (`apply_kinds` early-returns;
   backed up and restored): `kind_change_reaches_the_engine` **FAILED**
   at "the swapped kind reaches the engine's tracks": `left: BassPulse,
   right: SuperSaw`.

I verified the fold-failure myself by running these mutations; the
orchestrator's own mutation pass (plan verification step 4) is still
advised.

## Gates (exit codes captured honestly — no tail-piping)

| Gate | Command | Exit | Output line |
|---|---|---|---|
| fmt | `cargo fmt --all -- --check` | **0** | (empty — clean; the first run exited 1 with 3 hunks in `director.rs`, fixed by `cargo fmt --all`, which touched only my 3 files) |
| build | `cargo build --workspace` | **0** | `Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.06s` (2 pre-existing cargo "output filename collision" warnings: the `tinyhexagon` bins in th-app vs th-web — unrelated to this change) |
| clippy | `cargo clippy --workspace --all-targets -- -D warnings` | **0** | `Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.79s` |
| test | `cargo test --workspace` | **0** | 69 × `test result: ok`, 0 failures; th-synth lib: `test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 33.46s` (baseline was 28; +6 new: 5 spec tests + `kind_ids_follow_the_vendor_cycle_order`) |
| wasm | `cargo check -p th-synth --target wasm32-unknown-emscripten` | **0** | `Checking th-synth v0.1.0 … Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.39s` |

Bake fidelity (`score_default_matches_the_legacy_constants`) and the
canonical-text test (`the_baked_score_is_the_canonical_pretty_text`)
both pass — the default score still renders identical music.

## Deviations and why

1. **`SlotGain` struct literal in an existing test** updated for the two
   new fields (compile-forced; no semantic change).
2. **One extra test** beyond the spec's five: `kind_ids_follow_the_
   vendor_cycle_order` — item 1 requires the ids to match `ALL_KINDS`
   EXACTLY, so a test pinning that pairing (against the vendor const
   itself) is the non-vacuous way to keep it true.
3. **Fold also runs in `new_fallback`/`set_stage`/`set_score`** (not
   only per tick): required so the existing `slots_unlock_per_stage`
   (which reads `mute` immediately after `set_stage`, no tick) keeps
   passing, and so a mix change is current before the next tick. The
   spec's per-tick fold in `sequence()` is exactly as specified; these
   are additive, same fold.
4. **Role-aware Heartbeat branch** (swapped-into-non-drum slots keep the
   4-hit Euclidean default + neutral character): the plan doc is the
   source of truth and states this behaviour explicitly ("its
   `pattern_bits` fall back to the 4-hit Euclidean default"); the
   neutral 0.5 character is the vendor's documented neutral value (the
   plan does not specify).
5. **The diff for `apply_kinds` runs against the director's tracks**
   (the engine-kind mirror) rather than locking `handle.tracks` to
   read: the two are kept identical by construction (updated together
   in `apply_kinds`; `try_start_live` clones from them), and it avoids
   a lock just to find the change set. Noted for the reviewer.

## Residual risks / open questions

- **A missing `rebuild_graph()` call is not directly countable in a
  test**: the graph is a private `Arc<Mutex<Net>>` and the vendor is
  frozen, so no rebuild counter exists. `kind_change_reaches_the_engine`
  pins (a) the guard (gain-only → kind vector unchanged → the no-mutate
  no-rebuild path) and (b) the mutation itself (kind + label reach the
  engine's tracks — the rebuild's source). A deletion of the
  `rebuild_graph()` line specifically would leave the kind mutated but
  the DSP graph stale; that is for the orchestrator's mutation pass
  (plan verification step 4).
- The `t`-based tick arithmetic in the new tests relies on the same
  f32 step math as `sequence` (in-steps 0.125..0.625 / ≈0.5 with
  ≥0.075 margin inside the windows) — deterministic, not wall-clock.
- Nothing here touches SFX, the web boot, or the TUI; 11b (the DAW UI)
  can build against `th_synth::score::ALL_KIND_IDS` / `kind_id` /
  `kind_from_id` once this lands and the wasm is rebuilt.

## FIX PASS 1

### The vacuous-test finding (blocking)

The graph-rebuild GUARD in `MusicDirector::apply_kinds` (`if !changed
{ return; }`) was NOT covered by a real test. `kind_change_reaches_the_
engine` part 1 asserted only that the engine's kind VECTOR was unchanged
after a gain-only edit — and that holds equally if the guard skips the
work OR if the work is redone identically. Proven by mutation: replacing
`if !changed` with `if false` (always mutate + always `rebuild_graph`)
left the old test PASSING.

Why it matters (the user-facing half): without the guard, EVERY score
edit — including dragging a gain slider, which fires many `setScore`
calls per second — would tear down and rebuild the whole DSP graph.
`EngineHandle::rebuild_graph` (vendor `rust-synth-0.23.0/src/audio/
engine.rs:276`) discards the old graph's internal state — reverb tails,
oscillator phases (its own doc: "There's a small audible discontinuity
during preset swaps") — so that is an audible discontinuity on every
slider tick. The guard is real behavior, not a nicety. The plan spec
(`issue-011a-impl.md`, Tests section) had explicitly warned against
this weakening: "assert ... no rebuild happened when only a gain changed
(count rebuilds — e.g. a flag or by asserting the kind vector is
unchanged; do not weaken this to a no-op)". The kind-vector assert was
the weak of the two offered observables; this pass makes the rebuild
itself observable.

### The counter (what was added, exactly where it increments)

All in `crates/th-synth/src/director.rs`:

- **Field**: `MusicDirector.kind_rebuilds: u32` (plain field, not
  atomic — the director is single-threaded), with a doc comment stating
  it is the count of graph rebuilds caused by kind swaps, the
  test/instrumentation surface mirroring how `tracks`/`score()` are
  exposed for tests.
- **Initialization**: `kind_rebuilds: 0` in `new_fallback` — the ONLY
  constructor.
- **The increment — exactly one site**: inside `apply_kinds`, inside
  the `if let Some(e) = &self.live` block, immediately after
  `e.rebuild_graph()` and after the track-mutation loop. The guard's
  no-change early return never reaches this line, and nothing else in
  the file increments it (verified: the only other references are the
  field declaration, the init, the accessor, and the test's reads).
- **Accessor**: `#[must_use] pub fn kind_rebuilds(&self) -> u32` —
  matching the file's existing test-facing accessor style (`score()`,
  `is_live()`, `audio_context_state()` are all `#[must_use] pub fn`).
- **Construction does NOT bump the counter**: `new_fallback` builds the
  tracks via `build_tracks` (tracks CONSTRUCTED with the resolved
  kinds — no `rebuild_graph` call anywhere on the construction path),
  and `try_start_live` / `AudioEngine::start_in_memory` build a FRESH
  engine from those tracks (also no `rebuild_graph`). So a fresh
  director reads `kind_rebuilds() == 0`, and the test's post-construction
  snapshot is exact.

### The test rewrite (`kind_change_reaches_the_engine`)

The existing engine-kind/name assertions are ALL KEPT (they pin the
mutation reaching the engine); the counter asserts are ADDED:

1. Snapshot `let before = d.kind_rebuilds()` after construction,
   asserted `== 0` (construction is a fresh engine, not a rebuild).
2. Gain-only edit (`slots[3].gain = 0.3`): `kind_rebuilds() == before`
   (NO rebuild) AND the gain applied AND the kind vector unchanged
   (the latter demoted to a secondary assert).
3. The real swap (`slots[3].kind = Some("super_saw")`): the engine's
   track kind + name change (existing asserts) AND
   `kind_rebuilds() == before + 1`.
4. Second swap (`"pluck_saw"`): kind vector tracks the engine's current
   kind (existing) AND `kind_rebuilds() == before + 2`.
5. Re-applying the SAME kind: `kind_rebuilds()` did NOT advance
   (no-op diff, no redundant rebuild) AND the kind vector stays put
   (existing).

### The mutation check I ran (result, observed text)

Backed up `director.rs`, changed `if !changed {` to `if false {` (the
guard is gone: every `set_score` mutates the tracks and rebuilds), ran
`cargo test -p th-synth kind_change` — the test now **FAILS** (exit 101),
panicking at the NEW counter assert:

```
assertion `left == right` failed: a gain-only edit triggers NO graph rebuild (the guard)
  left: 1
 right: 0
```

(i.e. the gain-only `set_score` advanced the counter 0 → 1 — exactly the
behavior the old kind-vector assert could not see.) Restored the guard
(byte-identical to the backup, `diff` clean) and re-ran:
`test result: ok. 1 passed` (exit 0). The guard is now genuinely pinned
in both directions: a missing rebuild under-counts (assert 3 fails), a
rebuild that runs when it should not over-counts (asserts 1/5 fail).

### Gate lines (real output, exit codes captured honestly — no tail-piping of the gate)

- `cargo fmt --all` → **FMT_EXIT=0** (no output; clean)
- `cargo build --workspace` → **BUILD_EXIT=0** — `Compiling th-synth v0.1.0 … Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.60s` (the two pre-existing "output filename collision" warnings for the `tinyhexagon` bins in th-app vs th-web — unrelated to this change)
- `cargo clippy --workspace --all-targets -- -D warnings` → **CLIPPY_EXIT=0** — `Checking th-synth v0.1.0 … Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.81s`
- `cargo test --workspace` → **TEST_EXIT=0** — every suite `test result: ok` (0 failed everywhere); th-synth lib: `test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 35.55s`
- `cargo test -p th-synth kind_change` → **TEST_EXIT=0** — `test director::tests::kind_change_reaches_the_engine ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 33 filtered out; finished in 0.27s`

### Scope note

Only `crates/th-synth/src/director.rs` changed in this pass (the
counter field + init + increment + accessor + the test's new asserts
and doc-comment update). No behavioral change to any other path; the
uncommitted issue-11a work in this worktree is otherwise untouched.
