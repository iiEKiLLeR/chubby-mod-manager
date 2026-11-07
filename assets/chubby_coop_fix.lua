-- !chubby_coop_fix.lua
-- Injected by Chubby Mod Manager for MULTIPLAYER RE4R (v1.0.17)
-- Deep Root-Cause Stability & Performance Fixes for Co-op:
-- 1. Death & Respawn Crash Fix (Both Players):
--    - Replaces broken raw-memory GameOverRequest write_qword hack in ds_trigger_gameover / ds_handle_remote_death
--      that caused immediate GC use-after-free crashes on the surviving player's game when a partner died.
--    - Never blocks PauseManager:requestExecutionPause (required by RE Engine during death, GameOver, and checkpoint reload).
--    - Automatically suspends unpause hooks & cleans up Co-op companion/puppet objects during local/remote death & scene reload.
-- 2. Character Selection & Ada/Mercenaries Crash Fix (Both P1 & P2):
--    - Disables unstable Mercenaries voice-donor prefab injection (ch3a8z0_mc_body.pfb) and dangling C++ Wwise container pointer theft
--      (cs_apply_voice, vs_tick_donor, p1_cs_apply_voice, p1_vs_tick_donor, wpn_sound_apply).
--    - Patches both data.lua's _G.CS_DISABLE_LIST and sub 1.lua's private LUIS_SWAP disable list (using metatable-safe rawget/rawset)
--      so via.motion.MotionFsm2Layer:set_MotionFsm2Resource(nil) is NEVER called (fixes re4.exe+0x192596E animation thread crash).
--    - Prevents cs_clear_loaded_banks and LUIS_SWAP:clear_loaded_banks from shrinking DynamicMotionBankCount mid-animation.
-- 3. Random Gameplay Crash Fix:
--    - Guards sdk.typeof against nil type arguments (eliminates via.Scene:findComponents System.ArgumentNullException).
--    - Disables forced 3-second dead-enemy corpse destruction (DESPAWN_DEAD_ENABLED / mplus_destroy_corpse) in logic.lua.
--    - Atomic in-memory JSON packet cache + 0-byte file guard for json.load_file / json.dump_file.

local CFG = {
    smooth_movement     = true,
    no_coop_pause       = true,
    safe_death_sync     = true,
    safe_char_swap      = true,
    safe_corpse_despawn = true,
    pos_lerp_speed      = 24.0,
    rot_slerp_speed     = 20.0,
    teleport_dist_sq    = 16.0, -- 4.0m
}

local STATE = {
    theme_name          = "Obsidian Cyan",
    theme_id            = "obsidian_cyan",
    accent_hex          = 0xFFE9A50E,
    connected           = false,
    role                = "None",
    load_time           = os.clock(),
    last_role_check     = 0.0,
    last_theme_check    = 0.0,
    last_unpause_check  = 0.0,
    last_patch_check    = 0.0,
    last_frame_time     = os.clock(),
    dt                  = 0.016,
    gameover_cooldown   = 0.0,
    sim_wrapped         = false,
    pause_hooked        = false,
    gameover_hooked     = false,
    luis_swap_patched   = false,
    logic_env_patched   = false,
    smoothed_frames     = 0,
    unpaused_frames     = 0,
    crash_guards_fired  = 0,
}

local cached_ls_inner  = nil
local cached_logic_env = nil
local recorded_reset_callbacks = {}

-- ============================================================================
-- 0. EARLY SDK, JSON & SCRIPT-RESET GUARDS (Runs before data.lua / logic.lua / sub 1.lua)
-- ============================================================================
local json_memory_cache = {}

