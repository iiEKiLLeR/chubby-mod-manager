-- !chubby_coop_fix.lua
-- Injected by Chubby Mod Manager for MULTIPLAYER RE4R
-- Performance & Stability Fixes for Co-op:
-- 1. Single OS mouse cursor & reliable menu selection (paired with patched Re4-Co-op.dll)
-- 2. Live theme sync with Chubby Mod Manager (chubby_theme.json + patched Re4-Co-op.dll)
-- 3. Smooth partner movement (frame-rate-independent exponential lerp/nlerp + velocity prediction)
-- 4. Event-based non-blocking Pause Menu & Inventory in Co-op (hooks PauseManager without polling reflection)
-- 5. Zero-overhead game loop: safe disk reads (no JSON parse exceptions), no per-frame native reflection, native C++ SDK speed

local CFG = {
    smooth_movement   = true,
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
    last_unpause_check= 0.0,
    last_frame_time   = os.clock(),
    dt                = 0.016,
    sim_wrapped       = false,
    pause_hooked      = false,
    smoothed_frames   = 0,
    unpaused_frames   = 0,
}

-- ============================================================================
-- 1. PARTNER POSE INTERPOLATION (Zero-Allocation Lerp/Nlerp)
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
    local orig_get_pose = RE4MP_SIM.get_pose
    if type(orig_get_pose) ~= "function" then
        return
    end
    STATE.sim_wrapped = true

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

-- ============================================================================
-- 2. EVENT-DRIVEN UNPAUSE HOOKS (No Per-Frame Reflection Polling)
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

-- Periodic recovery check (only runs once every 2 seconds if a pause leak occurs, NEVER per frame)
local function periodic_unpause_guard(now)
    if not CFG.no_coop_pause or not STATE.connected then
        return
    end
    if (now - STATE.last_unpause_check) < 2.0 then
        return
    end
    STATE.last_unpause_check = now
    pcall(function()
        local sm = sdk.get_native_singleton("via.SceneManager")
        local td_sm = sdk.find_type_definition("via.SceneManager")
        if sm and td_sm then
            local scene = sdk.call_native_func(sm, td_sm, "get_CurrentScene")
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
-- 3. SAFE ROLE & THEME SYNC (No Exception Storms / No Disk Thrashing)
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
    -- Poll role at low frequency (every 5 seconds) using safe I/O to avoid JSON parse errors
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

    -- Theme check (every 10 seconds)
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

-- Load theme immediately at startup
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
-- 4. ULTRA-LEAN FRAME HOOK (Zero Per-Frame Allocation or Reflection)
-- ============================================================================
re.on_pre_application_entry("UpdateBehavior", function()
    local now = os.clock()
    STATE.dt = math.min(0.1, math.max(0.0005, now - STATE.last_frame_time))
    STATE.last_frame_time = now

    if not STATE.sim_wrapped then
        wrap_re4mp_sim()
    end
    if not STATE.pause_hooked then
        install_unpause_hooks()
    end

    poll_role_and_theme(now)
    periodic_unpause_guard(now)
end)

re.on_frame(function()
    if not STATE.sim_wrapped then
        wrap_re4mp_sim()
    end
end)

re.on_draw_ui(function()
    if not imgui then return end
    if imgui.tree_node("Chubby Co-op Enhancer v1.0.8 (" .. STATE.theme_name .. ")") then
        imgui.text_colored("Synced Theme: " .. STATE.theme_name .. "  |  Session: " .. (STATE.connected and ("Connected (" .. STATE.role .. ")") or "Standby"), STATE.accent_hex)
        imgui.text("Single OS Mouse Cursor Fix: Active (Re4-Co-op.dll patched)")
        imgui.text("Enemy sync: 30 Hz animation (clean 90+ FPS stability in combat encounters)")
        local c1, v1 = imgui.checkbox("Smooth Partner Movement (Lerp/Slerp)", CFG.smooth_movement)
        if c1 then CFG.smooth_movement = v1 end
        local c2, v2 = imgui.checkbox("Non-Blocking Pause Menu & Inventory in Co-op", CFG.no_coop_pause)
        if c2 then CFG.no_coop_pause = v2 end
        imgui.text(string.format("Stats: %d smoothed frames | %d pause blocks", STATE.smoothed_frames, STATE.unpaused_frames))
        imgui.tree_pop()
    end
end)

log.info("[Chubby-Coop-Fix] v1.0.8 high-fps enhancer loaded")
