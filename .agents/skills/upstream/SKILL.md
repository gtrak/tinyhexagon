# upstream — fork ↔ SSVOpenHexagon (Rust port) sync

Source of truth: this repo's git remotes + `.agents/skills/upstream/changelog.md`.
The changelog's **first line** is the single source of truth for the high-water
mark (HWM):

    <!-- high-water-mark: <40-hex-sha> -->

`tools/upstream-status.sh` parses the marker and prints the `HWM..upstream`
delta (`--check` exits 1 when the delta is non-empty; `--no-fetch` skips the
network). The script never writes; the HWM only moves via a commit that lands
an integration.

## Remotes

| Remote | URL | Role |
|---|---|---|
| `origin` | `https://github.com/gtrak/tinyhexagon` | this fork (plan 002 product) |
| `upstream` | `https://github.com/gtrak/SSVOpenHexagon` | the original Rust port (default branch `master`) |

Resolve `upstream/master`; fall back to `upstream/main` if the default branch
is ever renamed.

## HWM rule

The HWM is the newest upstream SHA that has been **fully triaged** into this
fork (integrated, reimplemented, deferred-with-tracking, or not-applicable,
one changelog entry per commit). It advances only in the "pull from upstream"
direction, and only in the same commit that lands the triaged work. Never
advance the HWM past an unassessed commit. The "fork → upstream" direction
never moves the HWM.

## Pull changes from upstream (upstream → fork)

Operator prompt: **"pull changes from upstream"**.

1. Run `tools/upstream-status.sh` (fetches `upstream`; prints HWM, upstream
   HEAD, commit count, `git log --oneline HWM..HEAD`, `git diff --stat`).
2. Read the HWM from the `changelog.md` marker (do not trust the printed HWM
   alone; the marker is the source of truth).
3. List `git log --oneline HWM..upstream/master` **oldest first**
   (`--reverse`).
4. Triage each commit with the mapping tables below:
   - touches a kept crate/module → **integrate** (port the diff) or
     **reimplement** in `th-*` code;
   - touches a removed crate or only fork-only behaviour → **not-applicable**
     (record the reason);
   - needed but blocked → **deferred** (open a tracking issue; record it in
     the entry).
   - If a commit cannot be assessed, **stop before it** — leave it untriaged
   for the next pull; do not move the marker past it.
5. Run the gate (`tools/gate.sh`) and dispatch the forked `reviewer`.
6. On PASS: land the integration in **one commit** that also prepends a
   changelog entry per triaged commit (newest first) and moves the marker to
   the new upstream HEAD.

Changelog entry per commit: date, SHA, subject, area, disposition, mapping
used, fork commit (for integrated/reimplemented), notes.

## Implement this change to upstream (fork → upstream)

Operator prompt: **"implement this change to upstream"**.

1. Identify the fork commit/feature that carries the change.
2. Map it to the upstream crate/module via the tables below (upstream paths
   may have moved — match by feature, not by path).
3. Prepare the branch/patch against `upstream/master` in the upstream
   tree.
4. Run the upstream gate in the upstream tree (upstream CI/tooling — not this
   repo's `tools/gate.sh`).
5. Record the entry under `## Fork → upstream` in `changelog.md`
   (`Status: proposed`, or `Status: landed upstream <sha>` once it lands).
6. **Do not move the HWM** and do not add entries to the upstream → fork
   section for this direction.

## Crate mapping (verified against `crates/` and plan 002)

| Upstream (`oh-*`) | Fork (`th-*`) | State / notes |
|---|---|---|
| `oh-math` | `th-math` | 1:1 |
| `oh-rng` | `th-rng` | 1:1 (parity-only state API may be trimmed) |
| `oh-data` | `th-data` | 1:1 + beat-hue setter |
| `oh-config` | `th-config` | 1:1 |
| `oh-assets` | `th-assets` | 1:1 |
| `oh-audio` | `th-audio` | 1:1, wired for SFX |
| `oh-lua` | `th-lua` | replay/steam hooks removed |
| `oh-game` | `th-game` | replay, Steam, Discord and the profile/score/PB system removed; soft collision, difficulty setter, beat, wall gaps added; upstream profile/score/PB changes triage as not-applicable/adapted, not kept |
| `oh-render` | `th-render` | `DrawOptions`/text suppression added |
| `oh-app` | `th-app` | adult modules removed; tiny loop/journey/touch added; binary `tinyhexagon` |
| `oh-android` | `th-android` | repurposed tiny entry |
| `oh-web` | `th-web` | repurposed tiny entry |
| `oh-replay` | — | removed |
| `oh-net` | — | removed |
| `oh-steam` | — | removed |
| `oh-discord` | — | removed |
| `oh-server-control` | — | removed |
| `oh-workshop-uploader` | — | removed |
| `oh-menu` | — | removed |
| `oh-menu-system` | — | removed |
| `oh-parity` | — | removed |
| `oh-additions` | — | removed; `fullscreen.rs` salvaged into `th-app` |
| — | `th-synth` | new (rust-synth) |

All 22 crates in `crates/` at baseline `4c31561a` are accounted for: 12
rename (10 kept + `oh-android`/`oh-web` repurposed), 10 removed, 1 new
(`th-synth`).

## Module mapping (altered files)

| Upstream path | Fork path |
|---|---|
| `oh-game/src/{update,hexagon_game,seams,lua_console}.rs` | `th-game/src/{update,hexagon_game,seams,lua_console}.rs` |
| `oh-game/src/lua_state.rs` | `th-game/src/lua_state.rs` (replay/Steam hooks `GameApi::in_replay`, `GameApi::unlock_achievement` removed in Issue 02) |
| `oh-game/src/status.rs` | `th-game/src/status.rs` (bump/flash fields added in Issue 04) |
| `oh-lua/src/api/steam.rs` | removed (registration list in `th-lua/src/api/mod.rs` updated) |
| `oh-data/src/style_data.rs` | `th-data/src/style_data.rs` |
| `oh-app/src/{compose,bridge,lifecycle,sigint,joystick}.rs` | `th-app/src/{compose,bridge,lifecycle,sigint,joystick}.rs` |
| `oh-app/src/windowed.rs` | `th-app/src/{tiny,draw_target}.rs` |
| `oh-app/src/menu_draw.rs` + the adult-only modules | removed |
| `oh-render/src/{graphics,window}.rs` | `th-render/src/{graphics,window}.rs` |
| `oh-android`, `oh-web` | repurposed (`th-android`, `th-web`) |
| `oh-additions/src/fullscreen.rs` | `th-app/src/web_fullscreen.rs` |

## Fork-only list (do not backport to upstream)

- soft collision / no death (burst/flash/slow/duck instead of dying)
- the 20-stage × 30 s (10-minute) escalation arc with plateau
- text suppression (no UI text)
- beat palette override (colours react to beat/bar/energy)
- kid touch mapping (hold halves + two corner icons)
- rust-synth integration (`th-synth`)
- the tiny content pack (`_RELEASE/Packs/tiny`)
- the removed crates (replay, net, steam, discord, server-control,
  workshop-uploader, menu, menu-system, parity, additions)

Upstream changes that touch the equivalent areas are adapted, not copied.

## Drift rules

- Upstream paths may be gone (upstream can refactor freely); **match by
  feature**, not by path.
- Never advance the HWM past an unassessed commit.
- Record the fork commit for reimplementations (changelog entry field).
- Record deferred items with a tracking issue.
- The reverse direction (fork → upstream) never moves the HWM.
