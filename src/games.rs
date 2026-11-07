//! The Resident Evil games the manager knows, and finding where they are installed.
//! Ids, folder names and program names follow the original's game table (strings near 0x140221300).

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct GameDef {
    pub id: &'static str,        // short id used in config ("re4r")
    pub folder: &'static str,    // folder under Games\ ("RE4R")
    pub name: &'static str,      // shown name
    pub exe: &'static str,       // program name inside the install folder
    pub steam_app: u32,          // Steam app id (0 = unknown)
    pub re_engine: bool,         // RE Engine game: root .pak mods become patch paks
}

pub const GAMES: &[GameDef] = &[
    GameDef { id: "rehd", folder: "REHD", name: "Resident Evil HD Remaster", exe: "bhd.exe", steam_app: 304240, re_engine: false },
    GameDef { id: "re0hd", folder: "RE0HD", name: "Resident Evil 0 HD", exe: "re0hd.exe", steam_app: 339340, re_engine: false },
    GameDef { id: "re2r", folder: "RE2R", name: "Resident Evil 2 Remake", exe: "re2.exe", steam_app: 883710, re_engine: true },
    GameDef { id: "re3r", folder: "RE3R", name: "Resident Evil 3 Remake", exe: "re3.exe", steam_app: 952060, re_engine: true },
    GameDef { id: "re4", folder: "RE4", name: "Resident Evil 4 (2005)", exe: "bio4.exe", steam_app: 254700, re_engine: false },
    GameDef { id: "re4r", folder: "RE4R", name: "Resident Evil 4 Remake", exe: "re4.exe", steam_app: 2050650, re_engine: true },
    GameDef { id: "re5", folder: "RE5", name: "Resident Evil 5", exe: "re5dx9.exe", steam_app: 21690, re_engine: false },
    GameDef { id: "re6", folder: "RE6", name: "Resident Evil 6", exe: "BH6.exe", steam_app: 221040, re_engine: false },
    GameDef { id: "re7", folder: "RE7", name: "Resident Evil 7", exe: "re7.exe", steam_app: 418370, re_engine: true },
    GameDef { id: "re8", folder: "RE8", name: "Resident Evil 8 Village", exe: "re8.exe", steam_app: 1196590, re_engine: true },
    GameDef { id: "rev", folder: "REV", name: "Resident Evil Revelations", exe: "rerev.exe", steam_app: 222480, re_engine: false },
    GameDef { id: "rev2", folder: "REV2", name: "Resident Evil Revelations 2", exe: "rerev2.exe", steam_app: 287290, re_engine: false },
    GameDef { id: "resistance", folder: "Resistance", name: "Resident Evil Resistance", exe: "REResistance.exe", steam_app: 952070, re_engine: true },
];

pub fn by_id(id: &str) -> usize {
    GAMES.iter().position(|g| g.id.eq_ignore_ascii_case(id)).unwrap_or(5)
}

/// Steam's own folder, from the registry, or the usual default.
fn steam_root() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Registry::*;
        let key: Vec<u16> = "Software\\Valve\\Steam\0".encode_utf16().collect();
        let val: Vec<u16> = "SteamPath\0".encode_utf16().collect();
        // Same order as the original (0x14007c580): current user first, then the machine key.
        for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            let mut buf = vec![0u16; 1024];
            let mut size: u32 = (buf.len() * 2) as u32;
            let rc = unsafe {
                RegGetValueW(hive, key.as_ptr(), val.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr() as *mut _, &mut size)
            };
            if rc == 0 {
                let len = buf.iter().position(|&c| c == 0).unwrap_or(0);
                let s = String::from_utf16_lossy(&buf[..len]);
                if !s.is_empty() {
                    return Some(PathBuf::from(s));
                }
            }
        }
    }
    let d = PathBuf::from("C:/Program Files (x86)/Steam");
    if d.exists() { Some(d) } else { None }
}

