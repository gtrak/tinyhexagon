#!/usr/bin/env bash
# Issue 08c — the tinyhexagon release packager (Linux + Windows): builds
# the `tinyhexagon` release binary, assembles it ALONGSIDE a pristine
# copy of the tracked `_RELEASE` (the script never writes into the
# tracked `_RELEASE`), and packages the result as
#
#   tinyhexagon-<tag>-linux-x86_64.tar.gz    (default, host build)
#   tinyhexagon-<tag>-windows-x86_64.zip     (--windows, the mingw-gnu
#                                             cross build via
#                                             tools/build-windows.sh)
#
# containing the `tinyhexagon` binary + the `_RELEASE/` data (the binary
# inside `_RELEASE/`, alongside `Assets/`, `Packs/`, `ConfigOverrides/`,
# `config.json` — the C++'s own install-root layout the game's
# `resolve_game_root` walk expects: the binary next to its data).
#
# Usage:
#   bash tools/package.sh                     # Linux: release build + stage + smoke
#   bash tools/package.sh --windows           # Windows cross build + stage (no smoke:
#                                             # a Linux host has no .exe runtime)
#   bash tools/package.sh --tag <tag>         # the artifact's <tag> (default:
#                                             # $ASSET_TAG — the release workflow's
#                                             # normalized github.ref_name — else "dev")
#   bash tools/package.sh --skip-smoke        # skip the packaged-binary smoke test
#
# The final artifacts land at the REPO ROOT (the release workflow's
# Upload steps look for them there); the staging lives under the
# gitignored target/:
#   target/package/linux-x86_64/_RELEASE/     the Linux staged install root
#   target/package/windows-x86_64/_RELEASE/   the Windows staged install root
#
# ---------------------------------------------------------------------
# Staging + the pristine-copy invariant
# ---------------------------------------------------------------------
# Everything is COPIED from the tracked `_RELEASE`; nothing is ever
# written back into it (verified at the end via `git status --porcelain`).
# The tiny runtime persists nothing (no config / profile / replay
# writes — plan 002 issue 07's no-storage contract), so the staged
# `config.json` stays the shipped one.
#
# ---------------------------------------------------------------------
# The smoke test (the packaged binary MUST be tested)
# ---------------------------------------------------------------------
# The tiny `tinyhexagon` binary is a WINDOWED game (there is no
# headless mode — the retired adult replay fixture smoke has no tiny
# twin; tools/fixtures/cube_1_apeirogon.ohr.z is the plan-001 smoke
# fixture, kept for history). So the packaged binary is tested
# WINDOWED through tools/smoke-windowed.sh (the same three phases the
# windowed column runs: the `--th-desktop-mode-probe` child, the
# shipped-config boot, the auto-resolution reapply — a real GL context
# up, no one-frame headless fallback, still in the frame loop at the
# timeout). The script's own skip-with-reason (a headless box with no
# Xvfb is NOT run, not faked) applies; on the GitHub linux runner the
# display packages come from tools/ci-deps.sh, so the smoke runs there.
# The Windows cross build is smoke-skipped-with-reason on a Linux host
# (no .exe runtime; covered by the windows-latest CI gate + a manual
# run).

set -euo pipefail
cd "$(dirname "$0")/.."

REPO_ROOT="$(pwd)"
RELEASE_SRC="${REPO_ROOT}/_RELEASE"

PLATFORM="linux"
TAG=""
SKIP_SMOKE=0

while [ $# -gt 0 ]; do
    case "$1" in
    --linux) PLATFORM="linux" ;;
    --windows) PLATFORM="windows" ;;
    --skip-smoke) SKIP_SMOKE=1 ;;
    --tag)
        [ $# -ge 2 ] || { echo "error: --tag needs a value" >&2; exit 2; }
        TAG="$2"; shift ;;
    --tag=*) TAG="${1#--tag=}" ;;
    -h | --help)
        grep '^#' "$0" | sed 's/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "unknown flag: $1 (see --help)" >&2
        exit 2
        ;;
    esac
    shift
done

