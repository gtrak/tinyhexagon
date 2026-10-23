#!/usr/bin/env node
// tools/web-tiny-probe.mjs — the tinyhexagon web build's browser smoke
// (plan 002 issue 08c: the four plan-001/08a probes — web-probe /
// web-stub-probe / web-fs-probe / web-persist-probe — replaced by this
// ONE probe of the REAL game; the adult-app IME / persistence /
// fullscreen-UX probes have no tiny twin: the kid game has no text
// entry, no persistence, and the fullscreen wiring is the 08a shell's
// two corner icons + the th-render #fs fallback). Launch headless
// Chrome over CDP, navigate to the staged / live page, wait for the
// real game to BOOT + START, assert the status contract, send ONE held
// key press (the first gesture — it must reach the frame's edge AND
// exercise the `resumeAudio()` export's path), re-assert, capture a
// screenshot, and exit 0 only when every assertion passes.
//
// Why this exists: the CI's `wasm` job proves the emscripten COMPILE
// (tools/package-web.sh); this proves the tiny runtime EXECUTES in a
// real browser — the shared `th_app::tiny` boot over the preloaded
// `_RELEASE` MEMFS tree, the journey's stages, the player's steering,
// the synth's live-vs-fallback state, and the first-gesture
// audio-unlock export (the 08 shell's half). The assertions are
// status-based (the `#status` poll): headless SwiftShader's WebGL is
// flaky for pixel assertions, but the rAF loop drives the SIMULATION
// regardless — the `status()` line is the game's own state.
//
// The audio-unlock expectation under the probe: the CI / local headless
// Chrome (148+) grants user activation for the synthetic CDP keydown
// (the 10-19 Chrome 148 run measured `audio=live`), so the probe
// asserts `audio=live` AFTER the first gesture — the fallback clock's
// pre-gesture half is still asserted at baseline. A crash on the
// rejected `AudioContext.resume()` promise (cpal 0.15.3's emscripten
// `spawn_local` panic arm) would surface as a console exception and
// fail the gate.
//
// No npm dependencies (Node >= 22 has a global WebSocket); the CDP
// launch / connect reuses tools/web-probe.mjs's (retired in 08c)
// approach: the OH_CHROME / agent-browser Chrome candidates, the
// per-PID debug port, the SwiftShader flags.
//
//   node tools/web-tiny-probe.mjs [URL] [--shot=PATH] [--timeout=SECS]
//
// URL       — default http://127.0.0.1:8080/ (the CI serves the staged
//             target/package/web/_WEB with `python3 -m http.server 8080
//             --bind 127.0.0.1 --directory target/package/web/_WEB`;
//             the live-page half points at
//             https://gtrak.github.io/tinyhexagon/).
// --shot    — the screenshot's path (default target/web-tiny.png).
// --timeout — the boot wait in seconds (default 120: the `.data` is the
//             22 MB `--preload-file` package + the engine's boot —
//             assets, config, Luau, the first frame).
// OH_SCORELAB=1 — the score lab's OPT-IN half (plan 002 issue 10b,
//             extended by 11b): the default CI probe stays fast —
//             without the flag the probe is the pre-10b behaviour
//             (the plain URL, no scorelab checks). With it, the
//             probe navigates with ?scorelab=1 (appended when absent)
//             and adds the overlay's DOM + the frozen
//             getScore/setScore contract's checks (via the page's
//             window.ohMod / window.ohSetScore hooks) + the 11b DAW
//             checks (the 8 track headers with M/S + the instrument
//             selects, the M/S -> getScore round trip with the
//             visible state classes, the Clear-solo escape hatch, the
//             <select> -> slots[i].kind round trip, the bass piano
//             roll's cell-click -> bass.notes round trip, and the
//             drum rack's shared step columns).
//
// Assertions (exit 0 only when ALL pass):
//   * the game boots + starts: `#status` reaches
//     `booted=true;started=true` (a `booted=false;error=...` line
//     fails the wait FAST with the recorded reason)
//   * the status line's field set: `booted=true;ticks=...;stage=...;
//     started=...;angle=...;audio=...;energy=...;help=...` —
//     `stage` an integer ≥ 0, `angle` finite, `energy` finite,
//     `audio` ∈ {live, fallback}, `help=auto` (no input sent yet —
//     the helping hand is still armed)
//   * one held `A` key press (the first gesture, ~1.2 s):
//     `audio=live` (the first-gesture unlock flipped the synth —
//     the Chrome 148 measurement; the fallback clock covered the
//     pre-gesture frames), `help=off` (any input disables the
//     helping hand — the input reached the frame's edge), `ticks`
//     advanced (the sim ran), `angle` turned counter-clockwise (the
//     steer reached the engine)
//   * the page's first-gesture wiring fired: a `[shell] audio
//     unlocked` console log (the page's gesture listener called the
//     export and the status flipped to live)
//   * `document.title` contains `tinyhexagon`; a `<canvas>` exists
//     with non-zero width / height; `window.ohStatus()` + `#status`
//     both contain `tinyhexagon` + `booted=true`
//   * (OH_SCORELAB=1 only) the score lab: the `#scorelab` overlay
//     exists, the canonical round-trip
//     `mod.setScore(mod.getScore()) === ""`, a deliberately broken
//     score returns a non-empty error (through the page's ohSetScore
//     push path — the same mod.setScore call + the status paint), and
//     the `#scorelab-status` span reflects that error; 11b adds the
//     DAW checks (the 8 track headers with M/S + the instrument
//     <select>s, M/S/Clear-solo -> getScore round trips with the
//     visible state classes, <select> -> slots[i].kind, the bass
//     piano roll's cell-click -> bass.notes, the drum rack's shared
//     step columns)
//   * clean console: no errors / exceptions (a `/favicon.ico` 404 +
//     the boot's autoplay "AudioContext was not allowed to start"
//     warning are tolerated — the OpenAL SFX device's pre-gesture
//     AudioContext, expected before the first gesture)
//   * no 4xx/5xx responses and no failed loads (a `/favicon.ico` 404
//     and the browser's OWN canceled aborts are tolerated)
//   * a screenshot captured (the browser-execution evidence)

