# Issue 09 (rework) — collision shrapnel: glass shards with a real physics sim

**Status:** DONE. Gate 585+/0 (585 passed / 0 failed / 1 ignored). `cargo fmt --all` clean. No commit made.

## What changed

Replaced the old "12 uniform wedges + 2-wall chain" shrapnel wholesale with a
glass-shard burst, per the operator brief ("make the wall turn into glass-like
shards that burst with a real physics sim").

| File | Change |
|---|---|
| `crates/th-game/src/shrapnel.rs` | Rewritten from scratch: the `Shrapnel` shard (polygon + physics state), the jittered-fan quad split, the shatter/chain contract, the cap, the pairwise bounce, unit tests (9). |
| `crates/th-game/src/update.rs` | `shatter_wall` now cuts the wall's actual quad into shards and spawns them (burst id + cap eviction); `update_shrapnel` adds the pairwise bounce after the per-shard integration; soft-hit branch keeps its one-bump + flash. |
| `crates/th-game/src/hexagon_game.rs` | `EngineState` gains `shrapnel_burst_seq` (u64, per-shatter counter); draw pass pushes the shard fans into the same `FastVertexVectorTris` wall batch (comment updated; call shape unchanged). |
| `crates/th-game/src/lua_console.rs` | Test-only `EngineState` literal: +`shrapnel_burst_seq: 0`. |
| `crates/th-game/tests/shrapnel.rs` | Rewritten: 5 integration tests pinning the new behavior. |

## Shard design

- **Cut from the wall's actual quad.** On a soft hit, the hit wall's 4-vertex
  shape is split into 6..10 irregular convex pieces by a *jittered fan*: a
  jittered interior point (quad centroid ± 0.25·span, clamped inside the quad)
  fans out to the 4 corners plus 2..6 jittered edge points; each consecutive
  perimeter pair + fan point is one triangular shard. All pieces tile the wall
  exactly (pinned by tests: every shard vertex is inside the wall quad).
- **Glass look.** Per-vertex color: the fan-centroid vertex carries the wall's
  own color; the perimeter vertices carry it lightened 55% toward white at a
  0.75× alpha (the "brightened edges, alpha-faded" glass read). All alphas
  scale with remaining life; the wall pass's SFML Alpha blend fades them. No
  new textures, no new render passes, no DrawTarget changes.
- **Real physics (all in FT units, 60 = 1 s, 240 Hz tick = 0.25).**
  - Spawn: each shard carries **mass ∝ area** (area/500, clamped 0.2..4.0) and
    velocity = **radial impulse from the impact point** (rng 2.0..5.5 momentum
    units ÷ mass) **+ tangential jitter** (±1.6 px/FT), clamped to 12 px/FT;
    plus decaying **tumble** (spin rng ±5/60 rad/FT, decay 0.02/FT).
  - Per tick: explicit-Euler position step, multiplicative drag (0.9/s →
    /66.7… per FT: `1 − 0.9/60·ft`), spin decay, life burn (48 FT = 0.8 s).
  - **Core bounce:** a shard that ends its step inside the breathing core edge
    (`radius × 1.05`) moving inward gets a radial reflection with
    `BOUNCE_KEEP = 0.5` (energy loss ~0.5), is pushed back onto the core edge,
    and its tumble flips (damped).
  - **Pairwise shard-shard bounce (kept, not skipped):** light
    circle-approximation elastic collisions (circumradius, inverse-mass
    positional separation + impulse, restitution 0.5) between shards of
    **different shatters** — a `burst` id per shatter event skips same-wall
    fragments (they tile the one wall and would otherwise teleport apart).
    O(n²) at the 64 cap ≈ 2k checks/tick — negligible.
- **Chain kept:** the blast still takes out the 2 nearest neighbour walls at
  5.4 FT / 10.8 FT (90/180 ms) hops; each victim shatters into its own shard
  set; **one bump per physical hit** (chain walls add no bumps) — pinned.
- **Cap:** `MAX_LIVE_SHARDS = 64`; on spawn the oldest shards are evicted
  first (`make_room`, front-of-vector = oldest).

## API kept (no app/test churn)

`HexagonGame::shrapnel()` / `shrapnel_chains()` getters unchanged in shape;
`shrapnel.rs` still exports `wall_mid_angle` / `angle_distance` /
`PendingChain`; `new_game` clears shards + chain queue; `draw_scene_batches`
still returns the same 4-tuple (shards ride the wall batch).

## Verification

- `bash tools/gate.sh`: fmt / build / clippy (`-D warnings`) / test — all
  `GATE_EXIT=0`. Test total: **585 passed, 0 failed, 1 ignored** (581+ gate).
- 9 unit tests in `shrapnel.rs`: quad split (count 6..10, all vertices inside
  the quad, glass tint, outward-from-impact impulse), exact
  position/drag/tumble/spin/life integration, exact core-bounce reflection +
  damping + push-out + tumble flip, pairwise bounce (separation + momentum
  exchange; same-burst skip; non-overlap no-op), cap eviction order, draw
  (9-vertex fan, per-vertex glass colors, life-fade, dead = transparent),
  seam-aware mid-angle/distance.
- 5 integration tests in `tests/shrapnel.rs`: soft hit → shards cut from the
  wall's quad (containment + radial impulse + one bump); chain kills exactly
  the 2 nearest walls at the exact hop ticks (22/44 post-hit ticks, with
  not-yet-fired pins at 21/43) and no extra bumps, far neighbour survives,
  chain exhausts and shards melt within life; loop physics wiring (life/speed/
  angle universal, exact Euler + drag + spin for every non-bounced shard);
  `new_game` clears; draw batches contain the shard fans (≥ shards·9 vertices,
  centroid vs edge colors/alpha, shared fan-centroid positions).
- Ran the shrapnel integration suite 5× back-to-back: stable (the cosmetic
  rng varies shard geometry per run; the assertions are property-based, not
  seed-pinned).

## Windows-hardening notes (KNOWN CONTEXT: flaky STATUS_ACCESS_VIOLATION in
th-app's tiny_runtime)

Reviewed the shrapnel/vertex-push path for plausible crash causes; hardened
where cheap:

- **Indexing:** every index in the new code is bounds-guaranteed — fixed-size
  arrays (`[Vec2; 3]`, `[Vec2; 4]`) or loop-bounded (`i in 0..n`,
  `j in i+1..n`, `k in 0..4`, `% 4`/`% n` on a known-length perimeter,
  `chunks_exact(9)` on a length-asserted slice). No unchecked gets on
  dynamically-sized data.
- **Division:** all guarded — core-bounce `1/mag` behind `mag > 1e-4`;
  pairwise `d.x/dist` behind `dist > 1e-4`; shard spawn uses a `dir_mag < 1e-3`
  fallback (never divides the zero vector); `1/mass` is safe (mass clamped to
  ≥ 0.2); `drain(0..evict)` clamped by `evict = excess.min(len)`.
- **No recursion** anywhere in the new path; **no large stack frames**
  (largest local is a 32-byte `[Vec2; 4]`; all collections are heap `Vec`s);
  no unsafe, no FFI, no new textures/allocation-heavy paths — the draw is the
  same `FastVertexVectorTris` push the walls already use (Vec-backed, so the
  "unsafe" C++ capacity assert has no ported analogue to trip on Windows).
- The pairwise bounce copies each `Shrapnel` pair out (it's `Copy`) before
  mutating, so the double loop holds no self-referential borrows.

I did not reproduce the flaky th-app crash (it was in the tiny_runtime/wasm
shell, not the engine); nothing in the shrapnel path above is a plausible
trigger, and the hardening above removes the usual suspects from this diff.

## Residual risks

- **Visual tuning is unverified in a window:** shard speed/spin/tint/alpha
  constants are chosen in-engine but not eyeballed in the desktop/web app
  (headless-only here). A taste pass with the operator is the natural next
  step; all constants are named `pub` in `shrapnel.rs` for easy adjustment.
- **Pairwise bounce only across different walls:** same-shatter fragments skip
  each other (design choice to avoid the tiling-fragments teleport artifact).
- The burst counter (`shrapnel_burst_seq`) is not reset by `new_game`
  (monotonic u64; no observable effect — only used to distinguish shatter
  events for the pairwise skip).

## Evidence

- `.agents/evidence/09-shards/gate.log.tail` — `tail -60` of `target/gate.log`
  (all 4 stages `GATE_EXIT=0`).
- `.agents/evidence/09-shards/gate-exit.txt` — stage exit codes.
- `.agents/evidence/09-shards/unit-tests.txt` — 9/9 unit tests.
- `.agents/evidence/09-shards/integration-tests.txt` — 5/5 integration tests.
