//! Small, careful file helpers: every replace goes through a temporary file and a rename,
//! so a crash or power cut never leaves a half-written game file behind.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = tmp_name(path);
    {
        let mut f = File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    replace(&tmp, path)
}

fn tmp_name(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".remh-tmp");
    PathBuf::from(s)
}

/// Rename `from` over `to`, clearing a read-only flag on the target first.
pub fn replace(from: &Path, to: &Path) -> io::Result<()> {
    if let Ok(meta) = fs::metadata(to) {
        let mut perm = meta.permissions();
        if perm.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            let _ = fs::set_permissions(to, perm);
        }
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            // Rename across drives or onto a locked file: fall back to copy + delete.
            fs::copy(from, to)?;
            let _ = fs::remove_file(from);
            Ok(())
        }
    }
}

/// Copy a file safely and return the CRC-32 of what was written.
pub fn copy_with_crc(src: &Path, dst: &Path) -> io::Result<u32> {
    if let Some(p) = dst.parent() {
        fs::create_dir_all(p)?;
    }
    let tmp = tmp_name(dst);
    let mut hasher = crc32fast::Hasher::new();
    {
        let mut input = File::open(src)?;
        let mut out = File::create(&tmp)?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n])?;
        }
        out.flush()?;
    }
    replace(&tmp, dst)?;
    Ok(hasher.finalize())
}

pub fn crc_file(path: &Path) -> io::Result<u32> {
    let mut f = File::open(path)?;
    let mut hasher = crc32fast::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize())
}

/// All files under `dir`, as paths relative to `dir` with forward slashes.
pub fn walk_files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                stack.push(p);
            } else if ft.is_file() {
                if let Ok(rel) = p.strip_prefix(dir) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    out.sort();
    out
}

/// Delete empty folders from `start` upwards, never touching `stop` itself.
pub fn prune_empty_dirs(start: &Path, stop: &Path) {
    let mut cur = start.to_path_buf();
    while cur.starts_with(stop) && cur != stop {
        if fs::remove_dir(&cur).is_err() {
            break;
        }
        match cur.parent() {
            Some(p) => cur = p.to_path_buf(),
            None => break,
        }
    }
}

/// True when a relative path stays inside its folder (no "..", no drive letters).
pub fn safe_rel(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.starts_with('/')
        && !rel.contains(':')
        && rel.split('/').all(|c| c != ".." && c != ".")
}

pub fn human_size(bytes: u64) -> String {
    let b = bytes as f64;
    if b >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} GB", b / 1024.0 / 1024.0 / 1024.0)
    } else if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB", b / 1024.0 / 1024.0)
    } else if b >= 1024.0 {
        format!("{:.0} KB", b / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

pub fn now_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (h, m, s) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
    format!("{:02}:{:02}:{:02} UTC", h, m, s)
}