import { spawn } from 'node:child_process';
import { setTimeout as sleep } from 'node:timers/promises';
import { writeFileSync, existsSync, mkdirSync } from 'node:fs';

const argv = process.argv.slice(2);
const url = argv.find((a) => !a.startsWith('--')) || 'http://127.0.0.1:8080/';
const shotPath = (argv.find((a) => a.startsWith('--shot=')) || '').slice(7) || 'target/web-tiny.png';
const timeoutSecs = Number((argv.find((a) => a.startsWith('--timeout=')) || '').slice(10) || 120);
// plan 002 issue 10b — the score lab's OPT-IN half (OH_SCORELAB=1):
// the default CI probe stays fast — without the flag the probe is the
// pre-10b behaviour (the plain URL, no scorelab checks). With it, the
// probe navigates with ?scorelab=1 (appended when absent) and adds the
// overlay's DOM + the frozen getScore/setScore contract's checks.
const scorelabProbe = process.env.OH_SCORELAB === '1' || process.env.OH_SCORELAB === 'true';
const probeUrl = scorelabProbe && !/[?&]scorelab=1(?:\b|$)/.test(url)
  ? url + (url.includes('?') ? '&' : '?') + 'scorelab=1'
  : url;

const CHROME = process.env.OH_CHROME
  || ['/home/gary/.agent-browser/browsers/chrome-148.0.7778.97/chrome',
      '/usr/bin/google-chrome', '/usr/bin/chromium'].find((p) => existsSync(p));
if (!CHROME) { console.error('web-tiny-probe: no chrome found; set OH_CHROME=/path/to/chrome'); process.exit(1); }
// The CDP port is PER-PID (the lane-058 lesson: a fixed port let a run
// silently drive a STALE Chrome left behind by an earlier run). The
// user-data-dir is per-run for the same reason (a reused profile +
// cache makes a fresh build look unchanged).
const PORT = Number(process.env.OH_CDP_PORT || (10000 + (process.pid % 20000)));

const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${PORT}`, '--no-sandbox',
  '--disable-gpu', '--disable-dev-shm-usage',
  '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
  '--window-size=1280,720', '--no-first-run',
  `--user-data-dir=/tmp/th-web-tiny-probe-profile-${process.pid}`, 'about:blank'],
  { stdio: ['ignore', 'ignore', 'ignore'] });
// The Chrome's death on ANY probe exit (a `timeout`'d run's SIGTERM
// must not orphan the Chrome on its CDP port).
process.on('SIGTERM', () => { try { chrome.kill('SIGKILL'); } catch { /* already dead */ } process.exit(143); });
process.on('SIGINT', () => { try { chrome.kill('SIGKILL'); } catch { /* already dead */ } process.exit(130); });
process.on('exit', () => { try { chrome.kill('SIGKILL'); } catch { /* already dead */ } });

const http = async (p, i) => (await fetch(`http://127.0.0.1:${PORT}${p}`, i)).json();
let up = false;
for (let i = 0; i < 60 && !up; i++) { try { await http('/json/version'); up = true; } catch { await sleep(500); } }
if (!up) { chrome.kill('SIGKILL'); console.error('web-tiny-probe: chrome never opened its debug port'); process.exit(1); }

