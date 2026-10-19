# Issue 03d — remove the stale libsodium prebuild from the Windows cross path

**Status:** READY. **Base:** post-03c `6d7820a4`.
**Why:** Issue 02b removed `th-net`/`sodiumoxide`/`libsodium-sys`, but
`tools/build-windows.sh` still prebuilds libsodium unconditionally and
`ci.yml`/`release.yml`/packaging still describe it. The Windows cross-check
now fails before cargo with `cannot find the vendored libsodium source`
(run 39309854944). Remove the dead prebuild and references; the cross-check
and the Windows release path should work without sodium.

## Read first

- `tools/build-windows.sh` (whole file)
- `.github/workflows/ci.yml` `windows-cross` job + header comments
- `.github/workflows/release.yml` Windows job + header comments
- `tools/package.sh` Windows branch/comment
- `tools/package-android.sh` header comment
- `.cargo/config.toml` (search `advapi32`/sodium)
- `rg -n 'libsodium|sodiumoxide|SODIUM|oh-net|th-net' tools .github .cargo`

## Steps

1. `tools/build-windows.sh`:
   - Remove the `--force-sodium` flag, `FORCE_SODIUM`, the `make` check
     (only the libsodium prebuild used it), and the whole libsodium
     prebuild block (`SODIUM_DIR`/`SODIUM_PREFIX`/`SODIUM_LIB`,
     `cargo fetch` for the source, `configure`/`make`/`make install`,
     `SODIUM_LIB_DIR` export).
   - Keep the rustup target, the MinGW gcc check, the `--check`/`--build`/
     `--release`/`--all-targets` cargo paths exactly as they are.
   - Rewrite the header: the C deps (Luau via `luau0-src`) cross-compile in
     place; no libsodium/sodiumoxide; remove the mingw-config-sub story.
2. `.cargo/config.toml`: remove the Windows `-ladvapi32` link flag only if
   its comment/only purpose was libsodium; if another Windows dep needs it,
   leave it and update the comment. Check with the link.
3. `.github/workflows/ci.yml`: remove the "incl. the libsodium
   cross-prebuild" comment; keep the cross-check job/step
   (`tools/build-windows.sh --check --all-targets`).
4. `.github/workflows/release.yml`: update the Windows header to the
   post-sodium reality (mingw-gnu still the release path; no libsodium
   prebuild); do not change the job's artifact names/steps.
5. `tools/package.sh`: update the Windows-branch comment (the full cargo
   cross-compile; no libsodium). Do not touch the `--with-steam` cleanup
   (Issue 08) or the Linux/web paths.
6. `tools/package-android.sh`: change the C-build comment to
   `luau0-src` (no libsodium-sys).
7. Grep the whole scoped tree; only historical prose may remain (prefer
   removing it); no live path may require sodium.

## Verification

```sh
bash -n tools/build-windows.sh
bash -n tools/package.sh
bash -n tools/package-android.sh
rg -n 'libsodium|sodiumoxide|SODIUM|oh-net|th-net' tools .github .cargo || echo no-sodium-refs
# if the host has the target + mingw:
rustup target list --installed | grep -qx x86_64-pc-windows-gnu \
  && command -v x86_64-w64-mingw32-gcc >/dev/null \
  && bash tools/build-windows.sh --check --all-targets
```

The cross-check must pass on CI (the definitive gate; the host may lack the
mingw target). The host full `tools/gate.sh` is not required for this
shell/CI-only change; run `cargo fmt --check` if you touch Rust (you
shouldn't).

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-003d-report.md`: the removed prebuild,
the updated references, the cross-check result (host/CI), and "No commit
made." Update `FEED-FORWARD.md` §08: the Windows cross path no longer needs
libsodium; 08 keeps the artifact/CI work. The orchestrator lands ONE
squashed commit after review; no worker commit.

## Risks

- Do not remove the Luau/C cross-build prerequisites (`mingw-w64`); only
  sodium.
- Do not change the Windows artifact names or the release job structure.
- `.cargo/config.toml` `-ladvapi32`: verify whether another dep needs it
  before removing; if unsure, keep it and update the comment.
