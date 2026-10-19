-- tiny pack: stars — the fastest of the three (still gentle). No
-- tutorial text, no score, no replay, no Steam. The app's stage
-- multiplier owns escalation (all in-level increments are zero).

u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "utils.lua")
u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "common.lua")
u_execDependencyScript("ohvrvanilla", "base", "vittorio romeo", "commonpatterns.lua")

-- this function adds a pattern to the timeline based on a key
function addPattern(mKey)
    if mKey == 0 then pBarrageSpiral(u_rndInt(1, 2), 1, 1)
    elseif mKey == 1 then pInverseBarrage(0)
    elseif mKey == 2 then pAltBarrage(u_rndInt(2, 3), 2)
    elseif mKey == 3 then pSpiral(10, 0)
    end
end

-- shuffle the keys, and then call them to add all the patterns
-- shuffling is better than randomizing - it guarantees all the patterns will be called
keys = { 0, 1, 2, 3 }
shuffle(keys)
index = 1

-- onInit is an hardcoded function that is called when the level is first loaded
function onInit()
    l_setSides(6)
    l_setSidesMin(6)
    l_setSidesMax(6)
    l_setSpeedMult(0.70)        -- meadow 0.55; river 0.62; stars
    l_setSpeedInc(0.0)          -- the app's stage multiplier owns escalation
    l_setSpeedMax(1.0)
    l_setRotationSpeed(0.04)
    l_setRotationSpeedMax(0.25)
    l_setRotationSpeedInc(0.0)
    l_setDelayMult(1.65)        -- meadow 1.75; river 1.70; stars
    l_setDelayInc(0.0)
    l_setFastSpin(0.0)
    l_setIncTime(999999)        -- no in-level increment; the app's arc dominates
    l_setTutorialMode(false)
    l_setBeatPulseMax(10)
    l_setBeatPulseDelayMax(60.0 / 70.0 * 60.0) -- optional; tune to the synth BPM
    l_setBeatPulseSpeedMult(0.4)
end

-- onLoad is an hardcoded function that is called when the level is started/restarted
function onLoad()
    t_wait(180) -- the first pattern is delayed ~3 s (180 frames)
end

-- onStep is an hardcoded function that is called when the level timeline is empty
-- onStep should contain your pattern spawning logic
function onStep()
    addPattern(keys[index])
    index = index + 1
    if index > #keys then
        index = 1
        shuffle(keys)
    end
end

-- onIncrement is an hardcoded function that is called when the level difficulty is incremented
function onIncrement()
end

-- onUnload is an hardcoded function that is called when the level is closed/restarted
function onUnload()
end