const target = await http(`/json/new?${encodeURIComponent(probeUrl)}`, { method: 'PUT' });
const ws = new WebSocket(target.webSocketDebuggerUrl);
let id = 0; const pending = new Map();
const logs = []; const errors = []; const bad = [];
// failed entries carry the URL (the requestId -> URL map from
// `requestWillBeSent`) so the /favicon.ico tolerance and the failure
// report can name the offender; `canceled` distinguishes the
// browser's OWN aborts (a re-navigation, the favicon's 404-abort)
// from genuine load failures (a refused connection, a dropped
// fetch).
const reqUrls = new Map();
const failed = [];
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
  const p = m.params || {};
  if (m.method === 'Network.requestWillBeSent') { reqUrls.set(p.requestId, p.url); return; }
  if (m.method === 'Runtime.consoleAPICalled') {
    // `error` AND `warning` console calls count (the emscripten boot's
    // own warnings would otherwise sail through the gate).
    const text = (p.args || []).map((a) => a.value ?? a.description ?? '').join(' ');
    if (p.type === 'error' || p.type === 'warning') errors.push(`[${p.type}] ${text}`); else logs.push(text);
  } else if (m.method === 'Runtime.exceptionThrown') {
    errors.push(`[exception] ${p.exceptionDetails?.exception?.description || p.exceptionDetails?.text}`);
  } else if (m.method === 'Network.responseReceived') {
    if (p.response.status >= 400) bad.push(`${p.response.status} ${p.response.url}`);
  } else if (m.method === 'Network.loadingFailed') {
    failed.push({ text: p.errorText, type: p.type, canceled: !!p.canceled, url: reqUrls.get(p.requestId) ?? '' });
  }
});
const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
await new Promise((r) => ws.addEventListener('open', r));
await send('Runtime.enable'); await send('Network.enable'); await send('Page.enable'); await send('Input.enable');
// Disable the cache outright: the probe measures what the server is
// serving RIGHT NOW (the stale-bytes lesson — the fresh per-run
// user-data-dir is the second half of that).
await send('Network.setCacheDisabled', { cacheDisabled: true });
// Re-navigate with the domains enabled: the `/json/new` target already
// started loading before this connection existed, so the second
// navigate is what the Network / Runtime capture actually sees.
await send('Page.navigate', { url: probeUrl });

// The status line (the `#status` poll's text, parsed into fields):
// `tinyhexagon web: booted=true;ticks=...;stage=...;started=...;angle=...;audio=...;energy=...;help=...`
// (or `...;booted=false;error=...` on a failed boot).
const readStatus = async () => {
  const r = await send('Runtime.evaluate', { expression: `(() => {
    let oh = '';
    if (typeof window.ohStatus === 'function') { try { oh = window.ohStatus() || ''; } catch { oh = ''; } }
    const s = document.getElementById('status');
    return JSON.stringify({ oh, statusText: s ? s.textContent : '', title: document.title,
      canvas: (() => { const c = document.querySelector('canvas'); return c ? { w: c.width, h: c.height } : null; })() });
  })()`, returnByValue: true });
  try { return JSON.parse(r.result?.result?.value ?? 'null'); } catch { return null; }
};
const parseLine = (line) => {
  const f = {};
  // The status line carries the `tinyhexagon web: ` display prefix before
  // the first field (`th-app/src/tiny.rs`'s `status_line` — both the
  // success and the `booted=false;error=` forms); without stripping it the
  // first segment parses as the key `tinyhexagon web: booted`, not `booted`.
  for (const part of String(line || '').replace(/^tinyhexagon web:\s*/, '').split(';')) {
    const i = part.indexOf('=');
    if (i > 0) f[part.slice(0, i).trim()] = part.slice(i + 1).trim();
  }
  return f;
};

// The wait: the game must BOOT and START (the 08a contract — not the
// 03c's module-resolve-only wait: the `status()` line's
// `booted=true;started=true` IS the engine's own "the journey is
// running" signal: the 22 MB `.data` preloads during the factory's
// boot, the assets / config / Luau composition follows, and the first
// recorded frame flips `started`). A `booted=false;error=...` line
// fails the wait FAST (the recorded reason is the diagnostic).
let baseline = null;
const deadline = Date.now() + timeoutSecs * 1000;
let lastLine = '';
while (Date.now() < deadline) {
  const v = await readStatus();
  if (v) lastLine = v.statusText || v.oh;
  const f = parseLine(lastLine);
  if (f.booted === 'false' && f.error) {
    chrome.kill('SIGKILL');
    console.error(`web-tiny-probe: FAIL — the boot recorded a failure: ${f.error}`);
    console.error(`  console errors: ${errors.filter((e) => !/favicon/i.test(e)).map((e) => e.slice(0, 160)).join(' | ') || 'none'}`);
    process.exit(1);
  }
  if (f.booted === 'true' && f.started === 'true') { baseline = { line: lastLine, f }; break; }
  await sleep(500);
}
if (!baseline) {
  chrome.kill('SIGKILL');
  console.error(`web-tiny-probe: FAIL — the game never reached booted=true;started=true within ${timeoutSecs}s`);
  console.error(`  last status  : ${lastLine || '(empty — the module never resolved)'}`);
  console.error(`  console errors: ${errors.filter((e) => !/favicon/i.test(e)).map((e) => e.slice(0, 160)).join(' | ') || 'none'}`);
  console.error(`  last logs     : ${logs.slice(-5).join(' | ') || '(none)'}`);
  process.exit(1);
}