pcall(function()
    if type(json) == "table" then
        local orig_load_file = json.load_file
        local orig_dump_file = json.dump_file

        if type(orig_dump_file) == "function" then
            json.dump_file = function(filepath, data, ...)
                if type(filepath) == "string" and type(data) == "table" then
                    json_memory_cache[filepath] = data
                end
                local ok, res = pcall(orig_dump_file, filepath, data, ...)
                if ok then return res end
                return false
            end
        end

        if type(orig_load_file) == "function" then
            json.load_file = function(filepath, ...)
                if type(filepath) == "string" then
                    local disk_path = filepath
                    if not filepath:find("^reframework[/\\]") and not filepath:find(":") then
                        disk_path = "reframework/data/" .. filepath
                    end
                    local f = io.open(disk_path, "r")
                    if not f then
                        -- File does not exist yet; return cached table or empty table for type_remaps without C++ parse error
                        if filepath == "type_remaps.json" or filepath == "type_remaps_auto.json" then
                            return {}
                        end
                        return json_memory_cache[filepath]
                    end
                    local first_bytes = f:read(4)
                    f:close()
                    if not first_bytes or #first_bytes < 2 then
                        -- 0-byte truncated file mid-write by Re4-Co-op.dll or partner script
                        STATE.crash_guards_fired = STATE.crash_guards_fired + 1
                        return json_memory_cache[filepath]
                    end
                end
                local ok, res = pcall(orig_load_file, filepath, ...)
                if ok and type(res) == "table" then
                    if type(filepath) == "string" then
                        json_memory_cache[filepath] = res
                    end
                    return res
                end
                if type(filepath) == "string" and json_memory_cache[filepath] ~= nil then
                    STATE.crash_guards_fired = STATE.crash_guards_fired + 1
                    return json_memory_cache[filepath]
                end
                return nil
            end
        end
    end
end)

pcall(function()
    if type(sdk) == "table" and type(sdk.typeof) == "function" then
        local orig_typeof = sdk.typeof
        local fallback_component_type = nil

        sdk.typeof = function(type_name)
            -- Only intercept nil / empty type_name (prevents via.Scene:findComponents System.ArgumentNullException)
            if type(type_name) ~= "string" or #type_name == 0 then
                STATE.crash_guards_fired = STATE.crash_guards_fired + 1
                if not fallback_component_type then
                    pcall(function()
                        fallback_component_type = orig_typeof("via.Component")
                    end)
                end
                return fallback_component_type
            end
            local ok, res = pcall(orig_typeof, type_name)
            if ok then
                return res
            end
            return nil
        end
    end
end)

pcall(function()
    if type(re) == "table" and type(re.on_script_reset) == "function" then
        local orig_on_script_reset = re.on_script_reset
        re.on_script_reset = function(fn, ...)
            if type(fn) == "function" then
                table.insert(recorded_reset_callbacks, fn)
            end
            return orig_on_script_reset(fn, ...)
        end
    end
end)

-- ============================================================================
-- 1. DEATH, GAMEOVER & SCENE TRANSITION STATE DETECTOR
-- ============================================================================
local function is_death_or_transition_active(now)
    now = now or os.clock()
    if now < STATE.gameover_cooldown then
        return true
    end
    if _G.ds_local_dead or _G.ds_remote_dead or _G.ds_self_triggering or _G.ds_remote_triggered then
        return true
    end
    if _G.cs_scene_unloading or _G.cs_transition_pending or _G.mp_remote_transitioning or _G.js_dbg_in_transition then
        return true
    end
    return false
end

local function safe_cleanup_coop_actors_for_reload()
    STATE.gameover_cooldown = os.clock() + 10.0
    pcall(function()
        if type(_G.cs_cleanup_for_reload) == "function" then
            _G.cs_cleanup_for_reload()
        end
    end)
    pcall(function()
        if type(cached_ls_inner) == "table" then
            local despawn_fn = rawget(cached_ls_inner, "despawn")
            if type(despawn_fn) == "function" and rawget(cached_ls_inner, "alive") then
                pcall(despawn_fn, cached_ls_inner)
            end
        end
    end)
end

-- ============================================================================
-- 2. MANAGED GAMEOVER HOOK (Delayed Until After Startup Init)
-- ============================================================================
local function install_gameover_hook(now)
    if STATE.gameover_hooked or not sdk then return end
    if (now - STATE.load_time) < 3.0 then return end

    local td_gom = sdk.find_type_definition("chainsaw.GameOverMediator")
    if not td_gom then return end
    STATE.gameover_hooked = true

    pcall(function()
        local m = td_gom:get_method("requestGameOver")
        if m then
            sdk.hook(m, function(args)
                STATE.gameover_cooldown = os.clock() + 12.0
                _G.ds_local_dead = true
                safe_cleanup_coop_actors_for_reload()
            end, function(ret)
                return ret
            end)
        end
    end)
end

