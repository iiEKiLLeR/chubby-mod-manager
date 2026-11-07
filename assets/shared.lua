-- shared.lua (Chubby Co-op Safe FSM Suppressor)
-- Safely disables MotionFsm2 component on Player 2 without nulling native MotionFsm2Resource pointers
-- (prevents re4.exe+0x192596E null-pointer dereference crash on character switch / death / reload).

local COMPANION_NAME = "Player 2"
local CHECK_INTERVAL = 2.5

local last_check_time = 0
local last_fixed_go   = nil

local function is_transition_or_dead()
    if _G.ds_local_dead or _G.ds_remote_dead or _G.ds_self_triggering or _G.ds_remote_triggered then
        return true
    end
    if _G.cs_scene_unloading or _G.cs_transition_pending or _G.mp_remote_transitioning then
        return true
    end
    return false
end

local function get_scene()
    local s
    pcall(function()
        s = sdk.call_native_func(
            sdk.get_native_singleton("via.SceneManager"),
            sdk.find_type_definition("via.SceneManager"),
            "get_CurrentScene"
        )
    end)
    return s
end

local function apply_fix(go)
    local count = 0
    pcall(function()
        local comps_obj = go:call("get_Components")
        if not comps_obj or not comps_obj.get_elements then return end
        local comps = comps_obj:get_elements()
        if not comps then return end
        for _, comp in ipairs(comps) do
            pcall(function()
                local td = comp:get_type_definition()
                if not (td and td:get_full_name() == "via.motion.MotionFsm2") then return end
                pcall(function() comp:call("set_Enabled", false) end)
                pcall(function() comp:call("set_Paused", true) end)
                count = count + 1
            end)
        end
    end)
    return count > 0
end

re.on_frame(function()
    local now = os.clock()
    if (now - last_check_time) < CHECK_INTERVAL then return end
    last_check_time = now

    if is_transition_or_dead() then
        last_fixed_go = nil
        return
    end

    local scene = get_scene()
    if not scene then return end

    local ok, go = pcall(function()
        return scene:call("findGameObject(System.String)", COMPANION_NAME)
    end)
    if not ok or not go then
        last_fixed_go = nil
        return
    end

    if go == last_fixed_go then return end

    if apply_fix(go) then
        last_fixed_go = go
        log.info("[P2-FSM-SUPPRESS] Safely disabled MotionFsm2 on " .. COMPANION_NAME)
    end
end)

log.info("[P2-FSM-SUPPRESS] Loaded (Chubby Crash-Safe Mode)")
