#!/usr/bin/env bash
# tools/upstream-status.sh — report this fork's delta against `upstream`.
#
# Parses the high-water mark (HWM) from
# .agents/skills/upstream/changelog.md (first line:
# `<!-- high-water-mark: <40-hex> -->`), verifies the `upstream` remote
# points at the SSVOpenHexagon port repo, fetches `upstream` (unless
# --no-fetch), and prints the HWM, upstream HEAD, commit count,
# `git log --oneline HWM..HEAD` and `git diff --stat HWM..HEAD`.
#
# Read-only for tracked files: the default path may update
# remote-tracking refs via `git fetch`; it never writes tracked files or
# the HWM. Manual use only (no scheduled workflow); see
# .agents/skills/upstream/SKILL.md.
#
# Options:
#   --no-fetch   skip `git fetch upstream` (use local refs only)
#   --check      exit 1 when the delta is non-empty, 0 when current
#                (same summary is printed either way)
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "$0")" && pwd -P)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd -P)"
CHANGELOG="$REPO_ROOT/.agents/skills/upstream/changelog.md"

usage() {
  printf '%s\n' \
    "usage: tools/upstream-status.sh [--no-fetch] [--check]" \
    "  --no-fetch  skip 'git fetch upstream' (use local refs)" \
    "  --check     exit 1 when the delta is non-empty, 0 when current"
}

NO_FETCH=0
CHECK=0
for arg in "$@"; do
  case "$arg" in
    --no-fetch) NO_FETCH=1 ;;
    --check)    CHECK=1 ;;
    -h|--help)  usage; exit 0 ;;
    *)
      printf 'error: unknown argument: %s\n' "$arg" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [ ! -f "$CHANGELOG" ]; then
  printf 'error: changelog not found: %s\n' "$CHANGELOG" >&2
  exit 1
fi

FIRST_LINE="$(head -n 1 "$CHANGELOG")"
case "$FIRST_LINE" in
  "<!-- high-water-mark: "*)
    HWM="${FIRST_LINE#<!-- high-water-mark: }"
    HWM="${HWM% -->}"
    ;;
  *)
    printf 'error: first line of %s is not the high-water-mark marker:\n  %s\n' \
      "$CHANGELOG" "$FIRST_LINE" >&2
    exit 1
    ;;
esac
if ! printf '%s' "$HWM" | grep -Eq '^[0-9a-f]{40}$'; then
  printf 'error: high-water mark is not 40 lowercase hex chars: %s\n' "$HWM" >&2
  exit 1
fi

if ! git -C "$REPO_ROOT" remote | grep -qx upstream; then
  printf 'error: git remote "upstream" is not configured (expected https://github.com/gtrak/SSVOpenHexagon)\n' >&2
  exit 1
fi

UPSTREAM_URL="$(git -C "$REPO_ROOT" remote get-url upstream)"
case "$UPSTREAM_URL" in
  "https://github.com/gtrak/SSVOpenHexagon"|"https://github.com/gtrak/SSVOpenHexagon.git"|"git@github.com:gtrak/SSVOpenHexagon"|"git@github.com:gtrak/SSVOpenHexagon.git")
    ;;
  *)
    printf 'error: upstream remote URL "%s" does not point at the SSVOpenHexagon port repo (expected https://github.com/gtrak/SSVOpenHexagon[.git] or the git@ SSH form)\n' "$UPSTREAM_URL" >&2
    exit 1
    ;;
esac

if [ "$NO_FETCH" -eq 0 ]; then
  if ! git -C "$REPO_ROOT" fetch upstream; then
    printf 'error: git fetch upstream failed (remote missing or network unavailable)\n' >&2
    exit 1
  fi
fi

UPSTREAM_REF=""
for cand in upstream/master upstream/main; do
  if git -C "$REPO_ROOT" rev-parse --verify -q "${cand}^{commit}" >/dev/null 2>&1; then
    UPSTREAM_REF="$cand"
    break
  fi
done
if [ -z "$UPSTREAM_REF" ]; then
  printf 'error: cannot resolve upstream/master or upstream/main (run without --no-fetch)\n' >&2
  exit 1
fi
UPSTREAM_HEAD="$(git -C "$REPO_ROOT" rev-parse "${UPSTREAM_REF}^{commit}")"

if ! git -C "$REPO_ROOT" merge-base --is-ancestor "$HWM" "$UPSTREAM_HEAD" 2>/dev/null; then
  MERGE_BASE="$(git -C "$REPO_ROOT" merge-base "$HWM" "$UPSTREAM_HEAD" 2>/dev/null || echo none)"
  printf 'warning: HWM %s is NOT an ancestor of %s (%s); merge base: %s\n' \
    "$HWM" "$UPSTREAM_REF" "$UPSTREAM_HEAD" "$MERGE_BASE" >&2
fi

COUNT="$(git -C "$REPO_ROOT" rev-list --count "$HWM..$UPSTREAM_HEAD")"

printf 'HWM:      %s\n' "$HWM"
printf 'upstream: %s @ %s\n' "$UPSTREAM_REF" "$UPSTREAM_HEAD"
printf 'commits:  %s\n' "$COUNT"
printf '\n'
git -C "$REPO_ROOT" log --oneline "$HWM..$UPSTREAM_HEAD"
printf '\n'
git -C "$REPO_ROOT" diff --stat "$HWM..$UPSTREAM_HEAD"

if [ "$CHECK" -eq 1 ]; then
  if [ "$COUNT" -gt 0 ]; then
    printf 'check: BEHIND — %s upstream commit(s) since HWM\n' "$COUNT"
    exit 1
  fi
  printf 'check: current — 0 upstream commits since HWM\n'
  exit 0
fi
