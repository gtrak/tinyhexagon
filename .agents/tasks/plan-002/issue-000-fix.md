# Issue 000 — review fix task spec

The forked reviewer returned `VERDICT: BLOCKING` on issue 000. Apply the two
blocking fixes and the actionable non-blocking notes, then re-run the checks.
Do not commit.

## Read first

- `.agents/plans/002-tinyhexagon/00-upstream-sync.md`
- `.agents/tasks/issue-000-impl.md`
- `.agents/skills/upstream/SKILL.md`
- `.agents/skills/upstream/changelog.md`
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md`
- `tools/upstream-status.sh`

## Blocking 1 — fill the issue-00 feed-forward section

In `.agents/plans/002-tinyhexagon/FEED-FORWARD.md`, replace the
`### 00 — upstream (pending)` block's empty fields with the verified facts and
mark it landed:

```
### 00 — upstream (LANDED)

- Baseline SHA / default branch:
  4c31561ac6409e24a1b838043a0bb747f504045d / master
- Remote verified: `upstream` = `https://github.com/gtrak/SSVOpenHexagon`
  (fetch/push); upstream HEAD == HWM == fork HEAD; delta 0.
- Mapping corrections: none. All 22 baseline crates accounted for:
  12 kept/repurposed (`oh-*` → `th-*`, android/web repurposed), 10 removed,
  1 new (`th-synth`).
- Open questions: none. `shellcheck` is unavailable in this environment; the
  script was checked with `bash -n` and synthetic-repo cases (behind/current,
  bad marker, missing remote).
```

## Blocking 2 — complete the upstream mapping

In `.agents/skills/upstream/SKILL.md`:

1. Add these rows to the module mapping table:
   - `oh-game/src/lua_state.rs` → `th-game/src/lua_state.rs` — replay/Steam
     hooks (`GameApi::in_replay`, `GameApi::unlock_achievement`) removed in
     Issue 02.
   - `oh-game/src/status.rs` → `th-game/src/status.rs` — bump/flash fields
     added in Issue 04.
2. Update the `oh-game` crate row so the removal is explicit: replay, Steam,
   Discord **and the profile/score/PB system** are removed; soft collision,
   difficulty setter, beat, wall gaps added. An upstream change in the
   profile/score/PB area must be triaged as not-applicable/adapted, not kept.

## Non-blocking — apply as well

### A. Create `.agents/tasks/TEMPLATE.md`

Issue 00 step 7 asks for the issue task-spec template. Create a reusable
template with these sections and a filled HWM/feed-forward header:

- Title / issue number.
- **HWM**: the current marker SHA and a reminder to run
  `tools/upstream-status.sh` before dispatch.
- Context and read-first links (plan, issue, upstream skill).
- Frozen contract: the issue's scope fence + deliverables.
- Constraints: no commit, no unrelated edits, fresh worker, skills are truth.
- Steps.
- Verification commands.
- Report format (including "what changed vs the plan" and "feed-forward").
- Feed-forward: the fields to record after landing, and the instruction to
  update the affected later issue files.

Keep it generic; the per-issue spec fills the placeholders.

### B. Verify the upstream URL, not just the remote name

In `tools/upstream-status.sh`, after the remote-name check, read
`git remote get-url upstream` and fail clearly unless it points at the
SSVOpenHexagon port repo (accept `https://…/SSVOpenHexagon[.git]` and the SSH
form; reject a C++-only or unrelated URL). Keep the `set -euo pipefail`
discipline and the existing output. Do not make the check network-dependent.

## Verification (paste the output)

```sh
bash -n tools/upstream-status.sh
bash tools/upstream-status.sh --no-fetch
bash tools/upstream-status.sh --check --no-fetch; echo "check_rc=$?"
head -1 .agents/skills/upstream/changelog.md
git remote get-url upstream
```

Also confirm the new template exists and the two mapping rows are present:
```sh
ls -l .agents/tasks/TEMPLATE.md
rg -n 'lua_state|status.rs|profile/score' .agents/skills/upstream/SKILL.md
```

Run `cargo fmt --all -- --check` (no Rust changes expected; it should pass).
Do not run the full gate.

## Report format

- Files changed.
- The verification output.
- The exact edits for the two blocking findings.
- Any deviation or open question.
- Do not commit.
