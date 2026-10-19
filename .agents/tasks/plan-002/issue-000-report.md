# Issue 000 report — upstream skill + changelog + status tooling

**Status:** implementation complete; forked review PASS (two review rounds, P2 notes applied); committed with this issue.
**Worker:** fresh `worker` on `local/local` (three runs: initial, review fix, P2 alignment).
**Orchestrator:** independently re-ran the targeted checks; full Rust gate not run
(docs + one shell script only).

## Deliverables

- `.agents/skills/upstream/SKILL.md` — both operator workflows, HWM rule,
  verified crate/module mapping, fork-only list, drift rules.
- `.agents/skills/upstream/changelog.md` — first-line marker
  `<!-- high-water-mark: 4c31561ac6409e24a1b838043a0bb747f504045d -->`,
  baseline entry, upstream→fork format, empty fork→upstream section.
- `tools/upstream-status.sh` — executable, `set -euo pipefail`, `--no-fetch`,
  `--check`, marker parse, 40-hex validation, remote name + URL check,
  `upstream/master`→`upstream/main` fallback, ancestor warning, never writes
  tracked files or the HWM (the default fetch updates remote-tracking refs).
- `.agents/skills/README.md` — upstream row added.
- `.agents/tasks/TEMPLATE.md` — reusable issue spec with HWM + feed-forward.
- `.agents/tasks/review-gate.md` — orchestrator-owned plan-002 protocol
  (replaces the retired port gate; committed with this issue).
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` — §00 filled as LANDED.

## Baseline (verified at dispatch and re-verified after the fix)

- `upstream` = `https://github.com/gtrak/SSVOpenHexagon`
- `upstream/master` HEAD = `4c31561ac6409e24a1b838043a0bb747f504045d`
- Fork HEAD = same; `merge-base` = same; delta 0.
- Default branch: `master`.

## Verification

Worker (initial + fix):

```
bash -n tools/upstream-status.sh                  -> clean
bash tools/upstream-status.sh --no-fetch          -> HWM/upstream HEAD same, commits: 0
bash tools/upstream-status.sh --check --no-fetch  -> check: current, rc=0
head -1 changelog.md                              -> marker (40-hex)
git remote get-url upstream                       -> https://github.com/gtrak/SSVOpenHexagon
cargo fmt --all -- --check                        -> rc 0
```

Synthetic-repo cases (worker, disposable `/tmp`): 2-commit-behind → list +
`--check` rc 1; bad 39-hex marker → rc 1; non-marker first line → rc 1;
unknown arg → rc 2; missing `upstream` remote → rc 1; missing changelog →
rc 1; URL check rejects `ssvin/SSVOpenHexagon` and `gtrak/tinyhexagon`,
accepts HTTPS/SSH forms of `SSVOpenHexagon`. `shellcheck` unavailable.

Orchestrator re-run (same tree):

```
bash -n tools/upstream-status.sh                  -> ok
bash tools/upstream-status.sh --no-fetch          -> commits: 0
bash tools/upstream-status.sh --check --no-fetch  -> check_rc=0
git remote get-url upstream                       -> https://github.com/gtrak/SSVOpenHexagon
head -1 .agents/skills/upstream/changelog.md      -> marker
rg -n 'lua_state|status\.rs|profile/score' SKILL.md -> lines 87, 113, 114
cargo fmt --all -- --check                        -> fmt_rc=0
```

## Review fix round

First forked reviewer: `VERDICT: BLOCKING` with two findings:

1. `FEED-FORWARD.md` §00 still `(pending)` → filled with baseline SHA/branch,
   remote verification, mapping corrections, open questions; marked LANDED.
2. `SKILL.md` module mapping omitted `oh-game/src/lua_state.rs` and
   `status.rs`, and the `oh-game` crate row omitted the profile/score/PB
   removal → both rows added; crate row updated with the triage rule.

Non-blocking notes applied: `TEMPLATE.md` created; upstream URL verification
added to the script. Worker report saved as this file.

## Deviations / open questions

- None. The full Rust gate was intentionally not run for this docs/script-only
  issue; the orchestrator ran `cargo fmt --check` and the script checks.
- The mapping rows carry notes in the fork-path column, matching the existing
  table style.

## Commit

All deliverables are in the worktree; no worker commit. The orchestrator
ran the checks and committed after the re-review returned `VERDICT: PASS`
(the P2 alignment notes are included in the same commit).
