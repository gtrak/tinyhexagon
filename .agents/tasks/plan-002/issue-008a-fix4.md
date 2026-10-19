# Issue 08a fix 4 — hide the child-visible status line + P2 cleanups

**Worktree:** `.agents/worktrees/slot-2` (base 08a code; reviewer PASS).
Main/slot-1 off-limits. **No commit.**

## Product fix (P2 #1, must ship)

`crates/th-web/site/index.html` renders `#status` top-right; the screenshot
shows the raw `tinyhexagon web: …` debug text to the child. The game is
text-free. Hide it visually while keeping the probe contract:

- add `#status { display: none; }` (or an equivalent visually-hidden rule)
  so `window.ohStatus()` and `#status.textContent` keep working unchanged;
- do not remove the element or the 1 s status poll.

## Small P2 cleanups

1. `crates/th-web/Cargo.toml`: remove the now-unused direct `js-sys`
   dependency (the `web.rs` uses the prelude `JsValue`), and its comment.
2. `crates/th-app/src/web_fullscreen.rs:13` — "No caller yet" is stale
   (`tiny_web` calls it).
3. `crates/th-render/src/window.rs` near `#fs` (~:3665) — the page does not
   declare `#fs`; the fallback creates it. Correct the comment.
4. Stale "headless probe stays fallback" comments: update the page/tiny.rs
   note to say the 10-19 Chrome 148 run measured `audio=live` (user
   activation was granted), while the no-activation fallback guard remains.

## Verify

- Rebuild/stage via the emsdk container (the Cargo dep removal may rebuild;
  use `bash -c 'export PATH=/home/gary/.cargo/bin:$PATH; bash tools/package-web.sh'`),
  then run the probe with `OH_CHROME` on a free port; expect 19/19 clean,
  `audio=live`, and a screenshot with **no status text**.
- `bash tools/gate.sh` stays 567.
- Update the 08a report + FEED-FORWARD §08a with this final pass; stop.
  No Android/packaging, no commit.