# The artifact's <tag>: --tag, else ASSET_TAG (release.yml's normalized
# github.ref_name — the workflow env, already filesystem-safe), else
# "dev" (a local run without a tag).
TAG="${TAG:-${ASSET_TAG:-dev}}"
case "$TAG" in */*|'') echo "error: unsafe artifact tag: '$TAG'" >&2; exit 2 ;; esac

if [ "$PLATFORM" = "linux" ]; then
    echo "== cargo build --workspace --release"
    cargo build --workspace --release
    BIN="target/release/tinyhexagon"
    PLATFORM_DIR="linux-x86_64"
else
    # The Windows cross build (tools/build-windows.sh: the full
    # cargo cross-compile + link of the .exe's, no libsodium prebuild —
    # Luau is the only C dep in the graph). Its prereqs (the rustup
    # target, mingw-w64) are checked inside that script.
    echo "== bash tools/build-windows.sh --build --release"
    bash tools/build-windows.sh --build --release
    BIN="target/x86_64-pc-windows-gnu/release/tinyhexagon.exe"
    PLATFORM_DIR="windows-x86_64"
fi

[ -f "$BIN" ] || { echo "error: release binary missing: $BIN" >&2; exit 1; }

# ---------------------------------------------------------------------
# The staging (a pristine copy of the tracked _RELEASE + the binary)
# ---------------------------------------------------------------------
STAGE_ROOT="${REPO_ROOT}/target/package"
STAGE="${STAGE_ROOT}/${PLATFORM_DIR}/_RELEASE"

echo "== staging ${STAGE} (from a pristine copy of ${RELEASE_SRC})"
rm -rf "${STAGE_ROOT}/${PLATFORM_DIR}"
mkdir -p "$STAGE"

# The layout's data entries (copied — never moved, never written back):
# `Assets/` + `Packs/` + `ConfigOverrides/` + `config.json` (the tracked
# _RELEASE/config.json is the binary's config root; the tiny runtime
# reads it and never writes it back).
cp -R "${RELEASE_SRC}/Assets" "$STAGE/"
cp -R "${RELEASE_SRC}/Packs" "$STAGE/"
cp -R "${RELEASE_SRC}/ConfigOverrides" "$STAGE/"
cp "${RELEASE_SRC}/config.json" "$STAGE/"

# The binary, alongside them.
cp "$BIN" "$STAGE/"
chmod +x "$STAGE/$(basename "$BIN")"

# ---------------------------------------------------------------------
# The smoke test: the packaged binary, windowed, in a sandboxed copy
# ---------------------------------------------------------------------
if [ "$SKIP_SMOKE" -eq 1 ]; then
    echo "== smoke test skipped (--skip-smoke)"
elif [ "$PLATFORM" != "linux" ]; then
    echo "== smoke test skipped-with-reason: no .exe runtime on this Linux host (covered by the windows-latest CI gate + a manual run)"
else
    # tools/smoke-windowed.sh: the three windowed-boot phases against
    # the STAGED binary (it sandboxes the binary + a copy of the
    # tracked _RELEASE itself — the same bytes as the staged one; the
    # pristine-copy invariant below proves that). Skips with reason
    # (exit 0) when no display / xvfb is available — not faked.
    echo "== smoke: tools/smoke-windowed.sh against the staged binary"
    bash tools/smoke-windowed.sh "${STAGE}/$(basename "$BIN")"
fi

# ---------------------------------------------------------------------
# The artifact (the tagged tar.gz / zip at the repo root)
# ---------------------------------------------------------------------
ARTIFACT=""
if [ "$PLATFORM" = "linux" ]; then
    ARTIFACT="${REPO_ROOT}/tinyhexagon-${TAG}-linux-x86_64.tar.gz"
    echo "== taring ${ARTIFACT#"$REPO_ROOT"/} (the _RELEASE layout: the binary inside)"
    tar czf "$ARTIFACT" -C "${STAGE_ROOT}/${PLATFORM_DIR}" _RELEASE
else
    ARTIFACT="${REPO_ROOT}/tinyhexagon-${TAG}-windows-x86_64.zip"
    echo "== zipping ${ARTIFACT#"$REPO_ROOT"/} (the _RELEASE layout: the binary inside)"
    if command -v zip >/dev/null; then
        (cd "${STAGE_ROOT}/${PLATFORM_DIR}" && zip -qr "$ARTIFACT" _RELEASE)
    else
        echo "== zip not found; using python3 (same layout: the _RELEASE dir at the zip root)"
        python3 - "$STAGE_ROOT/${PLATFORM_DIR}" "$ARTIFACT" <<'PY'
import sys, zipfile, os
root, out = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for dirpath, _dirnames, filenames in os.walk(os.path.join(root, "_RELEASE")):
        for name in filenames:
            full = os.path.join(dirpath, name)
            z.write(full, os.path.relpath(full, root))
PY
    fi
fi
[ -s "$ARTIFACT" ] || { echo "error: artifact not produced: $ARTIFACT" >&2; exit 1; }

# ---------------------------------------------------------------------
# The layout report + the pristine-copy invariant
# ---------------------------------------------------------------------
echo
echo "== staged layout (${STAGE}):"
(cd "$STAGE" && find . -maxdepth 1 | sort)
echo
echo "== artifact: ${ARTIFACT#"$REPO_ROOT"/} ($(du -h "$ARTIFACT" | cut -f1))"
echo
echo "== tracked _RELEASE untouched (git status):"
if [ -n "$(git status --porcelain -- _RELEASE)" ]; then
    git status --porcelain -- _RELEASE
    echo "error: the tracked _RELEASE was mutated" >&2
    exit 1
fi
echo "(clean)"
echo
echo "PACKAGED: ${ARTIFACT#"$REPO_ROOT"/} (staged: ${STAGE})"
