-- tiny pack: journey — the 09-taste continuous run.
--
-- One level for the whole ten minutes (the app never swaps levels): the
-- pattern language is chosen from the live difficulty multiplier, which
-- the runtime writes every frame (0.45 -> 2.15). The arena shape is the
-- app's stage schedule; the level only owns the walls / speed / rotation.
--
-- No tutorial text, no score, no replay, no Steam. All in-level
-- increments are zero — the app's continuous ramp owns escalation.

u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "utils.lua")
u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "common.lua")
u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "commonpatterns.lua")

-- onInit is a hardcoded function that is called when the level is first loaded
function onInit()
    l_setSides(6)
    l_setSidesMin(3)
    l_setWallSpawnDistance(1050) -- walls appear ~1 s after spawn instead of 3 s
    l_setSidesMax(8)
    l_setSpeedMult(0.40)        -- the 09 pass-2 base: the cursor is slower overall (the ramp also softened)
    l_setSpeedInc(0.0)          -- the app's continuous multiplier owns escalation
    l_setSpeedMax(2.5)
    l_setRotationSpeed(0.03)
    l_setRotationSpeedMax(0.18)
    l_setRotationSpeedInc(0.0)
    l_setDelayMult(1.05)        -- tight patterns (the old 1.65..1.75 left holes)
    l_setDelayInc(0.0)
    l_setFastSpin(0.0)
    l_setIncTime(999999)        -- no in-level increment; the app's arc dominates
    l_setTutorialMode(false)
    l_setBeatPulseMax(12)
    l_setBeatPulseDelayMax(60.0 / 70.0 * 60.0) -- re-armed by the app's beat call
    l_setBeatPulseSpeedMult(0.5)
end

-- onLoad is a hardcoded function that is called when the level is started/restarted
function onLoad()
    t_wait(30) -- half a second; the old 180 (3 s) was dead air
end

-- The pattern language escalates with the live difficulty. The rnd pick
-- keeps consecutive patterns varied; every branch stays inside the base
-- pattern library.
function addPattern()
    local d = u_getDifficultyMult()
    local r = u_rndInt(0, 99)

    if d < 0.65 then
        -- the first minute: single barrages and short spirals
        if r < 45 then pBarrage(getRandomSide(), 0)
        elseif r < 80 then pSpiral(3, 0)
        else pAltBarrage(2, 2) end
    elseif d < 1.05 then
        -- the second/third minute: spirals and the first 180-degree turns
        if r < 35 then pBarrageSpiral(1, 1, 1)
        elseif r < 65 then pAltBarrage(2, 2)
        elseif r < 85 then pSpiral(6, 0)
        else pInverseBarrage(0) end
    elseif d < 1.55 then
        -- the middle: denser spirals, alternating barrages, DM-timed bursts
        if r < 30 then pBarrageSpiral(2, 1, 1)
        elseif r < 55 then pAltBarrage(3, 2)
        elseif r < 75 then pSpiral(8, 0)
        elseif r < 90 then pInverseBarrage(1)
        else pDMBarrageSpiral(1, 1, 1) end
    else
        -- the fast ending: overlapping barrages, wide spirals, mirror turns
        if r < 25 then pBarrageSpiral(3, 1, 1)
        elseif r < 45 then pAltBarrage(4, 2)
        elseif r < 65 then pSpiral(10, 1)
        elseif r < 80 then pInverseBarrage(2)
        elseif r < 92 then pDMBarrageSpiral(2, 1, 1)
        else pMirrorSpiral(4, 0) end
    end
end

-- onStep is a hardcoded function that is called when the level timeline is empty
-- onStep should contain your pattern spawning logic
function onStep()
    addPattern()
end

-- onIncrement is a hardcoded function that is called when the level difficulty is incremented
function onIncrement()
end

-- onUnload is a hardcoded function that is called when the level is closed/restarted
function onUnload()
end
