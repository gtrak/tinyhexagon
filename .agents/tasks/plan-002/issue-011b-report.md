# Issue 11b report — the DAW UI: track headers, the grouped drum rack, piano rolls

**Worktree:** `.agents/worktrees/slot-2` (branch `slot-2`, base `6569a4f6` —
11a landed). Nothing committed (per the task: never commit).

**Changed files (uncommitted):**

- `crates/th-web/site/index.html` — the 10b score-lab panel replaced by the
  FL-studio-shaped DAW (query-gating, `?score=<url>` hot reload, `copyText`
  with its secure-context fallback, the JSON box / Download / Load / Choose
  File row, `syncFromEngine`/`pushScore`/`paintScoreStatus`, and
  `window.ohSetScore` all preserved — the DAW is the `if (scorelab)` branch
  only; without the flag the default page is byte-identical in behaviour).
- `tools/web-tiny-probe.mjs` — the `OH_SCORELAB=1` block extended with the 6
  DAW checks (all 8 pre-existing 10b checks kept, all passing).

## The layout

```
┌─ transport: stage 0 ── bpm 150 ── gate 0.70 ── swing 0.15 ── [Clear solo] ── status
├─ tracks ─────────────────────────────────────────────────────────────────
│ step ruler: 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16   (1/5/9/13 = beats, gold)
│ KICK  [M][S] [heartbeat ▾] ▮0.90  │ ▣ . . . . ▣ . . . . . . . . . . .   │
│ SNARE [M][S] [heartbeat ▾] ▮0.60  │ . . . . ▣ . . . . . . . . ▣ . . .   │  ◄ ONE grouped
│ HAT   [M][S] [heartbeat ▾] ▮0.26  │ . . ▣ . . ▣ . . ▣ . . ▣ . . . . ▣   │  rack block:
│                                                                          │  no gaps, one
│ BASS  [M][S] [bass_pulse ▾] ▮0.80 │ octave −1 — follows the bar's chord  │  shared grid
│ (PIANO ROLL: Y = pitch −12..+24, X = step; root row 0 + ±12 marked)      │  background +
│ LEAD  [M][S] [pluck_saw ▾]  ▮0.50 │ octave +1 — key root, no chord       │  beat lines
│ (PIANO ROLL: Y = pitch −5..+31, X = step)                                 │
│ PAD   [M][S] [pad_zimmer ▾] ▮0.20 │ bar chord major [0 9 5 7] · ratio 1.00
│ BELL  [M][S] [bell ▾]     ▮0.20   │ bar chord major [0 9 5 7] · ratio 2.00
│ SUB   [M][S] [drone_sub ▾]▮0.28   │ bar chord major [0 9 5 7] · ratio 0.25
└─ JSON row: [Copy JSON][Download][Load][Choose File] + the JSON box
```