-- ============================================================================
-- 3. SURGICAL PATCHES FOR DATA.LUA, LOGIC.LUA & SUB 1.LUA (Metatable-Safe)
-- ============================================================================
local function safe_disable_motfsm2_on_go(go)
    if not go then return 0 end
    local count = 0
    pcall(function()
        local comps_obj = go:call("get_Components")
        if comps_obj and comps_obj.get_elements then
            local comps = comps_obj:get_elements()
            if comps then
                for _, comp in ipairs(comps) do
                    pcall(function()
                        local td = comp:get_type_definition()
                        if td and td:get_full_name() == "via.motion.MotionFsm2" then
                            pcall(function() comp:call("set_Enabled", false) end)
                            pcall(function() comp:call("set_Paused", true) end)
                            count = count + 1
                        end
                    end)
                end
            end
        end
    end)
    return count
end

local function patch_logic_corpse_despawn()
    if not STATE.logic_env_patched and #recorded_reset_callbacks > 0 and debug and debug.getupvalue then
        for _, fn in ipairs(recorded_reset_callbacks) do
            for i = 1, 20 do
                local un, uv = debug.getupvalue(fn, i)
                if not un then break end
                if un == "t" and type(uv) == "table" then
                    local env = rawget(uv, 0)
                    if type(env) == "table" and rawget(env, "DESPAWN_DEAD_ENABLED") ~= nil then
                        cached_logic_env = env
                        STATE.logic_env_patched = true
                        break
                    end
                end
            end
            if STATE.logic_env_patched then break end
        end
    end
    if type(cached_logic_env) == "table" then
        if CFG.safe_corpse_despawn then
            if rawget(cached_logic_env, "DESPAWN_DEAD_ENABLED") == true then
                rawset(cached_logic_env, "DESPAWN_DEAD_ENABLED", false)
                STATE.crash_guards_fired = STATE.crash_guards_fired + 1
            end
            rawset(cached_logic_env, "mplus_destroy_corpse", function(...) return false end)
        else
            rawset(cached_logic_env, "DESPAWN_DEAD_ENABLED", true)
        end
    end
end

local function patch_sub1_luis_swap()
    if STATE.luis_swap_patched then return end
    if type(_G.LUIS_SWAP) ~= "table" then return end
    local tick_fn = rawget(_G.LUIS_SWAP, "tick")
    if type(tick_fn) ~= "function" or not debug or not debug.getupvalue then return end

    STATE.luis_swap_patched = true

    -- IMPORTANT: Luraph VM internal tables have an anti-tamper __index trap (while true do end).
    -- We ONLY follow upvalue 'v' (the closure value register table) and ONLY use rawget/rawset/next
    -- so no metatable (__index / __newindex / __pairs) is EVER invoked!
    local visited = {}
    local function scan_upvalue_v(val, depth)
        if depth > 10 or val == nil or visited[val] then return end
        visited[val] = true

        if type(val) == "table" then
            if rawget(val, "mode") == "strip_layers" or rawget(val, "setter") == "set_MotionFsm2Resource" then
                rawset(val, "mode", "component")
                rawset(val, "setter", nil)
                STATE.crash_guards_fired = STATE.crash_guards_fired + 1
            end
            if not cached_ls_inner and type(rawget(val, "spawn")) == "function" and type(rawget(val, "enforce")) == "function" then
                cached_ls_inner = val
            end
            for k, v in next, val, nil do
                scan_upvalue_v(v, depth + 1)
            end
        elseif type(val) == "function" then
            for i = 1, 25 do
                local un, uv = debug.getupvalue(val, i)
                if not un then break end
                if un == "v" then
                    scan_upvalue_v(uv, depth + 1)
                end
            end
        end
    end

    pcall(scan_upvalue_v, tick_fn, 0)

    if type(cached_ls_inner) == "table" then
        -- Prevent clear_loaded_banks from shrinking DynamicMotionBankCount on active P1 Motion component
        rawset(cached_ls_inner, "clear_loaded_banks", function(self)
            rawset(self, "loaded_banks", {})
            STATE.crash_guards_fired = STATE.crash_guards_fired + 1
        end)

        -- Wrap all LUIS_SWAP inner methods in pcall + transition guards using rawget/rawset
        local methods_to_wrap = {
            "spawn",
            "despawn",
            "update",
            "resolve_rooms",
            "enforce",
            "enforce_skin_materials",
            "load_ada_banks",
            "capture_p1_native_types",
            "sync_bank_types_from_p1",
            "mirror_tick",
        }
        for _, mname in ipairs(methods_to_wrap) do
            local orig_fn = rawget(cached_ls_inner, mname)
            if type(orig_fn) == "function" then
                rawset(cached_ls_inner, mname, function(self, ...)
                    if is_death_or_transition_active() and mname ~= "despawn" then
                        return
                    end
                    local ok, res = pcall(orig_fn, self, ...)
                    if not ok then
                        STATE.crash_guards_fired = STATE.crash_guards_fired + 1
                        return nil
                    end
                    return res
                end)
            end
        end
    end

    -- Also wrap _G.LUIS_SWAP.tick using rawset
    rawset(_G.LUIS_SWAP, "tick", function(...)
        if is_death_or_transition_active() then
            return
        end
        local ok = pcall(tick_fn, ...)
        if not ok then
            STATE.crash_guards_fired = STATE.crash_guards_fired + 1
        end
    end)

    log.info("[Chubby-Coop-Fix] Safely patched LUIS_SWAP & MotionFsm2 strip_layers (metatable-safe)")
