# Issue 10b report — the score lab UI: the step grid, the hot reload, the bake button

Worktree: `.agents/worktrees/slot-2` (branch `slot-2`, base `18e0d938`).
Changed files (nothing committed):

- `crates/th-web/site/index.html` — the page: `?scorelab=1` grid overlay + `?score=<url>` hot reload
- `tools/web-tiny-probe.mjs` — the opt-in (`OH_SCORELAB=1`) probe coverage

Rust was NOT touched (10a landed on the base commit; `getScore`/`setScore` were
real in the built wasm from the start).

## How to reach the editor

Serve the staged build (fresh port each time — stale servers have burned this
project before):

```sh
python3 -m http.server 8731 --bind 127.0.0.1 --directory target/package/web/_WEB
```

- **The grid editor**: `http://127.0.0.1:8731/?scorelab=1` — the overlay panel
  over the running game (the game keeps playing and is audible behind it).
- **The hot reload**: `http://127.0.0.1:8731/?score=http://127.0.0.1:8731/test-score.json`
  (any score file; same-origin — see the CORS note under Deviations). The two
  compose: `?scorelab=1&score=<url>` = grid + live file.
- **The probe**:
  - default (unchanged behaviour): `node tools/web-tiny-probe.mjs http://127.0.0.1:8731/ --timeout=240`
  - score lab: `OH_SCORELAB=1 node tools/web-tiny-probe.mjs http://127.0.0.1:8731/ --timeout=240`
    (the probe appends `?scorelab=1` itself when the flag is set)

## The overlay's structure

DOM (built by the module script's scorelab branch ONLY — with no query string
there is no `#scorelab` element in the DOM at all; the default page's DOM,
network, and behaviour are unchanged):

```
#scorelab (fixed, top-left, z-index 12, rgba(17,17,17,0.82) panel over the canvas)
├── .title                "score lab (stage N)" — re-read from the 1 s status poll's stage= field
├── .row (transport)
│   ├── bpm   <input range 60..220 step 1>   + value readout   → score.bpm.base
│   ├── gate  <input range 0..1 step 0.01>   + readout         → score.gate_open
│   ├── swing <input range 0..0.5 step 0.01> + readout         → score.swing
│   ├── <button id=scorelab-copy> "Copy JSON"                  → clipboard: mod.getScore() (the bake)
│   └── <span id=scorelab-status>                              → "" paints "ok" (green); the error verbatim (red)
├── .grid
│   ├── kick  16 .cell (click = toggle in the current stage's effective tier)
│   ├── snare 16 .cell
│   ├── hat   16 .cell
│   ├── bass  16 .cell (click = cycle a note through the palette)
│   └── lead  16 .cell
└── .row.gain
    └── 1..8 × (<label n> <input range 0..1 step 0.01> <readout>)  → score.slots[i].gain
```

- House style: plain dark monospace, `#111`-family, no framework (the
  `tools/audio-ladder.html` look). Semi-transparent so the game shows through.
- The cell count follows `score.steps_per_bar` (16 shipped); a reloaded score
  with a different count rebuilds the grid.
- The panel always shows the truth the engine is playing: it is sourced from
  `mod.getScore()` at boot (the 1 s poll retries while `getScore()` is `""`,
  i.e. before the director installs), re-read after a rejected push, and
  re-read after every applied hot reload.

### How each edit maps to a schema field (every edit → `mod.setScore(JSON.stringify(score))` → the status span)

