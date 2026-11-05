//! Settings file in the same simple "key: value" style the original used (Data\config.cfg).

use std::fs;
use std::path::{Path, PathBuf};

pub struct Config {
    path: PathBuf,
    entries: Vec<(String, String)>,
}

/// Defaults. Numbers marked in MECHANICS.md with their source.
const DEFAULTS: &[(&str, &str)] = &[
    ("win_width", "1280"),
    ("win_height", "720"),
    ("player_name", "Player"),
    ("network_ip", "127.0.0.1"),
    ("network_port", "9950"),
    ("MaxFPS", "60"),
    ("NoFocusMaxFPS", "30"),
    ("LimitFPS", "2"),
    ("NoFocusLimitFPS", "2"),
    ("AccurateSleepTiming", "0"),
    ("GameRunningFPS", "2"),
    ("SleepWhileGameRuns", "1"),
    ("MinimizeWhenGameStarts", "1"),
    ("ExitWhenGameStarts", "0"),
    ("BoostGamePriority", "1"),
    ("PauseTransfersDuringGame", "1"),
    ("AutoDeleteBackupFiles", "1"),
    ("ShowModPreviewScreenshots", "1"),
    ("WatchModsFolder", "1"),
    ("FluffyFolder", ""),
    ("current_game", "re4r"),
    ("theme", "obsidian_cyan"),
];

impl Config {
    pub fn load(path: &Path) -> Config {
        let mut c = Config { path: path.to_path_buf(), entries: Vec::new() };
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    let k = k.trim();
                    if !k.is_empty() {
                        c.entries.push((k.to_string(), v.trim().to_string()));
                    }
                }
            }
        }
        for (k, v) in DEFAULTS {
            if c.raw(k).is_none() {
                c.entries.push((k.to_string(), v.to_string()));
            }
        }
        c
    }

    fn raw(&self, key: &str) -> Option<&String> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get(&self, key: &str) -> String {
        self.raw(key).cloned().unwrap_or_default()
    }

    pub fn get_i(&self, key: &str) -> i64 {
        self.get(key).trim().parse().unwrap_or(0)
    }

    pub fn flag(&self, key: &str) -> bool {
        self.get_i(key) != 0
    }

    pub fn set(&mut self, key: &str, value: impl ToString) {
        let v = value.to_string();
        if let Some(e) = self.entries.iter_mut().find(|(k, _)| k == key) {
            e.1 = v;
        } else {
            self.entries.push((key.to_string(), v));
        }
    }

    pub fn toggle(&mut self, key: &str) {
        let v = !self.flag(key);
        self.set(key, v as i32);
    }

    pub fn save(&self) {
        let mut out = String::new();
        for (k, v) in &self.entries {
            out.push_str(k);
            out.push_str(": ");
            out.push_str(v);
            out.push('\n');
        }
        let _ = crate::fsx::write_atomic(&self.path, out.as_bytes());
    }
}
