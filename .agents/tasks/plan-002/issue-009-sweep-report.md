# Issue 09 final sweep — report (docs / comments / dead-code only)

**Status: DONE.** Base: master @ `bb300047` (worktree slot-1 was reset from
the stale `0f8f89be` WIP checkpoint to the stated base — that commit is an
orchestrator checkpoint already superseded by the 08b/08c landings on
master; it remains in the reflog). No `git commit` performed. Nothing
behavioral touched: journey timing, music, shrapnel, collisions are the
operator-tuned `bb300047` state, byte-for-byte.

## 1. Dead web-IME machinery (persistent P2) — removed

The tiny entry has no text entry, and no page declares `#ime`, so the whole
web IME half of the issue-046 F32 plumbing was dead on every reachable path
(`ime_active` could only ever be `false`). Removed from
`crates/th-render/src/window.rs` (−657/+21 lines in the file overall):

- `WebImeFeed` (struct + `Default` + impl) and its 5 host tests.
- `attach_web_ime_listeners` (the `compositionstart` / `compositionend` /
  `input` → `TextEntered` listener attach — only ever called on a `#ime`
  element that never exists).
- `web_ime_input_get` (the `#ime` get half, called from `Window::create`)
  + the `Window::create` plumbing: the `let ime = …` get, the
  `attach_web_listeners(…, ime)` argument, the `if let Some(ime_el)`
  feed-attach block, the `ime_input` `Window` field (+ both initializers
  + its field doc), and the `web_ime_input()` accessor (zero callers
  workspace-wide — verified).
- `web_ime_is_active` (the gate's DOM half; its only consumer was the
  keydown closure).
- The keydown closure's `ime_el` capture + `ime_active` computation;
  `web_keydown_ime_policy` → **`web_keydown_policy`** with the
  always-`false` `ime_active` parameter and its two branches dropped. The
  surviving body is the byte-identical issue-32 text/prevent policy that
  was always the reachable one (`ime_active` could never be `true`), so
  the web page's behavior is unchanged.
- The keydown policy's stale doc (the DOUBLE-TYPING-gate / `#ime` /
  profile-creation paragraphs) and the test-module header; the
  `web_keydown_policy_desktop_path_unchanged_when_ime_blurred` test was
  renamed `web_keydown_policy_pins_the_issue_32_text_and_prevent_halves`
  (its IME-blurred framing is gone) and gained the
  `is_composing` → no-text/no-prevent case (moved over from the deleted
  IME-arm test, which pinned the browser-native composition guard that is
  NOT `#ime`-specific and stays live in `WebKeydownState`).
- Stale cross-references: the `#fs` get-or-create doc no longer cites the
  `#ime` twin / `web_ime_input_get_or_create`; `WebKeydownState`'s doc
  drops the "IME gate" sixth item (the bundle is the 7-argument
  code/key/five-states again, so the test helper's
  `#[allow(clippy::too_many_arguments)]` went with the 8th argument).

**Kept (the core `TextEntered` seam — not web-IME-specific):**
`Event::TextEntered`, the `entered_text_sink` field +
`set_entered_text_sink` + `with_entered_text_sink` + the `apply_event`
fold arm, the winit `KeyboardInput` → `TextEntered` restore (desktop/
Android), and the web keydown → `TextEntered` text path (issue-32). NOTE
(task ask): the tiny runtime **never consumes** `TextEntered` — no
`set_entered_text_sink` caller and no `TextEntered` reference exists in
`th-app` / `th-game`; the sink stays `None` everywhere. It is the
upstream-port seam for the (retired) menu text entry — left in place per
the task's "keep the core machinery, note it" instruction rather than
ripped out (a later issue can drop the whole seam + sink + fold arm as
one dead-surface item; it is host-visible via the pub API, so only a
deliberate removal is warranted).

Left as-is: the `BatchPresses` fast-press machinery (issue-046 F32's
separate rAF-drain half — general web keydown/touch visibility, not IME)
and the page's own "No IME / `#ime`, no text entry" contract line in
`site/index.html` (accurate product statement).

## 2. Stale comment sweep

- `crates/th-render/src/audio.rs`: `AudioDevice::describe`'s doc cited the
  retired `tools/web-probe.mjs` "audio diagnostics" section (08c's
  no-runtime-code constraint kept it). Retargeted to the live observable:
  the status line's `audio` field (`th_app::tiny::web_status`'s
  `audio=live` / `audio=fallback`, asserted by `tools/web-tiny-probe.mjs`);
  the `describe()` line is now documented as the Rust-side detail.
- `crates/th-web/site/.gitignore`: still ignored the retired crate's
  artifacts (`oh-web.js` / `oh_web.wasm` / `oh_web.data` from
  `cargo build -p oh-web`) — the live artifacts are `tinyhexagon.js` /
  `.wasm` / `.data` from `th-web` (the 08c/03 names; `tools/package-web.sh`
  stages exactly those). Re-pointed the comments + the three ignore
  patterns (also corrected `debug/` → `release/`, the profile
  `package-web.sh` builds). No game behavior; the patterns were dead.
