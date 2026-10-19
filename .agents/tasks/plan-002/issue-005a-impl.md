# Issue 05a — `th-synth` host spike + the alsa-sys coexistence patch

**Worktree:** `/home/gary/dev/tinyhexagon/.agents/worktrees/slot-1` (branch
`slot-1`, base `9130c969`). The main worktree and slot-2 are OFF-LIMITS.
**Contract:** `.agents/plans/002-tinyhexagon/05-synth.md` (read it).
**Scope:** HOST-ONLY spike + the host `MusicDirector`; Android/wasm checks
are **05b**, later. Do not commit; the orchestrator lands one squashed commit.

## The real blocker (already found; do not rediscover)

`rust-synth 0.23` depends on `cpal 0.15.3` → `alsa 0.9` → `alsa-sys ^0.3.1`.
`th-render`/`th-audio` (rodio 0.23) already pull `cpal 0.18` → `alsa 0.11`
→ `alsa-sys 0.4.0`. Both `alsa-sys` crates declare `links = "alsa"`, so Cargo
refuses the graph. The sanctioned fix (plan 05's vendor path):

1. Copy the cached `alsa-sys 0.3.1` source into `vendor/alsa-sys-0.3.1/`
   (the registry cache has it; include its LICENSE).
2. In that vendor `Cargo.toml`, change `links = "alsa"` to
   `links = "alsa_0_3"` (the only change needed for the `links` conflict).
3. Add to the root `Cargo.toml`:
   ```toml
   [patch.crates-io]
   alsa-sys = { path = "vendor/alsa-sys-0.3.1", version = "0.3.1" }
   ```
4. Verify `cargo check -p th-synth` resolves the two `alsa-sys` versions
   (0.3.1 patched + 0.4.0 registry) and builds. If a second conflict appears,
   report it before considering a different approach; do not remove
   `rodio`/`th-audio` or silently drop music.

## Then (host only)

- Implement `th-synth` per the contract: 8 tracks, `MusicDirector`,
  `BeatView`, stage map, duck/recover, fallback clock, `start_on_gesture`
  cfg-gated. No Android/wasm target checks in 05a.
- Host tests: stage map monotonic/capped; beat/bar phases; duck/recover;
  `start()` without a device does not panic and `tick` advances the fallback;
  always 8 tracks.
- Write `.agents/skills/lib-rust-synth.md` and `lib-cpal.md` from the real
  crate source (host sections; the cross-target host/oboe notes marked
  `TODO(05b)`), index them in `.agents/skills/README.md`.
- No app/runtime wiring (07), no SFX, no graph rebuild on stage changes.

## Verification

```sh
cargo check -p th-synth
cargo test -p th-synth
bash tools/gate.sh
```

The gate (whole workspace) proves the two `alsa-sys` crates coexist with
`rodio`/`th-render`. A short desktop listen is optional; record the fallback
path if no device.

## Report / feed-forward

Write `.agents/tasks/plan-002/issue-005a-report.md` (patch path, the resolved
lock versions, director API, tests/gate, no commit). Update
`.agents/plans/002-tinyhexagon/FEED-FORWARD.md` **§05a only**; hand off the
05b target checks. No commit.
