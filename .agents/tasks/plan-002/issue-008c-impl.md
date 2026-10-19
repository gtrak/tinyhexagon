# Issue 08c — packaging, CI, release names

**Worktree:** `.agents/worktrees/slot-2`, base **the 08a commit**. Main/
slot-1 off-limits. **No commit.**

**Read:** `.agents/plans/002-tinyhexagon/08-android-web-packaging.md`
(Web + Packaging/CI), `issue-008a-report.md`, FEED-FORWARD §07/§08a, the
current `.github/workflows/{ci,release}.yml`, `tools/package*.sh`,
`tools/build-windows.sh`, `tools/check-android.sh`, `.cargo/config.toml`,
and the web probes.

## Goal

Every target ships as `tinyhexagon-*` with no adult-app leftovers, and the
fork's pages/release workflows publish the tinyhexagon builds.

## Steps

1. **Scripts.** `tools/package.sh` (Linux/Windows: `tinyhexagon` +
   `_RELEASE`), `package-web.sh`/`build-wasm-emsdk.sh`
   (`tinyhexagon.js/.wasm/.data` + shell), `build-windows.sh`,
   `check-android.sh`, `package-android.sh` (APK `tinyhexagon`). Remove the
   stale `--with-steam` and any `SSVOpenHexagon`/`open-hexagon`/`oh-` names.
2. **Known defects.** Fix the `th-app`/`th-web` host bin collision for the
   Windows release (e.g. a web-only bin gated to emscripten, or a
   distinct host no-op), and the web-probe `--shot=PATH`/CI Chrome path
   hardcodes where reasonable.
3. **Workflows.** `ci.yml`: gate matrix, windows-cross, android, wasm,
   pages — update job names/package names/artifact names and keep the
   browser smoke. `release.yml`: tag matrix and gh-pages; assets
   `tinyhexagon-<tag>-{linux-x86_64.tar.gz,windows-x86_64.zip,android-arm64-v8a.apk,web.zip}`.
   Use the fork's gh-pages URL/repo (`tinyhexagon`).
4. **Config/probes.** `.cargo/config.toml` comments/artifact names; the
   `tools/web-*.mjs` probes point at the real page/artifacts (no stub).
5. **Verify.** `actionlint` on both workflows; `bash tools/package.sh`
   (Linux package + Windows check or full cross build as the host allows);
   `bash tools/check-android.sh` (metadata/packaging smoke — the 08b APK
   build may land in parallel; coordinate via the orchestrator); the
   wasm build is 08a's; do not rebuild it unless a script rename breaks it.

## Constraints

- Rename only; keep the workflow structure and the target flags.
- Do not change the game/runtime code (08b owns Android, 08a owns web).
- No adult-app package names, URLs, or asset paths.

## Report

`.agents/tasks/plan-002/issue-008c-report.md`: the final artifact names,
pages/release URL, script/workflow changes, actionlint + package evidence;
FEED-FORWARD **§08c only**.
