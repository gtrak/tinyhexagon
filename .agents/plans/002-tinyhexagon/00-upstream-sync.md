# Issue 00 — Upstream skill, changelog and status tooling

**Status:** LANDED
**Depends:** —
**Unblocks:** every later issue (the HWM is checked before each dispatch)

## Objective

Make the fork's relationship with the original Rust port explicit and
repeatable. The operator will prompt either **"pull changes from upstream"**
or **"implement this change to upstream"**; an agent skill must describe both
workflows, the crate/module mapping, and the drift rules. The changelog's
most recent SHA is the high-water mark.

## Key decisions

- Remote: `upstream = https://github.com/gtrak/SSVOpenHexagon.git` (the Rust
  port). `origin` remains `gtrak/tinyhexagon`.
- The HWM lives at the top of `.agents/skills/upstream/changelog.md` as
  `<!-- high-water-mark: <40-char-sha> -->`.
- Manual only. No scheduled workflow; the script is invoked by the operator
  or the orchestrator.
- HWM advances only after the delta up to that SHA is triaged (integrated,
  reimplemented, deferred-with-tracking, or not-applicable).
- The reverse direction ("implement this change to upstream") is recorded in
  the changelog but does **not** move the HWM.

## Files

| File | Change |
|---|---|
| `.agents/skills/upstream/SKILL.md` | New: both workflows, mapping, drift rules. |
| `.agents/skills/upstream/changelog.md` | New: HWM marker + upstream entries + fork→upstream section. |
| `.agents/skills/README.md` | Add the upstream skill to the index. |
| `tools/upstream-status.sh` | New: fetch/summary/`--check`; parses the marker. |
| `.agents/plans/002-tinyhexagon/00-upstream-sync.md` | This issue. |

## Steps

1. Add the remote (local git config):
   ```sh
   git remote add upstream https://github.com/gtrak/SSVOpenHexagon.git
   git fetch upstream
   ```
   If it exists, verify it points at the port repository.
2. Determine the baseline SHA: the upstream commit this fork corresponds to.
   Check `git merge-base HEAD upstream/master` and the fork point
   (expected `4c31561a…`, but verify, do not assume). Write the full SHA in
   the changelog marker and as the first entry's SHA.
3. Write `SKILL.md`:
   - **Repos/remotes** and direction semantics.
   - **"Pull changes from upstream"**: fetch; read the HWM; list
     `HWM..upstream/master`; for each commit, use the mapping; implement or
     reimplement; run the gate + forked reviewer; prepend a changelog entry
     and move the marker in the same commit. If a commit cannot be assessed,
     stop before it.
   - **"Implement this change to upstream"**: identify the fork commit or
     feature; map it to the upstream crate/module; prepare the upstream
     branch/patch; run the upstream gate; record it in the changelog's
     reverse section; do not move the HWM.
   - **Mapping table** (initial):

     | Old | New | State / notes |
     |---|---|---|
     | `oh-math` | `th-math` | 1:1 |
     | `oh-rng` | `th-rng` | 1:1 (parity-only state API may be trimmed) |
     | `oh-data` | `th-data` | 1:1 + beat-hue setter |
     | `oh-config` | `th-config` | 1:1 |
     | `oh-assets` | `th-assets` | 1:1 |
     | `oh-lua` | `th-lua` | replay/steam hooks removed |
     | `oh-game` | `th-game` | replay/Steam/Discord removed; soft collision, difficulty setter, beat, wall gaps added |
     | `oh-render` | `th-render` | `DrawOptions`/text suppression added |
     | `oh-app` | `th-app` | adult modules removed; tiny loop/journey/touch added; binary `tinyhexagon` |
     | `oh-audio` | `th-audio` | 1:1, wired for SFX |
     | `oh-android` | `th-android` | repurposed tiny entry |
     | `oh-web` | `th-web` | repurposed tiny entry |
     | `oh-parity` | — | removed |
     | `oh-replay` | — | removed |
     | `oh-net` | — | removed |
     | `oh-steam` | — | removed |
     | `oh-discord` | — | removed |
     | `oh-server-control` | — | removed |
     | `oh-workshop-uploader` | — | removed |
     | `oh-menu` | — | removed |
     | `oh-menu-system` | — | removed |
     | `oh-additions` | — | removed; `fullscreen.rs` salvaged into `th-app` |
     | — | `th-synth` | new (rust-synth) |

   - **Module mapping for altered files**: `oh-game/src/{update,hexagon_game,
     seams,lua_console}.rs` → `th-game/src/…`; `oh-lua/src/api/steam.rs` →
     removed and the registration list updated; `oh-data/src/style_data.rs` →
     `th-data/src/style_data.rs`; `oh-app/src/{compose,bridge,lifecycle,
     sigint,joystick}.rs` → same; `oh-app/src/windowed.rs` →
     `th-app/src/{tiny,draw_target}.rs`; `oh-app/src/menu_draw.rs` and the
     adult modules → removed; `oh-render/src/{graphics,window}.rs` → same;
     `oh-android`/`oh-web` → repurposed; `oh-additions/src/fullscreen.rs` →
     `th-app/src/web_fullscreen.rs`.
   - **Fork-only list** (do not backport): soft collision/no death, the
     20-stage arc, text suppression, beat palette override, kid touch/icons,
     rust-synth integration, the tiny pack, removed crates.
   - **Drift rules**: upstream paths may be gone; match by feature; never
     advance the HWM past an unassessed commit; record the fork commit for
     reimplementations; record deferred items with a tracking issue.
4. Write `changelog.md`:
   ```
   <!-- high-water-mark: <sha> -->
   # Upstream changelog

   ## <date> — <sha> — <subject>
   - Area: …
   - Disposition: integrated | reimplemented | deferred | not-applicable
   - Mapping: …
   - Fork commit: …
   - Notes: …

   ## Fork → upstream
   ### <date> — <fork sha> — <subject>
   - Status: proposed | landed upstream <sha>
   - Mapping: …
   - Notes: …
   ```
   The latest upstream entry is at the top; the marker is the single source
   of truth for the HWM.
5. Write `tools/upstream-status.sh`:
   - Resolve the repo root; parse the marker from the changelog; fail with a
     clear message if the marker is missing/not a 40-char SHA.
   - `git fetch upstream` unless `--no-fetch`; resolve `upstream/master`
     (fallback `upstream/main`); verify the HWM is an ancestor.
   - Print HWM, upstream HEAD, ahead/behind count, `git log --oneline
     HWM..HEAD`, and `git diff --stat`.
   - `--check` exits 1 when the delta is non-empty (for manual use); it never
     writes.
6. Add the skill to `.agents/skills/README.md`.
7. Update the issue task-spec template (to be created in `.agents/tasks/`)
   to include the current HWM and the feed-forward notes.

## Verification

- `bash tools/upstream-status.sh` prints the HWM, upstream HEAD and a delta
  (zero if the baseline is current); `--check` exits 0 when current.
- The marker is a full 40-character SHA; the changelog has the baseline entry.
- `SKILL.md` contains both workflows, the mapping table, the fork-only list
  and the drift rules.
- `git remote -v` shows `upstream` and `origin`.

## Risks

- The upstream URL or baseline assumption is wrong: verify the remote and the
  merge base before writing the HWM; report a mismatch instead of guessing.
- A shallow clone: `git fetch upstream` must have enough history; use
  `--unshallow` or a full fetch if the merge base is missing.

## Feed-forward

Record the verified baseline SHA, the upstream default branch, and any
mapping corrections in `FEED-FORWARD.md`. Later issues check the HWM before
dispatch.
