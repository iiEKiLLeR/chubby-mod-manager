-- !chubby_coop_fix.lua
-- Injected by Chubby Mod Manager for MULTIPLAYER RE4R
-- Fixes:
-- 1. Single OS mouse cursor & reliable menu selection (paired with patched Re4-Co-op.dll)
-- 2. Live theme sync with Chubby Mod Manager (chubby_theme.json + patched Re4-Co-op.dll)
-- 3. SDK lookup cache (less stutter when the Co-op scripts scan an area)
-- 4. Smooth partner movement (frame-rate-independent exponential lerp/nlerp + velocity prediction)
-- 5. Non-blocking Pause Menu & Inventory during active Co-op sessions
-- v1.0.3: removed the disk-read cache, the 15 Hz send throttle and the enemy position
-- smoother - they slowed down enemy updates on the partner's screen.

local CFG = {
    smooth_movement   = true,
    lag_guard         = true,
    no_coop_pause     = true,
    pos_lerp_speed    = 24.0,
    rot_slerp_speed   = 20.0,
    teleport_dist_sq  = 16.0, -- 4.0m
}

local STATE = {
    theme_name        = "Obsidian Cyan",
    theme_id          = "obsidian_cyan",
    accent_hex        = 0xFFE9A50E,
    connected         = false,
    role              = "None",
    last_role_check   = 0.0,
    last_theme_check  = 0.0,
    last_frame_time   = os.clock(),
    dt                = 0.016,
    sim_wrapped       = false,
    pause_hooked      = false,
    cached_Lookups    = 0,
    smoothed_frames   = 0,
    unpaused_frames   = 0,
}

-- ============================================================================
-- 1. SDK & DISK I/O CACHE (Fixes Area / Gimmick Scan Lag)
-- ============================================================================
local td_cache = {}
local native_singleton_cache = {}
local managed_singleton_cache = {}
local managed_singleton_time = {}

if sdk then
    local orig_find_td = sdk.find_type_definition
    if orig_find_td then
        sdk.find_type_definition = function(name)
            if not CFG.lag_guard or not name then
                return orig_find_td(name)
            end
            local cached = td_cache[name]
            if cached ~= nil then
                STATE.cached_Lookups = STATE.cached_Lookups + 1
                return cached or nil
            end
            local ok, res = pcall(orig_find_td, name)
            if ok and res then
                td_cache[name] = res
                return res
            end
            td_cache[name] = false
            return nil
        end
    end

    local orig_get_native = sdk.get_native_singleton
    if orig_get_native then
        sdk.get_native_singleton = function(name)
            if not CFG.lag_guard or not name then
                return orig_get_native(name)
            end
            local cached = native_singleton_cache[name]
            if cached ~= nil then
                STATE.cached_Lookups = STATE.cached_Lookups + 1
                return cached
            end
            local ok, res = pcall(orig_get_native, name)
            if ok and res then
                native_singleton_cache[name] = res
                return res
            end
            return nil
        end
    end

    local orig_get_managed = sdk.get_managed_singleton
    if orig_get_managed then
        sdk.get_managed_singleton = function(name)
            if not CFG.lag_guard or not name then
                return orig_get_managed(name)
            end
            local now = os.clock()
            local t = managed_singleton_time[name]
            if t and (now - t) < 0.15 and managed_singleton_cache[name] ~= nil then
                STATE.cached_Lookups = STATE.cached_Lookups + 1
                return managed_singleton_cache[name]
            end
            local ok, res = pcall(orig_get_managed, name)
            if ok then
                managed_singleton_cache[name] = res
                managed_singleton_time[name] = now
                return res
            end
            return nil
        end
    end
end