// The baseline assertions (the game is running before ANY input):
// the status line's field set (booted=true;ticks=...;stage=...;
// audio=...;energy=...), the journey's stage, the angle's finiteness,
// the synth's pre-gesture state, the helping hand still armed (the
// probe's own input comes next — `help=auto` is the no-input-yet
// observable).
const stageNum = Number(baseline.f.stage);
const angleNum = Number(baseline.f.angle);
const energyNum = Number(baseline.f.energy);
const baseChecks = [
  ['baseline: started=true (the engine started the game)', baseline.f.started === 'true'],
  ['baseline: ticks is a number (the rAF loop is recording frames)', Number.isFinite(Number(baseline.f.ticks))],
  ['baseline: stage is an integer >= 0', Number.isInteger(stageNum) && stageNum >= 0],
  ['baseline: angle is finite', Number.isFinite(angleNum)],
  ['baseline: audio in {live, fallback} (the synth is armed one way or the other)', ['live', 'fallback'].includes(baseline.f.audio)],
  ['baseline: energy is finite (the beat clock is ticking)', Number.isFinite(energyNum)],
  ['baseline: help=auto (the helping hand is armed before any input)', baseline.f.help === 'auto'],
];

// The first-gesture input step: hold `A` ~1.2 s (the CDP's raw key
// down/up — the page's `window` keydown listener fires the
// `resumeAudio()` export in the SAME dispatch). The CI / local Chrome
// (148+) grants user activation for the synthetic keydown (the 10-19
// Chrome 148 measurement), so `audio` must flip to `live` — the
// fallback clock ran until this gesture, the live synth engine takes
// over from it. What must ALSO happen: no crash (the export's path
// ran) + the steer reached the frame's edge (help off, ticks
// advancing, the angle turning counter-clockwise).
const k = (type, extra = {}) => send('Input.dispatchKeyEvent', Object.assign({
  type, key: 'a', code: 'KeyA', windowsVirtualKeyCode: 65, nativeVirtualKeyCode: 65,
}, extra));
await k('rawKeyDown');
await sleep(1200);
await k('keyUp');
// Settle: let a few frames record AFTER the keyup (the post-read must
// see the input's effects + the export's path, not a mid-press frame).
await sleep(800);
const post = await readStatus();
const pf = parseLine(post?.statusText || post?.oh || '');
const audioStatsRaw = await send('Runtime.evaluate', { expression: 'JSON.stringify(window.__audioStats || {buffers:-1,maxPeak:-1})', returnByValue: true });
let audioStats = { buffers: -1, maxPeak: -1 };
try { audioStats = JSON.parse(audioStatsRaw.result.result.value); } catch { /* keep the sentinel */ }
console.log('  audioStats:', JSON.stringify(audioStats));
const postAngle = Number(pf.angle);
const inputChecks = [
  ['input: audio=live (the first gesture unlocked the live synth — the fallback clock is over)', pf.audio === 'live'],
  ['input: the WebAudio context is RUNNING (ctx=running — the setTimeout graph renders even suspended; only this proves playback)', pf.ctx === 'running'],
  ['input: the unlock attempt reached the export (attempts>=1 — a gesture that never increments this is a page/input problem)', Number(pf.attempts) >= 1],
  ['audio: the host buffers carry sound (buffer maxPeak >= 0.05 — the offline bus measures peak ~0.4; near zero = the graph is silent IN the browser)', audioStats.maxPeak >= 0.05],
  // The threshold is calibrated to the CURRENT audio architecture. The
  // one-mixer build (plan 002 issue 09): the music streams through the
  // OpenAL shim as ~21 ms queue buffers — the shim schedules each as it
  // fills, so the wrap count grows continuously (~17 by the snapshot).
  // The stall signature the check exists for was buffers=4 (the kick
  // SFX only — the queue extern bug): 10 sits safely between.
  ['audio: the host actually scheduled buffers (buffers >= 10 over the run — the stall was the SFX-only 4)', audioStats.buffers >= 10],
  ['input: help=off (any input disabled the helping hand — the input reached the frame)', pf.help === 'off'],
  ['input: ticks advanced (the sim ran through the key press)', Number(pf.ticks) > Number(baseline.f.ticks)],
  ['input: angle turned counter-clockwise (the A steer reached the engine)', Number.isFinite(postAngle) && postAngle < 0],
  ['input: the page fired the unlock wiring ([shell] audio unlocked logged)',
    logs.some((l) => /\[shell\] audio (unlocked|unlock requested)/.test(l))],
];