| UI action | Schema mutation |
|---|---|
| drum cell click (kick/snare/hat, step `i`) | The CURRENT stage's **effective tier** = the last tier with `stage <= stage` (the director's resolution, mirrored page-side). ON→OFF: pattern char `x`→`.`. OFF→ON: char `.`→`x`; if the step is in the tier's `exclude` (a hit forced off by the engine), ON instead clears the exclusion (and a redundant exclusion on a rest char is dropped). `exclude` semantics: the cell shows and toggles the EFFECTIVE state (hit char AND not-excluded). If no tier exists at/below the stage (impossible for a valid score — `Score::from_json` validates a stage-0 tier; defensive only), one is added at `stage: 0` with an all-rest pattern. |
| melodic cell click (bass/lead, step `i`) | `score.<line>.notes[i]` cycles through the frozen palette `null → 0 → 7 → 10 → 12 → 15 → 17 → 19 → 3 → null` (semitone offsets; a note outside the palette cycles to rest next). `octave_shift` (-1 bass / +1 lead) is NOT touched. |
| gain slider `n` (1..8) | `score.slots[n-1].gain` (0..1, step 0.01) |
| BPM slider | `score.bpm.base` (60..220); `bpm.per_stage` / `bpm.cap` untouched |
| gate / swing sliders | `score.gate_open`, `score.swing` |
| "Copy JSON" | `navigator.clipboard.writeText(mod.getScore())` — the canonical pretty JSON (issue 10a's `to_json_pretty`), i.e. the bake text verbatim |
| any rejected push | the engine's score is untouched → the page re-syncs from `getScore()` so the UI never shows a change that didn't land |

### Hot reload (`?score=<url>`)

`fetch(url, {cache:"no-store"})` every 500 ms. When the text differs from the
last ACCEPTED text: `mod.setScore(text)`; the outcome AND the error log
`[scorelab] reload: ok` / `[scorelab] reload: error: <msg>` to the console;
errors also paint the status span (`reload: <msg>` in red); a success resets
the span to green "ok" and re-syncs the panel. Note: an UNAPPLIED text (a
parse error, or the pre-boot `"score lab: no director"`) is retried on the
next poll — `lastText` only advances when the engine accepted the text (see
Deviations). `?score=` and `?scorelab=1` compose; both are inert without their
query string (the fetch loop exists only with `?score=` — no network activity
on the default page).

## Probe coverage (`tools/web-tiny-probe.mjs`)

Opt-in via `OH_SCORELAB=1` (the default CI probe is unchanged — same URL, no
extra checks, same speed). With the flag the probe navigates with
`?scorelab=1` (appended when the URL lacks it) and adds four labelled checks
(run after the input phase, so the director exists):

1. `document.getElementById("scorelab") !== null` — the overlay exists
2. `window.ohMod.setScore(window.ohMod.getScore()) === ""` — the canonical
   round-trip is accepted (`window.ohMod` is the page's module instance — the
   08a exposure of `mod`)
3. `window.ohSetScore("{ not json") !== ""` — a broken score returns an error
   (through the page's `ohSetScore` wrapper — the SAME `mod.setScore` call +
   the status paint + the re-sync; a raw export call cannot paint the page's
   UI, and check 4 is precisely about the span)
4. `#scorelab-status` textContent reflects the error (non-empty, not "ok")

Existing assertions are untouched.

## Verification (real commands + real output)

Prep: the wasm package was missing, so the emsdk container build ran first
(`podman run ... emsdk ... bash tools/package-web.sh` → `WEB PACKAGED:
/work/target/package/web/_WEB`). Because that build raced the page edit, the
staged `_WEB/index.html` was refreshed from `crates/th-web/site/index.html`
(`cp`) before serving — the orchestrator's own `package-web.sh` re-stages it
idempotently.

Syntax gates:

```
$ node --check tools/web-tiny-probe.mjs            → ok
$ <module script extracted from index.html> node --check /tmp/scorelab-module-check.mjs → ok
$ cargo fmt --all -- --check                        → FMT OK
```

Serve + probe (all against `python3 -m http.server 8731 --bind 127.0.0.1
--directory target/package/web/_WEB`):

1. **Default page** — `node tools/web-tiny-probe.mjs http://127.0.0.1:8731/ --shot=target/web-tiny-10b-default.png --timeout=240`:

```
web-tiny-probe: http://127.0.0.1:8731/
  baseline : tinyhexagon web: booted=true;ticks=14;stage=0;started=true;angle=0.0000;audio=fallback;ctx=suspended;attempts=0;energy=0.0000;help=auto;adiag=device present (OpenAL/Web Audio); context suspended; last source (no play yet); al error (no play yet)
  post     : tinyhexagon web: booted=true;ticks=106;stage=0;started=true;angle=-3.6244;audio=live;ctx=running;attempts=1;energy=0.0604;help=off;...
  PASS  the game booted + started (status line reached booted=true;started=true)
  ... (all 8 baseline + 9 input + DOM/status checks PASS, as pre-10b)
  PASS  no console errors (favicon + the boot autoplay warning tolerated)
  PASS  no 4xx/5xx responses (favicon tolerated)
  PASS  no failed loads (favicon / canceled aborts tolerated)
  console  : no errors
  non-200  : none
  loads    : none
  screenshot: target/web-tiny-10b-default.png
  RESULT   : PASS
```

2. **Score lab** — `OH_SCORELAB=1 node tools/web-tiny-probe.mjs http://127.0.0.1:8731/ --shot=target/web-tiny-10b-scorelab.png --timeout=240`
(probe line: `web-tiny-probe: http://127.0.0.1:8731/?scorelab=1`):

```
  PASS  ... (all pre-10b checks PASS — the overlay does not disturb the game)
  PASS  scorelab: the overlay exists (#scorelab rendered into the DOM with ?scorelab=1)
  PASS  scorelab: the canonical round-trip is accepted (mod.setScore(mod.getScore()) === "")
  PASS  scorelab: a broken score returns an error (mod.setScore("{ not json") !== "")
  PASS  scorelab: the status span reflects the error (non-empty, not "ok")
  ...
  RESULT   : PASS
```

The screenshot (`target/web-tiny-10b-scorelab.png`) shows the rendered panel:
`score lab (stage 0)` title; bpm 150 / gate 0.70 / swing 0.15; the drum rows
with exactly the stage-0 hits (kick 0,7,10 · snare 4,12 · hat 2,6,10,14);
bass `0 · · 12 0 · 7 · 0 · 10 12 0 10 3 ·`; lead `12 · 15 · 19 17 · 15 12 ·
10 · 7 10 · 12`; all 8 gains (0.90 0.60 0.26 0.80 0.50 0.20 0.20 0.28); and
the red status span showing the probe's last broken-score error verbatim
("score lab: score json: key must be a string at line 1 column 3").

3. **URL form** (the task's `... /?scorelab=1` form, no env flag) —
`node tools/web-tiny-probe.mjs http://127.0.0.1:8731/?scorelab=1 --shot=target/web-tiny-10b-urlform.png --timeout=240`:

```
  RESULT   : PASS   (console: no errors; non-200: none; loads: none)
```

4. **Hot reload end-to-end** (throwaway CDP script, NOT committed; second
   server `python3 -m http.server 8732` serving a modified copy — `bpm.base
   = 180`, kick tier-0 step 13 on, bass step 1 = 7):

```
booted: ok
after live reload: {"bpm":180,"kick":"x......x..x..x..","bass1":7,"span":"ok"}
reload logs: [log] [scorelab] reload: ok
after 404 reload: {"span":"reload: http 404"}
404 logs: present
```

i.e. the file's text was live-swapped into the engine (read back through the
real `mod.getScore()`), logged `[scorelab] reload: ok`, span green "ok"; a
missing file's 404 logged `[scorelab] reload: error: http 404` and painted the
span red.

5. **Rust gate** — `bash tools/gate.sh` (no Rust changes; belt-and-braces):

```
== cargo fmt --all -- --check            GATE_EXIT=0
== cargo build --workspace               GATE_EXIT=0
== cargo clippy --workspace --all-targets -- -D warnings
                                         GATE_EXIT=0
== cargo test --workspace                GATE_EXIT=0
```

(full stage logs in `target/gate.log`)

## Real exports vs the shim

The REAL `mod.setScore` / `mod.getScore` were reachable from the first test —
10a landed on the base commit (`th-web/src/web.rs` exports both;
`crates/th-synth/assets/score.json` is the baked default). **No shim was ever
written, and none exists in the final file** (the only
`typeof mod.setScore !== "function"` reference is a defensive error PAINT in
`pushScore` for a hypothetical pre-10a build — it prints a message, it does
not emulate the export; the probe's round-trip check exercised the real
export, and the hot-reload test read the engine's state back through the real
`getScore`).

## Deviations and why

1. **The overlay's DOM is built in JS, not static HTML.** The spec's
   write-first suggested "add the overlay's HTML/CSS + a stub `if
   (scorelab)` block"; I kept the CSS in the `<style>` block (inert without
   the element) but build the panel in the module script's `if (scorelab)`
   branch. Reason: the plan says "the panel exists ONLY with
   `?scorelab=1`" and the page must behave EXACTLY as before — a hidden
   static div would leave a `#scorelab` trace in the default DOM. With JS
   construction the default DOM is byte-identical and the probe's
   `getElementById("scorelab") !== null` check still holds (it runs with the
   flag).
2. **The probe's broken-score check goes through `window.ohSetScore`** (a
   page-exposed wrapper: the SAME `mod.setScore(json)` call + the status paint
   + the re-sync), while the round-trip check calls the raw exports via
   `window.ohMod`. A raw export call by definition cannot paint the page's
   status span, and the fourth spec check is that the span reflects the
   error — so the error half needs the page's own push path. Both checks
   still exercise the frozen export.
