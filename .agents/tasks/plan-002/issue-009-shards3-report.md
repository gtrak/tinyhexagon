# Issue 09 (shards-3) — physical collision propagation for glass shards

**Status:** DONE. `bash tools/gate.sh` → `GATE_EXIT=0` (all 4 stages). Test
total: **580 passed / 0 failed / 1 ignored** (the pre-existing th-lua
ignored). `cargo fmt --all` clean. No commit made.

## What changed

Replaced the leftover mid-angle chain queue (the glass-rework's 5.4 FT /
10.8 FT timer hops) with COLLISION-DRIVEN propagation: shards that sweep the
ring physically shatter the live walls they overlap. Add-on top of the
glass-shard rework (`8799758c`) — the burst, the physics, the draw and the
cap are untouched.

| File | Change |
|---|---|
| `crates/th-game/src/shrapnel.rs` | `Shrapnel` gains the lineage (`burst: u32`, `generation: u32`); dual-mode `make_shard` (~half radial spray — kept physics: mass-scaled impulse, tangential jitter, 12 px/FT clamp, 48-FT life — ~half tangential carriers: 6..21 px/FT ring sweep, 60-FT life, both ring directions, biased by the hitter's tangential velocity, small outward radial kick); `split_wall_quad` now takes `burst`/`generation`/`tangential` and cuts the fan biased at the impact point; core bounce switched to CROSSING semantics; DELETED `PendingChain`, `wall_mid_angle`, `angle_distance`; module docs: propagation contract + reach math (below); 9 unit tests (incl. new: crossing bounce, dual-mode aim, carrier direction bias). |
| `crates/th-game/src/update.rs` | NEW per-tick IMPACT PASS (`update_shrapnel_impacts`, called from `update_shrapnel` after motion/cull): snapshot `(piece, wall)` overlaps on live walls only, resolve in (piece, wall) vector order — gen < `MAX_PROPAGATION`: shatter that wall AT THE PIECE'S POSITION (hitter consumed; new shards inherit the blast's `burst`, `generation + 1`, carrier bias from the hitter's tangential velocity); gen ≥ 2: piece crumbles, wall survives. DELETED `queue_shrapnel_chain`, `update_shrapnel_chains`, the `shrapnel_chains()`-feeding queue. Soft branch: a wall already dead when the player's pass runs (shard-shattered earlier in the same tick) is not a second physical hit — no re-shatter, no second bump. `shatter_wall_at` is the one shatter site (player hit and shard impact alike). |
| `crates/th-game/src/hexagon_game.rs` | `EngineState`: `shrapnel_burst_seq: u64` → `next_burst: u32` (blast id counter, ALLOCATED on the player's physical hit only, reset to 0 by `new_game` with the shards). |
| `crates/th-game/src/lib.rs` | Getter rename: `shrapnel_chains()` (queued hops) → removed; `shrapnel()` unchanged. |
| `crates/th-game/src/lua_console.rs` | Test-only `EngineState` literal updated to `next_burst`. |
| `crates/th-game/src/seams.rs` | NEW `PcgCosmeticRng::with_seed(seed: u64)` — injectable seeded cosmetic rng for the propagation tests. |
| `crates/th-game/src/player.rs` | NEW `Player::tangential_speed(movement_dir, ft)` — the player's ring (tangential) speed in px/FT, the burst's carrier-direction bias input. |
| `crates/th-game/tests/shrapnel.rs` | Rewritten around the physical propagation (see below); seeded fixtures (`SEED = 42`). |
| `crates/th-game/tests/soft_collisions.rs` | `bump_serial_advances_once_per_killed_wall`: the post-blast "fresh collision" and "two overlapping walls" scenarios now use CUSTOM walls (see Residual decisions — the old CWall-at-the-blast-site scenario is no longer a player-kill scenario under the new physics). |

Deletion list verified empty by grep across `crates/`: `PendingChain`,
`shrapnel_chains`, `queue_shrapnel_chain`, `wall_mid_angle`,
`angle_distance`, `shrapnel_burst_seq`, and the 5.4/10.8 FT hop constants.

## Lineage

Every shard carries its BLAST's `burst` id (engine's `next_burst`, allocated
on the player's physical hit, reset by `new_game`) and its `generation`
(0 = the player hit's pieces; +1 per shard-impact shatter). Same-blast
shards skip the pairwise bounce (the rework's "fragments of one wall" rule,
generalized blast-wide). `MAX_PROPAGATION = 2`: a gen-2 piece crumbles on
impact (wall survives) — one player hit shatters a bounded set of walls.

## The impact pass (per tick, in `update_shrapnel` after motion/cull)

1. **Snapshot** the shard vector and the `(piece idx, wall idx)` hits: a
   piece's centroid vs each LIVE wall's quad (`CWall::is_overlapping`),
   first wall in vector order per piece. DEAD walls are never hit — a piece
   flying over a dead wall's former quad is not consumed (only live walls
   collide); the soft branch applies the same skip to the player's pass
   (a same-tick shard-shattered wall is not re-bumped by the player).
2. **Resolve in order**: re-find the piece by exact state match in the
   current vector (sound under the same-tick `make_room` front eviction —
   nothing else mutates a surviving piece); CONSUME THE HITTER FIRST; then
   shatter-at-position or crumble (gen ≥ 2, or the wall died earlier in the
   same tick → the piece crumbles, no second shatter/bump).
3. **No timers, no extra bumps**: `bump_serial` increments only on the
   player's physical hit (the flash rides per bump). Deterministic: hits
   resolve in (piece, wall) vector order on the pre-mutation snapshot.

## Spawn aim (per the plan: ~half each)

- **Radial spray (kept):** mass-scaled impulse from the impact point +
  tangential jitter, clamped 12 px/FT, 48-FT life, drag 0.9/s per FT,
  tumbling — the rework's physics, unchanged.
- **Tangential carriers (new):** rng 6..21 px/FT along the ring tangent,
  both ring directions, the direction biased by the HITTER'S tangential
  velocity at the impact (the sweep continues where the sweep was going),
  60-FT life, plus a small outward radial kick (0.3..1.2 px/FT) that keeps
  `|v·tangent| > |v·radial|` and `v·(position − impact) > 0` — the
  legacy radial-dot invariants, now spray-only.

## Reach vs the 60° arc-to-neighbour (the plan's tuning question)

A carrier's tangent is a FIXED world direction (no centripetal force), so a
piece flies along a CHORD: at chord distance d its radius is
`sqrt(r0² + d²)`. With the per-FT drag a piece flies `v·ft·Σ(1−d·ft)^k` ≈
`v·33.7 px` of chord over 48 FT (≈ `v·39.6 px` over the carriers' 60 FT) —
the 6..21 px/FT carriers fly **238..830 px** of chord.

- **In-band dwell** (the piece can still hit the neighbour span while it
  stays inside the walls' radial band): the dwell chord is
  `sqrt(band_outer² − r0²)` ≈ **87..256 px** for a mid-band piece
  (r0 ≈ 110, wall thickness 60..200 px) — a **~40..70° in-band sweep**,
  against the **60° arc to the neighbour span**. Most of the carrier
  distribution covers the full arc (thicker walls lengthen the dwell; the
  impact-biased fan cut clusters pieces near the hitter, shortening the
  remaining arc). The slow end at thin walls can miss the far corner of
  the neighbour span — the gen cap and the second carrier direction cover
  the miss.
- **Out-of-band reach:** past the band the chord keeps growing (final
  radius `sqrt(r0² + d²)` ≈ **260..840 px** from a 110 band) — the fast
  pieces sweep the whole annulus and can reach walls far outside the hit's
  band. In the seeded fixture the level's PATTERN walls (out at r ~800-1000
  by ~tick 200-250) get hit by the gen-1 chord sweeps, spawning **bounded
  gen-2 debris far out** (gen 2 crumbles on the next hit; the debris
  melts/crumbles out by ~tick 260-420 — probe data below). This is
  intended physics (a fragment flying straight hits what it crosses), and
  it is bounded in every case by the generation cap (2), the 48/60-FT
  lifetimes and the 64-shard cap (worst-case lineage: empty by tick ~720).

## Core bounce (crossing semantics, per the plan)

A shard bounces off the breathing core edge (`radius × 1.05`) ONLY when it
CROSSES from outside to inside within the tick (pre/post-step radii vs the
edge, inward post-drag velocity) — pieces born inside the edge fly free.
`BOUNCE_KEEP = 0.5` radial reflection, push back onto the edge, tumble
flipped (damped) — the rework's bounce, re-gated to crossings (unit test:
`the_core_bounce_uses_crossing_semantics`).

## Seeded determinism

The propagation tests inject `PcgCosmeticRng::with_seed(42)` (via the
existing `compose_with` seam); the cosmetic rng drives the carrier coin,
the carrier direction, the fan jitter — all pinned. Observed timeline
(seed 42, `apeirogon` level, injected walls at the 80..140 band,
`.agents/evidence/09-shards3/probe-timeline.txt`):

- **Fixture A (sides 0,1,5):** hit tick 1 (burst 0, 7 pieces, 3 carriers
  biased CW + CCW, 4 sprays) → tick 3: a gen-0 CW carrier overlaps side 5's
  quad → side 5 shatters AT THE CARRIER'S POSITION (gen-1 burst, 10
  pieces, carrier bias CW = the hitter's sweep), no bump → tick 6: a gen-0
  CCW carrier overlaps side 1 → same (gen-1, 9 pieces) → the gen-1 chord
  sweeps reach the pattern walls out at r ~800-1000 by ~tick 200-250 (gen-2
  debris: 9 → 19 → melts/crumbles out, gone by ~tick 420) → bump stays 1
  for all 460 probed ticks; the level's own pattern walls first enter the
  500-radius band at ~tick 329 (their player hits come ~tick 430+ — outside
  the tests' windows).
- **Fixture B (sides 0,1,2,3):** hit tick 1 → tick 6: gen-0 carrier
  shatters side 1 (hop 0→1) → tick 10: a gen-1 carrier shatters side 2
  (hop 1→2, gen-2 burst, 9 pieces) → the gen-2 pieces crumble on side 3's
  quad (observed: gen-2 count 9 → 8 by tick 20 — consumed before any gen-2
  melt is possible (gen-2 life ≥ 192 ticks from tick 10)) while side 3
  SURVIVES every tick → the whole blast (incl. far-field gen-2) is empty by
  tick 260 → bump stays 1; alive = `[3]` at the window end.

## Tests

- **Unit (9, `shrapnel.rs`):** quad split (count 6..10, containment, glass
  tint, outward impulse), dual-mode aim (both modes present, carrier speed
  6..21 ± kick, carrier tangential-dominant), carrier direction bias
  (hitter tangential + → more CCW carriers), exact
  position/drag/tumble/spin/life integration, crossing core bounce,
  pairwise bounce (separation + momentum, same-blast skip, non-overlap
  no-op), cap eviction order, draw fan.
- **Integration (6, `tests/shrapnel.rs`, seeded except the draw test):**
  1. soft hit → glass shards cut from the wall's quad (containment,
     dual-mode, one bump);
  2. **a shard impact shatters a neighbouring wall with no extra bump**
     (fixture A: the first neighbour death is a shard impact — bump still
     1; the gen-1 burst sits in the victim's sector at the walls' band with
     the blast's burst id; per-tick to tick 300: bump stays 1, no shard
     past gen 2, cap holds; the blast's near field is settled by tick 300
     (no gen-0, nothing in r < 250) while the bounded far-field debris may
     still be melting out);
  3. **the full 0→1→2 propagation crumbles at gen 2** (fixture B: hop 0→1
     then hop 1→2, gen-2 burst present, a gen-2 piece crumbles in the near
     field before it could melt (count-delta evidence: near-field gen-2
     AND total gen-2 both drop by tick ≤ 150 — a melt is impossible before
     tick ~194, and no far-field gen-2 exists that early), side 3 survives
     every tick, exactly two propagation shatters, alive = `[3]` at the end);
  4. `new_game` clears the shards AND the burst counter (the post-reset
     blast starts at burst 0, gen 0);
  5. loop physics wiring (life/speed/angle universal; exact Euler + drag +
     spin for every non-bounced shard; the crossing-bounce prediction);
  6. draw batches contain the shard fans (9-vertex fans, glass colors,
     life-fade, shared centroid positions).

- **The one pre-existing test the rework changed:**
  `soft_collisions::bump_serial_advances_once_per_killed_wall` — its
  "fresh CWall injected at the blast site" scenario now meets the blast's
  shards: a shard impact (physical propagation, NO bump) shatters that
  wall before the player's pass, so the wall is not a player kill. The
  post-blast collisions in that test now use CUSTOM walls (soft-killable,
  one bump per killed wall, and outside the shard impact pass — which
  covers `CWall`s only), isolating the bump bookkeeping the test is about.
  The first collision (CWall, tick 1) and the no-double-count section are
  unchanged.

## Residual risks / decisions

- **Far-field pattern-wall debris** is real physics and is pinned as
  bounded (gen ≤ 2, lifetimes, 64 cap) — not "everything empty at window
  end" (the level's pattern-wall cadence is game-RNG-driven and outside the
  tests' contract).
- **Custom walls are outside the shard impact pass** (it iterates
  `g.walls` / `CWall` only, matching the engine's wall-update structure);
  the plan's "live walls" is read as the CWall ring. If the operator wants
  shards to also shatter custom quads, that is a small follow-on (the
  impact pass gets a `cw_manager` pass).
- **Visual tuning is unverified in a window** (headless-only): the
  carrier speed/life/bias constants are named `pub` in `shrapnel.rs` for a
  taste pass.
- The `bump_serial` test's fixture stays UNSEEDED (its assertions are now
  seed-independent: the CWall first hit is pulse-deterministic, custom
  walls are immune to shards); the 5 propagation tests are seed-pinned.

## Evidence

- `.agents/evidence/09-shards3/gate-exit.txt` — final gate: all 4 stages
  `GATE_EXIT=0` (fmt check, build, clippy `-D warnings`, test).
- `.agents/evidence/09-shards3/gate.log.tail` — `tail -60` of
  `target/gate.log`.
- `.agents/evidence/09-shards3/unit-tests.txt` — 9/9 shrapnel unit tests.
- `.agents/evidence/09-shards3/integration-tests.txt` — 6/6 shrapnel
  integration tests + 9/9 soft_collisions (the changed test).
- `.agents/evidence/09-shards3/probe-timeline.txt` — the seed-42
  timelines (both fixtures, ticks 1..460: alive sides, shard counts,
  gen-2 counts, bump serial).