// plan 002 issue 10b — the score lab's checks (the OH_SCORELAB=1
// opt-in only; the default CI probe runs none of this). `window.ohMod`
// is the page's module instance (the 08a exposure — the probe's
// `mod.setScore(mod.getScore())` checks are the frozen exports called
// through it). The broken-score check goes through `window.ohSetScore`
// — the page's own push path (the SAME mod.setScore call + the status
// paint + the re-sync): a raw export call can't paint the page's UI,
// and the fourth check is precisely that the span reflects the error.
const scorelabChecks = [];
if (scorelabProbe) {
  const pageEval = async (expression) => {
    // returnByValue hands back the RAW value (a primitive — `true`, the
    // error string, the span's text): no JSON.parse (the error strings
    // are not JSON — parsing them would turn a non-empty error into a
    // parse-fail null).
    const r = await send('Runtime.evaluate', { expression, returnByValue: true });
    return r.result?.result?.value ?? null;
  };
  // The shipped score, captured BEFORE any of these checks mutate it —
  // the end-of-block restore puts the engine back on it (the Load check
  // changes the bpm, and a later assertion must not inherit that).
  const canonicalScore = await pageEval('typeof window.ohMod.getScore === "function" ? window.ohMod.getScore() : ""');

  // VISIBLE, not merely present: the panel is ALWAYS built now (so the
  // stepper is built in) and hidden until asked for. `offsetParent` is
  // useless here — the panel is `position: fixed`, which makes
  // offsetParent null even when it is on screen — so measure the rect.
  const overlayOk = await pageEval(
    '(() => { const e = document.getElementById("scorelab"); if (!e) return false; const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0; })()',
  );
  scorelabChecks.push(['scorelab: the overlay is visible (#scorelab shown with ?scorelab=1; it is always built now, so VISIBILITY is the contract)', overlayOk === true]);
  const roundTrip = await pageEval(
    'typeof window.ohMod.getScore === "function" && typeof window.ohMod.setScore === "function" && window.ohMod.setScore(window.ohMod.getScore()) === ""',
  );
  scorelabChecks.push(['scorelab: the canonical round-trip is accepted (mod.setScore(mod.getScore()) === "")', roundTrip === true]);
  const brokenErr = await pageEval('typeof window.ohSetScore === "function" ? window.ohSetScore("{ not json") : ""');
  scorelabChecks.push(['scorelab: a broken score returns an error (mod.setScore("{ not json") !== "")', typeof brokenErr === 'string' && brokenErr !== '']);
  const spanText = await pageEval('(() => { const s = document.getElementById("scorelab-status"); return s ? s.textContent : ""; })()');
  scorelabChecks.push(['scorelab: the status span reflects the error (non-empty, not "ok")', typeof spanText === 'string' && spanText !== '' && spanText !== 'ok']);

  // The JSON read/write surface (the bake + load). These exist because
  // the ORIGINAL Copy JSON only called navigator.clipboard.writeText,
  // which is SECURE-CONTEXT ONLY: on a plain-http origin (the LAN /
  // phone case — 192.168.x.x is not secure) it is `undefined` and the
  // click threw "Cannot read properties of undefined (reading
  // 'writeText')". This probe run is the regression guard. NOTE: the
  // local probe origin (127.0.0.1) IS a secure context, so a
  // server-side run cannot see the insecure path — the assertions here
  // are presencen + behaviour (the box, the buttons, Load actually
  // applying), which hold on BOTH; the insecure-Copy fallback is
  // verified by hand on the LAN origin (see the report).
  const jsonPresent = await pageEval(
    'JSON.stringify(["scorelab-json","scorelab-copy","scorelab-download","scorelab-load","scorelab-file"].map((i) => [i, !!document.getElementById(i)]))',
  );
  const jsonOk = typeof jsonPresent === 'string'
    && jsonPresent.includes('["scorelab-json",true]')
    && jsonPresent.includes('["scorelab-copy",true]')
    && jsonPresent.includes('["scorelab-download",true]')
    && jsonPresent.includes('["scorelab-load",true]')
    && jsonPresent.includes('["scorelab-file",true]');
  scorelabChecks.push(['scorelab: the JSON box + Copy/Download/Load/Choose File controls exist', jsonOk]);

  const boxHasScore = await pageEval(
    '(document.getElementById("scorelab-json")?.value || "").includes("steps_per_bar")',
  );
  scorelabChecks.push(['scorelab: the JSON box holds the canonical score (getScore text)', boxHasScore === true]);

  // Load must APPLY: paste an edited score, click Load, and see the
  // engine take it (getScore reflects it) while the span says ok.
  const loadErr = await pageEval(`(() => {
    const m = window.ohMod;
    const o = JSON.parse(m.getScore());
    o.bpm.base = 187;
    const box = document.getElementById("scorelab-json");
    box.value = JSON.stringify(o, null, 2);
    document.getElementById("scorelab-load").click();
    return String(m.getScore()).includes('"base": 187');
  })()`);
  scorelabChecks.push(['scorelab: Load applies an edited score to the engine (bpm 187 reachable via getScore)', loadErr === true]);

  // Broken JSON through Load must be rejected with the span painted.
  const loadBad = await pageEval(`(() => {
    const box = document.getElementById("scorelab-json");
    box.value = "{ not json";
    document.getElementById("scorelab-load").click();
    const s = document.getElementById("scorelab-status");
    return s ? s.textContent : "";
  })()`);
  scorelabChecks.push(['scorelab: Load rejects broken JSON and paints the span', typeof loadBad === 'string' && loadBad !== '' && loadBad !== 'ok']);

  // ---- 11b: the DAW checks. Every one of them drives the page's OWN
  // controls (the M/S buttons, the <select>s, the roll cells) and
  // reads the ENGINE's truth back through getScore() — so they fail if
  // the DAW is absent OR if an edit doesn't reach the engine. The
  // final restore below puts the engine back on the shipped score, so
  // none of these mutations leak into the rest of the run.

  // The track-header column: 8 headers, each with M + S + an
  // instrument <select> over ALL of the engine's kinds. The count is
  // taken FROM the engine (mod.getKindIds), not hardcoded: pinning a
  // literal here is what let the picker go stale at 14 of 15 kinds when
  // 11f appended fm_comp (the probe checked the stale number and passed).
  const headerState = await pageEval(`(() => {
    const want = (typeof window.ohMod.getKindIds === "function") ? window.ohMod.getKindIds().length : -1;
    let n = 0;
    for (let i = 0; i < 8; i++) {
      const r = document.getElementById("scorelab-track-" + i);
      const m = document.getElementById("scorelab-mute-" + i);
      const s = document.getElementById("scorelab-solo-" + i);
      const k = document.getElementById("scorelab-kind-" + i);
      if (r && m && s && k && k.tagName === "SELECT" && k.options.length === want) n++;
    }
    return JSON.stringify({ n, want });
  })()`);
  let headerCount = 0;
  let wantKinds = -1;
  try {
    const parsed = JSON.parse(headerState || "null");
    if (parsed) { headerCount = parsed.n; wantKinds = parsed.want; }
  } catch { /* stays 0 */ }
  scorelabChecks.push([
    `scorelab: the DAW renders 8 track headers with M/S + an instrument <select> offering every engine kind (${wantKinds})`,
    headerCount === 8 && wantKinds > 0,
  ]);

  // M toggles slots[i].muted in the engine AND the row dims visibly
  // (a hidden mute is indistinguishable from a bug when the operator
  // cannot hear the lane); M again clears both.
  const muteFlip = await pageEval(`(() => {
    const m = window.ohMod;
    const mutedOf = () => JSON.parse(m.getScore()).slots[1].muted;
    const dimmed = () => document.getElementById("scorelab-track-1").classList.contains("muted");
    document.getElementById("scorelab-mute-1").click();
    const a = mutedOf(); const d1 = dimmed();
    document.getElementById("scorelab-mute-1").click();
    const b = mutedOf(); const d0 = dimmed();
    return a === true && b === false && d1 === true && d0 === false;
  })()`);
  scorelabChecks.push(['scorelab: M sets slots[1].muted in getScore (row dims); M again clears it', muteFlip === true]);

  // S sets score.solo to the clicked index (the row highlights);
  // Clear solo nulls it; the soloed slot's own S nulls it too.
  const soloFlip = await pageEval(`(() => {
    const m = window.ohMod;
    const solo = () => JSON.parse(m.getScore()).solo;
    const hl = () => document.getElementById("scorelab-track-3").classList.contains("solo");
    document.getElementById("scorelab-solo-3").click();
    const a = solo(); const h1 = hl();
    document.getElementById("scorelab-clear-solo").click();
    const b = solo();
    document.getElementById("scorelab-solo-3").click();
    const c = solo();
    document.getElementById("scorelab-solo-3").click();
    const d = solo(); const h0 = hl();
    return a === 3 && h1 === true && b === null && c === 3 && d === null && h0 === false;
  })()`);
  scorelabChecks.push(['scorelab: S solos the slot in getScore (row highlights); Clear solo + the soloed S null it', soloFlip === true]);

  // The instrument <select> reaches slots[i].kind in the engine (the
  // select's options are the th-synth ALL_KIND_IDS set).
  const kindFlip = await pageEval(`(() => {
    const m = window.ohMod;
    const sel = document.getElementById("scorelab-kind-0");
    const kind = () => JSON.parse(m.getScore()).slots[0].kind;
    sel.value = "bell";
    sel.dispatchEvent(new Event("change", { bubbles: true }));
    const a = kind();
    sel.value = "shimmer";
    sel.dispatchEvent(new Event("change", { bubbles: true }));
    const b = kind();
    return a === "bell" && b === "shimmer";
  })()`);
  scorelabChecks.push(['scorelab: changing the <select> sets slots[0].kind in getScore', kindFlip === true]);

  // The bass piano roll exists and is a PICKER: mousedown on a cell
  // sets that pitch at the step (bass.notes[1] = 7 — the shipped bass
  // rests there); the note's own cell clicked again clears to rest.
  // The trailing window mouseup ends the roll's drag state cleanly.
  const rollFlip = await pageEval(`(() => {
    const m = window.ohMod;
    const cell = document.querySelector('#scorelab-roll-bass [data-pitch="7"][data-step="1"]');
    if (!cell || !cell.classList.contains("rcell")) return false;
    // The DAW edits the PHRASE's selected bar (issue 11g), so read the
    // bar's line, falling back to the top-level one when there is no
    // phrase. Reading the top-level line directly is what made this
    // check fail once the stepper started editing what actually plays.
    const note = () => {
      const sc = JSON.parse(m.getScore());
      const line = (Array.isArray(sc.bars) && sc.bars.length && sc.bars[0] && sc.bars[0].bass) || sc.bass;
      return line.notes[1];
    };
    cell.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    const a = note();
    cell.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    const b = note();
    window.dispatchEvent(new MouseEvent("mouseup"));
    return a === 7 && b === null;
  })()`);
  scorelabChecks.push(['scorelab: the bass roll sets the SELECTED BAR\'s bass.notes[1]=7 on a click and clears it on the second', rollFlip === true]);

  // The drum rack's 3 rows share the step columns (the grouped-block
  // contract: the same x for the same step across the rows) — 16
  // cells per row, all 48 present, first cells at identical x.
  const rackAlign = await pageEval(`(() => {
    const first = (sel) => { const c = document.querySelector(sel); return c ? c.offsetLeft : -1; };
    const a = first("#scorelab-track-0 .cell");
    const b = first("#scorelab-track-1 .cell");
    const c = first("#scorelab-track-2 .cell");
    const n = document.querySelectorAll("#scorelab .drumrow .cell").length;
    return a >= 0 && a === b && b === c && n === 48;
  })()`);
  scorelabChecks.push(['scorelab: the drum rack rows share the step x positions (48 cells, identical first-cell offsetLeft)', rackAlign === true]);

  // The rolls must stay COMPACT (the pitch-range regression): the first
  // implementation padded a full octave either side of the notes' range,
  // giving the bass a 37-row grid (notes spanned -12..+24) that pushed
  // the LEAD roll below a screen of empty rows. The range now rounds out
  // to octave BOUNDARIES instead, so each roll is at most ~3 octaves of
  // rows and both rolls are readable at once. A guard, because the
  // failure mode is invisible to every other assertion (the notes are
  // all still correct — you just cannot see half of them).
  const rollCompact = await pageEval(`(() => {
    const rows = (id) => { const el = document.getElementById(id); return el ? el.querySelectorAll(".rollrow").length : -1; };
    return JSON.stringify({ bass: rows("scorelab-roll-bass"), lead: rows("scorelab-roll-lead") });
  })()`);
  let rollRows = null;
  try { rollRows = JSON.parse(rollCompact || "null"); } catch { rollRows = null; }
  const rollOk = rollRows && rollRows.bass > 0 && rollRows.lead > 0
    && rollRows.bass <= 48 && rollRows.lead <= 48;
  scorelabChecks.push([
    `scorelab: the piano rolls stay compact (bass ${rollRows ? rollRows.bass : "?"} rows, lead ${rollRows ? rollRows.lead : "?"} rows; a full-octave pad regressed this to 37/44)`,
    rollOk === true,
  ]);

  // Leave the engine on the SHIPPED score (the Load check above changed
  // the bpm — do not leak state into the rest of the run). The canonical
  // text is the one captured BEFORE the mutations; a re-read of
  // getScore() here would just re-send the mutated score.
  await pageEval(`window.ohSetScore(${JSON.stringify(canonicalScore)})`);
}

