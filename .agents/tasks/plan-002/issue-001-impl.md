# Issue 001 — implementation task spec (`.agents` history, retire C++/parity)

## Context

- Active plan: `.agents/plans/002-tinyhexagon/PLAN.md`
- Issue contract: `.agents/plans/002-tinyhexagon/01-agents-history.md`
- Upstream skill: `.agents/skills/upstream/SKILL.md` (HWM `4c31561a…`)
- Issue 000 is landed (`3369fffa`). The plan-002 task files are currently at
  `.agents/tasks/issue-000-*.md`; move them to `.agents/tasks/plan-002/` as
  part of this issue so they do not collide with the archived port tasks.
- Do not commit. The orchestrator commits after the gate and the forked review.

## Working agreement

- Read the issue file and the plan before editing. The old record is history:
  move it, do not edit it.
- The only deletions are the C++ tree, the parity crate/tooling, the retired
  audit/evidence/workflow record (moved to the archive), and `docs/`.
  Keep `_RELEASE/`, `LICENSE`, `crates/`, `tools/` target scripts, and
  `.agents/skills/`.
- Do not mass-delete comments that cite the C++ in `crates/`; they are
  history, not build inputs.
- Write a report at `.agents/tasks/plan-002/issue-001-report.md` (create the
  directory) with the evidence and the final gate tail. Do not commit.

## Read first

1. `.agents/plans/002-tinyhexagon/01-agents-history.md`
2. `.agents/plans/002-tinyhexagon/PLAN.md` (§2.4, §5, issue index)
3. `.agents/plans/001-rust-port/{PLAN.md,DECISIONS.md,PARITY.md}` (summary
   material; do not edit them)
4. `README.md`, `tools/gate.sh`, root `Cargo.toml`, `docs/`
5. `ls .agents`, `ls .agents/tasks`, `ls tools`, `git submodule status`

## Steps

### 1. Move the old record into the archive (history, unedited)

Create `.agents/plans/archive/` and move:

```sh
mkdir -p .agents/plans/archive
git mv .agents/plans/001-rust-port .agents/plans/archive/001-rust-port
git mv .agents/plans/STATUS.md .agents/plans/archive/001-rust-port/STATUS.md
git mv .agents/audits .agents/plans/archive/001-rust-port/audits
git mv .agents/evidence .agents/plans/archive/001-rust-port/evidence
git mv .agents/workflows .agents/plans/archive/001-rust-port/workflows
git mv .agents/ISSUE-019-REPORT.md .agents/plans/archive/001-rust-port/ISSUE-019-REPORT.md
git mv .agents/WORKER-HYGIENE.md .agents/plans/archive/001-rust-port/WORKER-HYGIENE.md
```

Preserve the pre-plan-002 review protocol from git history (do not use the
current plan-002 `review-gate.md`):

```sh
mkdir -p .agents/plans/archive/001-rust-port/tasks
git show 4c31561a:.agents/tasks/review-gate.md \
  > .agents/plans/archive/001-rust-port/tasks/review-gate-port.md
```

Archive the old port task files, keeping the plan-002 files:

```sh
mkdir -p .agents/tasks/plan-002 .agents/plans/archive/001-rust-port/tasks
git mv .agents/tasks/issue-000-impl.md .agents/tasks/plan-002/
git mv .agents/tasks/issue-000-fix.md .agents/tasks/plan-002/
git mv .agents/tasks/issue-000-report.md .agents/tasks/plan-002/
# every remaining .agents/tasks/*.md except TEMPLATE.md and review-gate.md:
for f in .agents/tasks/*.md; do
  case "$f" in
    .agents/tasks/TEMPLATE.md|.agents/tasks/review-gate.md) ;;
    *) git mv "$f" .agents/plans/archive/001-rust-port/tasks/ ;;
  esac
done
```

Keep `.agents/tasks/TEMPLATE.md`, `.agents/tasks/review-gate.md`,
`.agents/skills/` (all skills), and `.agents/plans/002-tinyhexagon/`.
Do not edit any archived file.

### 2. Delete the C++ tree, parity crate and retired tooling

```sh
git submodule deinit -f --all
rm -rf cpp-original src include .gitmodules
rm -rf .git/modules        # only if it exists
rm -rf crates/oh-parity
rm -rf docs
rm -rf tools/cpp-oracle tools/oracle tools/parity.sh tools/lua-parity.sh \
       tools/check-additions-boundary.sh tools/additions-boundary-baseline.txt \
       tools/lane-worktree.sh tools/infra tools/k3-luajit-hang
```

- Remove `"crates/oh-parity",` from the `[workspace] members` list in the root
  `Cargo.toml`.
- Do not delete `_RELEASE/`, `LICENSE`, `tools/android*`, `tools/build-*`,
  `tools/package*.sh`, `tools/check-android.sh`, `tools/ci-deps.sh`,
  `tools/web-*.mjs`, or `tools/upstream-status.sh`.

### 3. Rewrite `tools/gate.sh`

Keep the existing harness structure (`set -u`, repo-root `cd`, log under
`target/`, `GATE_EXIT` per stage, failing-stage tail of the last 200 lines,
final `log:` line, non-zero exit on any failure). Run exactly four stages:

1. `cargo fmt --all -- --check`
2. `cargo build --workspace`
3. `cargo clippy --workspace --all-targets -- -D warnings`
4. `cargo test --workspace`

Remove the Lua-parity and additions-boundary stages and their header
comments. Update the header to describe the four stages and the failing-tail
rule. No network.

### 4. Write `.agents/HISTORY.md` (1–2 pages)

Summarize the port for the new direction. Cover, with real facts from the old
record (read it; do not invent):

- **What the port proved**: C++-oracle frame parity (frame counts/corpus from
  `PARITY.md`), RNG/replay byte-exactness, the Lua A/B corpus, the
  winit/glutin/glow render stack, Android device verification, the
  emscripten build, packaging/CI.
- **What carries forward**: engine simulation (`oh-game`), Lua content
  (`oh-lua`), `oh-render`, the winit/glutin/glow + cpal stack, the
  Android/emscripten entries, packaging/CI.
- **What is retired**: the C++ tree and oracle, parity corpus/tooling, adult
  menu/online/Steam/Discord/workshop/server/replay, the replay codec, the
  score/profile system, the additions ratchet.
- **The tinyhexagon direction**: no menus/text, no death, 10-minute arc,
  generated music, kid touch; link `.agents/plans/002-tinyhexagon/PLAN.md`.
- **Upstream relationship**: link `.agents/skills/upstream/SKILL.md` and the
  HWM changelog.
- **Pointers**: the archived plan
  (`.agents/plans/archive/001-rust-port/`), git history, the active plan.

Keep it useful, not exhaustive; the archive and git history are the full
record.

### 5. README placeholder

Replace `README.md` with a short placeholder (final README is Issue 09):

```markdown
# tinyhexagon

A kid-friendly fork of the Open Hexagon Rust port: no menus, no reading, no
fail; a slow 10-minute arc and generated music. The adult port is retired;
this repository is mid-transition to the tiny runtime.

Active plan: [.agents/plans/002-tinyhexagon/PLAN.md](.agents/plans/002-tinyhexagon/PLAN.md)

Build and run the current tree: `cargo run -p oh-app`
```

(If the binary command differs, use the actual working command and note it;
the rename to `tinyhexagon` lands in Issue 03.)

### 6. `.agents/skills/README.md`

Keep the existing skills and the upstream row. Add one line/row noting that
the rust-synth and cpal skills follow in Issue 05 (after the cross-target
spike), so a fresh agent knows they are pending rather than missing.

### 7. Regenerate/update the lock and run the gate

```sh
cargo build --workspace        # updates Cargo.lock after oh-parity removal
bash tools/gate.sh
```

If a test references a deleted path or the parity crate, fix only that
reference (delete the obsolete test or adjust the path) and record it in the
report and `FEED-FORWARD.md` §01. Do not weaken assertions. If the gate fails
for an unrelated pre-existing reason, report the exact failure; do not paper
over it.

### 8. Feed-forward

Update `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` §01 (currently
pending) with:

- The final archive path.
- The four gate stages and the final gate result.
- Any test files removed with the parity crate.
- Any reference fixes and deviations.

Do not mark Issue 001 LANDED in `PLAN.md`; the orchestrator does that at
commit.

## Verification (paste the output)

```sh
git submodule status
test ! -e cpp-original && test ! -e src && test ! -e include && test ! -e .gitmodules && echo "cpp-tree-gone"
test ! -d crates/oh-parity && echo "parity-crate-gone"
rg -n 'oh-parity' Cargo.toml crates tools/gate.sh || echo "no-parity-refs"
rg -n 'lua-parity|check-additions-boundary' tools/gate.sh || echo "no-parity-stages"
bash -n tools/gate.sh
grep -c '^run_stage ' tools/gate.sh   # expect 4
bash tools/gate.sh; echo "gate_rc=$?"
```

Also verify:
- `.agents/HISTORY.md` exists and links the archive, the active plan and the
  upstream skill.
- `.agents/plans/archive/001-rust-port/` contains the old plan, `mapping/`,
  `parity/`, `audits/`, `evidence/`, `workflows/`, `tasks/`, `STATUS.md`,
  `ISSUE-019-REPORT.md`, `WORKER-HYGIENE.md`.
- `.agents/tasks/plan-002/` contains the issue-000 task files; only
  `TEMPLATE.md` and `review-gate.md` remain at `.agents/tasks/`.
- `git status` shows the deletions/moves and no commit.

## Report

Write `.agents/tasks/plan-002/issue-001-report.md` with:

- Files moved/deleted.
- The gate result (all four stages) and the final tail (last ~40 lines).
- Any test/reference fixes.
- The feed-forward fields.
- Deviations/open questions.
- "No commit made."

## Risks

- Submodule/gitlink deletion can leave `.git/modules` state; remove it
  deliberately and confirm `git submodule status` is empty.
- Do not delete `_RELEASE` or the target tooling.
- The parity crate is the only corpus consumer; remove it before hunting
  references.
- The gate is long; let it finish and report the true result. Do not pipe it
  to `tail` (that hides the exit code); the harness already prints the failing
  tail.