- Grepped `crates/th-app`, `tools/`, `.github/workflows/`,
  `crates/th-web/src/web.rs`, `crates/th-web/site/index.html` for
  `web-stub` / `oh-web` / `headless`-as-product: no stale product
  references remain. The surviving hits are accurate: `headless` in
  ci.yml/smoke-windowed.sh/package.sh means headless-Chrome / no-X
  (correct usage); `tools/web-tiny-probe.mjs`'s two `web-probe.mjs` /
  `web-stub-probe` lines are explicit "replaced by this (retired in 08c)"
  provenance notes; the `03c stub` mentions in `index.html` /
  `th-web/Cargo.toml` / `bin/tinyhexagon.rs` are past-tense history of the
  replaced stub, not product framing. Nothing changed there.
- No workflow was touched → no actionlint run needed (task: "if a
  workflow was touched at all").

## 3. README.md — rewritten for the product

Now states the product (kid-friendly, no-text, one-button-each-side:
left/right + in-canvas fullscreen + reset, `A`/`←` `D`/`→`, `Esc`
closes), the one 20-stage × 30 s (~10 min) run always starting at stage 0
(no storage/profiles/high scores/replays), the generative chiptune
drum-and-bass (150→174 BPM), and the shatter-on-collision + two-nearest
chain. Four targets + build/tools scripts (gate, package.sh,
build-windows.sh, build-wasm-emsdk.sh + package-web.sh,
package-android.sh + check-android.sh, smoke-windowed.sh,
web-tiny-probe.mjs, upstream-status.sh, ci-deps.sh) + the CI workflows.
Short fork/upstream section: manual triaged syncs per
`.agents/skills/upstream/SKILL.md`; the HWM is the marker line at the top
of `.agents/skills/upstream/changelog.md` (the 00 issue's location —
`.agents/changelog.md` does not exist), parsed by
`tools/upstream-status.sh`. No emoji, no marketing. Facts verified
against `journey.rs` (MAX_STAGE/STAGE_DURATION_S/BPM), `tiny.rs` (keys,
icons, status line), `Cargo.toml` (default-members), and the tool
headers.

## 4. FEED-FORWARD.md — §09 extended

- `### 09 — tuning/docs (pending)`: the empty "Final tunables:" bullet is
  now filled with the post-pass-3/4 final values (pointer to the new
  section).
- New top-level `## 09 taste passes 3–4 (e7588188 + bb300047)` section
  (matching the `## 09 operator fix pass (bd983b62)` precedent) recording
  what later work must not undo: final mix (master 1.0; kick 0.9 / snare
  0.6 / bass 0.8 / lead 0.5; the pass-3 reverb cuts), the pass-4 groove
  (staccato 70% gate, odd-16th swing +15%, rolling bassline 11/13
  pickups, kick DnB syncopation 1/7/10), the every-gesture web unlock,
  the cursor (speedMult 0.33, ramp 0.055/stage, plateau 1.55), and the
  shrapnel (12 wedges, ~90/180 ms chain, one bump per hit; pass-4
  momentum/drag/tumble + 55%-keep core bounce) — with the exact deltas
  left in the taste report's "Taste pass 3" / "Taste pass 4" sections
  (`.agents/tasks/plan-002/issue-009-taste-report.md`), not duplicated.

## 5. Gate + evidence

Evidence in `.agents/evidence/09-sweep/`:

- `gate-baseline.log` / `gate-summary.txt` — pre-change gate at
  `bb300047`: 4/4 stages green, **581 passed / 0 failed / 1 ignored**.
- `gate-post.log` / `gate-post-tail.txt` — post-change gate: 4/4 stages
  green (fmt / build / clippy `-D warnings` / test), **575 passed / 0
  failed / 1 ignored**. The −6 is exactly the six removed dead tests
  (5 `WebImeFeed` routing tests + 1 IME-arm policy test); the surviving
  `web_keydown_policy_pins_the_issue_32_text_and_prevent_halves` covers
  the old desktop-arm assertions plus the moved composition case.
- `swept-patterns-grep.txt` — the swept patterns come back clean: no
  `WebImeFeed` / `web_ime_*` / `ime_input` / `ime_active` identifiers in
  `crates/`; `#ime` only in the tiny shell's own "No IME / `#ime`"
  contract line; no `web-probe.mjs` citation in crates/tools/workflows
  (the probe's own "retired in 08c" provenance line excepted); no
  `web-stub` / `oh-web` in th-app/th-web/tools/workflows.

## Deferred (noted, not done — out of this sweep's scope)

- The whole `TextEntered` seam (event + sink + winit/web-keydown folds)
  is unconsumed by the tiny runtime (see §1 note) — a clean follow-up
  dead-surface item, not web-IME-specific, so it was kept.
- The inert `#fs` `web_fs_button_get_or_create` seam (documented in the
  `## 09 operator fix pass (bd983b62)` feed-forward: "delete the function
  + the slot plumbing together" in a later cleanup) — untouched here;
  only its stale `#ime` doc cross-references were fixed.
- Live-web re-verification after the next push/CI (the page itself is
  unchanged by this sweep — no HTML/JS edits — so nothing new to probe).