end

local function patch_coop_globals()
    -- 1. Disable Mercenaries voice-donor prefab spawning & native Wwise container pointer theft
    if CFG.safe_char_swap then
        _G.diag_enable_voice_swap        = false
        _G.diag_vs_1_spawn_donor         = false
        _G.diag_vs_2_speaker_id          = false
        _G.diag_vs_3_motion_container    = false
        _G.diag_vs_4_vibration_container = false
        _G.diag_vs_5_hit_container       = false
        _G.diag_vs_6_haptics_container   = false
        _G.diag_vs_7_weapon_children     = false
        _G.diag_vs_8_body_apollo         = false
        _G.diag_vs_9_set_active_flag     = false
        _G.diag_vs_10_destroy_donor      = false
        _G.vs_hooks_bypass               = true
        _G.vs_donor_spawning             = false
        _G.vs_donor_pending_voice        = nil
        _G.p1_vs_donor_spawning          = false
        _G.p1_vs_donor_pending_voice     = nil

        local noop_voice = function(...) return false end
        _G.cs_apply_voice                = noop_voice
        _G.vs_tick_donor                 = noop_voice
        _G.vs_revert_voice               = noop_voice
        _G.p1_cs_apply_voice             = noop_voice
        _G.p1_vs_tick_donor              = noop_voice
        _G.p1_vs_revert_and_park         = noop_voice
        _G.p1_vs_on_companion_respawn    = noop_voice
        _G.wpn_sound_tick                = noop_voice
        _G.wpn_sound_apply               = noop_voice
        _G.wpn_sound_on_respawn          = noop_voice
    end

    -- 2. Disable forced dead-corpse destruction in logic.lua
    if CFG.safe_corpse_despawn then
        _G.mplus_destroy_corpse = function(...) return false end
    end
    patch_logic_corpse_despawn()

    -- 3. Patch data.lua's _G.CS_DISABLE_LIST and cs_strip_motfsm2_layers so MotionFsm2Resource is NEVER set to nil
    if type(_G.CS_DISABLE_LIST) == "table" then
        for _, entry in ipairs(_G.CS_DISABLE_LIST) do
            if type(entry) == "table" then
                if rawget(entry, "mode") == "strip_layers" or rawget(entry, "setter") == "set_MotionFsm2Resource" then
                    rawset(entry, "mode", "component")
                    rawset(entry, "setter", nil)
                    STATE.crash_guards_fired = STATE.crash_guards_fired + 1
                end
            end
        end
    end

    if type(_G.cs_strip_motfsm2_layers) == "function" and not _G.__chubby_motfsm_patched then
        _G.__chubby_motfsm_patched = true
        _G.cs_strip_motfsm2_layers = function(go)
            STATE.crash_guards_fired = STATE.crash_guards_fired + 1
            return safe_disable_motfsm2_on_go(go)
        end
    end

    -- 4. Prevent cs_clear_loaded_banks from shrinking DynamicMotionBankCount on active via.motion.Motion
    if type(_G.cs_clear_loaded_banks) == "function" and not _G.__chubby_clear_banks_patched then
        _G.__chubby_clear_banks_patched = true
        _G.cs_clear_loaded_banks = function()
            _G.cs_loaded_banks = {}
            _G.cs_active_bank_elements = {}
            STATE.crash_guards_fired = STATE.crash_guards_fired + 1
        end
    end

    -- 5. Wrap cs_apply_skin and cs_add_dynamic_motionbank in safe pcall guards
    if type(_G.cs_apply_skin) == "function" and not _G.__chubby_apply_skin_patched then
        _G.__chubby_apply_skin_patched = true
        local orig_cs_apply_skin = _G.cs_apply_skin
        _G.cs_apply_skin = function(skin_name, ...)
            if is_death_or_transition_active() then
                return false
            end
            if _G.cs_companion_go then
                safe_disable_motfsm2_on_go(_G.cs_companion_go)
            end
            local ok, res = pcall(orig_cs_apply_skin, skin_name, ...)
            if not ok then
                STATE.crash_guards_fired = STATE.crash_guards_fired + 1
                log.warn("[Chubby-Coop-Fix] Caught exception in cs_apply_skin(" .. tostring(skin_name) .. "): " .. tostring(res))
                return false
            end
            return res
        end
    end

    if type(_G.cs_add_dynamic_motionbank) == "function" and not _G.__chubby_add_bank_patched then
        _G.__chubby_add_bank_patched = true
        local orig_add_bank = _G.cs_add_dynamic_motionbank
        _G.cs_add_dynamic_motionbank = function(mot, path, ...)
            if not mot or type(path) ~= "string" or #path == 0 then
                return false
            end
            local ok, res = pcall(orig_add_bank, mot, path, ...)
            if not ok then
                STATE.crash_guards_fired = STATE.crash_guards_fired + 1
                return false
            end
            return res
        end
    end

    -- 6. Replace broken ds_trigger_gameover & ds_handle_remote_death (Fixes Simultaneous Death Crash!)
    if CFG.safe_death_sync and type(_G.ds_trigger_gameover) == "function" and not _G.__chubby_death_patched then
        _G.__chubby_death_patched = true

        _G.ds_trigger_gameover = function(reason_code)
            STATE.crash_guards_fired = STATE.crash_guards_fired + 1
            STATE.gameover_cooldown = os.clock() + 8.0
            _G.ds_remote_dead = true
            _G.ds_remote_triggered = true
            pcall(function()
                if type(_G.notif_push) == "function" then
                    _G.notif_push("Partner died - waiting for partner respawn...", "WARNING", 4.0)
                end
            end)
            safe_cleanup_coop_actors_for_reload()
            return true
        end

        _G.ds_handle_remote_death = function(sig)
            local is_dead = false
            if type(sig) == "number" then
                is_dead = (sig > 0)
            elseif type(sig) == "table" then
                is_dead = (rawget(sig, "dead") == true) or (type(rawget(sig, "seq")) == "number" and rawget(sig, "seq") > 0)
            elseif type(sig) == "boolean" then
                is_dead = sig
            end

            if is_dead then
                if not _G.ds_remote_dead then
                    _G.ds_remote_dead = true
                    _G.ds_remote_triggered = true
                    STATE.gameover_cooldown = os.clock() + 8.0
                    STATE.crash_guards_fired = STATE.crash_guards_fired + 1
                    pcall(function()
                        if type(_G.notif_push) == "function" then
                            _G.notif_push("Partner died - waiting for respawn...", "WARNING", 4.0)
                        end
                    end)
                    safe_cleanup_coop_actors_for_reload()
                end
            else
                if _G.ds_remote_dead then
                    _G.ds_remote_dead = false
                    _G.ds_remote_triggered = false
                    pcall(function()
                        if type(_G.notif_push) == "function" then
                            _G.notif_push("Partner respawned!", "SUCCESS", 3.0)
                        end
                    end)
                end
            end
        end
    end

    -- 7. Patch _G.LUIS_SWAP (sub 1.lua P1 Character Swapper) using metatable-safe rawget/rawset
    patch_sub1_luis_swap()
