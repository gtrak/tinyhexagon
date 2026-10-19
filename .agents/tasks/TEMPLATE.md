# Issue <NNN> — <title> — implementation task spec

Copy this template to `.agents/tasks/issue-<NNN>-impl.md` (or a clearer
name) and fill in every `<placeholder>`. The per-issue spec is the frozen
contract for the worker; deviations must be reported, not absorbed.

## HWM

- Current marker: `<40-hex-sha>` — line 1 of
  `.agents/skills/upstream/changelog.md` (at the time this template was
  written: `4c31561ac6409e24a1b838043a0bb747f504045d`).
- Run `tools/upstream-status.sh` **before dispatch** and note its output in
  the spec or the report; do not dispatch while the delta is non-empty
  without an explicit orchestrator decision.

## Context

- Active plan: `.agents/plans/<plan>/PLAN.md`
- Issue contract: `.agents/plans/<plan>/<NNN>-<short-name>.md`
- Upstream skill: `.agents/skills/upstream/SKILL.md` (crate/module mapping
  and drift rules; the changelog marker is the single source of truth for
  the HWM)
- Relevant feed-forward: `.agents/plans/<plan>/FEED-FORWARD.md` blocks for
  the earlier landed issues (state carried by fresh workers).

## Read first

1. `.agents/plans/<plan>/PLAN.md` (sections `<…>`)
2. `.agents/plans/<plan>/<NNN>-<short-name>.md`
3. `<repo state to verify: directories, Cargo.toml, git refs …>`

## Frozen contract

- Scope fence: `<what is in scope; explicitly list what is out of scope>`
- Deliverables:
  - `<file or artifact>`: `<change>`

## Constraints

- No commit: leave the worktree dirty; the orchestrator commits after the
  gate and the forked review.
- No unrelated edits: touch only the files the contract lists.
- Fresh worker: assume no memory of prior issues; the feed-forward notes
  carry the state.
- Skills are truth: `.agents/skills/*` outrank plan prose.

## Steps

1. `<step>`
2. `<step>`

## Verification (run these, paste the output in the report)

```sh
<verification commands>
```

Do not run the full Rust gate unless the contract says to; the orchestrator
runs `tools/gate.sh`, then dispatches the forked reviewer, and commits only
on PASS.

## Report format

- Files created/changed.
- The exact verification output.
- What changed vs the plan (accepted deviations, with reasons).
- Feed-forward: the fields recorded in `FEED-FORWARD.md` after landing.
- Any mismatch, deviation or open question.
- Do not commit.

## Feed-forward (record after landing)

Complete the `### <NNN> — <issue> (LANDED)` block in
`.agents/plans/<plan>/FEED-FORWARD.md` with:

- `<field>`: `<value>`
- `<field>`: `<value>`

Then update the affected later issue files in the same pass so no feed-
forward note contradicts a frozen issue file (see the rules at the top of
`FEED-FORWARD.md`).
