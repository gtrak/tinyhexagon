#!/usr/bin/env node
// tools/web-stub-probe.mjs — the stub wasm's browser smoke (plan 002
// issue 03c): launch headless Chrome over CDP, navigate to the staged /
// live page, wait for the stub's status to LAND, assert the DOM /
// status contract, capture a screenshot, and exit 0 only when every
// assertion passes.
//
// Why this exists: the CI's `wasm` job proves the emscripten COMPILE
// (tools/package-web.sh); this proves the stub EXECUTES in a real
// browser — the page's `window.ohStatus()` / `#status` poll must leave
// empty under the stub wasm (the `#[wasm_bindgen] status()` export),
// and the `pages` job deploys only if this passes (it needs: the wasm
// job). The assertions are DOM / status-based ON PURPOSE: headless
// SwiftShader's WebGL is flaky (the issue's risk note) and the stub
// draws nothing (the canvas may be black — the runtime lands in 07/08).
//
// No npm dependencies (Node >= 22 has a global WebSocket); the CDP
// launch / connect reuses tools/web-probe.mjs's approach (the OH_CHROME
// / agent-browser Chrome candidates, the per-PID debug port, the
// SwiftShader flags).
//
//   node tools/web-stub-probe.mjs [URL] [--shot=PATH] [--timeout=SECS]
//
// URL       — default http://127.0.0.1:8080/ (the CI serves the staged
//             target/package/web/_WEB with `python3 -m http.server 8080
//             --bind 127.0.0.1 --directory target/package/web/_WEB`;
//             the live-page half points at
//             https://gtrak.github.io/tinyhexagon/).
// --shot    — the screenshot's path (default target/web-stub.png).
// --timeout — the module-resolve wait in seconds (default 60: the
//             `.data` is the 22 MB `--preload-file` package, so the
//             wait is the MODULE RESOLVE, not the page load).
//
// Assertions (exit 0 only when ALL pass):
//   * `document.title` contains `tinyhexagon`
//   * a `<canvas>` exists with non-zero width / height
//   * `window.ohStatus()` and `#status` both contain `tinyhexagon`
//   * no console errors / exceptions (a `/favicon.ico` 404 is
//     tolerated — the same tolerance as tools/web-probe.mjs; GitHub
//     Pages serves no favicon and it is not the page's asset)
//   * no 4xx/5xx responses and no failed loads (a `/favicon.ico` 404
//     and the browser's OWN canceled aborts are tolerated — a canceled
//     load is never a server failure; an uncanceled one still fails)

import { spawn } from 'node:child_process';
import { setTimeout as sleep } from 'node:timers/promises';
import { writeFileSync, existsSync, mkdirSync } from 'node:fs';

const argv = process.argv.slice(2);
const url = argv.find((a) => !a.startsWith('--')) || 'http://127.0.0.1:8080/';
const shotPath = (argv.find((a) => a.startsWith('--shot=')) || '').slice(7) || 'target/web-stub.png';
const timeoutSecs = Number((argv.find((a) => a.startsWith('--timeout=')) || '').slice(10) || 60);

const CHROME = process.env.OH_CHROME
  || ['/home/gary/.agent-browser/browsers/chrome-148.0.7778.97/chrome',
      '/usr/bin/google-chrome', '/usr/bin/chromium'].find((p) => existsSync(p));
if (!CHROME) { console.error('web-stub-probe: no chrome found; set OH_CHROME=/path/to/chrome'); process.exit(1); }
// The CDP port is PER-PID (the lane-058 lesson in tools/web-probe.mjs:
// a fixed port let a run silently drive a STALE Chrome left behind by
// an earlier run). The user-data-dir is per-run for the same reason
// (a reused profile + cache makes a fresh build look unchanged).
const PORT = Number(process.env.OH_CDP_PORT || (10000 + (process.pid % 20000)));

const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${PORT}`, '--no-sandbox',
  '--disable-gpu', '--disable-dev-shm-usage',
  '--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader',
  '--window-size=1280,720', '--no-first-run',
  `--user-data-dir=/tmp/th-web-stub-probe-profile-${process.pid}`, 'about:blank'],
  { stdio: ['ignore', 'ignore', 'ignore'] });
// The Chrome's death on ANY probe exit (tools/web-probe.mjs's pattern:
// a `timeout`'d run's SIGTERM must not orphan the Chrome on its CDP
// port).
process.on('SIGTERM', () => { try { chrome.kill('SIGKILL'); } catch { /* already dead */ } process.exit(143); });
process.on('SIGINT', () => { try { chrome.kill('SIGKILL'); } catch { /* already dead */ } process.exit(130); });
process.on('exit', () => { try { chrome.kill('SIGKILL'); } catch { /* already dead */ } });

const http = async (p, i) => (await fetch(`http://127.0.0.1:${PORT}${p}`, i)).json();
let up = false;
for (let i = 0; i < 60 && !up; i++) { try { await http('/json/version'); up = true; } catch { await sleep(500); } }
if (!up) { chrome.kill('SIGKILL'); console.error('web-stub-probe: chrome never opened its debug port'); process.exit(1); }

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
    // Same policy as tools/web-probe.mjs: `error` AND `warning`
    // console calls count (the emscripten boot's own warnings would
    // otherwise sail through the gate).
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
await send('Runtime.enable'); await send('Network.enable'); await send('Page.enable');
// Disable the cache outright: the probe measures what the server is
// serving RIGHT NOW (the lane-058 stale-bytes lesson — the fresh
// per-run user-data-dir is the second half of that).
await send('Network.setCacheDisabled', { cacheDisabled: true });
// Re-navigate with the domains enabled: the `/json/new` target already
// started loading before this connection existed, so the second
// navigate is what the Network / Runtime capture actually sees
// (tools/web-probe.mjs's shape).
await send('Page.navigate', { url });

