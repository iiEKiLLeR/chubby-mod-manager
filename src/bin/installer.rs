#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use macroquad::prelude::*;
use std::env;
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PAYLOAD_EXE: &[u8] = include_bytes!("../../assets/chubby_mod_manager_payload.exe");
const PAYLOAD_UNRAR: &[u8] = include_bytes!("../../vendor/unrar.dll");
const APP_ICON_JPG: &[u8] = include_bytes!("../../assets/icon.jpg");

const DEFAULT_CONFIG: &str = "\
win_width: 1280
win_height: 760
player_name: KYOMU
network_ip: 127.0.0.1
network_port: 9950
MaxFPS: 60
NoFocusMaxFPS: 10
GameRunningFPS: 2
SleepWhileGameRuns: 1
MinimizeWhenGameStarts: 1
BoostGamePriority: 1
PauseTransfersDuringGame: 1
AutoDeleteBackupFiles: 1
ShowModPreviewScreenshots: 1
WatchModsFolder: 1
FluffyFolder: G:/Downloads\\Fluffy Mod Manager-14-3-020-1698963636
current_game: re4r
ExitWhenGameStarts: 0
modman_re4r_dir: g:/programs/steam\\steamapps\\common\\RESIDENT EVIL 4  BIOHAZARD RE4
LimitFPS: 2
NoFocusLimitFPS: 2
AccurateSleepTiming: 0
FluffyTakenOver: 1
FluffyTakeOverCopied: 0
theme: umbrella_crimson
";

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn default_install_dir() -> PathBuf {
    if let Ok(out) = Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            "(Get-ItemProperty -Path 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager' -Name InstallLocation -ErrorAction SilentlyContinue).InstallLocation",
        ])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !s.is_empty() {
            return PathBuf::from(s);
        }
    }
    if let Ok(local) = env::var("LOCALAPPDATA") {
        PathBuf::from(local).join("Programs").join("Chubby Mod Manager")
    } else {
        PathBuf::from("C:\\Chubby Mod Manager")
    }
}

fn desktop_dir() -> PathBuf {
    if let Ok(profile) = env::var("USERPROFILE") {
        let p = PathBuf::from(&profile).join("Desktop");
        if p.is_dir() {
            return p;
        }
    }
    PathBuf::from(".")
}

fn start_menu_dir() -> PathBuf {
    if let Ok(appdata) = env::var("APPDATA") {
        PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
    } else {
        desktop_dir()
    }
}

fn browse_for_folder(current: &str) -> Option<PathBuf> {
    let initial = Path::new(current.trim());
    let start_dir = if initial.is_dir() {
        initial.display().to_string()
    } else if let Some(parent) = initial.parent() {
        if parent.is_dir() {
            parent.display().to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         $f = New-Object System.Windows.Forms.Form; \
         $f.TopMost = $true; \
         $d = New-Object System.Windows.Forms.FolderBrowserDialog; \
         $d.Description = 'Choose where to install Chubby Mod Manager'; \
         $d.ShowNewFolderButton = $true; \
         if ('{}' -ne '' -and (Test-Path '{}')) {{ $d.SelectedPath = '{}' }}; \
         if ($d.ShowDialog($f) -eq [System.Windows.Forms.DialogResult]::OK) {{ \
             [Console]::Out.Write($d.SelectedPath) \
         }}",
        start_dir.replace('\'', "''"),
        start_dir.replace('\'', "''"),
        start_dir.replace('\'', "''"),
    );

    let out = Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-Sta", "-WindowStyle", "Hidden", "-Command", &script])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let picked = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if picked.is_empty() {
        return None;
    }
    let p = PathBuf::from(&picked);
    let leaf = p
        .file_name()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if leaf.contains("chubby") {
        Some(p)
    } else {
        Some(p.join("Chubby Mod Manager"))
    }
}

