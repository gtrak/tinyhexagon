# Issue 10b — the score lab UI: the step grid, the hot reload, the bake button

**Worktree:** `.agents/worktrees/slot-2` (branch `slot-2`), base
**`d97d4277`** (master). The main worktree and slot-1 are OFF-LIMITS. **Never
commit** — the orchestrator lands one squashed commit after review.

**Read first:**
1. `.agents/plans/002-tinyhexagon/10-scorelab.md` — the page contract and the
   frozen export names. THE SOURCE OF TRUTH.
2. `crates/th-web/site/index.html` — the WHOLE file. You are editing the page's
   module script (the `import Module from "./tinyhexagon.js"`, the
   `window.ohStatus`/`ohMod` exposure, the 1 s status poll, the gesture
   wiring).
3. `tools/web-tiny-probe.mjs` — the probe harness you extend (note how it
   launches Chrome, waits on `#status`, and asserts; keep its existing
   assertions intact).
4. `tools/audio-ladder.html` and `tools/webtone.html` — the house style for a
   diagnostic page (plain, dark, monospace, no framework).

Write-first: add the overlay's HTML/CSS + a stub `if (scorelab)` block, then
flesh out the grid before reading anything else.

## What to build

### 1. The lab overlay (`?scorelab=1` only)

A fixed, semi-transparent panel over the canvas (the game keeps running and
audible behind it — that is the point). Text-free by default remains true: the
panel exists ONLY with `?scorelab=1`.

Layout, top to bottom:

- **Transport row**: BPM slider (60–220, drives `score.bpm.base`; the ramp's
  `per_stage` untouched), a `gate_open` + `swing` slider pair, a "Copy JSON"
  button, and a status span showing `setScore`'s return (`""` = ok in green,
  the error in red).
- **Drum grid**: 3 rows (kick / snare / hat) × 16 step cells. A cell is ON when
  the CURRENT (highest matching tier) pattern for the current stage has `x` at
  that index. Clicking a cell toggles it in the tier currently in effect,
  creating the tier's `pattern` string edit — when no tier exists at or below
  the current stage for that drum, add one at stage 0 (the schema requires a
  stage-0 tier). Re-read the current stage from the status line's `stage=`
  field each second.
- **Melodic grid**: bass + lead rows × 16 cells. Clicking cycles a cell
  through `rest → 0 → 7 → 10 → 12 → 15 → 17 → 19 → 3 → rest` (a small note
  palette; the numbers are semitone offsets, matching the schema). The bass's
  `octave_shift: -1` and the lead's `+1` are NOT editable here.
- **Gain row**: 8 small sliders, one per `slots[i].gain`, labelled `1..8`.

Every edit: mutate the in-memory `score` object → `mod.setScore(JSON.stringify
(score))` → show the return. Also refresh the grid from `mod.getScore()` once
at boot (so the panel always shows the truth the engine is playing).

### 2. Hot reload (`?score=<url>`)

`const url = params.get("score")` — when present, poll
`fetch(url, {cache: "no-store"})` every 500 ms; when the text differs from the
last fetched text, call `mod.setScore(text)` and log both the outcome and the
error to the console (`[scorelab] reload: <ok|error>`). Errors also paint the
status span. This is the editor-driven loop: the operator edits
`crates/th-synth/assets/score.json` (or a copy) and saves.

Note in a comment that `?score=` and `?scorelab=1` compose (grid + live file),
and that both are inert without the query string.

### 3. Probe coverage (`tools/web-tiny-probe.mjs`)

Add a focused check (keep it OPT-IN via an env var or a flag so the default CI
probe stays fast, e.g. `OH_SCORELAB=1 node tools/web-tiny-probe.mjs <url>`):
launch with `?scorelab=1`, wait for boot, then evaluate in the page:

- `document.getElementById("scorelab") !== null` (the overlay exists),
- `mod.setScore(mod.getScore()) === ""` (a canonical round-trip is accepted),
- `mod.setScore("{ not json") !== ""` (a broken score returns an error),
- and the page's status span reflects the error.

Report those as clearly-labelled PASS/FAIL lines. Do not weaken the existing
assertions; if the harness's structure makes the opt-in awkward, extend the
harness cleanly and explain the choice in your report.

**Critical**: the exports `getScore`/`setScore` are being added to
`th-web/src/web.rs` by the parallel issue 10a. Code against exactly those
names. If `mod.setScore` is `undefined` when you test (10a not landed yet),
that is EXPECTED — implement against the frozen names, verify everything else
you can (DOM, layout, fetch loop, error painting), and say so in your report.
To test end-to-end before 10a lands, you MAY add a tiny page-side shim guarded
by `if (typeof mod.setScore !== "function")` that stores the score in a local
variable and applies edits to the grid only — clearly commented as a
temporary scaffold for the parallel-issue window, and REMOVED before you
report (the final file must call the real exports).

## Gates you must run before reporting

- `node --check` is not available for ES modules in HTML; instead validate by
  running the probe (below) and by eye.
- `bash tools/gate.sh` (the page is not compiled, but this proves you broke no
  Rust) — or at minimum `cargo fmt --all -- --check`.
- The probe against a LOCAL server you start yourself:
  `python3 -m http.server <a fresh port> --directory target/package/web/_WEB`
  then `node tools/web-tiny-probe.mjs http://127.0.0.1:<port>/` (default) and
  with `OH_SCORELAB=1`. If `target/package/web/_WEB` does not exist, the
  emsdk build is the orchestrator's — in that case say so and validate the page
  logic by extracting the module script to a temp `.mjs` and running it under
  `node` with a stubbed `mod`/DOM (describe exactly what you did).

## Report (write to `/home/gary/dev/tinyhexagon/.agents/tasks/plan-002/issue-010b-report.md`)

- The overlay's structure + how each edit maps to a schema field.
- The exact probe command + its output lines (paste).
- Whether the real `mod.setScore` was reachable, or the shim was needed (and
  confirm the shim is GONE from the final file).
- Deviations and why.
- Any skill-file correction (with the factual diff rationale).
