# Issue 03d report — remove the dead libsodium prebuild from the Windows cross path

Base: post-03c `6d7820a4` (clean tree at start; the spec file itself
untracked). th-net/sodiumoxide/libsodium-sys went in 02b, but
`tools/build-windows.sh` still prebuilt libsodium unconditionally, so the
CI `windows-cross` job died before cargo with `cannot find the vendored
libsodium source` (run 39309854944). This issue removes the dead prebuild
and the stale references; the cross-check and the Windows release path
now run without sodium. **Committed by the orchestrator after the forked `VERDICT: PASS`.**

## 1. `tools/build-windows.sh` (the dead prebuild, gone)

Removed:

- the `--force-sodium` flag (now rejected as `unknown flag`, rc 2) + the
  `FORCE_SODIUM` variable;
- the `make` prereq check (only the libsodium prebuild + the long-dead
  LuaJIT build used it — luau0-src and all other cc-rs deps invoke the
  cross gcc directly, never the host make);
- the whole libsodium prebuild block: `SODIUM_DIR`/`SODIUM_PREFIX`/
  `SODIUM_LIB`, the registry-cache lookup for `libsodium-sys-0.2.7` (with
  the `cargo fetch` fallback), the vendored-source error
  (`cannot find the vendored libsodium source` — the CI's failure line),
  the `configure --host=x86_64-w64-mingw32` / `make all` / `make install`
  subshell, and the `SODIUM_LIB_DIR` export.

Kept exactly: the rustup target self-install, the MinGW gcc check, and the
`--check` / `--build` / `--release` / `--all-targets` cargo paths
(`exec cargo check/build --workspace … --target x86_64-pc-windows-gnu`).

Header rewritten to the current reality: the C deps (Luau via luau0-src,
mlua 0.10.5 `luau` feature — D-037) cross-compile in place through the
mingw toolchain; there is no libsodium/sodiumoxide in the graph (th-net
went in 02b), so no cross-prebuild and no special link line. The
mingw-config-sub story (libsodium 1.0.18's `config.sub` rejecting the rust
triple, the canonical-MinGW-triple prebuild workaround) is removed with
the prebuild it documented. Also removed from the header: the now-stale
`rusqlite (bundled) + flate2 (system-zlib host build)` clause (both gone
from the graph since 02b/02c2 — flate2 is pure-Rust `miniz_oxide`+
`zlib-rs` now) and the last sentence of the "Linking" paragraph, which
named the deleted `-ladvapi32` flag and the deleted crates (`oh-steam`'s
`steam` feature, `oh-discord`) — both crates removed in 02b/02c1, and
`th-app` has no `[features]` table at all. The MSVC-not-cross-buildable
NOTE and the default-linker statement stay.

## 2. `.cargo/config.toml` (the `-ladvapi32` flag, removed)

The `[target.x86_64-pc-windows-gnu]` section's only purpose was the
cross-built static libsodium (`SystemFunction036` / RtlGenRandom,
`randombytes_sysrandom.c:366`). No other dep needs it — verified:

- **graph check**: the only Windows RNG users are `getrandom` 0.4.3 (via
  `rand` 0.10.3 → rodio) and 0.3.4 (via ahash → winit); on
  `x86_64-pc-windows-gnu` (`target_vendor = "pc"`) both select the
  `ProcessPrng`/`bcryptprimitives` backend — the `advapi32`-linking
  `windows_legacy` backend is cfg-selected only for
  `target_vendor = "win7"` or an explicit `getrandom_backend` override.
- **link check (definitive)**: after the removal, a FULL cross-build +
  link (`bash tools/build-windows.sh --build`, workspace, dev profile)
  exits 0 and the produced `tinyhexagon.exe` import table contains NO
  `advapi32` (imports: `api-ms-win-core-synch-l1-2-0.dll`,
  `bcryptprimitives.dll` (ProcessPrng), `KERNEL32`, `msvcrt`, `ntdll`,
  `USERENV`, `WS2_32`).

So the flag + its comment block are removed entirely (no live consumer
would leave the line half-undocumented). The Android comment's cc-rs
crate list is fixed from `(luau0-src, libsodium-sys)` to `(luau0-src)`.
Nothing else in the file touched (android/wasm sections verbatim).

## 3. `.github/workflows/ci.yml` (one comment)

`windows-cross` job: the cross-check step's comment drops "incl. the
libsodium cross-prebuild; see the script" → "the fast cross signal (see
the script)". The job, its steps, and the command
(`bash tools/build-windows.sh --check --all-targets`) are untouched —
the cross-check job stays exactly as the Windows-target regression
catcher the header warns to never drop.

## 4. `.github/workflows/release.yml` (header + one step comment)

The WINDOWS header is updated to the post-sodium reality: the lane-037
findings (libsodium-sys autoconf wall; libsqlite3-sys/libz-sys llvm-lib
wall) are marked as HISTORY (those C deps are all gone from the graph
since 02b/02c2 — th-net removed, flate2 pure-Rust), and a POST-SODIUM
(03d) paragraph states: the only cc-rs dep the cross path compiles is Luau
(luau0-src), in place, no prebuild / no SODIUM_LIB_DIR / no -ladvapi32;
mingw-gnu is STILL the release path. The dead "If a future libsodium-sys
ships a msvc target…" paragraph is removed with the prebuild it described.
The windows job's step comment
("the libsodium-sys autoconf wall + the missing llvm-lib") now reads
"the C-dep autoconf / llvm-lib walls; post-sodium the cross path compiles
Luau only". **No job structure, step, or artifact name changed**
(`tinyhexagon-<tag>-windows-x86_64.zip` etc. verbatim).

## 5. `tools/package.sh` (two comments, the --with-steam logic untouched)

- The Windows-branch build comment: "the libsodium cross-prebuild + the
  full cargo cross-compile + link" → "the full cargo cross-compile + link
  of the .exe's, no libsodium prebuild — th-net went in issue 02b"; the
  prereq list drops `make` (no longer checked by build-windows.sh).
- The `--with-steam` staging comment's dangling cross-reference ("the
  same idiom tools/build-windows.sh uses for the libsodium-sys source")
  is trimmed to "found in the cargo registry cache" — comment-only; the
  steam-stub staging logic itself is untouched (Issue 08 owns the
  `--with-steam` cleanup, per the spec). Linux/web paths untouched.

## 6. `tools/package-android.sh` (one comment)

Header: "CC/AR env the C builds in the graph (luau0-src, libsodium-sys)
need." → "CC/AR env the C build in the graph (luau0-src) needs."

## 7. Grep sweep

`rg -n 'libsodium|sodiumoxide|SODIUM|oh-net|th-net' tools .github .cargo`
after the change: the ONLY hits are the deliberately-kept historical /
post-sodium prose in `release.yml`'s WINDOWS section (the lane-037
findings marked as history + the POST-SODIUM paragraph) and the single
"no libsodium/sodiumoxide in the graph" line in `build-windows.sh`'s new
header. No live path references sodium; no scoped tree outside
`tools`/`.github`/`.cargo` is affected.