end

-- ============================================================================
-- 4. PARTNER POSE INTERPOLATION (Zero-Allocation Lerp/Nlerp)
-- ============================================================================
local pose_state = {
    has_pose      = false,
    last_seq      = -1,
    last_pkt_time = 0.0,
    sx = 0.0, sy = 0.0, sz = 0.0,
    sqx = 0.0, sqy = 0.0, sqz = 0.0, sqw = 1.0,
    tx = 0.0, ty = 0.0, tz = 0.0,
    tqx = 0.0, tqy = 0.0, tqz = 0.0, tqw = 1.0,
    vx = 0.0, vy = 0.0, vz = 0.0,
}

local function quat_nlerp(ax, ay, az, aw, bx, by, bz, bw, t)
    local dot = ax * bx + ay * by + az * bz + aw * bw
    if dot < 0.0 then
        bx, by, bz, bw = -bx, -by, -bz, -bw
    end
    local rx = ax + (bx - ax) * t
    local ry = ay + (by - ay) * t
    local rz = az + (bz - az) * t
    local rw = aw + (bw - aw) * t
    local mag = math.sqrt(rx * rx + ry * ry + rz * rz + rw * rw)
    if mag > 1e-6 then
        local inv = 1.0 / mag
        return rx * inv, ry * inv, rz * inv, rw * inv
    end
    return bx, by, bz, bw
