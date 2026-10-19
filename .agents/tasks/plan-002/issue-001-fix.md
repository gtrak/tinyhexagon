# Issue 001 — review fix task spec

The forked reviewer found one blocking (P0) packaging regression and three P2
alignment issues. Apply them, verify, and report. Do not commit.

## Read first

- `.agents/plans/002-tinyhexagon/01-agents-history.md`
- `.agents/tasks/plan-002/issue-001-impl.md`
- `.agents/tasks/plan-002/issue-001-report.md`
- `tools/package.sh` (the smoke-test section, ~lines 85–120, 230–270)
- `.github/workflows/release.yml:165`
- `.agents/HISTORY.md`
- `.agents/plans/002-tinyhexagon/FEED-FORWARD.md` (§01)
- `.agents/skills/README.md`

## P0 — the release smoke fixture was deleted with the C++ tree

`tools/package.sh:112` points `FIXTURE` at
`cpp-original/test/fixtures/cube_1_apeirogon.ohr.z`, and `:242` exits 1 when
it is missing. `cpp-original/` is now gone, and
`.github/workflows/release.yml:165` runs `bash tools/package.sh` for the Linux
release without `--skip-smoke`, so the tagged Linux release would fail after a
full build.

Fix:

1. Restore the committed fixture from the pre-issue-001 tree (the current
   HEAD, `3369fffa`, still has it) to a path that survives the C++ deletion:

   ```sh
   mkdir -p tools/fixtures
   git show 3369fffa:cpp-original/test/fixtures/cube_1_apeirogon.ohr.z \
     > tools/fixtures/cube_1_apeirogon.ohr.z
   ```

   Verify it is the 101-byte zlib fixture (`wc -c`, `file`).

2. In `tools/package.sh`:
   - Set `FIXTURE="${REPO_ROOT}/tools/fixtures/cube_1_apeirogon.ohr.z"`.
   - Update the module-doc path at ~line 90 from
     `cpp-original/test/fixtures/...` to `tools/fixtures/...`.
   - The doc pointer at ~line 59 to the deleted
     `cpp-original/release/README.md` must be removed or retargeted to the
     archive/git history; do not leave a live-looking path to a deleted file.
   - Keep everything else (the sandbox copy, the headless invocation, the
     pinned final time, the `--skip-smoke` option) unchanged.

3. Verify the packaged-binary smoke test actually runs. Run:

   ```sh
   bash tools/package.sh
   ```

   Do **not** pass `--skip-smoke` and do not weaken the smoke assertions. If
   the release build is slow, let it finish. Report the smoke lines and the
   exit code. If it fails for an unrelated pre-existing reason, report the
   exact failure; do not hide it.

## P2 — align the status/history/skills docs

1. `.agents/plans/002-tinyhexagon/FEED-FORWARD.md`: change
   `### 01 — \`.agents\` history (pending)` to `(LANDED)`.
2. `.agents/HISTORY.md`:
   - `deviations.md` range `F1…F30` at ~lines 6 and 49 is wrong; the
     archive's register runs through `F37`. Use `F1…F37` (verify the maximum
     in `.agents/plans/archive/001-rust-port/deviations.md`; do not guess).
   - The pointer at ~line 114 says the port landed through `3369fffa`; that
     is plan 002's Issue 000 commit. The pre-plan-002 baseline is
     `4c31561a`. Say the port landed through `4c31561a`, and that plan 002
     starts at `3369fffa`.
3. `.agents/skills/README.md` (~lines 23–27): the `038 layout note` still
   says the C++ tree is at `cpp-original/` and `.gitmodules` exists. After
   Issue 001 that is false. Add a short **Plan 002 note** that the C++ tree,
   root `src`/`include` symlinks and `.gitmodules` were removed by Issue 001;
   the record is archived at `.agents/plans/archive/001-rust-port/`, and
   skill citations to `cpp-original/...` refer to that archived tree / git
   history. Keep the library skills.
4. Add one line to `FEED-FORWARD.md` §01 (open questions/deviations) noting
   that `.github/workflows/ci.yml` header comments still describe the retired
   lua-parity/cpp-oracle stages; the active gate job already calls the
   four-stage `tools/gate.sh`, and the comment cleanup belongs to Issue
   08/09. Do not change the workflow in this issue.

## Report

Update `.agents/tasks/plan-002/issue-001-report.md` with:

- The P0 fix (fixture path, restored bytes, `package.sh` changes).
- The `bash tools/package.sh` result and the smoke lines.
- The P2 edits.
- Any deviation/open question.
- "No commit made."

## Verification (paste the output)

```sh
test -f tools/fixtures/cube_1_apeirogon.ohr.z && wc -c tools/fixtures/cube_1_apeirogon.ohr.z
file tools/fixtures/cube_1_apeirogon.ohr.z
rg -n 'cpp-original/test/fixtures' tools/package.sh || echo "no-cpp-fixture-path"
rg -n 'tools/fixtures' tools/package.sh
bash -n tools/package.sh
bash tools/package.sh; echo "package_rc=$?"
grep -n '^### 01' .agents/plans/002-tinyhexagon/FEED-FORWARD.md
rg -n 'F1…F37|4c31561a|3369fffa' .agents/HISTORY.md
```

Also confirm no unrelated files changed and no commit was made.