## 8. Verification

Host (this box: `x86_64-pc-windows-gnu` target installed + mingw gcc 13
present):

- `bash -n tools/build-windows.sh` / `tools/package.sh` /
  `tools/package-android.sh` — all OK.
- `bash tools/build-windows.sh --check --all-targets` — **PASS**
  (`cargo check --workspace --all-targets --target
  x86_64-pc-windows-gnu`, "Finished dev profile", exit 0). This is the
  exact CI command; it previously died before cargo.
- `bash tools/build-windows.sh --build` (full cross-compile + LINK, the
  definitive `-ladvapi32`-removal check) — **PASS** (exit 0);
  `target/x86_64-pc-windows-gnu/debug/tinyhexagon.exe` produced; its
  import table carries no advapi32 (see §2).
- `bash tools/build-windows.sh --help` — header renders clean;
  `--force-sodium` now exits 2 as an unknown flag.
- No Rust touched → `cargo fmt --check` not required (spec: "you
  shouldn't"). The host `tools/gate.sh` is not required for a
  shell/CI-only change (per spec); the host target compiles are
  unaffected by any of these edits (all changes are target-scoped to
  windows-gnu or comments/scripts).

CI (the definitive gate, per the spec): the `windows-cross` job
(`--check --all-targets`) and the release `windows` job
(`package.sh --windows` → full cross-build) run on the push; the host
runs above are the same commands on the same target.

## 9. Feed-forward

`FEED-FORWARD.md` §08 gains a "Windows cross path (03d)" bullet: the
prebuild + the advapi32 flag are gone; the cross path needs only the
rustup target + mingw-w64 gcc (Luau is the sole cc-rs dep); 08 keeps the
Windows artifact/CI work (job structure, artifact names, ci-deps.sh's
mingw-w64 prereq all untouched).

## Residual risks

- None blocking. The two CI legs (per-push cross-check, release
  cross-build) re-prove the change on the ubuntu-latest runner; the host
  already ran both command shapes green, including the link that the
  check path cannot exercise.
- If a future dep re-introduces a Windows C lib needing
  `SystemFunction036` (e.g. a new sodium-style pin), the cross link will
  fail with an undefined `SystemFunction036` — re-add the flag then with
  the then-current rationale (the pre-03d comment in git history is the
  reference).
