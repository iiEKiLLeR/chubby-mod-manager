#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
//! Chubby Mod Manager: a mod manager for Resident Evil games with a co-op host system.

mod app;
mod config;
mod coop;
mod fsx;
mod games;
mod mods;
mod rar;
mod screens;
mod selftest;
mod sys;
mod ui;

use app::{Action, App, TITLE};
use config::Config;
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

pub fn arg_value(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn app_icon() -> Option<macroquad::miniquad::conf::Icon> {
    let img = image::load_from_memory(ui::APP_ICON_JPG).ok()?;
    let filt = image::imageops::FilterType::Lanczos3;
    let s16 = img.resize_exact(16, 16, filt).to_rgba8();
    let s32 = img.resize_exact(32, 32, filt).to_rgba8();
    let s64 = img.resize_exact(64, 64, filt).to_rgba8();
    let mut small = [0u8; 16 * 16 * 4];
    let mut medium = [0u8; 32 * 32 * 4];
    let mut big = [0u8; 64 * 64 * 4];
    small.copy_from_slice(s16.as_raw());
    medium.copy_from_slice(s32.as_raw());
    big.copy_from_slice(s64.as_raw());
    Some(macroquad::miniquad::conf::Icon { small, medium, big })
}

fn enable_dpi_awareness() {
    unsafe {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if !user32.is_null() {
            let func = GetProcAddress(user32, b"SetProcessDpiAwarenessContext\0".as_ptr());
            if let Some(set_dpi) = func {
                // DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2 = -4 as isize
                let set_dpi: extern "system" fn(isize) -> i32 =
                    std::mem::transmute(set_dpi);
                let _ = set_dpi(-4);
            }
        }
    }
}

fn get_system_dpi_scale() -> f32 {
    unsafe {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if !user32.is_null() {
            let func = GetProcAddress(user32, b"GetDpiForSystem\0".as_ptr());
            if let Some(get_dpi) = func {
                let get_dpi: extern "system" fn() -> u32 = std::mem::transmute(get_dpi);
                let dpi = get_dpi();
                if dpi > 0 {
                    return dpi as f32 / 96.0;
                }
            }
        }
    }
    1.0
}

fn window_conf() -> Conf {
    enable_dpi_awareness();
    if std::env::args().any(|a| a == "--install-silent") {
        app::run_self_silent_install();
        std::process::exit(0);
    }
    // Release helper: write update.json (version + patch notes + GitHub download link).
    if let Some(out) = arg_value("--write-update-json") {
        let json = app::build_update_manifest_json(&app::github_installer_url(app::VERSION));
        let _ = std::fs::write(&out, json);
        std::process::exit(0);
    }
    let data = arg_value("--data").map(PathBuf::from).unwrap_or_else(exe_dir);
    let cfg = Config::load(&data.join("Data").join("config.cfg"));
    let is_shot = arg_value("--screenshot").is_some();
    let is_test = arg_value("--self-test").is_some();
    let headless = is_shot || is_test;
    let (w, h) = if is_shot {
        (1480, 820)
    } else if is_test {
        (1280, 720)
    } else {
        (
            cfg.get_i("win_width").clamp(1100, 3840) as i32,
            cfg.get_i("win_height").clamp(640, 2160) as i32,
        )
    };
    let scale = if headless { 1.0 } else { get_system_dpi_scale().max(1.0) };
    let win_w = (w as f32 * scale).round() as i32;
    let win_h = (h as f32 * scale).round() as i32;
    Conf {
        window_title: TITLE.to_string(),
        window_width: win_w,
        window_height: win_h,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        icon: app_icon(),
        ..Default::default()
    }
}

/// A second copy was started (for example by dropping a zip on the program's icon)?
/// Hand the files to the copy that is already open and quit.
fn hand_off_to_running_copy(files: &[PathBuf]) -> bool {
    use std::io::Write;
    let addr = "127.0.0.1:9951".parse().unwrap();
    if let Ok(mut s) = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(300)) {
        let list: Vec<String> = files.iter().map(|p| p.to_string_lossy().to_string()).collect();
        let _ = s.write_all(format!("REMH-OPEN\n{}\n", list.join("\n")).as_bytes());
        return true;
    }
    false
}

fn listen_for_other_copies() {
    let Ok(l) = std::net::TcpListener::bind("127.0.0.1:9951") else { return };
    std::thread::spawn(move || {
        for s in l.incoming().flatten() {
            use std::io::Read;
            let mut text = String::new();
            let _ = s.take(1 << 16).read_to_string(&mut text);
            let mut lines = text.lines();
            if lines.next() == Some("REMH-OPEN") {
                for l in lines.filter(|l| !l.is_empty()) {
                    sys::push_dropped(PathBuf::from(l));
                }
            }
        }
    });
}

/// What one 1 ms sleep really costs on this computer: like the original's start-up
/// (0x140004520: Sleep(1), read the clock, Sleep(1), read the clock, "Default sleep time").
fn measure_sleep_cost_ms() -> f64 {
    std::thread::sleep(std::time::Duration::from_millis(1));
    let a = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(1));
    a.elapsed().as_secs_f64() * 1000.0
}