// The DOM / status contract.
const page = post || {};
const titleOk = /tinyhexagon/.test(page.title || '');
const canvasOk = !!page.canvas && page.canvas.w > 0 && page.canvas.h > 0;
const ohOk = /tinyhexagon/.test(post?.oh || '') && /booted=true/.test(post?.oh || '');
const statusOk = /tinyhexagon/.test(page.statusText || '') && /booted=true/.test(page.statusText || '');

// The screenshot (the browser-execution evidence — the CI uploads it,
// the orchestrator's live-page run reads it with the local model's
// image capability).
const shot = await send('Page.captureScreenshot', { format: 'png' });
let shotOk = false;
if (shot.result?.data) {
  const shotDir = shotPath.replace(/\/[^/]*$/, '');
  if (shotDir !== shotPath) mkdirSync(shotDir, { recursive: true });
  writeFileSync(shotPath, Buffer.from(shot.result.data, 'base64'));
  shotOk = true;
}
chrome.kill('SIGKILL');

// A /favicon.ico 404 is tolerated (GitHub Pages serves no favicon and it
// is not the page's asset) — including the favicon's CANCELED
// `net::ERR_ABORTED` (the browser's own abort after the 404; a canceled
// load is never a server failure, while an UNCANCELED one — a refused
// connection, a dropped fetch — is). The boot's autoplay warning ("The
// AudioContext was not allowed to start") is the OpenAL SFX device's
// pre-gesture AudioContext — the 08a shell's EXPECTED first line (the
// device resumes itself on the first gesture; the synth's fallback
// clock covers the gap).
const TOLERATED = /(favicon|AudioContext.*not allowed to start|AudioCtx.*not allowed to start|autoplay)/i;
// The score lab's DELIBERATE rejections (OH_SCORELAB=1): the probe feeds
// bad JSON on purpose to prove it is rejected gracefully, and the page
// logs the rejection to the console while ALSO painting it in the status
// span (checked right above). That is the designed path — a defect would
// be a silent failure or a crash, both of which still fail. Scoped to
// exactly these two messages so any OTHER scorelab error still counts.
const SCORELAB_REJECT = /\[scorelab\] (load rejected|setScore rejected):/;
const realErrors = errors.filter(
  (e) => !TOLERATED.test(e) && !(scorelabProbe && SCORELAB_REJECT.test(e)),
);
const realBad = bad.filter((b) => !/favicon/.test(b));
const realFailed = failed
  .filter((f) => !/favicon/i.test(f.url))
  .filter((f) => !(f.canceled && /ERR_ABORTED/.test(f.text)))
  .map((f) => `${f.text} (${f.type}) ${f.url}`);
