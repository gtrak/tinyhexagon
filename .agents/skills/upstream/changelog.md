<!-- high-water-mark: 4c31561ac6409e24a1b838043a0bb747f504045d -->

# Upstream changelog (tinyhexagon ↔ gtrak/SSVOpenHexagon Rust port)

The **most recent upstream SHA below is the high-water mark (HWM)**. The
marker line (first line of this file) is its single source of truth and is
parsed by `tools/upstream-status.sh`. Upstream → fork entries are listed
newest-first; the HWM advances only after every commit from the old HWM up to
the new SHA has been triaged (see `SKILL.md`).

## 2026-10-19 — 4c31561a — land(058): the menu's mouse surface, ported site-by-site — and the first-mouse-frame divergence the measurement caught

- Area: baseline (whole upstream repository)
- Disposition: baseline (integrated — fork point)
- Mapping: n/a
- Fork commit: 4c31561ac6409e24a1b838043a0bb747f504045d
- Notes: verified at plan 002 dispatch — `upstream/master` == `HEAD` ==
  `git merge-base HEAD upstream/master` (delta 0).

## Entry format (upstream → fork)

One entry per triaged upstream commit, newest-first, above this block:

```
## <date> — <sha> — <subject>
- Area: …
- Disposition: integrated | reimplemented | deferred | not-applicable
- Mapping: upstream path/crate → fork path/crate (from SKILL.md)
- Fork commit: <sha>
- Notes: …
```

`deferred` entries must name the tracking issue; `not-applicable` entries
state the reason.

## Fork → upstream

- (no entries yet)