3. **`lastText` advances only on an accepted text** in the hot-reload loop.
   Taken literally, the spec's "when the text differs from the last fetched
   text" would mark a pre-boot (or unparseable) fetch as "seen" and never
   retry it — the file's score would be silently dropped if the first poll
   raced the director's install. The retry-on-reject is the only way the
   editor-driven loop is reliable; the happy path is identical to the spec.
4. **Swing slider range 0..0.5** (gate 0..1). The spec fixed only the BPM
   range (60..220); the shipped swing is 0.15, and the schema accepts any
   non-negative fraction — 0.5 gives headroom without reaching nonsense.
   Noted here rather than decided silently.
5. **The probe's `pageEval` returns the raw CDP value** (no `JSON.parse`):
   my first scorelab run failed checks 3–4 because the helper parsed the
   return value, and a non-empty error string ("score lab: score json: ...")
   is not JSON — the parse failed and the error became `null`. The helper now
   returns `result.value` verbatim (documented in a comment in the probe).
   The page was fine; the harness was the bug.

### CORS note for `?score=`

`fetch` is subject to same-origin policy: a `?score=` URL on a different
origin (e.g. a second `http.server` on another port) fails with "Failed to
fetch" unless that server sends CORS headers (my first reload test hit
exactly this, cross-port; same-origin works). In practice the operator serves
the score file from the same origin as the page — e.g. serve the repo root
(or the `_WEB` directory) and point `?score=` at
`http://127.0.0.1:<same port>/crates/th-synth/assets/score.json`. No page
change: `mode:"no-cors"` would make the response unreadable.

## Skill-file correction

None needed: `.agents/skills/` contains only the upstream-library skill
files (`lib-*.md` + `upstream`); none documents the web probe, the tiny page,
or the score lab (verified by recursive grep for `web-tiny-probe`,
`scorelab`, `ohStatus`, `tinyhexagon.js` — zero matches).

## Residual risks

- The "Copy JSON" clipboard write was not exercised in the probe
  (headless clipboard permissions); the code path is a plain
  `navigator.clipboard.writeText` with an error paint + console log. A real
  browser on 127.0.0.1 (a secure context) should grant it.
- The overlay assumes `steps_per_bar <= ~64` for readability (the schema's
  max is 64; the panel scrolls).
- `target/gate.log` / the probe screenshots under `target/` are gitignored
  artifacts, not staged files.