end

local function wrap_re4mp_sim()
    if STATE.sim_wrapped or type(RE4MP_SIM) ~= "table" then
        return
    end
    local orig_get_pose = rawget(RE4MP_SIM, "get_pose")
    if type(orig_get_pose) ~= "function" then
        return
    end
    STATE.sim_wrapped = true

    rawset(RE4MP_SIM, "get_pose", function(...)
        local x, y, z, qx, qy, qz, qw, seq = orig_get_pose(...)
        if not CFG.smooth_movement or type(x) ~= "number" or type(y) ~= "number" or type(z) ~= "number" then
            return x, y, z, qx, qy, qz, qw, seq
        end
        local now = os.clock()
        if not pose_state.has_pose then
            pose_state.has_pose = true
            pose_state.last_seq = seq or 0
            pose_state.last_pkt_time = now
            pose_state.sx, pose_state.sy, pose_state.sz = x, y, z
            pose_state.tx, pose_state.ty, pose_state.tz = x, y, z
            pose_state.sqx, pose_state.sqy, pose_state.sqz, pose_state.sqw = qx or 0, qy or 0, qz or 0, qw or 1
            pose_state.tqx, pose_state.tqy, pose_state.tqz, pose_state.tqw = pose_state.sqx, pose_state.sqy, pose_state.sqz, pose_state.sqw
            return x, y, z, qx, qy, qz, qw, seq
        end

        if seq ~= pose_state.last_seq then
            local pkt_dt = math.max(0.008, now - pose_state.last_pkt_time)
            local dx = x - pose_state.tx
            local dy = y - pose_state.ty
            local dz = z - pose_state.tz
            local dist_sq = dx * dx + dy * dy + dz * dz
            if dist_sq > CFG.teleport_dist_sq then
                pose_state.sx, pose_state.sy, pose_state.sz = x, y, z
                pose_state.vx, pose_state.vy, pose_state.vz = 0.0, 0.0, 0.0
            elseif pkt_dt < 0.35 then
                local ivx, ivy, ivz = dx / pkt_dt, dy / pkt_dt, dz / pkt_dt
                pose_state.vx = pose_state.vx * 0.4 + ivx * 0.6
                pose_state.vy = pose_state.vy * 0.4 + ivy * 0.6
                pose_state.vz = pose_state.vz * 0.4 + ivz * 0.6
            else
                pose_state.vx, pose_state.vy, pose_state.vz = 0.0, 0.0, 0.0
            end
            pose_state.tx, pose_state.ty, pose_state.tz = x, y, z
            pose_state.tqx, pose_state.tqy, pose_state.tqz, pose_state.tqw = qx or 0, qy or 0, qz or 0, qw or 1
            pose_state.last_seq = seq
            pose_state.last_pkt_time = now
        end

        local dt = math.min(0.05, math.max(0.001, STATE.dt))
        local age = math.min(0.06, math.max(0.0, now - pose_state.last_pkt_time))
        local px = pose_state.tx + pose_state.vx * age * 0.5
        local py = pose_state.ty + pose_state.vy * age * 0.5
        local pz = pose_state.tz + pose_state.vz * age * 0.5

        local alpha_p = 1.0 - math.exp(-CFG.pos_lerp_speed * dt)
        local alpha_r = 1.0 - math.exp(-CFG.rot_slerp_speed * dt)

        pose_state.sx = pose_state.sx + (px - pose_state.sx) * alpha_p
        pose_state.sy = pose_state.sy + (py - pose_state.sy) * alpha_p
        pose_state.sz = pose_state.sz + (pz - pose_state.sz) * alpha_p
        pose_state.sqx, pose_state.sqy, pose_state.sqz, pose_state.sqw = quat_nlerp(
            pose_state.sqx, pose_state.sqy, pose_state.sqz, pose_state.sqw,
            pose_state.tqx, pose_state.tqy, pose_state.tqz, pose_state.tqw,
            alpha_r
        )
        STATE.smoothed_frames = STATE.smoothed_frames + 1
        return pose_state.sx, pose_state.sy, pose_state.sz, pose_state.sqx, pose_state.sqy, pose_state.sqz, pose_state.sqw, seq
    end)
