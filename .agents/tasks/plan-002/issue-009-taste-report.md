# Issue 09-taste — report (operator-directed feel pass)

**Status: LANDED (operator taste pass; final 09 tuning/docs/playtests still
open).** Implemented directly in the main worktree at the operator's
request ("better suited to you instead of the subagent"), not via a worker.

## What changed

### 1. One continuous journey level (no mid-run `new_game`)

The old runtime called `new_game` at every even stage: that cleared every
wall, replayed the level's `t_wait(180)` (≈3 s of empty arena) and re-read
the level — the abrupt transitions the operator saw. Now the run is a
single level (`{pack}_journey`) for the whole ten minutes:

- `tiny.rs::stage_up` no longer calls `new_game`; it changes the arena
  shape (below), the music stage, the flash and the `levelUp.ogg` cue.
- `journey.lua` selects its pattern language from the live
  `u_getDifficultyMult()` (the runtime writes the continuous multiplier
  every frame), so the walls evolve instead of swapping level scripts.
- The level sets `l_setWallSpawnDistance(1050)` (walls visible after ~1 s
  instead of ~3 s off-screen), `l_setDelayMult(1.05)` (the old 1.65–1.75
  left holes) and `t_wait(30)` on load (was 180).
- The shared base pattern library's trailing waits were shortened
  (`pBarrage` 6.5→2.0, `pSpiral` 6.5→2.0, `pBarrageSpiral` 6.1→1.6,
  `pDMBarrageSpiral` 6.7→2.2, `pInverseBarrage` 9.9→5.0 / trailing
  2.5→1.2, `pAltBarrage` trailing →0.35×). This also densifies the three
  06 reference levels, which are no longer played.

### 2. Continuous, much wider difficulty ramp

`journey.rs`: `difficulty = 0.45 + 0.085 · t`, `t = stage +
elapsed/30` capped at 20 → **0.45..2.15** (was 0.55..1.15). The runtime
writes `set_difficulty_mult(journey.difficulty())` **every frame** before
the engine ticks, so the wall speed/delay ramp smoothly within a stage.
The wall speed is `speed_mult · difficulty^0.65` with the level's
`l_setSpeedMult(0.55)` → ~98 px/s at the start to ~264 px/s at the
plateau (≈2.7×; the old arc was ~1.6×). BPM `70 + 2.5·t` → 120 (was
100).

### 3. Shape tour (not always a hexagon)

`sides_for_stage`: `[6,6,4,4,3,3,5,5,6,6,7,7,4,4,8,8,5,5,6,6,6]` — a
two-stage dwell through 3..8 sides, back to the hexagon at the plateau.
The runtime calls the existing `set_sides` at even stages; existing walls
keep their geometry, so the arena warps around the wall stream instead of
clearing it. `reset_stage` restores the current stage's shape.

### 4. Generative score instead of eight drones

`th-synth/director.rs` keeps the 8 fixed rust-synth tracks but gives them
a score:

- **Bar-wise chord progression**: pad/drone/bass/kick follow
  I–vi–IV–V (major) and i–VI–III–VII (minor, stages 8..16); the melody
  voices stay on the key root.
- **Arpeggios**: pluck/bell/shimmer/lead run the rust-synth `arp` at
  depth 1 (in-scale pentatonic walks every two beats).
- **Evolving kick**: 2 → 4 → 5 → 7 → 9 hits per 16 steps
  (`euclidean_bits`), with a click character that grows.
- **Movement**: slow cutoff/frequency LFOs on the pads/leads, pulse
  depths on the pad/drone, the root shift every 3 stages, and earlier
  layer unlocks (pluck 0, bell 2, bass 3, kick 4, shimmer 7, saw 11).
- The web fallback clock still produces a valid `BeatView`; the visual
  pulse no longer depends on live audio (see below).

### 5. Trippier, dynamic color and pulse

The 06 styles were static (`dynamic: false`), so the 04b hue was inert.
The new `journey` style:

- `hue_min 0 / hue_max 360`, dynamic `main` (arena), **white player**
  (contrast), dynamic complementary `wall_color`, three dark dynamic
  background chunks (hue offsets 0/40/80, `dynamic_darkness` 2.0–3.0),
  `pulse_max 1.4`.
- The app's base hue is continuous: `(100 + 28·t) mod 360` — a rainbow
  cycle roughly every 13 stages.
- The beat shimmer widened from 1.5° to 6°.
- `apply_music_beat` receives a synthesized pulse
  (`0.7·live_energy + 0.45·(1−beat_phase)^2.5`, clamp 1.4), so the colors
  and central polygon pulse on the web fallback clock and in quiet
  passages too.

## Verification

- `bash tools/gate.sh`: **572 passed, 0 failed** (fmt, build, clippy -D
  warnings, tests). New/updated tests: journey continuous ramp + hue +
  shape schedule, music chord/arp/kick-pattern, runtime 65 s (journey
  level, square at stage 2, no death) + 10-minute plateau at 2.15/120,
  tiny pack's fourth level, release-tree/data counts (+1 level, +1
  style), beat shimmer 6°.
