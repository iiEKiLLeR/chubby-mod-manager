//! Opening .rar mods. RAR cannot be unpacked by free Rust code, so this uses UnRAR.dll
//! (the same library the original imports; a copy is built in), or 7-Zip / WinRAR if present.

use std::path::{Path, PathBuf};

/// The person's own UnRAR.dll, taken from their Fluffy Mod Manager download (vendor/unrar.dll),
/// built into the program so .rar mods work without the Fluffy folder.
#[cfg(windows)]
const BUNDLED_UNRAR: &[u8] = include_bytes!("../vendor/unrar.dll");

pub fn extract(archive: &Path, dest: &Path, dll_dirs: &[PathBuf]) -> Result<(), String> {
    #[cfg(windows)]
    {
        for d in dll_dirs {
            for name in ["unrar.dll", "UnRAR64.dll", "unrar64.dll"] {
                let p = d.join(name);
                if p.is_file() && std::fs::metadata(&p).map(|m| m.len() > 0).unwrap_or(false) {
                    return dll::extract(&p, archive, dest);
                }
            }
        }
        // None found: write the built-in copy into the data folder and use it.
        if let Some(d) = dll_dirs.first() {
            let p = d.join("unrar.dll");
            if crate::fsx::write_atomic(&p, BUNDLED_UNRAR).is_ok() {
                return dll::extract(&p, archive, dest);
            }
        }
    }
    let tools: [(&str, Vec<String>); 4] = [
        ("C:/Program Files/7-Zip/7z.exe", vec!["x".into(), "-y".into(), format!("-o{}", dest.display())]),
        ("C:/Program Files (x86)/7-Zip/7z.exe", vec!["x".into(), "-y".into(), format!("-o{}", dest.display())]),
        ("C:/Program Files/WinRAR/UnRAR.exe", vec!["x".into(), "-y".into(), "-o+".into()]),
        ("C:/Program Files (x86)/WinRAR/UnRAR.exe", vec!["x".into(), "-y".into(), "-o+".into()]),
    ];
    for (exe, args) in tools {
        if Path::new(exe).is_file() {
            let mut cmd = std::process::Command::new(exe);
            cmd.args(&args).arg(archive);
            if exe.ends_with("UnRAR.exe") {
                cmd.arg(format!("{}\\", dest.display()));
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000);
            }
            let st = cmd.status().map_err(|e| e.to_string())?;
            return if st.success() { Ok(()) } else { Err(format!("{} could not unpack it", exe)) };
        }
    }
    Err("no RAR reader found. Set your Fluffy Mod Manager folder in Options (it contains unrar.dll), or install 7-Zip.".into())
}

#[cfg(windows)]
mod dll {
    use std::path::Path;
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    #[repr(C)]
    struct OpenData {
        arc_name: *const i8,
        arc_name_w: *const u16,
        open_mode: u32,
        open_result: u32,
        cmt_buf: *mut i8,
        cmt_buf_size: u32,
        cmt_size: u32,
        cmt_state: u32,
        flags: u32,
        callback: usize,
        user_data: isize,
        op_flags: u32,
        cmt_buf_w: *mut u16,
        reserved: [u32; 25],
    }

    type OpenFn = unsafe extern "system" fn(*mut OpenData) -> isize;
    type CloseFn = unsafe extern "system" fn(isize) -> i32;
    type ReadHeaderFn = unsafe extern "system" fn(isize, *mut u8) -> i32;
    type ProcessFn = unsafe extern "system" fn(isize, i32, *const u16, *const u16) -> i32;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn extract(dll: &Path, archive: &Path, dest: &Path) -> Result<(), String> {
        unsafe {
            let lib = LoadLibraryW(wide(&dll.to_string_lossy()).as_ptr());
            if lib.is_null() {
                return Err(format!("could not load {}", dll.display()));
            }
            let get = |n: &str| GetProcAddress(lib, format!("{}\0", n).as_ptr());
            let (Some(open), Some(close), Some(read), Some(process)) = (get("RAROpenArchiveEx"), get("RARCloseArchive"), get("RARReadHeaderEx"), get("RARProcessFileW")) else {
                FreeLibrary(lib);
                return Err("unrar.dll is missing functions".into());
            };
            let open: OpenFn = std::mem::transmute(open);
            let close: CloseFn = std::mem::transmute(close);
            let read: ReadHeaderFn = std::mem::transmute(read);
            let process: ProcessFn = std::mem::transmute(process);
            let arc = wide(&archive.to_string_lossy());
            let mut od: OpenData = std::mem::zeroed();
            od.arc_name_w = arc.as_ptr();
            od.open_mode = 1; // RAR_OM_EXTRACT
            let h = open(&mut od);
            if h == 0 || od.open_result != 0 {
                FreeLibrary(lib);
                return Err(format!("RAR open error {}", od.open_result));
            }
            let dst = wide(&dest.to_string_lossy());
            let mut header = vec![0u64; 4096]; // 32 KB, larger than RARHeaderDataEx
            let mut result = Ok(());
            loop {
                let r = read(h, header.as_mut_ptr() as *mut u8);
                if r == 10 {
                    break; // end of archive
                }
                if r != 0 {
                    result = Err(format!("RAR header error {}", r));
                    break;
                }
                let p = process(h, 2, dst.as_ptr(), std::ptr::null()); // RAR_EXTRACT
                if p != 0 {
                    result = Err(format!("RAR extract error {} (damaged or password protected?)", p));
                    break;
                }
            }
            close(h);
            FreeLibrary(lib);
            result
        }
    }
}