-- ============================================================================
-- 2. POSE & ENEMY INTERPOLATION + BLOB / FILE I/O GUARD
-- ============================================================================
local pose_state = {
    has_pose = false,
    last_seq = -1,
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
    STATE.sim_wrapped = true

    -- v1.0.3: removed the file_read cache and the send_blob throttle that used to live here.
    -- The Co-op mod sends enemy state through these calls, so caching reads for 0.45 s and
    -- capping sends to 15 Hz whenever the game dipped under ~45 fps made enemies look
    -- choppy (10-20 fps) on the partner's screen. Data now flows at the full sync rate.

    -- Smooth partner & remote pose stream with velocity dead-reckoning + exponential lerp/nlerp
    local orig_get_pose = RE4MP_SIM.get_pose
    if type(orig_get_pose) == "function" then
        RE4MP_SIM.get_pose = function(...)
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
        end
    end
end

-- ============================================================================
-- 3. SCENE HELPER
-- v1.0.3: removed the old "enemy transform smoother". It pulled every enemy's position
-- back toward where it was last frame - on BOTH players, including the host who runs the
-- real enemy AI - and it never touched animations at all. Enemy animation smoothness now
-- comes from the Co-op mod's own sync at a higher rate (60 Hz enemy animation by default).
-- ============================================================================
local function get_current_scene()
    local scene = nil
    pcall(function()
        local sm = sdk.get_native_singleton("via.SceneManager")
        local td = sdk.find_type_definition("via.SceneManager")
        if sm and td then
            scene = sdk.call_native_func(sm, td, "get_CurrentScene")
        end
    end)
    return scene
end

-- ============================================================================
-- 4. NON-BLOCKING PAUSE MENU & INVENTORY IN CO-OP
-- ============================================================================
local function install_unpause_hooks()
    if STATE.pause_hooked or not sdk then
        return
    end
    local td_pause = sdk.find_type_definition("chainsaw.PauseManager")
    if not td_pause then
        return
    end
    STATE.pause_hooked = true

    local methods_to_guard = {
        "requestPause",
        "setPause",
        "startPause",
        "requestExecutionPause",
    }
    for _, mname in ipairs(methods_to_guard) do
        pcall(function()
            local m = td_pause:get_method(mname)
            if m then
                sdk.hook(m, function(args)
                    if CFG.no_coop_pause and STATE.connected then
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

local function keep_coop_world_unpaused()
    if not CFG.no_coop_pause or not STATE.connected then
        return
    end
    pcall(function()
        local scene = get_current_scene()
        if scene then
            local upd = scene:call("get_Update")
            if upd == false then
                scene:call("set_Update", true)
                STATE.unpaused_frames = STATE.unpaused_frames + 1
            end
            local ts = scene:call("get_TimeScale")
            if type(ts) == "number" and ts < 0.05 then
                scene:call("set_TimeScale", 1.0)
                STATE.unpaused_frames = STATE.unpaused_frames + 1
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
-- 5. ROLE & CHUBBY THEME SYNC
-- ============================================================================
local function poll_role_and_theme(now)
    if (now - STATE.last_role_check) > 0.6 then
        STATE.last_role_check = now
        pcall(function()
            if json and json.load_file then
                local r = json.load_file("RE4MP_role.json")
                if type(r) == "table" then
                    STATE.connected = (r.connected == true)
                    STATE.role = tostring(r.role or "None")
                end
            end
        end)
    end

    if (now - STATE.last_theme_check) > 2.0 then
        STATE.last_theme_check = now
        pcall(function()
            if json and json.load_file then
                local th = json.load_file("chubby_theme.json")
                if type(th) == "table" and th.name then
                    STATE.theme_name = tostring(th.name)
                    STATE.theme_id = tostring(th.id or "obsidian_cyan")
                    if type(th.accent_abgr) == "number" then
                        STATE.accent_hex = th.accent_abgr
                    end
                end
            end
        end)
    end
end

-- ============================================================================
-- 6. FRAME & UI CALLBACKS
-- ============================================================================
re.on_pre_application_entry("UpdateBehavior", function()
    local now = os.clock()
    STATE.dt = math.min(0.1, math.max(0.0005, now - STATE.last_frame_time))
    STATE.last_frame_time = now
    wrap_re4mp_sim()
    install_unpause_hooks()
    poll_role_and_theme(now)
    keep_coop_world_unpaused()
end)

re.on_frame(function()
    wrap_re4mp_sim()
end)

re.on_draw_ui(function()
    if not imgui then return end
    if imgui.tree_node("Chubby Co-op Enhancer v1.0.3 (" .. STATE.theme_name .. ")") then
        imgui.text_colored("Synced Theme: " .. STATE.theme_name .. "  |  Session: " .. (STATE.connected and ("Connected (" .. STATE.role .. ")") or "Standby"), STATE.accent_hex)
        imgui.text("Single OS Mouse Cursor Fix: Active (Re4-Co-op.dll patched)")
        imgui.text("Enemy sync: full rate (60 Hz enemy animation by default - set in the Co-op menu > Advanced Hz)")
        local c1, v1 = imgui.checkbox("Smooth Partner Movement (Lerp/Slerp)", CFG.smooth_movement)
        if c1 then CFG.smooth_movement = v1 end
        local c2, v2 = imgui.checkbox("SDK Lookup Cache (less stutter on area scans)", CFG.lag_guard)
        if c2 then CFG.lag_guard = v2 end
        local c3, v3 = imgui.checkbox("Non-Blocking Pause Menu & Inventory in Co-op", CFG.no_coop_pause)
        if c3 then CFG.no_coop_pause = v3 end
        imgui.text(string.format("Stats: %d cached lookups | %d smoothed frames | %d pause blocks", STATE.cached_Lookups, STATE.smoothed_frames, STATE.unpaused_frames))
        imgui.tree_pop()
    end
end)

log.info("[Chubby-Coop-Fix] Loaded successfully")
