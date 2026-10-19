# Issue 03 — rename `oh-` → `th-`, artifacts → `tinyhexagon`

**Base:** post-upstream-sync HEAD `d1b3c82d`. **Contract:**
`.agents/plans/002-tinyhexagon/03-rename.md`.
Mechanical rename; **no behaviour edits**. Do not commit.

## Decisions

- Add a minimal `th-app` **stub binary** named `tinyhexagon` now
  (`src/bin/tinyhexagon.rs`, prints a one-line "lands in Issue 07" message
  and exits 0). Issue 07 replaces it with the real runtime. This makes
  `cargo run` work with `default-members = ["crates/th-app"]` and lets the
  packaging scripts name a real binary.
- `th-web` gets a `[[bin]] name = "tinyhexagon"` so the wasm output is
  `tinyhexagon.js` / `tinyhexagon.wasm`; update the page/probes/scripts.
- `th-android` keeps the package metadata but the cdylib is `th_android`,
  `apk_name = "tinyhexagon"`, label `tinyhexagon`.
- Historical C++/removed-crate comments may stay as `oh-` references if they
  are not live paths; record them as rename exceptions. Do not touch
  `.agents/`, `_RELEASE/`, `Cargo.lock` (regenerate), `target/`, or the
  `ohvrvanilla`/`ohr.z`/`ohdb.sqlite`/`SSVOpenHexagon` strings.

## Steps

### 1. Rename the crate directories

```sh
git mv crates/oh-math crates/th-math
git mv crates/oh-rng crates/th-rng
git mv crates/oh-data crates/th-data
git mv crates/oh-config crates/th-config
git mv crates/oh-assets crates/th-assets
git mv crates/oh-lua crates/th-lua
git mv crates/oh-game crates/th-game
git mv crates/oh-audio crates/th-audio
git mv crates/oh-render crates/th-render
git mv crates/oh-app crates/th-app
git mv crates/oh-android crates/th-android
git mv crates/oh-web crates/th-web
```

### 2. Token replacement (dry-run first)

List the scoped files (after the `git mv`):

```sh
rg -l -e 'oh_' -e 'oh-' -e 'open-hexagon' \
  crates tools .github .cargo CODEOWNERS
```

Review the list for false positives (the strings to leave: `ohvrvanilla`,
`ohr.z`, `ohdb.sqlite`, `SSVOpenHexagon`, "Open Hexagon", historical
`oh-steam`/`oh-menu`/`oh-additions` doc comments). Then replace in Rust/
TOML/SH/YML/HTML/MJS:

```sh
files=$(rg -l -e 'oh_' -e 'oh-' -e 'open-hexagon' \
  crates tools .github .cargo CODEOWNERS)
perl -pi -e 's/\boh_/th_/g; s/\boh-/th-/g; s/open-hexagon/tinyhexagon/g' $files
```

- `oh_math` → `th_math` (Rust crate identifiers), `oh-math` → `th-math`
  (package/path/log names), `open-hexagon` → `tinyhexagon`.
- Update the log prefixes too (`[oh-render]` → `[th-render]`,
  `[oh-web-fs]` → `[th-web-fs]`, `[oh-audio]` → `[th-audio]`); the
  `tools/smoke-windowed.sh` assertion must match the new prefix.
- Do not replace the historical exception strings above; if the dry-run
  shows a `oh_steam`/`oh_menu`/`oh_additions` doc-only mention, leave it and
  record it.

### 3. Root manifest

- `members` are now `crates/th-*`.
- Add `default-members = ["crates/th-app"]`.
- `cargo metadata`/`Cargo.lock` regenerate after the manifests settle.

### 4. `th-app` stub binary

- Add `crates/th-app/src/bin/tinyhexagon.rs`:

```rust
fn main() {
    eprintln!("tinyhexagon: the tiny runtime lands in plan 002 Issue 07");
}
```

- Add to `crates/th-app/Cargo.toml`:

```toml
[[bin]]
name = "tinyhexagon"
path = "src/bin/tinyhexagon.rs"
```

- `cargo run` must resolve the default member and print that line.

### 5. `th-android`

- `[lib] name = "th_android"` (the replacement should do this).
- `apk_name = "tinyhexagon"`, application `label = "tinyhexagon"`.
- `tools/package-android.sh`: `libth_android.so`, `tinyhexagon.apk`.
- Leave the Android `package` id as-is unless the plan says otherwise;
  record it.

### 6. `th-web`

- Add `[[bin]] name = "tinyhexagon" path = "src/main.rs"`.
- Output is `tinyhexagon.js` / `tinyhexagon.wasm`; update
  `crates/th-web/site/index.html`, `tools/package-web.sh`,
  `tools/build-wasm-emsdk.sh`, `tools/web-*.mjs`, and the `.cargo/config.toml`
  preload/output names (keep the `_RELEASE` preload path).
- The emscripten `__wasm_call_ctors` trap fix and `web.rs` stub stay.

### 7. Tools and CI

- `tools/package.sh`: binary `tinyhexagon`, staged `_RELEASE/tinyhexagon`,
  release artifact names; keep the fixture path and the smoke assertions
  (the stub will not pass the headless replay smoke until 07/08; that is
  expected — do not run packaging as the issue gate).
- `tools/package-android.sh`, `tools/package-web.sh`,
  `tools/build-windows.sh`, `tools/check-android.sh`,
  `tools/build-wasm-emsdk.sh`, `tools/ci-deps.sh`, `tools/smoke-windowed.sh`,
  `tools/web-*.mjs`: update names only.
- `.github/workflows/ci.yml` / `release.yml`: package/path/binary/artifact
  names and the pages URL (`https://gtrak.github.io/tinyhexagon/`); keep
  every job and its dependency edges. Artifact names
  `tinyhexagon-<tag>-{linux-x86_64.tar.gz,windows-x86_64.zip,android-arm64-v8a.apk,web.zip}`.
- `CODEOWNERS`: update if it names crates.
- `README.md`: `cargo run` now runs the stub; note the runtime lands in 07.

### 8. Lock and gate

```sh
cargo metadata --format-version 1 --no-deps > /dev/null
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bash tools/gate.sh
```

## Verification

```sh
cargo metadata --no-deps --format-version 1 | grep -o '"name":"[^"]*"' | grep 'oh-' && exit 1 || echo no-oh-packages
rg -n -e 'oh_' -e 'oh-' -e 'open-hexagon' crates tools .github .cargo CODEOWNERS \
  || echo rename-clean
cargo run --quiet; echo "run_rc=$?"
bash tools/gate.sh; echo "gate_rc=$?"
```

`rg` may only hit the recorded historical doc exceptions; list them in the
report. `cargo run` must print the stub line and exit 0. Gate 4/4 green.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-003-report.md` with:

- Final crate names, binary/artifact names, default member.
- The replacement file list and the exception list.
- The `th-web`/`th-android` output names and the stub-bin state.
- CI/workflow names and pages URL.
- Gate result/tail, lockfile regen, "No commit made."

Update `FEED-FORWARD.md` §03: final names, artifact names, exceptions, stub
state, and any renamed CI job IDs/pages URL. Do not mark 03 LANDED in
`PLAN.md`; the orchestrator does that at commit.

## Risks

- Blind replacement can hit historical prose/URLs; review the dry-run list.
- `Cargo.lock` must be regenerated; a stale lock hides old names.
- The stub bin is temporary; do not implement the tiny runtime here (07).
- Packaging scripts are renamed but not run; the headless/windowed smokes
  need the real binary (07/08).
