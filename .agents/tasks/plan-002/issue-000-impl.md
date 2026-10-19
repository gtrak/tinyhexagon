# Issue 000 — implementation task spec (upstream skill + changelog + status tooling)

## Context

- Active plan: `.agents/plans/002-tinyhexagon/PLAN.md`
- Issue contract: `.agents/plans/002-tinyhexagon/00-upstream-sync.md`
- Repo: `tinyhexagon` (fork). `origin` = `gtrak/tinyhexagon`;
  `upstream` already exists = `https://github.com/gtrak/SSVOpenHexagon`
  (the Rust port).
- Verified upstream HEAD at dispatch:
  `4c31561ac6409e24a1b838043a0bb747f504045d` (branch `master`), identical to
  this fork's HEAD. That is the baseline high-water mark.
- No scheduled workflow is wanted. The operator will prompt
  "pull changes from upstream" or "implement this change to upstream".
- Do not commit. Leave the deliverables in the worktree; the orchestrator
  commits after the gate and the forked review.

## Working agreement

- Read the plan, the issue file, and the existing `.agents/skills/README.md`
  before writing.
- Write the deliverables listed below; do not modify unrelated files.
- Use the real repo state for the mapping (crate directories, `Cargo.toml`
  members, module files). Do not invent mappings.
- The changelog is the single source of truth for the HWM: the most recent
  SHA is at the top and is machine-readable.
- Keep the docs concise and greppable. No large prose essays.
- Report any mismatch (remote, baseline SHA, missing paths) instead of
  guessing.

## Read first

1. `.agents/plans/002-tinyhexagon/PLAN.md` (especially §2.5 and §7)
2. `.agents/plans/002-tinyhexagon/00-upstream-sync.md`
3. `.agents/skills/README.md` (index format)
4. Root `Cargo.toml` and `ls crates/` (for the mapping)
5. `git remote -v`, `git log --oneline -1`, `git rev-parse HEAD`

## Deliverables

### 1. `.agents/skills/upstream/SKILL.md`

A source-verified workflow skill for two directions:

- **Pull changes from upstream** (upstream → fork):
  fetch; read the HWM from `changelog.md`; list `HWM..upstream/master`; triage
  each commit using the mapping; implement/reimplement; run the gate and the
  forked reviewer; prepend a changelog entry and move the marker in the same
  commit that lands the integration. If a commit cannot be assessed, stop
  before it. Record deferred items with a tracking issue; record
  not-applicable commits with the reason.
- **Implement this change to upstream** (fork → upstream):
  identify the fork commit/feature; map it to the upstream crate/module;
  prepare the upstream branch/patch; run the upstream gate; record it in the
  changelog's reverse section; do not move the HWM.

Include:

- The repo/remote semantics and the exact HWM rule.
- The crate mapping table (old `oh-*` → new `th-*`, removed crates, new
  `th-synth`). Verify against the current `crates/` list and the plan's
  removal list.
- The module mapping for altered files, at minimum:
  `oh-game/src/{update,hexagon_game,seams,lua_console}.rs` →
  `th-game/src/…`; `oh-lua/src/api/steam.rs` → removed; `oh-data/src/
  style_data.rs` → `th-data/src/style_data.rs`; `oh-app/src/
  {compose,bridge,lifecycle,sigint,joystick}.rs` → same;
  `oh-app/src/windowed.rs` → `th-app/src/{tiny,draw_target}.rs`;
  `oh-app/src/menu_draw.rs` and the adult modules → removed;
  `oh-render/src/{graphics,window}.rs` → same; `oh-android`/`oh-web` →
  repurposed; `oh-additions/src/fullscreen.rs` →
  `th-app/src/web_fullscreen.rs`.
- The fork-only list (do not backport): soft collision/no death, the
  20-stage/10-minute arc, text suppression, beat palette override, kid
  touch/icons, rust-synth integration, the tiny pack, the removed crates.
- Drift rules: upstream paths may be gone; match by feature; never advance
  the HWM past an unassessed commit; record the fork commit for
  reimplementations; the reverse direction never moves the HWM.

### 2. `.agents/skills/upstream/changelog.md`

Exact top marker (the verified baseline):

```
<!-- high-water-mark: 4c31561ac6409e24a1b838043a0bb747f504045d -->
```

Then:

- A short explanation that the top SHA is the HWM.
- A baseline entry for `4c31561a` (date, subject, area, disposition
  `integrated`/baseline, mapping `n/a`, fork commit `4c31561a`, notes).
- The upstream → fork table/format described in the issue file.
- A `## Fork → upstream` section (empty initially, or a one-line placeholder).

### 3. `tools/upstream-status.sh`

Executable bash, `set -euo pipefail`, no writes, no network unless fetching:

- Resolve repo root from the script path.
- Parse the marker from `.agents/skills/upstream/changelog.md`; fail clearly
  if missing or not 40 hex chars.
- `--no-fetch` skips `git fetch upstream`; otherwise fetch it (fail clearly
  if the remote is missing).
- Resolve `upstream/master`, falling back to `upstream/main`.
- Verify the HWM is an ancestor; print a warning and the merge base if not.
- Print: HWM, upstream HEAD, commit count, `git log --oneline HWM..HEAD`,
  and `git diff --stat HWM..HEAD`.
- `--check`: exit 1 when the delta is non-empty; exit 0 otherwise. It still
  prints the same summary.
- Do not modify the changelog or the HWM.
- Keep it POSIX-friendly bash; quote paths.

### 4. `.agents/skills/README.md`

Add the upstream skill to the index table (skill name, purpose, source of
truth = this repo's remotes + `.agents/skills/upstream/`). Keep the existing
format.

## Verification (run these, paste the output in the report)

```sh
bash -n tools/upstream-status.sh
bash tools/upstream-status.sh --no-fetch
bash tools/upstream-status.sh --check --no-fetch; echo "check_rc=$?"
git remote -v
head -1 .agents/skills/upstream/changelog.md
```

Expected: the HWM is `4c31561a...`, the delta is 0 commits, `check_rc=0`,
`upstream` exists, the marker is the first line.

If shellcheck is available, run it on the script; otherwise note that it was
not available. Do not run the full Rust gate (this issue touches only docs and
one shell script); the orchestrator will run the gate.

## Report format

- Files created/changed.
- The exact verification output.
- The mapping table summary (crate/module) and any corrections to the plan.
- Any mismatch (remote URL, baseline, missing files).
- Deviations and open questions.
- Do not commit.
