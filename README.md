# tinyhexagon

A kid-friendly fork of the SSVOpenHexagon Rust port: no text to read, no
menus, no fail. Hold left or right and the arena turns; the in-canvas icons
reset the stage and toggle fullscreen. Touch: either half of the screen.
Keys: `A`/`←` and `D`/`→`; `Esc` closes the window.

One continuous run: 20 stages at 30 s each (~10 minutes), always starting
at stage 0 — no storage, no profiles, no high scores, no replays. The
score is generative chiptune drum-and-bass (`th-synth`, 150→174 BPM).
On a collision the wall shatters into spinning shrapnel that bounces off
the core, and the blast chains into the two nearest walls.

## Targets

Four targets build from this tree (the gate is host-only; the other three
are CI-checked):

- **Linux x86_64 desktop** — `cargo build --release -p th-app`
  (bin `tinyhexagon`; `cargo run` launches it).
- **Windows x86_64 desktop** — cross build via `tools/build-windows.sh`.
- **Web (wasm32-unknown-emscripten)** — `tools/build-wasm-emsdk.sh` +
  `tools/package-web.sh`; the page is `crates/th-web/site/index.html`
  (the `#status` probe hook + first-gesture audio unlock).
- **Android (aarch64)** — `tools/package-android.sh`
  (NDK via `tools/android-env.sh`; `tools/check-android.sh` is the CI check).

Release artifacts: `tools/package.sh` (Linux tar.gz / Windows zip, the
binary alongside a pristine `_RELEASE` copy) and the CI workflows —
`.github/workflows/ci.yml` (gate on Linux + Windows, the Windows cross
check, the Android NDK check, the wasm build + the headless-Chrome probe
`tools/web-tiny-probe.mjs`, and the pages deploy) and
`.github/workflows/release.yml` (tagged artifacts for all four targets +
the web page).

## Tools

- `bash tools/gate.sh` — the repo gate: `cargo fmt --all -- --check`,
  `cargo build --workspace`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` (stage log: `target/gate.log`).
- `tools/smoke-windowed.sh` — boots a packaged binary windowed (Xvfb +
  Mesa on a headless box; skip-with-reason otherwise).
- `tools/web-tiny-probe.mjs` — the headless-Chrome CDP probe for the web
  page (waits on the `#status` line: boot, stage, audio unlock).
- `tools/upstream-status.sh` — the upstream delta since the high-water
  mark (see below).
- `tools/ci-deps.sh` — the CI apt prerequisites.

## Fork / upstream

Fork of the Rust port of
[SSVOpenHexagon](https://github.com/gtrak/SSVOpenHexagon) (remote
`upstream`; this fork is remote `origin`). Upstream sync is manual and
triaged: the procedure is `.agents/skills/upstream/SKILL.md`, and the
high-water mark (the newest fully-triaged upstream SHA) is the marker line
at the top of `.agents/skills/upstream/changelog.md` (parsed by
`tools/upstream-status.sh`). Plan:
[.agents/plans/002-tinyhexagon/PLAN.md](.agents/plans/002-tinyhexagon/PLAN.md).