- **Track headers** (one per slot): the ROLE name (kick/snare/hat/bass/
  lead/pad/bell/sub — the driver that sequences the slot, fixed by index),
  the **M**/**S** toggles, the **instrument `<select>`** over the 8
  `ALL_KIND_IDS`, and the slot's **gain** slider (the 10b gain row moved
  into the header — one control set per slot).
- **The drum rack** (the operator's ask): the shared step ruler (1..16,
  every 4th — the beat — emphasized) + the three drum rows sit adjacent
  with **no gaps**, on one continuous grid background, with a light
  vertical beat line (every 4th column's edge) running the rack's full
  height, so they read as ONE piano-roll-idiom block. Cell x positions are
  shared: all step columns are a 23px CSS-grid column (`--cellw`) at the
  same x in the ruler, the rack, and both rolls (the probe asserts the
  three rows' first cells have identical `offsetLeft`). Hit cells are
  filled; on/off only (no invented drum velocity).
- **Piano rolls** (bass/lead): Y = pitch (rows labelled with the semitone
  offset — the schema's value; the root row `0` tinted + bold, the octave
  rows `±12` in a distinct colour), X = step. The range is the union of
  the line's notes + its root, padded one octave either way (bass
  −12..+24, lead −5..+31 for the shipped score; rebuilt only when that
  union changes). **Click sets the pitch at the step** (`notes[i] =
  offset`); **clicking the note's own cell clears to rest** (`null`) — the
  10b palette-cycle is gone, the grid IS the picker. **Drag-to-draw**:
  `mousedown` starts the paint, `mouseenter`-while-button-held writes the
  same pitch into each step passed over, `mouseup` on `window` ends it;
  touch (`touchstart`/`touchmove` resolving the cell under the finger via
  `elementFromPoint`) is the cheap optional half. The `octave_shift` is a
  static readout in the roll header ("octave −1 — the bass follows the
  bar's chord" / "octave +1 — the lead stays on the key root (ignores the
  chord)").
- **Harmony lanes** (pad/bell/sub): read-only chord display — the stage's
  chord table (major/minor by the minor window) + the slot's engine ratio.
  **Deviation (anticipated by the spec):** the status line exposes no bar
  counter (only `stage=`), so the live bar's resolved offset is NOT
  available client-side; the display shows stage + table + ratio instead,
  as the spec's fallback prescribes. No editable per-step data was
  invented (the schema has none).
- **M/S visibility** (the critical-correctness note): a muted slot's M
  button turns red and its whole row dims; a soloed slot's S button turns
  gold and its row gets a gold outline. S sets `score.solo = i` (another
  slot moves it; the soloed slot's own S clears it); **Clear solo**
  (transport) is the explicit escape hatch (`score.solo = null`) — a
  baked solo would silence the game.
- **Instrument picker**: on change sets `slots[i].kind = id` and pushes
  (the engine's `rebuild_graph` discontinuity is expected and documented;
  not hidden).

## Interaction map (control → schema field)

| Control (DOM) | Schema field |
|---|---|
| M button `#scorelab-mute-i` | `slots[i].muted` (toggle) |
| S button `#scorelab-solo-i` | `score.solo = i` / `null` |
| Clear solo `#scorelab-clear-solo` | `score.solo = null` |
| kind select `#scorelab-kind-i` | `slots[i].kind` (an `ALL_KIND_IDS` id) |
| gain slider (in the track-i header) | `slots[i].gain` |
| bpm / gate / swing sliders (transport) | `bpm.base` / `gate_open` / `swing` |
| drum cell (tracks 0-2, step n) | `kick/snare/hat` tiers: effective toggle of step n in the CURRENT stage's effective tier (the 10b `toggleDrum` logic, unchanged — hit↔rest char, `exclude` clearing, stage-0 tier creation) |
| roll cell mousedown/drag (bass/lead) | `bass.notes[i]` / `lead.notes[i]` = that pitch / `null` |
| JSON box / Copy / Download / Load / Choose File | unchanged 10b surface (`getScore` text ↔ `setScore`) |
| stage read `#scorelab-stage` (+ title) | read-only: the `stage=` field of the 1 s status poll (the drum rack shows that stage's effective tier; the chord display's major/minor table follows it) |

Every edit goes through the same `pushScore` path as 10b (`mod.setScore(JSON.stringify(lab.score))` →
`""` or the human-facing error in `#scorelab-status` → re-sync on rejection).

## Screenshots' findings (CDP-driven, headless Chrome, `?scorelab=1`)

- `/tmp/daw-top.png` — transport (bpm 150 / gate 0.70 / swing 0.15 /
  stage 0 / Clear solo) + the drum rack reads as ONE grouped block: the
  ruler's beats 1/5/9/13 gold, the three rows gapless on one background
  with the beat lines through the whole rack. Stage-0 patterns confirmed
  by eye: kick 1/8/11, snare 5/13, hat 3/7/11/15.
- `/tmp/daw-bass.png` — the bass roll (−12..+24) shows EXACTLY the
  shipped line `0,12,0,7,0,10,12,0,10,3`: DOM read of the lit cells
  `[[0,0],[3,12],[4,0],[6,7],[8,0],[10,10],[11,12],[12,0],[13,10],[14,3]]`
  (step, offset). Root row 0 tinted/bold, ±12 distinct.
- `/tmp/daw-lead.png` — the lead roll (−5..+31) shows EXACTLY
  `12,15,19,17,15,12,10,7,10,12`: `[[0,12],[2,15],[4,19],[5,17],[7,15],
  [8,12],[10,10],[12,7],[13,10],[15,12]]`.
- `/tmp/daw-mix.png` — M/S state is visible: the HAT row dimmed with a
  red M (muted), the BASS row outlined gold with a filled S (soloed).
- `/tmp/daw-chords.png` — the chord lanes: "bar chord major [0 9 5 7] ·
  ratio 1.00 / 2.00 / 0.25 · stage 0" + the JSON row (Copy JSON /
  Download / Load / Choose File + the canonical box).
- Probe shots: `/tmp/daw-probe-default.png`, `/tmp/daw-probe-scorelab.png`.

Cosmetic note (not fixed, out of scope): the panel's native UA scrollbar
renders light against the dark theme.

## Verification (commands + real output)

The `_WEB` package was stale (built pre-11a), so it was rebuilt via the
task's podman recipe. My page edits landed after the first staging pass,
so the packager ran a second time to re-stage (cargo build cached); the
final run's `_WEB` was confirmed to carry both the 11a wasm and the DAW
page before serving.

```
$ podman run --rm -v "$PWD":/work -v /home/gary/.rustup:/home/gary/.rustup \
    -v /home/gary/.cargo:/home/gary/.cargo -e RUSTUP_HOME=/home/gary/.rustup \
    -e CARGO_HOME=/home/gary/.cargo -w /work docker.io/emscripten/emsdk:latest \
    bash -c 'export PATH=/home/gary/.cargo/bin:$PATH; bash /work/tools/package-web.sh'
== zip: /work/target/package/web/tinyhexagon-web.zip (9.4M)
WEB PACKAGED: /work/target/package/web/_WEB (zip: .../tinyhexagon-web.zip)      [rc=0]

$ python3 -m http.server 18700 --bind 127.0.0.1 --directory target/package/web/_WEB

$ node tools/web-tiny-probe.mjs http://127.0.0.1:18700/ --shot=/tmp/daw-probe-default.png --timeout=240
  RESULT   : PASS

$ OH_SCORELAB=1 node tools/web-tiny-probe.mjs http://127.0.0.1:18700/ --shot=/tmp/daw-probe-scorelab.png --timeout=240
  PASS  scorelab: the overlay exists (#scorelab rendered into the DOM with ?scorelab=1)
  PASS  scorelab: the canonical round-trip is accepted (mod.setScore(mod.getScore()) === "")
  PASS  scorelab: a broken score returns an error (mod.setScore("{ not json") !== "")
  PASS  scorelab: the status span reflects the error (non-empty, not "ok")
  PASS  scorelab: the JSON box + Copy/Download/Load/Choose File controls exist
  PASS  scorelab: the JSON box holds the canonical score (getScore text)
  PASS  scorelab: Load applies an edited score to the engine (bpm 187 reachable via getScore)
  PASS  scorelab: Load rejects broken JSON and paints the span
  PASS  scorelab: the DAW renders 8 track headers with M/S + an instrument <select>
  PASS  scorelab: M sets slots[1].muted in getScore (row dims); M again clears it
  PASS  scorelab: S solos the slot in getScore (row highlights); Clear solo + the soloed S null it
  PASS  scorelab: changing the <select> sets slots[0].kind in getScore
  PASS  scorelab: the bass roll sets bass.notes[1]=7 on a cell click and clears it to rest on the second
  PASS  scorelab: the drum rack rows share the step x positions (48 cells, identical first-cell offsetLeft)
  RESULT   : PASS
  (exit 0; the run's baseline: booted=true;ticks=13;stage=0;started=true;...audio=fallback;
   post: ...audio=live;ctx=running;attempts=1;...help=off;buffers=157;bufPeak=0.7915)
```

The probe's end-of-run state restore is intact and now covers the new
checks: `canonicalScore` is captured from `getScore()` BEFORE any
mutation; every DAW check (mute/solo/kind/roll edits) runs before the
final `window.ohSetScore(canonicalScore)` put-back, so the
mixed/solo/kind edits cannot leak into later assertions.

```
$ cargo fmt --all            # clean (no Rust changes; HTML/JS only)

$ bash tools/gate.sh > /tmp/g.log 2>&1; echo $?
0
$ grep '^GATE_EXIT=' /tmp/g.log
GATE_EXIT=0
GATE_EXIT=0
GATE_EXIT=0
GATE_EXIT=0
# 70 test suites in target/gate.log, every one "test result: ok", 0 failed
```

## Deviations / notes

1. **No live bar in the status line** → the harmony lanes show
   stage + chord table + ratio (the spec's prescribed fallback). A bar
   counter in `th-app`'s status line would be the natural follow-up.
2. **`[drop stage tier]` in the transport sketch** → rendered as a
   read-only stage read: the engine's stage is driven by the game
   (there is no `setStage` wasm export), so a stage picker would be
   decoration; the rack and the chord display both track the current
   stage, which the read names.
3. **The kind `<select>` writes `kind` explicitly** even when it matches
   the slot's built-in kind (behaviour-identical; the score JSON then
   carries an explicit `kind`).
4. **Touch painting** implemented (the "optional but preferred" half).
5. The 10b "melodic grid" (palette-cycle) and the standalone gain row are
   gone by design — replaced by the rolls and the header gains.

## Could not verify

- The insecure-LAN-origin `copyText` fallback (this run was
  127.0.0.1, a secure context) — the code path is unchanged from 10b,
  whose LAN verification stands; the probe's Copy-JSON checks (box +
  behaviour) pass here.
- The audible `rebuild_graph` discontinuity on a kind swap (expected per
  the plan; headless runs measure buffers, not ears).
- Touch drag-painting on a real device (the `elementFromPoint` path is
  in place but unexercised by headless Chrome).

## Exact URL to reach the DAW

Serve `target/package/web/_WEB` (this run: `python3 -m http.server 18700
--bind 127.0.0.1 --directory target/package/web/_WEB`), then:

- **the DAW:** `http://127.0.0.1:18700/?scorelab=1`
- **DAW + hot reload:** `http://127.0.0.1:18700/?scorelab=1&score=<url>`
- the default page (no flag) is unchanged.
