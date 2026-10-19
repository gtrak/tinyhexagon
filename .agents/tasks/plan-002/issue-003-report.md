# Issue 03 — rename `oh-` → `th-`, artifacts → `tinyhexagon` (report)

**Base:** post-upstream-sync HEAD `d1b3c82d`. Mechanical rename, no behaviour
edits. Gate 4/4 green. Committed by the orchestrator after the forked `VERDICT: PASS`.

## Final crate names

All 12 kept crates renamed (directories via `git mv`, package `name`, path
deps, Rust identifiers, tools, CI):

| Old | New |
|---|---|
| `oh-math` | `th-math` |
| `oh-rng` | `th-rng` |
| `oh-data` | `th-data` |
| `oh-config` | `th-config` |
| `oh-assets` | `th-assets` |
| `oh-lua` | `th-lua` |
| `oh-game` | `th-game` |
| `oh-audio` | `th-audio` |
| `oh-render` | `th-render` |
| `oh-app` | `th-app` |
| `oh-android` | `th-android` |
| `oh-web` | `th-web` |

Rust identifiers follow (`oh_*` → `th_*`), log prefixes included
(`[oh-render]` → `[th-render]`, `[oh-web-fs]` → `[th-web-fs]`,
`[oh-audio]` → `[th-audio]` — the `tools/smoke-windowed.sh` assertion and the
`tools/web-*.mjs` console-line greps were updated to the new prefixes; the
`--oh-desktop-mode-probe` CLI flag is `--th-desktop-mode-probe` on both sides
(`th-app::desktop_mode::PROBE_ARG` + the smoke script)).

`cargo metadata --no-deps` lists zero `oh-*` packages; `Cargo.lock` carries
exactly the 12 `th-*` packages.

## Binary / artifact names

