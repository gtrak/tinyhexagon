#!/usr/bin/env bash
# Issue 22 — the x86_64-pc-windows-gnu cross-build helper (Linux/macOS host).
#
#   bash tools/build-windows.sh            # cargo check --workspace --target x86_64-pc-windows-gnu
#   bash tools/build-windows.sh --all-targets   # also check the test/bench targets
#   bash tools/build-windows.sh --build    # FULL cross-build (compiles + links the .exe's)
#   bash tools/build-windows.sh --build --release  # the RELEASE profile (issue 24 packaging)
#
# Prereqs (both on the GitHub ubuntu-latest runner and any dev box with
# mingw-w64 installed):
#   * the rustup target:     rustup target add x86_64-pc-windows-gnu
#   * the MinGW cross gcc:   x86_64-w64-mingw32-gcc (Debian/Ubuntu: apt install mingw-w64)
#
# The C deps cross-compile in place: Luau (mlua 0.10.5 `luau` feature via
# luau0-src — D-037: the port's single Lua VM, statically built from
# C++ source through the same mingw cross toolchain) goes through cc as
# usual. There is no libsodium/sodiumoxide in the graph (th-net went in
# issue 02b), so there is no cross-prebuild and no special link line.
# NOTE: the MSVC target (x86_64-pc-windows-msvc) is NOT cross-buildable from
# Linux (the C++/C deps' msvc path needs the native MSVC toolchain) — it is
# covered by the windows-latest CI job instead.
#
# Linking: 'cargo check' never links, so the check path needs nothing beyond
# the above. '--build' links with the target's default linker
# (x86_64-w64-mingw32-gcc — no linker override needed as long as mingw-w64
# is installed).

set -u
cd "$(dirname "$0")/.."

TARGET="x86_64-pc-windows-gnu"
MODE="check"
EXTRA_TARGETS=()
RELEASE=0

for arg in "$@"; do
    case "$arg" in
    --check) MODE="check" ;;
    --build) MODE="build" ;;
    --all-targets) EXTRA_TARGETS=(--all-targets) ;;
    --release) RELEASE=1 ;;
    -h | --help)
        grep '^#' "$0" | sed 's/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "unknown flag: $arg (see --help)" >&2
        exit 2
        ;;
    esac
done

CROSS_GCC="x86_64-w64-mingw32-gcc"
command -v "$CROSS_GCC" >/dev/null || {
    echo "error: $CROSS_GCC not found (Debian/Ubuntu: sudo apt install mingw-w64)" >&2
    exit 1
}

# ---------------------------------------------------------------------
# rustup target
# ---------------------------------------------------------------------
if ! rustup target list --installed | grep -qx "$TARGET"; then
    echo "== installing rustup target $TARGET"
    rustup target add "$TARGET"
fi

# ---------------------------------------------------------------------
# The cargo step
# ---------------------------------------------------------------------
case "$MODE" in
check)
    echo "== cargo check --workspace ${EXTRA_TARGETS[*]:-} --target $TARGET"
    exec cargo check --workspace "${EXTRA_TARGETS[@]+"${EXTRA_TARGETS[@]}"}" --target "$TARGET"
    ;;
build)
    if [ "$RELEASE" -eq 1 ]; then
        echo "== cargo build --workspace --release --target $TARGET (full cross-compile + link, release profile)"
        exec cargo build --workspace --release --target "$TARGET"
    else
        echo "== cargo build --workspace --target $TARGET (full cross-compile + link)"
        exec cargo build --workspace --target "$TARGET"
    fi
    ;;
esac