end

-- ============================================================================
-- 5. SAFE EVENT-DRIVEN UNPAUSE HOOKS (Respects Death, GameOver & Scene Reloads)
-- ============================================================================
local function install_unpause_hooks(now)
    if STATE.pause_hooked or not sdk then
        return
    end
    if (now - STATE.load_time) < 3.0 then
        return
    end
    local td_pause = sdk.find_type_definition("chainsaw.PauseManager")
    if not td_pause then
        return
    end
    STATE.pause_hooked = true

    -- NOTE: requestExecutionPause is intentionally NOT blocked because RE Engine requires it
    -- to safely halt character/motion worker threads during death, GameOver, and checkpoint reloads.
    local methods_to_guard = {
        "requestPause",
        "setPause",
        "startPause",
    }
    for _, mname in ipairs(methods_to_guard) do
        pcall(function()
            local m = td_pause:get_method(mname)
            if m then
                sdk.hook(m, function(args)
                    if CFG.no_coop_pause and STATE.connected and not is_death_or_transition_active() then
                        STATE.unpaused_frames = STATE.unpaused_frames + 1
                        return sdk.PreHookResult.SKIP_ORIGINAL
                    end
                end, function(ret)
                    return ret
                end)
            end
        end)
    end
end

local function periodic_unpause_guard(now)
    if not CFG.no_coop_pause or not STATE.connected then
        return
    end
    if (now - STATE.last_unpause_check) < 2.0 then
        return
    end
    STATE.last_unpause_check = now
    if is_death_or_transition_active(now) then
        return
    end
    pcall(function()
        local sm = sdk.get_native_singleton("via.SceneManager")
        local td_sm = sdk.find_type_definition("via.SceneManager")
        if sm and td_sm then
            local scene = sdk.call_native_func(sm, td_sm, "get_CurrentScene")
            if scene then
                local ts = scene:call("get_TimeScale")
                if type(ts) == "number" and ts < 0.05 then
                    scene:call("set_TimeScale", 1.0)
                    STATE.unpaused_frames = STATE.unpaused_frames + 1
                end
            end
        end
        local app = sdk.get_native_singleton("via.Application")
        local td_app = sdk.find_type_definition("via.Application")
        if app and td_app then
            local gs = sdk.call_native_func(app, td_app, "get_GlobalSpeed")
            if type(gs) == "number" and gs < 0.05 then
                sdk.call_native_func(app, td_app, "set_GlobalSpeed", 1.0)
                STATE.unpaused_frames = STATE.unpaused_frames + 1
            end
        end
    end)
end

-- ============================================================================
-- 6. SAFE ROLE & THEME SYNC (No Exception Storms / No Disk Thrashing)
-- ============================================================================
local function safe_read_file(path)
    local f = io.open(path, "r")
    if not f then return nil end
    local content = f:read("*a")
    f:close()
    if content and #content >= 4 and content:sub(1, 1) == "{" then
        return content
    end
    return nil
end

