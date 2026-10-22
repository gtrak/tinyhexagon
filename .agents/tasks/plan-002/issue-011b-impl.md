# Issue 11b — the DAW UI: track headers, the grouped drum rack, piano rolls

**Worktree:** `.agents/worktrees/slot-2` (branch `slot-2`), base **the landed
11a commit** (the orchestrator will tell you the exact SHA — do NOT start from
an older base). Main worktree and slot-1 are OFF-LIMITS. **Never commit.**

**Read first:**
1. `.agents/plans/002-tinyhexagon/11-daw.md` — the model, the role/timbre
   split, the mix semantics. THE SOURCE OF TRUTH.
2. `crates/th-synth/src/score.rs` — the FINAL schema (from 11a): `SlotGain`
   now has `kind: Option<String>` + `muted: bool`, and `Score` has
   `solo: Option<u32>`. Read `ALL_KIND_IDS` for the instrument ids.
3. `crates/th-web/site/index.html` — the WHOLE file. You are REPLACING the
   issue-10b panel with the DAW. Keep: the `?scorelab=1` gating, the
   `?score=<url>` hot reload, `copyText`, the JSON box / Download / Load /
   Choose File row, `syncFromEngine`/`pushScore`/`paintScoreStatus`, and the
   `window.ohSetScore` probe hook.
4. `tools/web-tiny-probe.mjs` — the `OH_SCORELAB=1` checks you must keep
   passing (and extend).
5. `tools/audio-ladder.html` — the house style (plain, dark, monospace).

Write-first: restructure the panel into a track-header column + a lane area,
render the drum rack, then add the piano rolls.

## The layout (FL-Studio-shaped)

```
┌─ transport: [play info] bpm ── gate ── swing ── [Clear solo] ── [drop stage tier] ── status
├─ tracks ────────────────────────────────────────────────────────────
│ KICK    [M][S] [heartbeat ▾] ▮gain  ▮pan?   | step grid, 16 cells
│ SNARE   [M][S] [heartbeat ▾] ▮gain          | step grid, 16 cells
│ HAT     [M][S] [heartbeat ▾] ▮gain          | step grid, 16 cells
│ BASS    [M][S] [bass_pulse▾] ▮gain          | PIANO ROLL  (pitch x step)
│ LEAD    [M][S] [pluck_saw ▾] ▮gain          | PIANO ROLL  (pitch x step)
│ PAD     [M][S] [pad_zimmer▾] ▮gain          | chord display (bar-following)
│ BELL    [M][S] [bell ▾]     ▮gain          | chord display
│ SUB     [M][S] [drone_sub ▾]▮gain          | chord display
└─ JSON row (existing)
```

A **track header** per slot: the name (its ROLE — kick/snare/hat/bass/lead/
pad/bell/sub), a **M** and **S** toggle, an **instrument picker** (a `<select>`
over `ALL_KIND_IDS`), and the slot's **gain** slider (moving the existing gain
row into the header — one control per slot, not a separate strip).

### The drum rack, "grouped together like a piano roll" (the operator's ask)

The three drum lanes (kick/snare/hat) must read as ONE grouped instrument
block, in the visual idiom of a piano roll:

- Render the three rows adjacently with **no gap between them**, sharing a
  single continuous grid background, so they visually form one 3-row rack
  (the way a drum rack's three "pitches" stack in an FL piano roll).
- A shared **step ruler** on top of the rack (step numbers 1..16, with every
  4th — the beat — emphasized), and a light vertical line on beat boundaries
  running through the whole rack, so beats read at a glance.
- Each row keeps its own label + M/S/instrument header, but the cell area is
  one aligned block: same x for the same step across all three rows.
- A hit cell is filled and shows a **velocity-ish size hint is NOT needed**
  (the schema has no drum velocity — do not invent one); just on/off.
- The existing tier semantics stay: the displayed state is the CURRENT stage's
  effective tier; a click toggles that tier (creating a stage-0 tier if a
  hand-built score lacks one), and clicking a step forced off by `exclude`
  clears the exclusion. Keep the issue-10b `toggleDrum` logic — it is correct.

### The piano rolls (bass + lead)

A real pitch grid, FL-style:

- **Y = pitch**, **X = step**. The pitch range: the union of the line's notes
  plus its root, padded one octave either way, so a typical line is fully
  visible. Label the rows with the **semitone offset** (the schema's value) —
  and mark the root row (offset 0) and the octave rows (±12) distinctly so the
  grid is readable.
- A note **cell is clickable**: clicking sets that pitch at that step (i.e.
  `notes[i] = <that offset>`), clicking the note's own cell again clears it to
  rest (`null`). This replaces the 10b "cycle through a palette" interaction —
  a roll must let you pick the pitch directly. Keep the palette as a
  keyboard-free fallback? NO — the grid IS the picker now; simpler and it is
  what was asked for.
- **Drag to draw**: mousedown on a cell sets the note, and dragging across
  steps with the button held writes the same pitch to each step passed over
  (a `mousedown`/`mouseenter`-while-down pattern, plus `mouseup` on window).
  This is the basic FL interaction and makes writing a line fast. Touch:
  `touchstart`/`touchmove` resolving to the cell under the finger is optional
  but preferred if cheap.
- Show the **octave_shift** as a static readout per roll (bass -1, lead +1)
  and make clear the bass follows the bar's chord while the lead does not
  (a one-line note in the roll's header).

### Harmony lanes (pad/bell/sub)

They follow the bar chord; there is no per-step data. Render a **read-only
chord display**: for the current bar, the resolved offset (from
`chords.major`/`minor` and the bar number) and the slot's ratio, so the
operator can see what the pad is doing. Do NOT invent editable per-step data
for these — the schema has none and the engine ignores it. Get the current bar
from the status line if it is available; if it is not, show the stage and the
chord table instead and say so in your report.

### Mix controls

- **M** toggles `score.slots[i].muted` (pushScore). **S** toggles
  `score.solo` between `Some(i)` and `None` (clicking the soloed slot's S
  clears it; clicking another moves the solo). A **Clear solo** button in the
  transport is the explicit escape hatch (a baked solo would silence the
  game — the plan requires this button).
- The M/S buttons must reflect state visibly (muted = dimmed row, soloed =
  highlighted) because a hidden mute is indistinguishable from a bug when the
  operator cannot hear that lane.
- Instrument picker: on change, set `score.slots[i].kind = id` and push. The
  engine's graph rebuild is audible (a brief discontinuity) — that is expected
  and documented in the plan; do not try to hide it.

### State the JSON row must keep doing

`copyText` (the secure-context fallback — do not regress it), the JSON box
mirroring the engine (skip refresh while focused), Load, Choose File,
Download. All of issue 10b's probe checks must keep passing.

## Probe coverage (extend `OH_SCORELAB=1`)

Add checks that would fail if the feature were absent:
- the track-header column renders 8 headers with M/S + an instrument `<select>`;
- clicking M on a slot sets `muted: true` in `getScore()`, clicking again
  clears it;
- clicking S sets `solo` to that index in `getScore()`; Clear solo nulls it;
- changing a `<select>` changes that slot's `kind` in `getScore()`;
- the bass piano roll exists and setting a note cell changes `bass.notes[i]`
  in `getScore()`;
- the drum rack's three rows share the same cell x positions (assert the three
  rows' first cells have identical `offsetLeft`);
- **leave the engine on the shipped score at the end** (capture the canonical
  score BEFORE mutating, as the existing code does — the mixed/solo/kind edits
  must not leak into the rest of the run).

## Verification you must run

- `bash tools/gate.sh > /tmp/g.log 2>&1; echo $?` and grep the GATE_EXIT lines
  (never pipe the gate to tail).
- Serve on a **fresh high port** (stale servers have burned this project) and
  run BOTH: `node tools/web-tiny-probe.mjs http://127.0.0.1:<port>/` (default
  PASS) and `OH_SCORELAB=1 ... ` (PASS).
- **Screenshot the panel** and verify by eye that the drum rack reads as one
  grouped block and the piano rolls show the actual notes of the shipped
  score (bass 0,12,0,7,0,10,12,0,10,3 / lead 12,15,19,17,15,12,10,7,10,12).
- Report the exact commands + real output lines.

## Report (write to `.agents/tasks/plan-002/issue-011b-report.md` RELATIVE)

The layout, the interaction map (each control -> schema field), the screenshots'
findings, the probe lines, deviations, and anything you could not verify.