/// Every Steam library's "steamapps/common" folder.
pub fn steam_common_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Some(root) = steam_root() else { return out };
    // Libraries from libraryfolders.vdf ("path") and, like the original, config\config.vdf
    // ("BaseInstallFolder_N"), then Steam's own folder (function 0x14007c580).
    let mut lines = Vec::new();
    if let Ok(vdf) = fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        lines.extend(vdf.lines().filter(|l| l.trim().starts_with("\"path\"")).map(|l| l.to_string()));
    }
    if let Ok(vdf) = fs::read_to_string(root.join("config").join("config.vdf")) {
        lines.extend(vdf.lines().filter(|l| l.trim().starts_with("\"BaseInstallFolder_")).map(|l| l.to_string()));
    }
    out.push(root.join("steamapps").join("common"));
    {
        for line in &lines {
            let t = line.trim();
            {
                let parts: Vec<&str> = t.split('"').collect();
                if parts.len() >= 4 {
                    let p = PathBuf::from(parts[3].replace("\\\\", "\\")).join("steamapps").join("common");
                    if !out.contains(&p) {
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

/// Look for the game's program one or two folders deep in every Steam library.
pub fn detect_install(g: &GameDef, common: &[PathBuf]) -> Option<PathBuf> {
    for c in common {
        let Ok(rd) = fs::read_dir(c) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.join(g.exe).is_file() {
                return Some(p);
            }
            if g.exe == "bio4.exe" && p.join("Bin32").join("bio4.exe").is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// Look for an unpacked Fluffy Mod Manager in the usual download places on every drive.
pub fn detect_fluffy() -> Option<PathBuf> {
    let mut places: Vec<PathBuf> = Vec::new();
    if let Ok(home) = std::env::var("USERPROFILE") {
        for sub in ["Downloads", "Desktop", "Documents"] {
            places.push(Path::new(&home).join(sub));
        }
    }
    for drive in b'C'..=b'H' {
        let root = PathBuf::from(format!("{}:/", drive as char));
        places.push(root.join("Downloads"));
        places.push(root.clone());
    }
    for p in places {
        let Ok(rd) = fs::read_dir(&p) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if n.starts_with("fluffy") && e.path().join("Modmanager.exe").is_file() {
                return Some(e.path());
            }
        }
    }
    None
}

pub fn exe_path(g: &GameDef, dir: &Path) -> PathBuf {
    let p = dir.join(g.exe);
    if !p.exists() && g.exe == "bio4.exe" {
        return dir.join("Bin32").join("bio4.exe");
    }
    p
}

/// Built-in CHARACTERS categories for each Resident Evil game (matching Fluffy Mod Manager's
/// category list for each title).
pub fn character_categories(game_id: &str) -> &'static [&'static str] {
    match game_id {
        "re2r" => &["Leon", "Claire", "Ada", "Sherry", "Annette", "Hunk", "Enemies", "Multiple"],
        "re3r" => &["Jill", "Carlos", "Tyrell", "Nicholai", "Enemies", "Multiple"],
        "re4r" => &["Ada", "Ashley", "Leon", "Luis", "Krauser", "Wesker", "Hunk", "Enemies", "Multiple"],
        "re4" => &["Leon", "Ashley", "Ada", "Luis", "Krauser", "Wesker", "Hunk", "Enemies"],
        "re5" => &["Chris", "Sheva", "Jill", "Wesker", "Josh", "Excella", "Enemies"],
        "re6" => &["Leon", "Helena", "Chris", "Piers", "Jake", "Sherry", "Ada", "Carla", "Enemies"],
        "re7" => &["Ethan", "Mia", "Zoe", "Chris", "Enemies"],
        "re8" => &["Ethan", "Chris", "Rose", "Lady Dimitrescu", "Heisenberg", "Donna", "Moreau", "Duke", "Enemies"],
        "rehd" => &["Jill", "Chris", "Rebecca", "Wesker", "Barry", "Enemies"],
        "re0hd" => &["Rebecca", "Billy", "Wesker", "Enemies"],
        "rev" => &["Jill", "Chris", "Parker", "Jessica", "Keith", "Quint", "Rachel", "Hunk", "Enemies"],
        "rev2" => &["Claire", "Moira", "Barry", "Natalia", "Wesker", "Hunk", "Enemies"],
        "resistance" => &["Survivors", "Masterminds", "Creatures"],
        _ => &["Main Characters", "Enemies", "Multiple"],
    }
}

/// Built-in OTHER categories for each Resident Evil game.
pub fn other_categories(game_id: &str) -> &'static [&'static str] {
    match game_id {
        "re4r" => &["Co-op & Multiplayer", "RE Framework", "Weapons", "Models", "Textures", "Animations", "Music", "Text", "Misc"],
        _ if GAMES.iter().any(|g| g.id == game_id && g.re_engine) => {
            &["RE Framework", "Weapons", "Models", "Textures", "Animations", "Music", "Text", "Misc"]
        }
        _ => &["Weapons", "Models", "Textures", "Animations", "Music", "Text", "Misc"],
    }
}
