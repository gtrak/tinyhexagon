# Issue 09-taste — journey feel pass (operator-directed)

**Status:** IN PROGRESS (main worktree; the operator asked for this directly,
not via a worker).

## Feedback being addressed

1. Music is escalating drones → rework into a generated score.
2. Level transitions are abrupt with too much empty space between walls →
   one continuous journey level, no mid-run `new_game`, denser patterns.
3. Difficulty does not visibly increase → continuous 0.45..2.15 ramp,
   slower start, faster end.
4. Always a hexagon → stage shape schedule (3..8 sides).
5. Colors/effects should get trippier → dynamic hue style, continuous hue
   drift, stronger pulse, synthetic beat energy on the fallback clock.

## Design

- **Journey**: `journey` level id (single level), 20×30 s, difficulty
  `0.45 + 0.085*t` where `t = stage + elapsed/30` (capped 20), BPM
  `70 + 2.5*t` (cap 120), hue `100 + 28*t` (mod 360), shape schedule
  `[6,6,4,4,3,3,5,5,6,6,7,7,4,4,8,8,5,5,6,6,6]`.
- **Engine use**: no new engine API — `set_difficulty_mult` every frame,
  `set_sides` at stage marks, `apply_music_beat` every frame.
- **Level**: `journey.lua` picks patterns from the live
  `u_getDifficultyMult()`; `onLoad` wait 30 frames instead of 180; the
  pattern library's trailing waits shortened.
- **Music**: bar-wise chord progression (major I–vi–IV–V, minor
  i–VI–III–VII from stage 8), chord-following pad/drone/bass/kick,
  key-root arpeggios on pluck/bell/shimmer/lead, evolving kick pattern,
  LFO movement, root shift every 3 stages, layer unlocks earlier.
- **Style**: new `journey` style, dynamic main/player/wall/background
  colors, hue 0..360, 3 background hues, pulse max 1.4.
- **Tests**: journey table, music chord/arp/pattern, runtime 60 s (shape,
  single level), tiny pack (journey), host gate; desktop screenshot at
  stage 2 via `--tiny-shot`.