// The wait: the stub's status must LAND (the issue's risk note — the
// module RESOLVE, not the page load: the 22 MB `.data` preloads during
// the factory's boot). `window.ohStatus` is defined the moment the
// module scope resumes after `await Module()`; `#status` leaves empty
// within 1 s of that (the page's poll).
let settled = null;
const deadline = Date.now() + timeoutSecs * 1000;
while (Date.now() < deadline) {
  const r = await send('Runtime.evaluate', { expression: `(() => {
    const ok = typeof window.ohStatus === 'function';
    let oh = '';
    if (ok) { try { oh = window.ohStatus() || ''; } catch { oh = ''; } }
    const s = document.getElementById('status');
    return JSON.stringify({ ok, oh, statusText: s ? s.textContent : '' });
  })()`, returnByValue: true });
  try {
    const v = JSON.parse(r.result?.result?.value ?? 'null');
    if (v && v.ok && v.oh && v.statusText) { settled = v; break; }
  } catch { /* the evaluate failed — keep waiting */ }
  await sleep(500);
}
if (!settled) {
  chrome.kill('SIGKILL');
  console.error(`web-stub-probe: FAIL — the stub status never landed within ${timeoutSecs}s (the module resolve / the page's poll)`);
  const real = errors.filter((e) => !/favicon/i.test(e));
  console.error(`  console errors: ${real.length ? real.map((e) => e.slice(0, 160)).join(' | ') : 'none'}`);
  console.error(`  last logs     : ${logs.slice(-5).join(' | ') || '(none)'}`);
  process.exit(1);
}

// The assertions (DOM / status-based — see the header).
const a = await send('Runtime.evaluate', { expression: `(() => {
  const c = document.querySelector('canvas');
  return JSON.stringify({
    title: document.title,
    canvas: c ? { w: c.width, h: c.height } : null,
    ohStatus: (typeof window.ohStatus === 'function') ? window.ohStatus() : null,
    statusText: (document.getElementById('status') || { textContent: '' }).textContent || '',
  });
})()`, returnByValue: true });
const page = JSON.parse(a.result?.result?.value ?? '{}');

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

// A /favicon.ico 404 is tolerated (tools/web-probe.mjs's rule: GitHub
// Pages serves no favicon and it is not the page's asset) — including
// the favicon's CANCELED `net::ERR_ABORTED` (the browser's own abort
// after the 404; a canceled load is never a server failure, while an
// UNCANCELED one — a refused connection, a dropped fetch — is).
const realErrors = errors.filter((e) => !/favicon/i.test(e));
const realBad = bad.filter((b) => !/favicon/.test(b));
const realFailed = failed
  .filter((f) => !/favicon/i.test(f.url))
  .filter((f) => !(f.canceled && /ERR_ABORTED/.test(f.text)))
  .map((f) => `${f.text} (${f.type}) ${f.url}`);
const checks = [
  ['title contains "tinyhexagon"', /tinyhexagon/.test(page.title || '')],
  ['a <canvas> exists with non-zero width/height', !!page.canvas && page.canvas.w > 0 && page.canvas.h > 0],
  ['window.ohStatus() contains "tinyhexagon"', /tinyhexagon/.test(page.ohStatus || '')],
  ['#status contains "tinyhexagon"', /tinyhexagon/.test(page.statusText || '')],
  ['no console errors (favicon tolerated)', realErrors.length === 0],
  ['no 4xx/5xx responses (favicon tolerated)', realBad.length === 0],
  ['no failed loads (favicon / canceled aborts tolerated)', realFailed.length === 0],
  [`screenshot captured (${shotPath})`, shotOk],
];
const pass = checks.every(([, ok]) => ok);
console.log(`web-stub-probe: ${url}`);
console.log(`  status   : ${page.ohStatus ?? '(ohStatus unreachable)'}`);
for (const [name, ok] of checks) console.log(`  ${ok ? 'PASS' : 'FAIL'}  ${name}`);
console.log(`  console  : ${realErrors.length ? realErrors.map((e) => e.slice(0, 160)).join(' | ') : 'no errors'}`);
console.log(`  non-200  : ${realBad.length ? realBad.join(' | ') : 'none'}`);
console.log(`  loads    : ${realFailed.length ? realFailed.join(' | ') : 'none'}`);
console.log(`  screenshot: ${shotPath}`);
console.log(`  RESULT   : ${pass ? 'PASS' : 'FAIL'}`);
process.exit(pass ? 0 : 1);