const checks = [
  ['the game booted + started (status line reached booted=true;started=true)', baseline.f.booted === 'true' && baseline.f.started === 'true'],
  ...baseChecks,
  ...inputChecks,
  ...scorelabChecks,
  ['document.title contains "tinyhexagon"', titleOk],
  ['a <canvas> exists with non-zero width/height', canvasOk],
  ['window.ohStatus() contains "tinyhexagon" + "booted=true"', ohOk],
  ['#status contains "tinyhexagon" + "booted=true"', statusOk],
  ['no console errors (favicon + the boot autoplay warning tolerated)', realErrors.length === 0],
  ['no 4xx/5xx responses (favicon tolerated)', realBad.length === 0],
  ['no failed loads (favicon / canceled aborts tolerated)', realFailed.length === 0],
  [`screenshot captured (${shotPath})`, shotOk],
];
const pass = checks.every(([, ok]) => ok);
console.log(`web-tiny-probe: ${probeUrl}`);
console.log(`  baseline : ${baseline.line}`);
console.log(`  post     : ${post?.statusText || post?.oh || '(unreachable)'}`);
for (const [name, ok] of checks) console.log(`  ${ok ? 'PASS' : 'FAIL'}  ${name}`);
console.log(`  console  : ${realErrors.length ? realErrors.map((e) => e.slice(0, 160)).join(' | ') : 'no errors'}`);
console.log(`  non-200  : ${realBad.length ? realBad.join(' | ') : 'none'}`);
console.log(`  loads    : ${realFailed.length ? realFailed.join(' | ') : 'none'}`);
console.log(`  screenshot: ${shotPath}`);
console.log(`  RESULT   : ${pass ? 'PASS' : 'FAIL'}`);
process.exit(pass ? 0 : 1);