- Desktop screenshot at stage 2 (`xvfb-run … -- --tiny-shot=65`):
  `.agents/evidence/09/stage2.png` — dark hue-washed background, red
  complementary walls, cyan square arena, white player.
- The runtime still writes no config/profiles/replays (the existing
  no-persistence test passes).

## P2 / residual (feed-forward)

1. **Listen + playtest** — the arpeggio is rust-synth's random pentatonic
   walk (seeded per track); it may want a fixed motif or a different
   mix/arp depth by ear. The lead/supersaw is fairly bright at the late
   root shifts; tune gains by ear.
2. **Shape warp** — the stage shape change happens with old walls on
   screen; it reads as a warp (the flash masks most of it). If it's too
   jarring, gate the change on a wall-empty frame or fade the flash.
3. **First-wall wait** — the 0.45 start + 1050 spawn distance gives the
   first wall ~1 s after spawn (visible while it travels in). The helper
   hand only engages after 2 s idle; playtest the first 10 s with a child.
4. **Base pattern library is global** — the shortened trailing waits also
   apply to the three reference levels (not played). If those are kept
   as content later, give the journey its own pattern wrappers instead.
5. **Live web** — the deployed page updates on the next push/CI; verify
   the new page's audio (the first-gesture unlock) and the fallback pulse.
6. 08b/08c are still separate landings; this pass does not touch Android
   or packaging.

## Taste pass 2 (09-taste-2) — original styles + chiptune DnB + wasm audio

Commit 6780a935. Operator feedback: the invented dynamic hue looked wonky,
the ambient score didn't read as music, and the web page was silent.

- **Arena = the original upstream levels in order**: 10 blocks x 2 stages
  (pointless, flatteringshape, seconddimension, apeirogon, commando(4),
  euclideanpc(8), pi(24), labyrinth(3), goldenratio, hypercube
  acceleradiant). The app swaps the real upstream `StyleData` live and
  `set_sides`; the style's own hue/pulse wins (the app's beat-hue override
  is removed; `apply_music_pulse` only bumps the central polygon).
  Desktop evidence: stage2 shows upstream flatteringshape (green/gray
  bevel) at 6 sides — `.agents/evidence/09/stage2-flatteringshape.png`.
- **Music = step-sequenced chiptune drum-and-bass**: 3x Heartbeat drums
  (kick/snare/hat 16-step bitmasks that densify), BassPulse syncopated
  line over the bar chord, PluckSaw A-minor motif, pad/bell/sub on the
  chord (I-vi-IV-V, i-VI-III-VII dark half), BPM 150->174.
- **Wasm audio verified fixed**: gesture-time start (`resumeAudio` ->
  `MusicDirector::start_on_gesture` inside the activation, thread_local
  `Rc<RefCell<MusicDirector>>`) instead of the flag-consumed-next-rAF
  path. Fresh emsdk build + probe on a REAL page:
  `audio=live;energy=0.0403` (the synth graph produces output), th-audio
  unlocked on first input, no console errors. Evidence:
  `.agents/evidence/09/web-stage1-audio-live.png`, `web-status-line.txt`.
  (The old web-probe.mjs's "booted NO / user activation no" lines parse
  the ADULT status format — the new `tools/web-tiny-probe.mjs` from 08c
  replaces it; the Module.status read above is authoritative.)

## Taste pass 3 (09 fix pass 2, shrapnel) — mix, speed, chain shatter

Operator feedback after playing the deployed build: desktop music audible
but faint + overly reverby; web silent (SFX audible); cursor still fast;
and the walls should EXPLODE.

- **Mix**: master 0.55 → 0.9; drum/bass/lead gains up (kick 0.78, snare
  0.52, hat 0.26, bass 0.68, lead 0.40); reverb mixes cut hard (drums
  0.05, bass 0.08, lead 0.20, pad/bell 0.28, sub 0.15) — the reverb
  smear is what made it both faint and mushy.
- **Slower cursor**: speedMult 0.40 → 0.33, the ramp softened again
  (0.055/stage; plateau 1.55).
- **Web unlock hardened**: the shell calls `resumeAudio()` on EVERY
  gesture until the status says `audio=live` — a first gesture whose
  activation check failed used to lock the page on the silent fallback
  forever (the SFX device unlocks through emscripten's own listeners,
  which is exactly why SFX could work while the synth stayed silent).
- **Collision shrapnel (engine)**: a soft hit now SHATTERS the wall —
  12 spinning wedge pieces spawn at the impact point (the wall vertex
  nearest the player), fly outward with the cosmetic rng, and fade over
  ~0.8 s (48 FT units); the pieces draw into the wall vertex batch
  (one draw call, the wall's own color). The blast chains into the TWO
  nearest neighbour walls ~90 ms and ~180 ms later (a plain
  mid-angle + delay queue — the victim is re-looked-up at fire time, so
  the wall `retain` cannot invalidate it; chains do not re-chain).
  ONE bump per physical hit (the chain walls die "for free" — no
  extra slow-motion/duck). Engine tests: `th-game/tests/shrapnel.rs`
  (5 tests) + the shrapnel.rs unit tests; gate 579/0.