local function poll_role_and_theme(now)
    if (now - STATE.last_role_check) > 5.0 then
        STATE.last_role_check = now
        local raw = safe_read_file("reframework/data/RE4MP_role.json")
        if raw and json and json.parse then
            local ok, r = pcall(json.parse, raw)
            if ok and type(r) == "table" then
                STATE.connected = (r.connected == true)
                STATE.role = tostring(r.role or "None")
            end
        end
    end

    if (now - STATE.last_theme_check) > 10.0 then
        STATE.last_theme_check = now
        local raw = safe_read_file("reframework/data/chubby_theme.json")
        if raw and json and json.parse then
            local ok, th = pcall(json.parse, raw)
            if ok and type(th) == "table" and th.name then
                STATE.theme_name = tostring(th.name)
                STATE.theme_id = tostring(th.id or "obsidian_cyan")
                if type(th.accent_abgr) == "number" then
                    STATE.accent_hex = th.accent_abgr
                end
            end
        end
    end
end

pcall(function()
    local raw = safe_read_file("reframework/data/chubby_theme.json")
    if raw and json and json.parse then
        local ok, th = pcall(json.parse, raw)
        if ok and type(th) == "table" and th.name then
            STATE.theme_name = tostring(th.name)
            STATE.theme_id = tostring(th.id or "obsidian_cyan")
            if type(th.accent_abgr) == "number" then
                STATE.accent_hex = th.accent_abgr
            end
        end
    end
end)

-- ============================================================================
-- 7. ULTRA-LEAN FRAME HOOKS
-- ============================================================================
re.on_pre_application_entry("UpdateBehavior", function()
    local now = os.clock()
    STATE.dt = math.min(0.1, math.max(0.0005, now - STATE.last_frame_time))
    STATE.last_frame_time = now

    if not STATE.sim_wrapped then
        wrap_re4mp_sim()
    end
    if not STATE.pause_hooked then
        install_unpause_hooks(now)
    end
    if not STATE.gameover_hooked then
        install_gameover_hook(now)
    end
    if (now - STATE.last_patch_check) > 0.25 then
        STATE.last_patch_check = now
        patch_coop_globals()
    end

    poll_role_and_theme(now)
    periodic_unpause_guard(now)
end)

re.on_frame(function()
    if not STATE.sim_wrapped then
        wrap_re4mp_sim()
    end
    if not STATE.luis_swap_patched or not STATE.logic_env_patched then
        patch_coop_globals()
    end
end)

re.on_draw_ui(function()
    if not imgui then return end
    if imgui.tree_node("Chubby Co-op Enhancer v1.0.17 (" .. STATE.theme_name .. ")") then
        imgui.text_colored("Synced Theme: " .. STATE.theme_name .. "  |  Session: " .. (STATE.connected and ("Connected (" .. STATE.role .. ")") or "Standby"), STATE.accent_hex)
        imgui.text("Single OS Mouse Cursor Fix: Active (Re4-Co-op.dll patched)")
        local c1, v1 = imgui.checkbox("Smooth Partner Movement (Lerp/Slerp)", CFG.smooth_movement)
        if c1 then CFG.smooth_movement = v1 end
        local c2, v2 = imgui.checkbox("Safe Co-op Pause Menu (Auto-Yields on Death/Reload)", CFG.no_coop_pause)
        if c2 then CFG.no_coop_pause = v2 end
        local c3, v3 = imgui.checkbox("Death & Respawn Crash Protection (Both Players)", CFG.safe_death_sync)
        if c3 then CFG.safe_death_sync = v3 end
        local c4, v4 = imgui.checkbox("Character Switch Crash Protection (Ada / Mercenaries)", CFG.safe_char_swap)
        if c4 then CFG.safe_char_swap = v4; patch_coop_globals() end
        local c5, v5 = imgui.checkbox("Disable Forced Dead-Enemy Corpse Destruction", CFG.safe_corpse_despawn)
        if c5 then CFG.safe_corpse_despawn = v5; patch_logic_corpse_despawn() end
        imgui.text(string.format("Stats: %d smoothed frames | %d pause blocks | %d crash guards triggered", STATE.smoothed_frames, STATE.unpaused_frames, STATE.crash_guards_fired))
        imgui.tree_pop()
    end
end)

log.info("[Chubby-Coop-Fix] v1.0.17 stability & crash-protection suite loaded")
