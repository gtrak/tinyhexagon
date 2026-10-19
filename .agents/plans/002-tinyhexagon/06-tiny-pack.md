# Issue 06 — Tiny content pack

**Status:** PROPOSED
**Depends:** 03

## Objective

Add three deliberately gentle levels and three styles under
`_RELEASE/Packs/tiny/`. They use the existing Luau level API and the `base`
pack's pattern helpers. No shaders, no music ids, no scores, no tutorial text.

## Pack layout

```
_RELEASE/Packs/tiny/
  pack.json
  Levels/meadow.json
  Levels/river.json
  Levels/stars.json
  Scripts/Levels/meadow.lua
  Scripts/Levels/river.lua
  Scripts/Levels/stars.lua
  Styles/meadow.json
  Styles/river.json
  Styles/stars.json
```

## pack.json

- `disambiguator`: `tinyhexagon`
- `name`: `tiny`
- `author`: `tinyhexagon`
- `version`: 1
- `priority`: a value that loads after `base` (the exact priority is not
  important; keep the dependency).
- `dependencies`: `base` (`ohvrvanilla`, `vittorio romeo`, min_version 1).

## Levels

Each level JSON:

```json
{
  "id": "meadow",
  "name": "meadow",
  "description": "",
  "author": "tinyhexagon",
  "menuPriority": 0,
  "selectable": true,
  "styleId": "meadow",
  "musicId": "nullMusicId",
  "luaFile": "Scripts/Levels/meadow.lua",
  "unscored": true
}
```

- `unscored: true` avoids the score/validator/profile path (which is being
  removed). Do not add `difficultyMults`; the app passes its own multiplier.
- No `musicId` that resolves to an OGG; the synth owns the music.
- The level scripts use the `base` helpers via
  `u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "...")`.

## Script skeleton (per level)

Model on `_RELEASE/Packs/tutorial/Scripts/Levels/babysteps.lua`, but without
the tutorial steps/text.

```lua
u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "utils.lua")
u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "common.lua")
u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "commonpatterns.lua")

function addPattern(mKey)
    if mKey == 0 then pBarrageSpiral(u_rndInt(1, 2), 1, 1)
    elseif mKey == 1 then pInverseBarrage(0)
    elseif mKey == 2 then pAltBarrage(u_rndInt(2, 3), 2)
    elseif mKey == 3 then pSpiral(10, 0)
    end
end

keys = { 0, 1, 2, 3 }
shuffle(keys)
index = 1

function onInit()
    l_setSides(6)
    l_setSidesMin(6)
    l_setSidesMax(6)
    l_setSpeedMult(0.55)        -- meadow; river 0.62; stars 0.70
    l_setSpeedInc(0.0)          -- the app's stage multiplier owns escalation
    l_setSpeedMax(1.0)
    l_setRotationSpeed(0.04)
    l_setRotationSpeedMax(0.25)
    l_setRotationSpeedInc(0.0)
    l_setDelayMult(1.75)        -- river 1.70; stars 1.65
    l_setDelayInc(0.0)
    l_setFastSpin(0.0)
    l_setIncTime(999999)        -- no in-level increment; the app's arc dominates
    l_setTutorialMode(false)
    l_setBeatPulseMax(10)
    l_setBeatPulseDelayMax(60.0 / 70.0 * 60.0) -- optional; tune to the synth BPM
    l_setBeatPulseSpeedMult(0.4)
end

function onLoad() end

function onStep()
    addPattern(keys[index])
    index = index + 1
    if index > #keys then
        index = 1
        shuffle(keys)
    end
end

function onIncrement() end
function onUnload() end
```

- `l_setSpeedMult` / `l_setDelayMult` / rotation are the per-level flavour;
  the stage multiplier (0.55 → 1.15) scales the speed.
- Keep the first pattern delayed ~3 s: either add a `t_wait(180)` at the start
  of the timeline in `onLoad`, or make the first `onStep` wait. Verify the
  timeline starts with an empty gap.
- No `u_setLastReplay`, score, Steam, or swap-dependent patterns.
- `pSpiral`/`pBarrageSpiral` arguments are starting points; verify on device.

## Styles

Each style JSON:

```json
{
  "id": "meadow",
  "hue_min": 100,
  "hue_max": 160,
  "hue_increment": 0.5,
  "hue_ping_pong": false,
  "pulse_min": 0.0,
  "pulse_max": 1.2,
  "pulse_increment": 0.02,
  "3D_depth": 0,
  "3D_skew": 0.0,
  "3D_spacing": 0.0,
  "main": { "main": true, "dynamic": false, "value": [255,255,255,255], "pulse": [0,0,0,0] },
  "cap_color": { "legacy": true, "index": 0 },
  "colors": [
    { "dynamic": false, "dynamic_offset": false, "dynamic_darkness": 1.0, "value": [60,80,60,255], "pulse": [20,30,20,0] },
    { "dynamic": false, "dynamic_offset": false, "dynamic_darkness": 1.0, "value": [40,55,45,255], "pulse": [10,15,10,0] }
  ]
}
```

- Palette families: meadow ~100–160 (green), river ~190–250 (blue),
  stars ~270–330 (violet).
- The engine's beat override sets the hue; the style's `hue_min` is the base
  family. Keep the colours bright enough for a child on a phone.
- `3D_*` are zero/off; the kid app also disables 3D in its config.
- No `Shaders/` directory and no shader references.

## Files

| File | Change |
|---|---|
| `_RELEASE/Packs/tiny/**` | New pack. |
| `th-assets`/`th-app` test | Loads the pack and runs the levels headless. |

## Steps

1. Copy the `tutorial` pack's pack.json shape and dependency style.
2. Write the three level JSONs and scripts; start from `babysteps.lua` and
   strip the tutorial logic.
3. Write the three style JSONs.
4. Add a test that loads `tiny` via `Assets`, finds all three levels/styles,
   and runs each level headless for 30 s (no Lua error, `has_died` false with
   soft collisions).
5. Check the `base` helper calls are still available after the engine de-scope
   (no removed replay/score/Steam globals).
6. Gate + forked reviewer.

## Verification

- `Assets` loads `tiny` and all three levels/styles; no load errors.
- Each level runs 30 s headless with no Lua error; the first walls appear
  after ~3 s.
- The engine reports six sides and the authored speed/delay.
- The style `hue_min` values match the palette table.
- No shader/music/replay/score paths are referenced by the pack.

## Risks

- Lua helper semantics (delays, pattern arguments) may need tuning; the issue
  is about content loading, not perfect level design (Issue 09 tunes).
- `l_setIncTime`/`l_setSpeedInc` semantics must be verified; if the engine
  still auto-increments, set the increments to zero and keep the app's
  multiplier as the only escalation.
- The `base` helpers may call removed Lua globals; in that case add a no-op
  registration or adjust the script (do not restore replay/Steam).

## Feed-forward

Record the final level ids, speed/delay values, the first-pattern delay, and
any Lua globals that had to be no-oped. Issue 07's journey table and Issue
09's tuning use these values.
