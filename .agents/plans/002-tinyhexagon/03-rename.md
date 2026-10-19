# Issue 03 — Rename `oh-` → `th-` and rename artifacts

**Status:** LANDED
**Depends:** 02

## Objective

Rename every kept crate, path and binary reference from the `oh-` prefix to
`th-`, and rename the product/artifacts to `tinyhexagon`. This is a mechanical
change with no behaviour edits. Historical C++ citations in comments are left
alone (they are history, not build inputs).

## Rename map

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
| `oh-synth` (new, Issue 05) | `th-synth` |

Removed by Issue 02: `oh-parity`, `oh-replay`, `oh-net`, `oh-steam`,
`oh-discord`, `oh-server-control`, `oh-workshop-uploader`, `oh-menu`,
`oh-menu-system`, `oh-additions`. Nothing to rename.

## Binary/artifact names

| Target | Name |
|---|---|
| Desktop binary | `tinyhexagon` (the `th-app` bin) |
| Android APK | `tinyhexagon.apk` |
| Android cdylib | `th_android` (package `th-android`) |
| wasm JS / wasm | `tinyhexagon.js` / `tinyhexagon.wasm` (or the packaging script renames the crate output) |
| Linux release | `tinyhexagon-<tag>-linux-x86_64.tar.gz` |
| Windows release | `tinyhexagon-<tag>-windows-x86_64.zip` |
| Android release | `tinyhexagon-<tag>-android-arm64-v8a.apk` |
| Web release | `tinyhexagon-<tag>-web.zip` |

## Files

- `crates/oh-*` → `crates/th-*` (directories).
- Every `Cargo.toml`: package `name`, path dependencies, feature references
  (`oh-game/...` → `th-game/...`).
- Rust sources: `oh_x` → `th_x` identifiers/imports; `oh-` in crate/log
  references.
- Tests: `CARGO_BIN_EXE_*` names for `th-server-control`/
  `th-workshop-uploader` (removed in Issue 02; delete those tests), and the
  `tinyhexagon` binary references added in Issue 07.
- `tools/`: `package.sh`, `package-android.sh`, `package-web.sh`,
  `build-windows.sh`, `check-android.sh`, `build-wasm-emsdk.sh`,
  `ci-deps.sh`, `web-*.mjs`, `android/` names.
- `.cargo/config.toml`: `th-web`/`tinyhexagon` artifact comments and the
  `--preload-file` path (keep `_RELEASE`).
- `.github/workflows/ci.yml`, `release.yml`: package/binary/artifact names,
  pages URL.
- Root `Cargo.toml`: members; add
  `default-members = ["crates/th-app"]`.
- `Cargo.lock`: regenerate.
- `rust-toolchain.toml`: unchanged.
- `CODEOWNERS`: update if it names crates.

## Steps

1. Delete the removed crates/tests first (Issue 02), so the rename surface is
   only the kept crates.
2. `git mv` the crate directories.
3. Update manifests (package names, path deps, features).
4. Dry-run the token replacement and review the match list before applying:
   ```sh
   rg -l 'oh_' crates tools .github .cargo
   rg -l 'oh-' crates tools .github .cargo
   ```
   Replace `oh_` → `th_` and `oh-` → `th-` in Rust/TOML/SH/YML. Do **not**
   replace `ohr.z`, `ohdb.sqlite`, "Open Hexagon", or historical citations.
5. Rename the `open-hexagon` bin to `tinyhexagon` in `th-app/Cargo.toml`;
   update all `cargo run`/packaging references. The bin may be a stub until
   Issue 07.
6. Update `tools/` scripts and CI workflows. Keep every job (gate,
   windows-cross, android, wasm, pages, release) and only change names.
7. Add `default-members = ["crates/th-app"]`.
8. `cargo fmt`, regenerate `Cargo.lock`, run the gate.

## Verification

- `cargo metadata --no-deps --format-version 1` lists no `oh-*` package.
- `rg 'oh-' crates tools .github .cargo` returns only historical comments,
  no code/path/dependency hits.
- `cargo build --workspace`, clippy, tests green.
- `cargo run` resolves the default member and attempts the tinyhexagon binary
  (headless stub is acceptable until Issue 07).
- `bash tools/gate.sh` green.
- `actionlint` on the workflows (or the equivalent manual check) is clean.
- Artifact names in `release.yml` are `tinyhexagon-*`.

## Risks

- Blind replacement can hit false positives (prose, URLs, file extensions).
  Review the dry-run match list.
- `Cargo.lock` must be regenerated; a stale lock hides old package names.
- The `tinyhexagon` bin may not exist until Issue 07; keep the tree green with
  a stub that prints a clear "not wired yet" message, or defer the bin rename
  to Issue 07 (preferred) and keep the package rename here. Record the choice
  in `FEED-FORWARD.md`.

## Feed-forward

Record the final crate names, the binary/artifact names, and any rename
exceptions (strings intentionally left). Update the upstream skill's mapping
if a name differs from Issue 00's initial table.