/// The original's frame limiter (main loop 0x1400057c0, frame time from 0x1400054f0):
/// frame time = 1.0 / fps * 1000 ms.
/// mode 0: no limit. mode 1: wait in a tight loop until the frame time has passed.
/// mode 2 ("approximate", the default): sleep 1 ms at a time while
///         slept + sleep cost < frame time (the frame's own work is not subtracted).
/// mode 3: like 2, but with AccurateSleepTiming on the frame's work time is subtracted.
fn limit_frame(mode: i64, fps: f64, accurate: bool, sleep_cost_ms: f64, frame_start: f64) {
    let frame_ms = 1.0 / fps.max(1.0) * 1000.0;
    let work_ms = (get_time() - frame_start) * 1000.0;
    match mode {
        1 => {
            while (get_time() - frame_start) * 1000.0 < frame_ms {
                std::thread::yield_now();
            }
        }
        2 | 3 => {
            let target = if accurate && mode == 3 { frame_ms - work_ms } else { frame_ms };
            let start = std::time::Instant::now();
            let mut slept = 0.0;
            while slept + sleep_cost_ms < target {
                std::thread::sleep(std::time::Duration::from_millis(1));
                slept = start.elapsed().as_secs_f64() * 1000.0;
            }
        }
        _ => {}
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    unsafe {
        #[link(name = "opengl32")]
        unsafe extern "system" {
            fn glEnable(cap: u32);
            fn glHint(target: u32, mode: u32);
        }
        glEnable(0x809D); // GL_MULTISAMPLE
        glEnable(0x0B20); // GL_LINE_SMOOTH
        glHint(0x0C52, 0x1102); // GL_LINE_SMOOTH_HINT -> GL_NICEST
    }
    let shot = arg_value("--screenshot");
    let test = arg_value("--self-test");
    let headless = shot.is_some() || test.is_some();
    let data = if test.is_some() {
        selftest::sandbox()
    } else {
        arg_value("--data").map(PathBuf::from).unwrap_or_else(exe_dir)
    };
    let opened: Vec<PathBuf> = std::env::args()
        .skip(1)
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .filter(|p| {
            let e = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            ["zip", "7z", "rar", "pak"].contains(&e.as_str())
        })
        .collect();
    if !headless {
        if hand_off_to_running_copy(&opened) {
            return;
        }
        listen_for_other_copies();
        coop::start_update_server();
    }
    prevent_quit();
    let mut app = App::new(data, headless);
    // Screenshot helpers (for the README): pick a tab / preview a theme without saving it.
    if shot.is_some() {
        app.nav_w = 184.0;
        app.cat_w = 212.0;
        app.modlist_pct = 55.0;
        app.preview_pct = 42.0;
        app.coop_top_pct = 50.0;
        app.coop_top_h = 265.0;
        app.coop_session_pct = 46.0;
        app.options_pct = 48.0;
        if let Some(tab) = arg_value("--shot-tab") {
            app.tab = match tab.to_ascii_lowercase().as_str() {
                "coop" | "co-op" => app::Tab::Coop,
                "presets" => app::Tab::Presets,
                "options" => app::Tab::Options,
                "log" => app::Tab::Log,
                "patch-notes" | "patch_notes" => {
                    app.show_update_modal = true;
                    app.update_status = format!("Running v{} (latest installed)", app::VERSION);
                    app::Tab::Mods
                }
                _ => app::Tab::Mods,
            };
        }
        if let Some(id) = arg_value("--shot-theme") {
            if let Some(i) = ui::THEMES.iter().position(|t| t.id.eq_ignore_ascii_case(&id)) {
                app.ui.theme_idx = i;
            }
        }
        app.toast = None;
        app.log.clear();
    }
    if !opened.is_empty() {
        app.actions.push_back(Action::Drop(opened));
    }
    let mut tester = test.map(selftest::Tester::new);
    let mut drop_ready = false;
    let sleep_cost_ms = measure_sleep_cost_ms();
    loop {
        let t0 = get_time();
        if !drop_ready && app.frame > 1 {
            drop_ready = sys::enable_drop(TITLE) || app.frame > 600;
        }
        if let Some(t) = tester.as_mut() {
            t.before(&mut app);
        }
        app.update();
        clear_background(app.ui.theme().bg_top);
        app.draw();
        if let Some(t) = tester.as_mut() {
            if t.after(&mut app) {
                app.leave();
                break;
            }
        }
        if let Some(p) = &shot {
            if (app.frame >= 60 && !app.is_busy()) || app.frame >= 300 {
                let img = get_screen_data();
                if let Some(dir) = Path::new(p).parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                img.export_png(p);
                break;
            }
        }
        if is_quit_requested() || app.quit {
            break;
        }
        // Frame limiter: the original's modes (main loop 0x1400057c0); the window idles when
        // it is not in front (see MECHANICS.md "Power").
        if !headless {
            let mode_key = if app.ui.active { "LimitFPS" } else { "NoFocusLimitFPS" };
            let mode = app.cfg.get_i(mode_key);
            let accurate = app.cfg.flag("AccurateSleepTiming");
            limit_frame(mode, app.target_fps, accurate, sleep_cost_ms, t0);
        }
        next_frame().await;
    }
    if !headless {
        app.cfg.set("win_width", screen_width() as i32);
        app.cfg.set("win_height", screen_height() as i32);
        app.cfg.save();
    }
}
