//! Windows-only helpers: drag-and-drop onto the window, finding the running game,
//! raising its priority and minimizing ourselves.
//!
//! Mouse note (the double-cursor fix): the original registers the mouse for raw input with
//! RIDEV_INPUTSINK (flags 0x100, function 0x1400074c0) so it keeps reading the mouse while the
//! game has focus, and it moves the real cursor with SetCursorPos (function 0x14000ba40).
//! This program never registers raw input, never moves the cursor, never draws its own cursor
//! and only reads the mouse while its own window is in front.

use std::path::PathBuf;
use std::sync::Mutex;

static DROPPED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

pub fn push_dropped(p: PathBuf) {
    DROPPED.lock().unwrap().push(p);
}

/// Paths dropped on the window since the last call.
pub fn take_dropped() -> Vec<PathBuf> {
    std::mem::take(&mut *DROPPED.lock().unwrap())
}

#[cfg(windows)]
mod win {
    use super::DROPPED;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    use windows_sys::Win32::System::Threading::*;
    use windows_sys::Win32::UI::Shell::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    static OLD_PROC: AtomicIsize = AtomicIsize::new(0);
    static HWND_VAL: AtomicIsize = AtomicIsize::new(0);

    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if msg == WM_DROPFILES {
            let hdrop = wp as HDROP;
            unsafe {
                let count = DragQueryFileW(hdrop, 0xFFFF_FFFF, std::ptr::null_mut(), 0);
                let mut list = DROPPED.lock().unwrap();
                for i in 0..count {
                    let len = DragQueryFileW(hdrop, i, std::ptr::null_mut(), 0) as usize;
                    let mut buf = vec![0u16; len + 1];
                    DragQueryFileW(hdrop, i, buf.as_mut_ptr(), (len + 1) as u32);
                    list.push(PathBuf::from(String::from_utf16_lossy(&buf[..len])));
                }
                DragFinish(hdrop);
            }
            return 0;
        }
        let old = OLD_PROC.load(Ordering::SeqCst);
        unsafe {
            let f: WNDPROC = std::mem::transmute(old);
            CallWindowProcW(f, hwnd, msg, wp, lp)
        }
    }

    pub fn find_window(title: &str) -> HWND {
        let cached = HWND_VAL.load(Ordering::SeqCst);
        if cached != 0 {
            return cached as HWND;
        }
        let cls: Vec<u16> = "MINIQUADAPP\0".encode_utf16().collect();
        let t: Vec<u16> = format!("{}\0", title).encode_utf16().collect();
        let h = unsafe { FindWindowW(cls.as_ptr(), t.as_ptr()) };
        if !h.is_null() {
            HWND_VAL.store(h as isize, Ordering::SeqCst);
        }
        h
    }

    pub fn enable_drop(title: &str) -> bool {
        let h = find_window(title);
        if h.is_null() || OLD_PROC.load(Ordering::SeqCst) != 0 {
            return !h.is_null();
        }
        unsafe {
            // Let Explorer drop onto us even when we run as administrator.
            ChangeWindowMessageFilterEx(h, WM_DROPFILES, MSGFLT_ALLOW, std::ptr::null_mut());
            ChangeWindowMessageFilterEx(h, WM_COPYDATA, MSGFLT_ALLOW, std::ptr::null_mut());
            ChangeWindowMessageFilterEx(h, 0x0049, MSGFLT_ALLOW, std::ptr::null_mut());
            DragAcceptFiles(h, 1);
            // Dark title bar, caption colour 0x262626, like the original's WM_CREATE (0x140042aa0).
            let dark: u32 = 1;
            let caption: u32 = 0x0026_2626;
            let _ = windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(h, 20, &dark as *const u32 as *const _, 4);
            let _ = windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute(h, 35, &caption as *const u32 as *const _, 4);
            let old = SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
            OLD_PROC.store(old, Ordering::SeqCst);
        }
        true
    }

    pub fn is_foreground(title: &str) -> bool {
        let h = find_window(title);
        !h.is_null() && unsafe { GetForegroundWindow() } == h
    }

    pub fn minimize(title: &str) {
        let h = find_window(title);
        if !h.is_null() {
            unsafe { ShowWindow(h, SW_MINIMIZE) };
        }
    }

    /// Process id of the first running program with this file name.
    pub fn find_process(exe: &str) -> Option<u32> {
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE {
                return None;
            }
            let mut e: PROCESSENTRY32W = std::mem::zeroed();
            e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut found = None;
            if Process32FirstW(snap, &mut e) != 0 {
                loop {
                    let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(0);
                    let name = String::from_utf16_lossy(&e.szExeFile[..len]);
                    if name.eq_ignore_ascii_case(exe) {
                        found = Some(e.th32ProcessID);
                        break;
                    }
                    if Process32NextW(snap, &mut e) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snap);
            found
        }
    }

    pub fn set_priority(pid: u32, high: bool) -> bool {
        unsafe {
            let h = OpenProcess(PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return false;
            }
            let class = if high { HIGH_PRIORITY_CLASS } else { ABOVE_NORMAL_PRIORITY_CLASS };
            let ok = SetPriorityClass(h, class) != 0;
            CloseHandle(h);
            ok
        }
    }

    /// Our own window works in the background at the lowest priority while a game runs.
    pub fn self_background(on: bool) {
        unsafe {
            let class = if on { BELOW_NORMAL_PRIORITY_CLASS } else { NORMAL_PRIORITY_CLASS };
            SetPriorityClass(GetCurrentProcess(), class);
        }
    }
}

#[cfg(windows)]
pub use win::{enable_drop, find_process, is_foreground, minimize, self_background, set_priority};

#[cfg(not(windows))]
pub fn enable_drop(_: &str) -> bool { false }
#[cfg(not(windows))]
pub fn is_foreground(_: &str) -> bool { true }
#[cfg(not(windows))]
pub fn minimize(_: &str) {}
#[cfg(not(windows))]
pub fn find_process(_: &str) -> Option<u32> { None }
#[cfg(not(windows))]
pub fn set_priority(_: u32, _: bool) -> bool { false }
#[cfg(not(windows))]
pub fn self_background(_: bool) {}

/// Buttons held on the first Xbox-style controller (0 when none). Only asked while our
/// window is in front, so the controller never drives this program during the game.
pub fn pad_buttons() -> u16 {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::UI::Input::XboxController::*;
        let mut st: XINPUT_STATE = std::mem::zeroed();
        if XInputGetState(0, &mut st) == 0 {
            return st.Gamepad.wButtons;
        }
    }
    0
}

/// Open a folder or link with the system (Explorer, Steam).
pub fn open(target: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", target])
            .creation_flags(0x0800_0000)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(target).spawn();
    }
}
