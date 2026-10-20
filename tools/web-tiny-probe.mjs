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
//   * the page's first-gesture wiring fired: a `[shell] audio unlock
//     requested` console log (the page's `keydown` listener called the
//     export)
//   * `document.title` contains `tinyhexagon`; a `<canvas>` exists
//     with non-zero width / height; `window.ohStatus()` + `#status`
//     both contain `tinyhexagon` + `booted=true`
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

const target = await http(`/json/new?${encodeURIComponent(url)}`, { method: 'PUT' });
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
await send('Page.navigate', { url });

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
const postAngle = Number(pf.angle);
const inputChecks = [
  ['input: audio=live (the first gesture unlocked the live synth — the fallback clock is over)', pf.audio === 'live'],
  ['input: help=off (any input disabled the helping hand — the input reached the frame)', pf.help === 'off'],
  ['input: ticks advanced (the sim ran through the key press)', Number(pf.ticks) > Number(baseline.f.ticks)],
  ['input: angle turned counter-clockwise (the A steer reached the engine)', Number.isFinite(postAngle) && postAngle < 0],
  ['input: the page fired the unlock export ([shell] audio unlock requested logged)',
    logs.some((l) => /\[shell\] audio unlock requested/.test(l))],
];

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
const realErrors = errors.filter((e) => !TOLERATED.test(e));
const realBad = bad.filter((b) => !/favicon/.test(b));
const realFailed = failed
  .filter((f) => !/favicon/i.test(f.url))
  .filter((f) => !(f.canceled && /ERR_ABORTED/.test(f.text)))
  .map((f) => `${f.text} (${f.type}) ${f.url}`);
const checks = [
  ['the game booted + started (status line reached booted=true;started=true)', baseline.f.booted === 'true' && baseline.f.started === 'true'],
  ...baseChecks,
  ...inputChecks,
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
console.log(`web-tiny-probe: ${url}`);
console.log(`  baseline : ${baseline.line}`);
console.log(`  post     : ${post?.statusText || post?.oh || '(unreachable)'}`);
for (const [name, ok] of checks) console.log(`  ${ok ? 'PASS' : 'FAIL'}  ${name}`);
console.log(`  console  : ${realErrors.length ? realErrors.map((e) => e.slice(0, 160)).join(' | ') : 'no errors'}`);
console.log(`  non-200  : ${realBad.length ? realBad.join(' | ') : 'none'}`);
console.log(`  loads    : ${realFailed.length ? realFailed.join(' | ') : 'none'}`);
console.log(`  screenshot: ${shotPath}`);
console.log(`  RESULT   : ${pass ? 'PASS' : 'FAIL'}`);
process.exit(pass ? 0 : 1);