| Target | Name |
|---|---|
| Desktop binary | `tinyhexagon` (the `th-app` `[[bin]]`, `crates/th-app/src/bin/tinyhexagon.rs`) |
| Android APK | `tinyhexagon.apk` (`apk_name = "tinyhexagon"`) |
| Android cdylib | `th_android` (`[lib] name = "th_android"`, `libth_android.so` in `lib/arm64-v8a/`; the `tools/package-android.sh` unzip/readelf/nm checks + the `th-android` manifest comments all updated) |
| Android app label | `tinyhexagon` (was "Open Hexagon"; the spec's decision) |
| Android `package` id | `info.vittorioromeo.openhexagon` — UNCHANGED (the plan does not rename it; recorded exception) |
| wasm JS / wasm / data | `tinyhexagon.js` / `tinyhexagon.wasm` / `tinyhexagon.data` (via `th-web` `[[bin]] name = "tinyhexagon" path = "src/main.rs"`; emcc names all three after the bin) |
| Linux release | `tinyhexagon-<tag>-linux-x86_64.tar.gz` |
| Windows release | `tinyhexagon-<tag>-windows-x86_64.zip` |
| Android release | `tinyhexagon-<tag>-android-arm64-v8a.apk` |
| Web release | `tinyhexagon-<tag>-web.zip` (inner zip `tinyhexagon-web.zip`) |
| `th-android` asset staging dir | `target/th-apk-assets` (metadata `assets` key + `tools/package-android.sh` agree) |

`default-members = ["crates/th-app"]` in the root `Cargo.toml` — `cargo run`
resolves the default member and runs the stub:

```
$ cargo run --quiet
tinyhexagon: the tiny runtime lands in plan 002 Issue 07   (rc=0)
```

## Stub-bin state

`crates/th-app/src/bin/tinyhexagon.rs` is a temporary one-line stub (spec
code, verbatim, plus a module doc naming the issue-07 replacement). Issue 07
replaces the file contents, keeping the bin name/path.

Known cosmetic consequence (accepted, spec-mandated names): on the HOST
target dir the `th-app` bin `tinyhexagon` and the `th-web` bin `tinyhexagon`
(no-op `fn main()`) share an output filename — cargo prints an "output
filename collision" WARNING (not an error) during `cargo build --workspace`
/ clippy. `cargo run` builds th-app's unit explicitly and is unaffected; the
wasm target dir only ever contains th-web's bin, so the page contract
(`tinyhexagon.js` next to `index.html`) is unaffected. Issue 08 re-adds the
real web entry; if the collision becomes a problem there, th-web's host no-op
bin can be dropped (the wasm bin name is what matters for the artifacts).

## Replacement file list

Dry-run scope (per spec): `rg -l -e 'oh_' -e 'oh-' -e 'open-hexagon'
crates tools .cargo CODEOWNERS` → **148 files** (all under `crates/` after
the `git mv`, plus `tools/` 11, `.github/workflows/` 2, `.cargo/config.toml`).
Two perl passes, then targeted restores:

1. **Pass 1 (artifact names first):** `oh-web.js` → `tinyhexagon.js`,
   `oh_web.wasm` → `tinyhexagon.wasm`, `oh_web.data` → `tinyhexagon.data`
   in `.cargo/config.toml`, `tools/build-wasm-emsdk.sh`,
   `tools/package-web.sh`, `tools/web-persist-probe.mjs`,
   `.github/workflows/ci.yml`, `crates/th-web/site/index.html`
   (the page's `import Module from "./tinyhexagon.js"` included).
2. **Pass 2 (mechanical):** `s/\boh_/th_/g; s/\boh-/th-/g;
   s/open-hexagon/tinyhexagon/g` over the 148-file list. This also renamed
   the live `open-hexagon` strings: the headless window title
   (`th-render/src/window.rs` `Window::headless`), the lifecycle-override
   test argv (`th-app/src/lifecycle.rs`), the `th-android` `apk_name`, and
   the tools/CI binary + artifact names.
3. **Pass 3 (embedded token the `\b` missed):** `liboh_android.so` →
   `libth_android.so` (8 lines: `tools/package-android.sh` ×5,
   `crates/th-android/Cargo.toml` ×3) — the live cdylib name.
4. **Pass 4 (exception restores):** see below.

Manual (non-mechanical) edits on top:

- Root `Cargo.toml`: members → `crates/th-*`, `default-members` added,
  stale plan-001 comment block updated (the `oh-game` token in the
  `[profile.release]` LTO note → `th-game`).
- `crates/th-app/Cargo.toml`: `[[bin]] name = "tinyhexagon"
  path = "src/bin/tinyhexagon.rs"`; new stub file.
- `crates/th-web/Cargo.toml`: `[[bin]] name = "tinyhexagon"
  path = "src/main.rs"`.
- `crates/th-android/Cargo.toml`: `label = "tinyhexagon"` (with a note);
  `package` id untouched.
- `.github/workflows/ci.yml`: the two fork-guards
  `github.repository == 'gtrak/SSVOpenHexagon'` → `'gtrak/tinyhexagon'`
  (the GitHub repo WAS renamed — `git remote` = `gtrak/tinyhexagon`, so the
  old guard would have silently disabled the wasm upload + pages jobs);
  pages-URL comments → `gtrak.github.io/tinyhexagon/`.
- `.github/workflows/release.yml`: pages-URL comments →
  `https://gtrak.github.io/tinyhexagon/` (header + deploy step).
- `tools/web-probe.mjs`: usage-example URL →
  `https://gtrak.github.io/tinyhexagon/` (same pages-URL class).
- `README.md`: build line now documents `cargo run` → the `th-app`
  `tinyhexagon` stub (one-line message, exit 0; runtime lands in Issue 07).
- `cargo fmt --all`: the rename shifted import sort positions (`o…` → `t…`
  reorders against `std`/`serde`/`rodio` etc.) — 16 files re-sorted,
  import-order only.

## Exception list (strings intentionally left as `oh-*`)

22 lines, all doc/comments (no live path, no live dep, no live string):

| String | Location | Reason |
|---|---|---|
| `oh-net` (×3) | `.cargo/config.toml:9`, `.github/workflows/release.yml:45`, `tools/build-windows.sh:105` | removed crate (Issue 02) — historical libsodium/sodiumoxide citation |
| `oh-steam` (×6) + `oh_steam` (×1) | `crates/th-assets/src/assets.rs` (×7), `tools/build-windows.sh:39` | removed crate — historical Steam-surface doc |
| `oh-discord` (×1) | `tools/build-windows.sh:40` | removed crate — historical doc |
| `oh-server-control` / `oh-workshop-uploader` (×2) | `tools/package.sh:69-70` | removed crates (issues 18-19) — historical doc |
| `oh-additions` (×6) | `crates/th-web/site/index.html:353`, `crates/th-app/Cargo.toml:71`, `crates/th-render/src/texture.rs:552`, `crates/th-app/src/bridge.rs:258`, `crates/th-app/src/lib.rs:60`, `crates/th-app/src/web_fullscreen.rs:11` | removed crate — "salvaged from oh-additions" history notes |
| `oh-lanes` (×1) | `tools/build-wasm-emsdk.sh:7` | the EXTERNAL lane dir (`/home/gary/dev/oh-lanes`) — not a repo path; renaming the doc would break it |

Also left (out of the replacement patterns, recorded for completeness):
`SSVOpenHexagon` strings (C++ path citations, package descriptions, the
page `<title>`, the keystore DN `O=SSVOpenHexagon` in
`tools/package-android.sh`, the `upstream-status.sh` upstream-remote checks
— which are correct: `upstream` IS `gtrak/SSVOpenHexagon`), `"Open
Hexagon"` (README fork description, the C++ Lua-API deprecation strings,
`config.rs` pack comments), `ohr.z` (fixture
`cube_1_apeirogon.ohr.z`), `ohdb.sqlite`, `ohvrvanilla_vittorio_romeo_*`
(test pack/level ids), `oh.filelist` (the APK asset-marker filename —
pattern-invisible dot name, written by `tools/package-android.sh`),
`OH_*` env vars (`OH_CDP_PORT`, `OH_CHROME`, …), the `window.ohStatus` /
`ohWebModule` / `ohWebPersistBoot` / `ohstatechange` page-JS handles
(camelCase, pattern-invisible; the probes rely on them),
`tools/android/openhexagon-release.keystore` (tracked keystore identity +
alias/password — a live file, not a token), and `crates/oh-parity/oh030b-corpus/`
in `.gitignore` (stale rule for the Issue-02-removed crate; inert, out of
scope — Issue 09 dead-surface sweep candidate).

## CI / workflow names + pages URL

- Job IDs UNCHANGED (all jobs + dependency edges kept): `ci.yml` — `gate`,
  `windows-cross`, `android`, `wasm`, `pages`; `release.yml` — `gate`,
  `linux`, `windows`, `android`, `wasm`, `release`, `gh-pages`.
- Artifact names in both workflows: `tinyhexagon-<tag>-…` (all four + the
  inner `tinyhexagon-web.zip`).
- Pages URL: `https://gtrak.github.io/tinyhexagon/` (the repo is
  `gtrak/tinyhexagon`; gh-pages follows the repo rename). Fork-guards now
  `github.repository == 'gtrak/tinyhexagon'`.
- `CODEOWNERS`: `* @SuperV1234` only — no crate names, no change needed.

## Verification

```
$ cargo metadata --no-deps --format-version 1 | grep -o '"name":"[^"]*"' | grep 'oh-'
(no match) → no-oh-packages
$ rg -n -e 'oh_' -e 'oh-' -e 'open-hexagon' crates tools .github .cargo CODEOWNERS
→ 22 lines, ALL in the exception list above (rename-clean otherwise)
$ cargo run --quiet
tinyhexagon: the tiny runtime lands in plan 002 Issue 07
run_rc=0
$ bash -n tools/{package,package-android,package-web,build-windows,check-android,build-wasm-emsdk,ci-deps,smoke-windowed}.sh → all OK
(actionlint not installed in this environment — the workflow changes are comment/value-string only, no YAML structure change)
```

Gate (`bash tools/gate.sh`), tail of `target/gate.log`:

```
== cargo fmt --all -- --check   GATE_EXIT=0
== cargo build --workspace      GATE_EXIT=0
== cargo clippy --workspace --all-targets -- -D warnings   GATE_EXIT=0
== cargo test --workspace       GATE_EXIT=0   (60 suites, 509 passed / 0 failed)
gate_rc=0
```

`Cargo.lock` regenerated (12 `th-*` packages, zero `oh-*` package names).
Packaging was NOT run (per spec — the headless/windowed smokes need the real
binary from 07/08; the stub will not pass them). Committed by the orchestrator after the forked `VERDICT: PASS`.