fn read_clipboard_text() -> Option<String> {
    let out = Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-Sta", "-WindowStyle", "Hidden", "-Command", "Get-Clipboard"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn create_shortcut(lnk_path: &Path, target_exe: &Path, work_dir: &Path, icon_path: &Path, desc: &str) -> Result<(), String> {
    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $s = $ws.CreateShortcut('{}'); \
         $s.TargetPath = '{}'; \
         $s.WorkingDirectory = '{}'; \
         $s.IconLocation = '{},0'; \
         $s.Description = '{}'; \
         $s.Save()",
        lnk_path.display().to_string().replace('\'', "''"),
        target_exe.display().to_string().replace('\'', "''"),
        work_dir.display().to_string().replace('\'', "''"),
        icon_path.display().to_string().replace('\'', "''"),
        desc.replace('\'', "''"),
    );
    let status = Command::new("powershell.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Failed to create shortcut".into())
    }
}

fn perform_install(install_dir: &Path, make_desktop: bool, make_start_menu: bool) -> Result<PathBuf, String> {
    if install_dir.as_os_str().is_empty() {
        return Err("Please enter or browse for a valid installation path.".into());
    }
    fs::create_dir_all(install_dir).map_err(|e| format!("Cannot create {}: {}", install_dir.display(), e))?;
    let data_dir = install_dir.join("Data");
    fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;

    let exe_path = install_dir.join("Chubby Mod Manager.exe");
    let old_exe = install_dir.join("Chubby Mod Manager.exe.old");
    let _ = fs::remove_file(&old_exe);
    if exe_path.exists() {
        let _ = fs::rename(&exe_path, &old_exe);
    }
    fs::write(&exe_path, PAYLOAD_EXE).map_err(|e| format!("Cannot write {}: {}", exe_path.display(), e))?;
    let _ = fs::remove_file(&old_exe);
    let _ = fs::write(install_dir.join("unrar.dll"), PAYLOAD_UNRAR);

    let cfg_path = data_dir.join("config.cfg");
    if !cfg_path.exists() {
        let _ = fs::write(&cfg_path, DEFAULT_CONFIG.as_bytes());
    }

    let uninst_cmd = install_dir.join("Uninstall Chubby Mod Manager.cmd");
    let uninst_body = "\
@echo off\r\n\
del /f /q \"%USERPROFILE%\\Desktop\\Chubby Mod Manager.lnk\" 2>nul\r\n\
del /f /q \"%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs\\Chubby Mod Manager.lnk\" 2>nul\r\n\
reg delete \"HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager\" /f 2>nul\r\n\
echo Chubby Mod Manager shortcuts removed.\r\n";
    let _ = fs::write(&uninst_cmd, uninst_body.as_bytes());

    if make_desktop {
        let lnk = desktop_dir().join("Chubby Mod Manager.lnk");
        let _ = create_shortcut(
            &lnk,
            &exe_path,
            install_dir,
            &exe_path,
            "Chubby Mod Manager - RE Engine & Co-op Mod Hub",
        );
    }

    if make_start_menu {
        let lnk = start_menu_dir().join("Chubby Mod Manager.lnk");
        let _ = create_shortcut(
            &lnk,
            &exe_path,
            install_dir,
            &exe_path,
            "Chubby Mod Manager - RE Engine & Co-op Mod Hub",
        );
    }

    let _ = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager",
            "/v",
            "DisplayName",
            "/d",
            "Chubby Mod Manager",
            "/f",
        ])
        .status();
    let _ = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager",
            "/v",
            "InstallLocation",
            "/d",
            &install_dir.display().to_string(),
            "/f",
        ])
        .status();
    let _ = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager",
            "/v",
            "DisplayIcon",
            "/d",
            &exe_path.display().to_string(),
            "/f",
        ])
        .status();
    let _ = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager",
            "/v",
            "Publisher",
            "/d",
            "Chubby Mod Manager",
            "/f",
        ])
        .status();
    let _ = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager",
            "/v",
            "UninstallString",
            "/d",
            &uninst_cmd.display().to_string(),
            "/f",
        ])
        .status();

    let _ = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager",
            "/v",
            "DisplayVersion",
            "/d",
            VERSION,
            "/f",
        ])
        .status();

    Ok(exe_path)
}

