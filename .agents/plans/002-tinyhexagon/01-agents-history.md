# Issue 01 — `.agents` history summary/archive; retire C++ and parity tooling

**Status:** PROPOSED
**Depends:** 00

## Objective

Make the engineering record useful for the new direction without deleting it.
Summarize the port's history into a direction-oriented document, archive the
old detailed record, keep the library skills, and remove the C++ reference
tree and parity tooling that the fork no longer uses.

## Key decisions

- Summarize, don't delete. The old record moves to
  `.agents/plans/archive/001-rust-port/`; git history remains the full record.
- `.agents/skills/` stays authoritative. Keep the existing skills; the
  upstream skill is added by Issue 00; rust-synth/cpal skills are added by
  Issue 05 after the cross-target spike.
- Parity tooling is retired: no parity stage in the gate, no corpus consumer
  in the workspace.
- `_RELEASE/` and `LICENSE` stay.

## Files

| File / path | Change |
|---|---|
| `.agents/HISTORY.md` | New summary. |
| `.agents/plans/archive/001-rust-port/` | Move the old plan, decisions, mapping, audits, evidence and parity corpus here. |
| `.agents/skills/` | Keep; update the index if needed. |
| `cpp-original/`, `src/`, `include/`, `.gitmodules` | Delete (submodules deinit first). |
| `crates/oh-parity/` | Delete; remove from workspace members. |
| `tools/cpp-oracle/`, `tools/oracle/`, `tools/parity.sh`, `tools/lua-parity.sh` | Delete. |
| `tools/check-additions-boundary.sh`, `tools/additions-boundary-baseline.txt` | Delete. |
| `tools/lane-worktree.sh`, `tools/infra/`, `tools/k3-luajit-hang/` | Delete. |
| `docs/` | Delete (`music-content.md` is superseded by generated music). |
| `tools/gate.sh` | Four stages: fmt / build / clippy / test; keep the failing-stage tail and log. |
| `README.md` | Short placeholder; expanded in Issue 09. |

## Steps

1. `git submodule deinit -f --all`; remove the C++ tree and `.gitmodules`;
   remove any `.git/modules` state; remove the `src`/`include` symlinks.
2. Move `001-rust-port` to `.agents/plans/archive/001-rust-port/` (plan,
   decisions, mapping, parity, audits, evidence). Do not edit the archived
   files; they are history.
3. Write `.agents/HISTORY.md` (~1–2 pages):
   - The 1:1 port and what it proved: parity, the C++ oracle, RNG/replay
     byte-exactness, the Lua parity corpus, the render stack, Android device
     verification, the emscripten build, packaging.
   - What carries forward: engine simulation, Lua content, `th-render`,
     winit/glutin/glow + cpal, Android/emscripten entries, packaging/CI.
   - What is retired: C++ tree and oracle, parity corpus/tooling, adult
     menu/online/Steam/Discord/workshop/server/replay, replay codec,
     score/profile system, additions ratchet.
   - The tinyhexagon direction, the engine alterations (link to plan 002),
     the upstream relationship (link to the upstream skill).
   - Pointers: git history, the archived plan, the active plan.
4. Delete the parity/lane/additions tooling listed above.
5. Rewrite `tools/gate.sh` to the four stages. Keep the log path, per-stage
   `GATE_EXIT`, and the failing-stage tail (last 200 lines). No network, no
   parity, no ratchet.
6. Remove `oh-parity` from workspace members and delete the crate.
7. Keep `.agents/skills/`; ensure `.agents/skills/README.md` links the
   upstream skill (Issue 00) and notes that rust-synth/cpal skills follow.
8. Replace `README.md` with a short tinyhexagon placeholder (one paragraph,
   build command, pointer to the plan). Issue 09 writes the final README.

## Verification

- `cargo build --workspace`, `cargo clippy --workspace --all-targets -D
  warnings`, `cargo test --workspace` green.
- No test reads a deleted path; `rg '\.agents/plans/001-rust-port/parity'
  crates` returns nothing.
- `git submodule status` clean; `cpp-original/` absent; the root symlinks
  absent.
- `bash tools/gate.sh` runs exactly four stages and exits 0.
- `.agents/HISTORY.md` exists and links the archived plan, the active plan,
  and the upstream skill.

## Risks

- Some tests may reference `_RELEASE` (keep it) or the deleted corpus. The
  parity crate is the only corpus consumer; remove it first.
- Submodule deletion can leave `.git/modules` state; clean it deliberately.
- Do not mass-delete comments citing the C++ in `crates/`; they are history,
  not build inputs. Only the tree/tooling/corpus go.

## Feed-forward

Record in `FEED-FORWARD.md`: the final gate stages, any test files removed
with the parity crate, and the exact archive path. Later issues reference the
archive for port history.