fn load_window_icon() -> Option<miniquad::conf::Icon> {
    let img = image::load_from_memory(APP_ICON_JPG).ok()?.to_rgba8();
    fn resize_raw<const N: usize>(src: &image::RgbaImage, side: u32) -> [u8; N] {
        let resized = image::imageops::resize(src, side, side, image::imageops::FilterType::Lanczos3);
        let mut out = [0u8; N];
        let raw = resized.as_raw();
        let len = raw.len().min(N);
        out[..len].copy_from_slice(&raw[..len]);
        out
    }
    Some(miniquad::conf::Icon {
        small: resize_raw::<{ 16 * 16 * 4 }>(&img, 16),
        medium: resize_raw::<{ 32 * 32 * 4 }>(&img, 32),
        big: resize_raw::<{ 64 * 64 * 4 }>(&img, 64),
    })
}

fn enable_dpi_awareness() {
    unsafe {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
        let user32 = GetModuleHandleA(b"user32.dll\0".as_ptr());
        if !user32.is_null() {
            let func = GetProcAddress(user32, b"SetProcessDpiAwarenessContext\0".as_ptr());
            if let Some(set_dpi) = func {
                let set_dpi: extern "system" fn(isize) -> i32 = std::mem::transmute(set_dpi);
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

fn conf() -> Conf {
    enable_dpi_awareness();
    let scale = get_system_dpi_scale().max(1.0);
    Conf {
        window_title: format!("Chubby Mod Manager v{} Setup", VERSION),
        window_width: (680.0 * scale).round() as i32,
        window_height: (450.0 * scale).round() as i32,
        window_resizable: false,
        high_dpi: true,
        sample_count: 4,
        icon: load_window_icon(),
        ..Default::default()
    }
}

fn arg_value(flag: &str) -> Option<String> {
    let args: Vec<String> = env::args().collect();
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

fn has_flag(flag: &str) -> bool {
    env::args().any(|a| a == flag)
}

#[macroquad::main(conf)]
async fn main() {
    let mut path_input = arg_value("--dir")
        .unwrap_or_else(|| default_install_dir().display().to_string());

    if has_flag("--install-silent") {
        let target = PathBuf::from(path_input.trim());
        if let Err(e) = perform_install(&target, true, true) {
            eprintln!("{}", e);
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    let shot_path = arg_value("--screenshot");
    let badge = Texture2D::from_file_with_format(APP_ICON_JPG, Some(ImageFormat::Jpeg));
    badge.set_filter(FilterMode::Linear);

    let win_dir = env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let load_font = |name: &str| {
        let p = format!("{}\\Fonts\\{}", win_dir, name);
        fs::read(&p).ok().and_then(|b| {
            let mut f = load_ttf_font_from_bytes(&b).ok()?;
            f.set_filter(FilterMode::Linear);
            Some(f)
        })
    };
    let font = load_font("segoeui.ttf").or_else(|| load_font("arial.ttf"));
    let bold = load_font("segoeuib.ttf").or_else(|| load_font("arialbd.ttf")).or_else(|| font.clone());

    let d_txt = |s: &str, x: f32, y: f32, size: f32, c: Color| {
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font: font.as_ref(),
                font_size: size as u16,
                color: c,
                ..Default::default()
            },
        );
    };
    let d_btxt = |s: &str, x: f32, y: f32, size: f32, c: Color| {
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font: bold.as_ref(),
                font_size: size as u16,
                color: c,
                ..Default::default()
            },
        );
    };
    let m_txt = |s: &str, size: f32| -> f32 {
        measure_text(s, font.as_ref(), size as u16, 1.0).width
    };
    let m_btxt = |s: &str, size: f32| -> f32 {
        measure_text(s, bold.as_ref(), size as u16, 1.0).width
    };

    let mut path_focused = false;
    let mut opt_desktop = true;
    let mut opt_start_menu = true;
    let mut opt_launch = true;
    let mut status_msg = String::from("Choose your install folder, then click INSTALL NOW.");
    let mut installed_exe: Option<PathBuf> = None;
    let mut is_error = false;
    let mut frame_idx = 0u32;

    let bg_top = Color::from_rgba(16, 11, 14, 255);
    let bg_bot = Color::from_rgba(10, 7, 9, 255);
    let panel = Color::from_rgba(24, 16, 20, 255);
    let panel2 = Color::from_rgba(35, 22, 28, 255);
    let field_bg = Color::from_rgba(15, 10, 13, 255);
    let line = Color::from_rgba(58, 34, 44, 255);
    let accent = Color::from_rgba(225, 29, 72, 255);
    let accent_hi = Color::from_rgba(251, 113, 133, 255);
    let text = Color::from_rgba(252, 238, 242, 255);
    let muted = Color::from_rgba(184, 142, 154, 255);
    let good = Color::from_rgba(52, 211, 153, 255);

    loop {
        frame_idx += 1;
        let w = screen_width();
        let h = screen_height();
        let (mx, my) = mouse_position();
        let clicked = is_mouse_button_pressed(MouseButton::Left);

        // Handle keyboard input when path text box is focused
        while let Some(c) = get_char_pressed() {
            if path_focused && installed_exe.is_none() && !c.is_control() {
                path_input.push(c);
            }
        }
        if path_focused && installed_exe.is_none() {
            if is_key_pressed(KeyCode::Backspace) {
                path_input.pop();
            }
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if ctrl && is_key_pressed(KeyCode::V) {
                if let Some(clip) = read_clipboard_text() {
                    path_input = clip.lines().next().unwrap_or("").trim().trim_matches('"').to_string();
                }
            }
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Escape) {
                path_focused = false;
            }
        }

        clear_background(bg_bot);
        draw_rectangle(0.0, 0.0, w, 90.0, bg_top);
        draw_line(0.0, 90.0, w, 90.0, 1.0, line);
        draw_line(0.0, 0.0, w, 0.0, 2.0, accent);

        // Header badge & title
        draw_rectangle(24.0, 19.0, 52.0, 52.0, accent);
        draw_texture_ex(
            &badge,
            25.0,
            20.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(50.0, 50.0)),
                ..Default::default()
            },
        );
        d_btxt(&format!("CHUBBY MOD MANAGER  v{}", VERSION), 90.0, 44.0, 24.0, text);
        d_txt(
            &format!("SETUP WIZARD v{}  |  CHOOSE INSTALL LOCATION & SHORTCUTS", VERSION),
            90.0,
            66.0,
            13.5,
            accent_hi,
        );

        // Main card
        let cx = 24.0;
        let cy = 108.0;
        let cw = w - 48.0;
        let ch = 250.0;
        draw_rectangle(cx, cy, cw, ch, panel);
        draw_rectangle_lines(cx, cy, cw, ch, 1.0, line);

        d_btxt(
            "INSTALLATION FOLDER  (click to edit path or click Browse...)",
            cx + 18.0,
            cy + 26.0,
            13.5,
            muted,
        );

        // Editable path box + Browse... button
        let browse_w = 116.0;
        let box_x = cx + 18.0;
        let box_y = cy + 38.0;
        let box_h = 38.0;
        let box_w = cw - 36.0 - browse_w - 10.0;
        let browse_x = box_x + box_w + 10.0;

        let box_hov = mx >= box_x && mx <= box_x + box_w && my >= box_y && my <= box_y + box_h;
        let browse_hov = mx >= browse_x && mx <= browse_x + browse_w && my >= box_y && my <= box_y + box_h;

        if clicked && installed_exe.is_none() {
            path_focused = box_hov;
        }

        draw_rectangle(box_x, box_y, box_w, box_h, field_bg);
        draw_rectangle_lines(
            box_x,
            box_y,
            box_w,
            box_h,
            if path_focused { 1.8 } else { 1.0 },
            if path_focused { accent_hi } else if box_hov { accent } else { line },
        );

        // Fit displayed text inside box_w
        let mut shown = path_input.clone();
        if path_focused && ((get_time() * 2.0) as i32 % 2 == 0) {
            shown.push('|');
        }
        while m_txt(&shown, 15.0) > box_w - 20.0 && shown.len() > 4 {
            shown.remove(0);
        }
        d_txt(&shown, box_x + 10.0, box_y + 24.0, 15.0, text);

        // Browse... button
        draw_rectangle(
            browse_x,
            box_y,
            browse_w,
            box_h,
            if browse_hov { accent } else { panel2 },
        );
        draw_rectangle_lines(
            browse_x,
            box_y,
            browse_w,
            box_h,
            1.0,
            if browse_hov { accent_hi } else { line },
        );
        let btxt = "BROWSE...";
        let btw = m_btxt(btxt, 14.0);
        d_btxt(
            btxt,
            browse_x + (browse_w - btw) * 0.5,
            box_y + 24.0,
            14.0,
            text,
        );

        if browse_hov && clicked && installed_exe.is_none() {
            path_focused = false;
            if let Some(picked) = browse_for_folder(&path_input) {
                path_input = picked.display().to_string();
                is_error = false;
                status_msg = format!("Selected folder: {}", path_input);
            }
        }

        // Checkboxes
        let rows = [
            ("Create Desktop shortcut (\"Chubby Mod Manager\")", opt_desktop, 0usize),
            ("Create Start Menu shortcut", opt_start_menu, 1usize),
            ("Launch Chubby Mod Manager immediately after installation", opt_launch, 2usize),
        ];

        for (i, (label, val, id)) in rows.iter().enumerate() {
            let ry = cy + 98.0 + i as f32 * 42.0;
            let rx = cx + 18.0;
            let rw = cw - 36.0;
            let rh = 34.0;
            let hov = mx >= rx && mx <= rx + rw && my >= ry && my <= ry + rh;
            if hov {
                draw_rectangle(rx, ry, rw, rh, panel2);
            }
            let bx = rx + 8.0;
            let by = ry + 7.0;
            draw_rectangle(bx, by, 20.0, 20.0, if *val { accent } else { panel2 });
            draw_rectangle_lines(bx, by, 20.0, 20.0, 1.0, if *val { accent_hi } else { line });
            if *val {
                draw_line(bx + 4.0, by + 10.0, bx + 9.0, by + 15.0, 2.2, WHITE);
                draw_line(bx + 9.0, by + 15.0, bx + 16.0, by + 5.0, 2.2, WHITE);
            }
            d_txt(label, bx + 32.0, ry + 22.0, 15.0, text);
            if hov && clicked && installed_exe.is_none() {
                match *id {
                    0 => opt_desktop = !opt_desktop,
                    1 => opt_start_menu = !opt_start_menu,
                    2 => opt_launch = !opt_launch,
                    _ => {}
                }
            }
        }

        // Status bar & Install button
        let status_col = if is_error {
            Color::from_rgba(248, 113, 113, 255)
        } else if installed_exe.is_some() {
            good
        } else {
            muted
        };
        d_txt(&status_msg, 24.0, h - 38.0, 15.0, status_col);

        let btn_w = 190.0;
        let btn_h = 44.0;
        let btn_x = w - 24.0 - btn_w;
        let btn_y = h - 22.0 - btn_h;
        let btn_hov = mx >= btn_x && mx <= btn_x + btn_w && my >= btn_y && my <= btn_y + btn_h;
        let btn_col = if installed_exe.is_some() {
            good
        } else if btn_hov {
            accent_hi
        } else {
            accent
        };
        draw_rectangle(btn_x, btn_y, btn_w, btn_h, btn_col);
        let btn_label = if installed_exe.is_some() {
            "OPEN APP NOW"
        } else {
            "INSTALL NOW"
        };
        let tw = m_btxt(btn_label, 17.0);
        d_btxt(
            btn_label,
            btn_x + (btn_w - tw) * 0.5,
            btn_y + 28.0,
            17.0,
            WHITE,
        );

        if btn_hov && clicked {
            let install_dir = PathBuf::from(path_input.trim());
            if let Some(exe) = &installed_exe {
                let _ = Command::new(exe).current_dir(&install_dir).spawn();
                std::process::exit(0);
            } else {
                match perform_install(&install_dir, opt_desktop, opt_start_menu) {
                    Ok(exe) => {
                        is_error = false;
                        status_msg = format!("Installed to {}!", install_dir.display());
                        if opt_launch {
                            let _ = Command::new(&exe).current_dir(&install_dir).spawn();
                            std::process::exit(0);
                        }
                        installed_exe = Some(exe);
                    }
                    Err(e) => {
                        is_error = true;
                        status_msg = format!("Install error: {}", e);
                    }
                }
            }
        }

        if let Some(ref sp) = shot_path {
            if frame_idx >= 3 {
                let img = get_screen_data();
                if let Some(parent) = Path::new(sp).parent() {
                    let _ = fs::create_dir_all(parent);
                }
                img.export_png(sp);
                std::process::exit(0);
            }
        }

        next_frame().await;
    }
}
