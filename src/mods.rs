//! The mod library for one game: finding mods (folders, .zip, .7z, .rar, .pak),
//! installing them with backups, uninstalling them cleanly, checking and repairing.

use crate::fsx;
use crate::games::GameDef;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Archive types the original accepts on drop (window procedure 0x140042aa0: "rar", "7z", "7zip", "zip").
pub const ARCHIVE_EXTS: &[&str] = &["zip", "7z", "7zip", "rar"];

#[derive(Clone, Default)]
pub struct ModEntry {
    pub key: String,
    pub source_name: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub categories: Vec<String>,
    pub cat_group: String,
    pub cat_name: String,
    pub screenshot: Option<PathBuf>,
    pub root: PathBuf,
    pub files: Vec<String>,
    pub size: u64,
    pub crc: u32,
    pub installed: bool,
    pub conflicts: Vec<String>,
    pub is_archive: bool,
    pub needs_reframework: bool,
    pub homepage: String,
    pub requires: String,
    pub addon_for: String,
    pub menu_priority: i32,
    pub unicode_info: bool,
}

#[derive(Clone, Default)]
pub struct InstalledFile {
    pub src_rel: String,
    pub dest: String,
    pub crc: u32,
}

#[derive(Clone, Default)]
pub struct InstalledMod {
    pub key: String,
    pub name: String,
    pub crc: u32,
    pub files: Vec<InstalledFile>,
}

#[derive(Clone, Default)]
pub struct State {
    pub mods: Vec<InstalledMod>,
    pub originals: Vec<(String, bool)>,
    pub game_stamp: String,
}

#[derive(Clone)]
pub struct Library {
    pub game: GameDef,
    pub game_dir: Option<PathBuf>,
    pub base: PathBuf,
    pub mods: Vec<ModEntry>,
    pub state: State,
    pub auto_delete_backups: bool,
    pub game_updated: bool,
    pub warnings: Vec<String>,
    pub rar_dirs: Vec<PathBuf>,
    /// Files done / total during an install or uninstall, read by the window for its progress line.
    pub progress: std::sync::Arc<(std::sync::atomic::AtomicUsize, std::sync::atomic::AtomicUsize)>,
}

#[derive(Default, Clone)]
pub struct VerifyReport {
    pub checked: usize,
    pub missing: Vec<String>,
    pub changed: Vec<String>,
}

fn pak_number(name: &str) -> Option<u32> {
    let l = name.to_ascii_lowercase();
    let rest = l.strip_prefix("re_chunk_000.pak.patch_")?;
    rest.strip_suffix(".pak")?.parse().ok()
}

fn pak_name(n: u32) -> String {
    format!("re_chunk_000.pak.patch_{:03}.pak", n)
}

/// 16-byte empty KPKA v4 PAK header written by Fluffy Mod Manager (function 0x1400de800)
/// as a 5-file barrier (`re_chunk_000.pak.patch_NNN.pak`) between official game patch PAKs
/// and mod patch PAKs so game updates do not collide with mod PAK numbers.
const DUMMY_PAK: &[u8; 16] = b"KPKA\x04\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

fn is_dummy_pak(p: &Path) -> bool {
    fs::metadata(p).map(|m| m.len() == 16).unwrap_or(false)
        && fs::read(p).map(|b| b.as_slice() == DUMMY_PAK).unwrap_or(false)
}

/// Ensure REFramework's `re2_fw_config.txt` enables `LooseFileLoader_Enabled=true`
/// (required for `natives/stm/...` loose-file mods to load in RE Engine games)
/// and uses F5 (VK_F5 = 116) for the menu keybind.
pub fn ensure_reframework_config(game_dir: &Path) {
    let fw_cfg = game_dir.join("re2_fw_config.txt");
    if fw_cfg.is_file() {
        if let Ok(txt) = fs::read_to_string(&fw_cfg) {
            let mut new_txt = txt.clone();
            if new_txt.contains("LooseFileLoader_Enabled=false") {
                new_txt = new_txt.replace("LooseFileLoader_Enabled=false", "LooseFileLoader_Enabled=true");
            } else if !new_txt.contains("LooseFileLoader_Enabled=") {
                if !new_txt.ends_with('\n') && !new_txt.is_empty() {
                    new_txt.push_str("\r\n");
                }
                new_txt.push_str("LooseFileLoader_Enabled=true\r\n");
            }
            if new_txt.contains("REFrameworkConfig_MenuKey_V2=45") {
                new_txt = new_txt.replace("REFrameworkConfig_MenuKey_V2=45", "REFrameworkConfig_MenuKey_V2=116");
            }
            if new_txt != txt {
                let _ = fsx::write_atomic(&fw_cfg, new_txt.as_bytes());
            }
        }
    } else if game_dir.join("dinput8.dll").is_file() {
        let default_cfg = "LooseFileLoader_Enabled=true\r\nREFrameworkConfig_MenuKey_V2=116\r\n";
        let _ = fsx::write_atomic(&fw_cfg, default_cfg.as_bytes());
    }
}

fn ext_of(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default()
}

fn mtime(p: &Path) -> u64 {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

const SKIP_DIRS: &[&str] = &[".git", "__macosx", ".github"];
const GAME_TOP_DIRS: &[&str] = &["natives", "reframework", "pak_mods", "nativepc", "bin32", "nativedx11x64", "archive", "mods"];

impl Library {
    pub fn new(game: GameDef, game_dir: Option<PathBuf>, games_root: &Path) -> Library {
        let base = games_root.join(game.folder);
        let _ = fs::create_dir_all(base.join("Mods"));
        let _ = fs::create_dir_all(base.join("Presets"));
        let mut lib = Library {
            game,
            game_dir,
            base,
            mods: Vec::new(),
            state: State::default(),
            auto_delete_backups: true,
            game_updated: false,
            warnings: Vec::new(),
            rar_dirs: Vec::new(),
            progress: Default::default(),
        };
        lib.load_state();
        lib
    }

    pub fn mods_dir(&self) -> PathBuf {
        self.base.join("Mods")
    }
    fn backup_dir(&self) -> PathBuf {
        self.base.join("Backup")
    }
    fn staging_dir(&self) -> PathBuf {
        self.base.join("Staging")
    }
    fn state_path(&self) -> PathBuf {
        self.base.join("installed.txt")
    }
    pub fn presets_dir(&self) -> PathBuf {
        self.base.join("Presets")
    }

    fn game_stamp(&self) -> String {
        match &self.game_dir {
            Some(d) => {
                let exe = crate::games::exe_path(&self.game, d);
                let size = fs::metadata(&exe).map(|m| m.len()).unwrap_or(0);
                format!("{}-{}", size, mtime(&exe))
            }
            None => String::new(),
        }
    }

    // ---------- installed state ----------

    fn load_state(&mut self) {
        let mut st = State::default();
        if let Ok(text) = fs::read_to_string(self.state_path()) {
            for line in text.lines() {
                let p: Vec<&str> = line.split('\t').collect();
                match p.as_slice() {
                    ["stamp", s] => st.game_stamp = s.to_string(),
                    ["orig", d, f] => st.originals.push((d.to_string(), *f == "1")),
                    ["mod", k, n, c] => st.mods.push(InstalledMod {
                        key: k.to_string(),
                        name: n.to_string(),
                        crc: u32::from_str_radix(c, 16).unwrap_or(0),
                        files: Vec::new(),
                    }),
                    ["file", s, d, c] => {
                        if let Some(m) = st.mods.last_mut() {
                            m.files.push(InstalledFile {
                                src_rel: s.to_string(),
                                dest: d.to_string(),
                                crc: u32::from_str_radix(c, 16).unwrap_or(0),
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
        self.state = st;
        if !self.state.mods.is_empty() && !self.state.game_stamp.is_empty() {
            let now = self.game_stamp();
            if !now.is_empty() && now != self.state.game_stamp {
                self.game_updated = true;
            }
        }
    }

    pub fn save_state_public(&mut self) {
        self.save_state();
    }

    fn save_state(&mut self) {
        self.state.game_stamp = self.game_stamp();
        let mut out = String::new();
        out.push_str(&format!("stamp\t{}\n", self.state.game_stamp));
        for (d, f) in &self.state.originals {
            out.push_str(&format!("orig\t{}\t{}\n", d, if *f { 1 } else { 0 }));
        }
        for m in &self.state.mods {
            out.push_str(&format!("mod\t{}\t{}\t{:08x}\n", m.key, m.name.replace('\t', " "), m.crc));
            for f in &m.files {
                out.push_str(&format!("file\t{}\t{}\t{:08x}\n", f.src_rel, f.dest, f.crc));
            }
        }
        let _ = fsx::write_atomic(&self.state_path(), out.as_bytes());
    }

    pub fn is_installed(&self, key: &str) -> bool {
        self.state.mods.iter().any(|m| m.key == key)
    }

    // ---------- scanning ----------

    /// Rebuild the list of mods from the Mods folder. Archives are unpacked once into
    /// Staging and reused until the archive changes, so later scans and installs are fast.
    pub fn scan(&mut self) {
        let mut crc_cache = self.load_crc_cache();
        let mut found = Vec::new();
        self.warnings.clear();
        let Ok(rd) = fs::read_dir(self.mods_dir()) else { return };
        let mut entries: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
        entries.sort();
        let mut live_staging = Vec::new();
        for p in entries {
            let fname = p.file_name().unwrap().to_string_lossy().to_string();
            if fname.ends_with(".part") || fname.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                let crc = folder_crc(&p);
                let size = fsx::walk_files(&p).iter().map(|r| fs::metadata(p.join(r)).map(|m| m.len()).unwrap_or(0)).sum();
                self.add_roots(&p, &fname, size, crc, false, &mut found);
                continue;
            }
            let ext = ext_of(&p);
            let size = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            if ext == "pak" {
                if size == 0 {
                    continue;
                }
                let crc = cached_crc(&mut crc_cache, &p);
                found.push(ModEntry {
                    key: fname.clone(),
                    source_name: fname.clone(),
                    name: p.file_stem().unwrap().to_string_lossy().to_string(),
                    root: self.mods_dir(),
                    files: vec![fname.clone()],
                    size,
                    crc,
                    is_archive: true,
                    ..Default::default()
                });
            } else if ARCHIVE_EXTS.contains(&ext.as_str()) {
                if is_later_rar_part(&fname) {
                    continue;
                }
                let crc = cached_crc(&mut crc_cache, &p);
                let stage = self.staging_dir().join(sanitize(&fname));
                live_staging.push(stage.clone());
                match ensure_staged(&p, &stage, crc, &self.rar_dirs) {
                    Ok(()) => self.add_roots(&stage, &fname, size, crc, true, &mut found),
                    Err(e) => self.warnings.push(format!("Could not open {}: {}", fname, e)),
                }
            }
        }
        // Remove stale unpacked copies of archives that are gone.
        if let Ok(rd) = fs::read_dir(self.staging_dir()) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() && !live_staging.contains(&p) {
                    let _ = fs::remove_dir_all(&p);
                }
            }
        }
        self.save_crc_cache(&crc_cache);
        self.migrate_legacy_pak_mods(&found);
        // Installed state, conflicts, hints.
        let mut owners: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, m) in found.iter().enumerate() {
            for f in &m.files {
                if let Some(d) = self.plain_dest(f) {
                    owners.entry(d.to_ascii_lowercase()).or_default().push(i);
                }
            }
        }
        let names: Vec<String> = found.iter().map(|m| m.name.clone()).collect();
        let has_rf = self.game_dir.as_ref().map(|d| d.join("dinput8.dll").exists()).unwrap_or(true);
        let cat_overrides = self.load_category_overrides();
        for (i, m) in found.iter_mut().enumerate() {
            m.installed = self.state.mods.iter().any(|s| s.key == m.key);
            let mut c: Vec<String> = Vec::new();
            for f in &m.files {
                if let Some(d) = plain_dest_static(self.game.re_engine, f) {
                    for &j in &owners[&d.to_ascii_lowercase()] {
                        if j != i && !c.contains(&names[j]) {
                            c.push(names[j].clone());
                        }
                    }
                }
            }
            m.conflicts = c;
            m.needs_reframework = self.game.re_engine
                && !has_rf
                && m.files.iter().any(|f| {
                    let fl = f.to_ascii_lowercase();
                    fl.starts_with("natives/") || fl.starts_with("reframework/")
                });
            let (grp, cname, tags) = classify_mod(
                self.game.id,
                &m.name,
                &m.description,
                &m.categories,
                &m.files,
                cat_overrides.get(&m.key),
            );
            m.cat_group = grp;
            m.cat_name = cname;
            m.categories = tags;
        }
        // Installed mods whose source vanished still show, so they can be uninstalled.
        for s in &self.state.mods {
            if !found.iter().any(|m| m.key == s.key) {
                let (grp, cname, tags) = classify_mod(self.game.id, &s.name, "", &[], &[], cat_overrides.get(&s.key));
                found.push(ModEntry {
                    key: s.key.clone(),
                    source_name: s.key.clone(),
                    name: format!("{} (source file missing)", s.name),
                    installed: true,
                    crc: s.crc,
                    cat_group: grp,
                    cat_name: cname,
                    categories: tags,
                    ..Default::default()
                });
            }
        }
        for m in &found {
            if m.unicode_info {
                self.warnings.push(format!("The modinfo.ini for the mod \"{}\" is unicode. This is not supported.", m.name));
            }
        }
        if let Some(w) = self.other_manager_warning() {
            self.warnings.push(w);
        }
        // Higher MenuPriority first (modinfo "MenuPriority", default 0), then by name.
        found.sort_by(|a, b| b.menu_priority.cmp(&a.menu_priority).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        self.mods = found;
    }

    fn add_roots(&self, dir: &Path, source: &str, size: u64, crc: u32, is_archive: bool, out: &mut Vec<ModEntry>) {
        let roots = find_roots(dir);
        let multi = roots.len() > 1;
        for root in roots {
            let mut m = ModEntry {
                source_name: source.to_string(),
                root: root.clone(),
                size,
                crc,
                is_archive,
                ..Default::default()
            };
            let sub = root.strip_prefix(dir).map(|r| r.to_string_lossy().replace('\\', "/")).unwrap_or_default();
            m.key = if multi && !sub.is_empty() { format!("{}|{}", source, sub) } else { source.to_string() };
            let stem = Path::new(source).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            m.name = if multi { root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or(stem) } else { stem };
            let mut shot_name = String::new();
            if let Some(ini) = find_ci(&root, "modinfo.ini") {
                // Rules from the original's reader (function 0x140094720): a UTF-16 file is refused,
                // each line splits at its first '=' only, key and value lose leading/trailing spaces
                // and tabs, empty values are ignored, keys ignore capital letters (compare function
                // 0x14019a040), at most 10 categories.
                let bytes = fs::read(&ini).unwrap_or_default();
                if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
                    m.unicode_info = true;
                } else {
                    let text = String::from_utf8_lossy(&bytes);
                    for line in text.lines() {
                        let line = line.trim_start_matches('\u{feff}');
                        let Some(cut) = line.find('=') else { continue };
                        let k = line[..cut].trim_matches([' ', '\t']).to_ascii_lowercase();
                        let v = line[cut + 1..].trim_matches([' ', '\t', '\r']).to_string();
                        if v.is_empty() {
                            continue;
                        }
                        match k.as_str() {
                            "name" => m.name = v,
                            "version" => m.version = v,
                            "author" => m.author = v,
                            "description" => m.description = v.replace("\\n", "\n"),
                            "screenshot" => shot_name = v,
                            "category" if m.categories.len() < 10 => m.categories.push(v),
                            "homepage" => m.homepage = v,
                            "addonfor" => m.addon_for = v,
                            "requirement" | "required" | "requires" | "require" => m.requires = v,
                            "menupriority" => m.menu_priority = v.parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
            }
            if !shot_name.is_empty() && root.join(&shot_name).is_file() {
                m.screenshot = Some(root.join(&shot_name));
            } else {
                for n in ["screenshot.png", "screenshot.jpg", "screenshot.jpeg"] {
                    if let Some(p) = find_ci(&root, n) {
                        m.screenshot = Some(p);
                        break;
                    }
                }
            }
            let shot_rel = m.screenshot.as_ref().and_then(|s| s.file_name()).map(|n| n.to_string_lossy().to_lowercase());
            m.files = fsx::walk_files(&root)
                .into_iter()
                .filter(|rel| {
                    let l = rel.to_lowercase();
                    if l.split('/').any(|c| SKIP_DIRS.contains(&c)) {
                        return false;
                    }
                    if l.ends_with(".pak") && fs::metadata(root.join(rel)).map(|m| m.len() == 0).unwrap_or(false) {
                        return false;
                    }
                    if !l.contains('/') {
                        if l == "modinfo.ini" || Some(&l) == shot_rel.as_ref() {
                            return false;
                        }
                        let e = l.rsplit('.').next().unwrap_or("");
                        if ["txt", "md", "url", "png", "jpg", "jpeg", "ini", "nfo", "pdf", "remh-stamp"].contains(&e) && l != "dinput8.ini" {
                            return false;
                        }
                    }
                    // Folders of other mods packed in the same archive are not ours.
                    fsx::safe_rel(rel)
                })
                .collect();
            if !m.files.is_empty() {
                out.push(m);
            }
        }
    }

    fn plain_dest(&self, rel: &str) -> Option<String> {
        plain_dest_static(self.game.re_engine, rel)
    }

    /// Migrate any `.pak` mod files that an earlier build placed in `pak_mods/x0001-*.pak`
    /// over to `re_chunk_000.pak.patch_NNN.pak` (with the 5-file dummy PAK barrier), and
    /// ensure `re2_fw_config.txt` enables `LooseFileLoader_Enabled=true`.
    fn migrate_legacy_pak_mods(&mut self, found: &[ModEntry]) {
        if !self.game.re_engine {
            return;
        }
        let Some(gd) = self.game_dir.clone() else { return };
        ensure_reframework_config(&gd);
        let mut changed = false;
        let mut pending: Vec<InstalledFile> = Vec::new();
        for mi in 0..self.state.mods.len() {
            for fi in 0..self.state.mods[mi].files.len() {
                let src_rel = self.state.mods[mi].files[fi].src_rel.clone();
                let old_dest = self.state.mods[mi].files[fi].dest.clone();
                if self.plain_dest(&src_rel).is_none() && old_dest.to_ascii_lowercase().starts_with("pak_mods/") {
                    let new_dest = pak_name(self.allocate_patch_pak(&gd, &pending));
                    let old_path = gd.join(&old_dest);
                    let new_path = gd.join(&new_dest);
                    let moved = if old_path.is_file() {
                        fs::rename(&old_path, &new_path).is_ok()
                            || copy_retry(&old_path, &new_path).map(|_| { let _ = fs::remove_file(&old_path); }).is_ok()
                    } else if let Some(src_mod) = found.iter().find(|m| m.key == self.state.mods[mi].key) {
                        copy_retry(&src_mod.root.join(&src_rel), &new_path).is_ok()
                    } else {
                        false
                    };
                    if moved {
                        if let Some(p) = old_path.parent() {
                            fsx::prune_empty_dirs(p, &gd);
                        }
                        self.state.originals.retain(|(d, _)| !d.eq_ignore_ascii_case(&old_dest));
                        self.state.originals.push((new_dest.clone(), false));
                        self.state.mods[mi].files[fi].dest = new_dest;
                        pending.push(self.state.mods[mi].files[fi].clone());
                        changed = true;
                    }
                }
            }
        }
        if changed {
            self.save_state();
        }
    }

    // ---------- install / uninstall ----------

    /// Find the next patch PAK slot (`re_chunk_000.pak.patch_NNN.pak`) in `game_dir`.
    /// Matches Fluffy Mod Manager's `0x1400deb40`: scans `001..999`, ensures 5 16-byte dummy
    /// PAKs separate the game's own patch PAKs from mod patch PAKs, and returns the first free
    /// mod patch PAK number.
    fn allocate_patch_pak(&self, game_dir: &Path, pending: &[InstalledFile]) -> u32 {
        let mut seen_dummy = false;
        let mut first_free = 1u32;
        for i in 1..1000u32 {
            let name = pak_name(i);
            let p = game_dir.join(&name);
            if !p.is_file() {
                first_free = i;
                break;
            }
            if is_dummy_pak(&p) {
                seen_dummy = true;
            } else if seen_dummy {
                let owned = self.state.mods.iter().any(|m| m.files.iter().any(|f| f.dest.eq_ignore_ascii_case(&name)))
                    || pending.iter().any(|f| f.dest.eq_ignore_ascii_case(&name));
                if !owned {
                    seen_dummy = false;
                }
            }
        }
        if !seen_dummy {
            for k in 0..5u32 {
                let dp = game_dir.join(pak_name(first_free + k));
                let _ = fsx::write_atomic(&dp, DUMMY_PAK);
            }
            first_free + 5
        } else {
            first_free
        }
    }

    /// Remove trailing 16-byte dummy patch PAKs once no mod patch PAKs remain after them
    /// (matches Fluffy Mod Manager's `0x1400de900`).
    fn cleanup_dummy_paks(game_dir: &Path) {
        let mut last_real = 0u32;
        for i in 1..1000u32 {
            let p = game_dir.join(pak_name(i));
            if !p.is_file() {
                break;
            }
            if !is_dummy_pak(&p) {
                last_real = i;
            }
        }
        for i in (last_real + 1)..1000u32 {
            let p = game_dir.join(pak_name(i));
            if !p.is_file() {
                break;
            }
            if is_dummy_pak(&p) {
                let _ = fs::remove_file(&p);
            }
        }
    }

    fn owner_count(&self, dest: &str) -> usize {
        let d = dest.to_ascii_lowercase();
        self.state.mods.iter().filter(|m| m.files.iter().any(|f| f.dest.to_ascii_lowercase() == d)).count()
    }

    fn original_of(&self, dest: &str) -> Option<bool> {
        let d = dest.to_ascii_lowercase();
        self.state.originals.iter().find(|(x, _)| x.to_ascii_lowercase() == d).map(|(_, f)| *f)
    }

    pub fn install(&mut self, key: &str) -> Result<String, String> {
        let game_dir = self.game_dir.clone().ok_or("Set the game's install folder first (Options).")?;
        if self.is_installed(key) {
            return Ok("Already installed.".into());
        }
        let m = self.mods.iter().find(|m| m.key == key).cloned().ok_or("Mod not found.")?;
        if m.files.is_empty() {
            return Err("This mod has no files to install.".into());
        }
        let mut im = InstalledMod { key: m.key.clone(), name: m.name.clone(), crc: m.crc, files: Vec::new() };
        let mut overridden: Vec<String> = Vec::new();
        if self.state.mods.is_empty() {
            self.on_first_install(&game_dir);
        }
        let (mut backed_up, mut over_others) = (0, 0);
        let mut slice = std::time::Instant::now();
        self.progress.0.store(0, std::sync::atomic::Ordering::Relaxed);
        self.progress.1.store(m.files.len(), std::sync::atomic::Ordering::Relaxed);
        for rel in &m.files {
            self.progress.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            copy_slice_yield(&mut slice);
            let src = m.root.join(rel);
            let dest = match self.plain_dest(rel) {
                Some(d) => d,
                None => pak_name(self.allocate_patch_pak(&game_dir, &im.files)),
            };
            if !fsx::safe_rel(&dest) {
                continue;
            }
            let dpath = game_dir.join(&dest);
            if self.owner_count(&dest) == 0 && self.original_of(&dest).is_none() {
                if dpath.is_file() {
                    if let Err(e) = copy_retry(&dpath, &self.backup_dir().join(&dest)) {
                        self.rollback(&game_dir, &im);
                        return Err(format!("Could not back up {}: {}", dest, e));
                    }
                    backed_up += 1;
                    self.state.originals.push((dest.clone(), true));
                } else {
                    self.state.originals.push((dest.clone(), false));
                }
            } else if self.owner_count(&dest) > 0 {
                over_others += 1;
                for o in &self.state.mods {
                    if o.files.iter().any(|f| f.dest.eq_ignore_ascii_case(&dest)) && !overridden.contains(&o.name) {
                        overridden.push(o.name.clone());
                    }
                }
            }
            match copy_retry(&src, &dpath) {
                Ok(crc) => im.files.push(InstalledFile { src_rel: rel.clone(), dest, crc }),
                Err(e) => {
                    self.rollback(&game_dir, &im);
                    return Err(format!("Could not write {}: {} (is the game running or the folder read-only?)", dest, e));
                }
            }
        }
        let n = im.files.len();
        if self.game.re_engine {
            ensure_reframework_config(&game_dir);
        }
        self.state.mods.push(im);
        self.save_state();
        self.refresh_flags();
        // Same wording as the original's summary (function 0x14009da10).
        let mut msg = format!("Installed mod: {}.", m.name);
        if backed_up > 0 {
            msg.push_str(&format!(" Backed up {} files.", backed_up));
        }
        if over_others > 0 {
            msg.push_str(&format!(" Copied {} files. (overwrote {} files from other mods: {})", n, over_others, overridden.join(", ")));
        } else {
            msg.push_str(&format!(" Copied {} files.", n));
        }
        Ok(msg)
    }

    /// Undo a half-finished install (the mod isn't in the installed list yet).
    fn rollback(&mut self, game_dir: &Path, im: &InstalledMod) {
        let tmp = im.clone();
        self.state.mods.push(tmp);
        let idx = self.state.mods.len() - 1;
        self.remove_files_of(game_dir, idx);
        self.state.mods.pop();
        self.save_state();
    }

    pub fn uninstall(&mut self, key: &str) -> Result<String, String> {
        let game_dir = self.game_dir.clone().ok_or("Set the game's install folder first (Options).")?;
        let idx = self.state.mods.iter().position(|m| m.key == key).ok_or("Mod is not installed.")?;
        self.progress.0.store(0, std::sync::atomic::Ordering::Relaxed);
        self.progress.1.store(self.state.mods[idx].files.len(), std::sync::atomic::Ordering::Relaxed);
        let (problems, restored) = self.remove_files_of(&game_dir, idx);
        let im = self.state.mods.remove(idx);
        let mut extra = String::new();
        if self.state.mods.is_empty() {
            if let Some(w) = self.on_all_removed(&game_dir) {
                extra = format!(" {}", w);
            }
        }
        self.save_state();
        self.refresh_flags();
        if problems.is_empty() {
            Ok(format!("Uninstalled mod: {}. Restored {} files.{}", im.name, restored, extra))
        } else {
            Err(format!("Uninstalled mod with errors: {} ({} files failed to copy): {}", im.name, problems.len(), problems.join("; ")))
        }
    }

    /// Take mod `idx` off disk: restore whatever should be there without it.
    fn remove_files_of(&mut self, game_dir: &Path, idx: usize) -> (Vec<String>, usize) {
        let mut problems = Vec::new();
        let mut restored = 0;
        let files = self.state.mods[idx].files.clone();
        let mut slice = std::time::Instant::now();
        for f in files.iter().rev() {
            self.progress.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            copy_slice_yield(&mut slice);
            let dpath = game_dir.join(&f.dest);
            if let Some(n) = pak_number(&f.dest).filter(|_| self.plain_dest(&f.src_rel).is_none()) {
                let _ = fs::remove_file(&dpath);
                self.close_pak_gap(game_dir, n, idx);
                Self::cleanup_dummy_paks(game_dir);
                self.state.originals.retain(|(d, _)| !d.eq_ignore_ascii_case(&f.dest));
                continue;
            }
            let owns = |m: &InstalledMod| m.files.iter().any(|x| x.dest.eq_ignore_ascii_case(&f.dest));
            if self.state.mods[idx + 1..].iter().any(owns) {
                continue; // a mod installed later still covers this file
            }
            if let Some(prev) = self.state.mods[..idx].iter().rev().find(|m| owns(m)).cloned() {
                let pf = prev.files.iter().find(|x| x.dest.eq_ignore_ascii_case(&f.dest)).unwrap();
                match self.mods.iter().find(|m| m.key == prev.key) {
                    Some(src) if src.root.join(&pf.src_rel).is_file() => {
                        if let Err(e) = fsx::copy_with_crc(&src.root.join(&pf.src_rel), &dpath) {
                            problems.push(format!("{}: {}", f.dest, e));
                        }
                    }
                    _ => problems.push(format!("{}: source of '{}' is missing", f.dest, prev.name)),
                }
                continue;
            }
            match self.original_of(&f.dest) {
                Some(true) => {
                    let b = self.backup_dir().join(&f.dest);
                    match fsx::copy_with_crc(&b, &dpath) {
                        Ok(_) => {
                            restored += 1;
                            if self.auto_delete_backups {
                                let _ = fs::remove_file(&b);
                                if let Some(p) = b.parent() {
                                    fsx::prune_empty_dirs(p, &self.backup_dir());
                                }
                            }
                        }
                        Err(e) => {
                            problems.push(format!("restore {}: {}", f.dest, e));
                            continue;
                        }
                    }
                }
                _ => {
                    if dpath.exists() {
                        if let Err(e) = fs::remove_file(&dpath) {
                            problems.push(format!("delete {}: {}", f.dest, e));
                            continue;
                        }
                    }
                    if let Some(p) = dpath.parent() {
                        fsx::prune_empty_dirs(p, game_dir);
                    }
                }
            }
            self.state.originals.retain(|(d, _)| !d.eq_ignore_ascii_case(&f.dest));
        }
        (problems, restored)
    }

    /// Patch paks must be numbered without gaps: move later mod paks down by one.
    fn close_pak_gap(&mut self, game_dir: &Path, removed: u32, skip_idx: usize) {
        let mut ours: Vec<(u32, usize, usize)> = Vec::new();
        for (mi, m) in self.state.mods.iter().enumerate() {
            if mi == skip_idx {
                continue;
            }
            for (fi, f) in m.files.iter().enumerate() {
                if let Some(n) = pak_number(&f.dest) {
                    if n > removed {
                        ours.push((n, mi, fi));
                    }
                }
            }
        }
        ours.sort();
        for (n, mi, fi) in ours {
            let new = pak_name(n - 1);
            let old = self.state.mods[mi].files[fi].dest.clone();
            if fs::rename(game_dir.join(&old), game_dir.join(&new)).is_ok() {
                self.state.mods[mi].files[fi].dest = new;
            }
        }
    }

    /// Before the first mod goes in (function 0x14009da10 undoes these when the last one leaves):
    /// RE Engine games get their crash reporter renamed so Capcom isn't sent reports caused by
    /// mods (change-log, "temporarily rename the crash reporter"), and a marker file says who modded it.
    fn on_first_install(&self, game_dir: &Path) {
        if self.game.re_engine {
            let cr = game_dir.join("CrashReport.exe");
            let bk = game_dir.join(CRASH_BACKUP);
            if cr.is_file() && !bk.exists() {
                let _ = fs::rename(&cr, &bk);
            }
        }
        let _ = fs::write(game_dir.join(MARKER), "This game has mods installed by Chubby Mod Manager. Uninstall them there before verifying or reinstalling the game.\r\n");
    }

    /// After the last mod is uninstalled: remove the marker, put the crash reporter back and
    /// warn about a leftover 'natives' folder (same rules and text as function 0x14009da10).
    fn on_all_removed(&self, game_dir: &Path) -> Option<String> {
        let _ = fs::remove_file(game_dir.join(MARKER));
        if self.game.re_engine {
            let cr = game_dir.join("CrashReport.exe");
            let bk = game_dir.join(CRASH_BACKUP);
            if bk.is_file() {
                if cr.exists() {
                    let _ = fs::remove_file(&bk);
                } else {
                    let _ = fs::rename(&bk, &cr);
                }
            }
            if game_dir.join("natives").exists() {
                return Some("The 'natives' folder still exists after all mods are uninstalled. Reinstall the game if you encounter issues.".into());
            }
        }
        None
    }

    /// Another mod manager's files in the game folder while this one has nothing installed
    /// (the original warns about "remnants of modded files from another instance", 0x1400979a0).
    pub fn other_manager_warning(&self) -> Option<String> {
        let gd = self.game_dir.as_ref()?;
        if gd.join("ModdedByFluffyModManager.txt").is_file() {
            return Some("Fluffy Mod Manager still has mods installed in this game. Uninstall them in Fluffy first (or verify the game files in Steam), then install them here.".into());
        }
        if self.state.mods.is_empty() && gd.join(MARKER).is_file() {
            return Some("There are remnants of modded files from another copy of Chubby Mod Manager. Uninstall the mods there, or verify the game files in Steam.".into());
        }
        None
    }

    pub fn uninstall_all(&mut self) -> Result<String, String> {
        let keys: Vec<String> = self.state.mods.iter().rev().map(|m| m.key.clone()).collect();
        let n = keys.len();
        let mut errs = Vec::new();
        for k in keys {
            if let Err(e) = self.uninstall(&k) {
                errs.push(e);
            }
        }
        if errs.is_empty() { Ok(format!("Uninstalled all {} mods.", n)) } else { Err(errs.join(" | ")) }
    }

    pub fn refresh_flags(&mut self) {
        let keys: Vec<String> = self.state.mods.iter().map(|m| m.key.clone()).collect();
        for m in &mut self.mods {
            m.installed = keys.contains(&m.key);
        }
    }

    /// Check every installed file is still the one we put there (a game update or
    /// "verify files" in Steam can silently undo mods).
    pub fn verify(&self) -> VerifyReport {
        let mut r = VerifyReport::default();
        let Some(gd) = &self.game_dir else { return r };
        let mut seen: HashMap<String, u32> = HashMap::new();
        for m in &self.state.mods {
            for f in &m.files {
                seen.insert(f.dest.to_ascii_lowercase(), f.crc); // last installed wins
            }
        }
        for m in &self.state.mods {
            for f in &m.files {
                if seen.get(&f.dest.to_ascii_lowercase()) != Some(&f.crc) {
                    continue;
                }
                r.checked += 1;
                let p = gd.join(&f.dest);
                if !p.is_file() {
                    r.missing.push(f.dest.clone());
                } else if fsx::crc_file(&p).ok() != Some(f.crc) {
                    r.changed.push(f.dest.clone());
                }
            }
        }
        r
    }

    /// Put back every installed mod file that is missing or changed.
    pub fn repair(&mut self) -> Result<String, String> {
        let gd = self.game_dir.clone().ok_or("No game folder set.")?;
        let rep = self.verify();
        let bad: Vec<String> = rep.missing.iter().chain(rep.changed.iter()).map(|s| s.to_ascii_lowercase()).collect();
        let mut fixed = 0;
        let mut errs = Vec::new();
        for m in self.state.mods.clone().iter().rev() {
            let Some(src) = self.mods.iter().find(|x| x.key == m.key).cloned() else { continue };
            for f in &m.files {
                if bad.contains(&f.dest.to_ascii_lowercase()) {
                    match fsx::copy_with_crc(&src.root.join(&f.src_rel), &gd.join(&f.dest)) {
                        Ok(_) => fixed += 1,
                        Err(e) => errs.push(format!("{}: {}", f.dest, e)),
                    }
                }
            }
        }
        self.game_updated = false;
        self.save_state();
        if errs.is_empty() { Ok(format!("Repaired {} file(s); everything matches again.", fixed)) } else { Err(errs.join("; ")) }
    }

    // ---------- adding / removing mod files ----------

    /// Put dropped files or folders into the Mods folder. Like the original (0x140076c20) a
    /// dropped file is moved there; `copy` keeps the source (used when importing from Fluffy).
    pub fn import(&self, paths: &[PathBuf], copy: bool) -> Vec<Result<String, String>> {
        let mut out = Vec::new();
        for p in paths {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name.is_empty() {
                continue;
            }
            let ext = ext_of(p);
            if p.is_file() && !ARCHIVE_EXTS.contains(&ext.as_str()) && ext != "pak" {
                out.push(Err(format!("{}: only .zip, .7z, .7zip, .rar, .pak files or mod folders can be added.", name)));
                continue;
            }
            if ext == "pak" && !self.game.re_engine {
                out.push(Err(format!("Skipped moving {} as this is not an RE Engine game.", name)));
                continue;
            }
            let mut dest = self.mods_dir().join(&name);
            let same = fs::canonicalize(&dest).ok().zip(fs::canonicalize(p).ok()).map(|(a, b)| a == b).unwrap_or(false);
            if dest == *p || same {
                out.push(Ok(format!("Skipped moving {} as it's already at destination path.", name)));
                continue;
            }
            if dest.exists() {
                if p.is_file() && fsx::crc_file(p).ok() == fsx::crc_file(&dest).ok() {
                    out.push(Ok(format!("{} was already added (identical file).", name)));
                    continue;
                }
                let installed = self.mods.iter().any(|m| m.source_name == name && m.installed);
                if installed {
                    let stem = p.file_stem().unwrap().to_string_lossy().to_string();
                    let e = if ext.is_empty() { String::new() } else { format!(".{}", ext) };
                    let mut i = 2;
                    loop {
                        dest = self.mods_dir().join(format!("{} ({}){}", stem, i, e));
                        if !dest.exists() {
                            break;
                        }
                        i += 1;
                    }
                } else if p.is_dir() {
                    let _ = fs::remove_dir_all(&dest);
                }
            }
            let shown = dest.file_name().unwrap().to_string_lossy().to_string();
            if !copy && fs::rename(p, &dest).is_ok() {
                out.push(Ok(format!("Moved {} to mod directory.", shown)));
                continue;
            }
            // Different drive (or copying on purpose): copy, then remove the source when moving.
            let res = if p.is_dir() { copy_dir(p, &dest) } else { fsx::copy_with_crc(p, &dest).map(|_| ()) };
            match res {
                Ok(()) if copy => out.push(Ok(format!("Added {}", shown))),
                Ok(()) => {
                    let _ = if p.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) };
                    out.push(Ok(format!("Moved {} to mod directory.", shown)));
                }
                Err(e) => out.push(Err(format!("Failed to move {} to mod directory: {}", name, e))),
            }
        }
        out
    }

    pub fn delete_source(&self, key: &str) -> Result<String, String> {
        let m = self.mods.iter().find(|m| m.key == key).ok_or("Mod not found.")?;
        if m.installed {
            return Err("Uninstall the mod before deleting it.".into());
        }
        let p = self.mods_dir().join(&m.source_name);
        let r = if p.is_dir() { fs::remove_dir_all(&p) } else { fs::remove_file(&p) };
        r.map(|_| format!("Deleted {} from the Mods folder.", m.source_name)).map_err(|e| e.to_string())
    }

    // ---------- presets ----------

    pub fn save_preset(&self, name: &str) -> Result<String, String> {
        let keys: Vec<String> = self.state.mods.iter().map(|m| m.key.clone()).collect();
        fsx::write_atomic(&self.presets_dir().join(format!("{}.txt", sanitize(name))), keys.join("\n").as_bytes())
            .map(|_| format!("Saved preset '{}' ({} mods).", name, keys.len()))
            .map_err(|e| e.to_string())
    }

    pub fn list_presets(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(self.presets_dir())
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| {
                        let n = e.file_name().to_string_lossy().to_string();
                        n.strip_suffix(".txt").map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    pub fn read_preset(&self, name: &str) -> Vec<String> {
        fs::read_to_string(self.presets_dir().join(format!("{}.txt", name)))
            .map(|t| t.lines().filter(|l| !l.trim().is_empty()).map(|l| l.to_string()).collect())
            .unwrap_or_default()
    }

    /// Make the installed set exactly `want` (in that order). Used by presets and co-op sync.
    pub fn apply_set(&mut self, want: &[String]) -> Result<String, String> {
        let mut errs = Vec::new();
        // Keep the longest prefix that already matches, uninstall the rest (newest first).
        let cur: Vec<String> = self.state.mods.iter().map(|m| m.key.clone()).collect();
        let keep = cur.iter().zip(want.iter()).take_while(|(a, b)| a == b).count();
        for k in cur[keep..].iter().rev() {
            if let Err(e) = self.uninstall(k) {
                errs.push(e);
            }
        }
        let mut n = 0;
        for k in &want[keep..] {
            if self.mods.iter().any(|m| &m.key == k) {
                match self.install(k) {
                    Ok(_) => n += 1,
                    Err(e) => errs.push(e),
                }
            } else {
                errs.push(format!("missing mod {}", k));
            }
        }
        if errs.is_empty() {
            Ok(format!("Mods now match ({} installed, {} changed).", want.len(), n + cur.len() - keep))
        } else {
            Err(errs.join("; "))
        }
    }

    fn load_crc_cache(&self) -> HashMap<String, (u64, u64, u32)> {
        let mut m = HashMap::new();
        if let Ok(t) = fs::read_to_string(self.base.join("crc-cache.txt")) {
            for l in t.lines() {
                let p: Vec<&str> = l.split('\t').collect();
                if p.len() == 4 {
                    m.insert(p[0].to_string(), (p[1].parse().unwrap_or(0), p[2].parse().unwrap_or(0), u32::from_str_radix(p[3], 16).unwrap_or(0)));
                }
            }
        }
        m
    }

    fn save_crc_cache(&self, m: &HashMap<String, (u64, u64, u32)>) {
        let mut s = String::new();
        for (k, (a, b, c)) in m {
            s.push_str(&format!("{}\t{}\t{}\t{:08x}\n", k, a, b, c));
        }
        let _ = fsx::write_atomic(&self.base.join("crc-cache.txt"), s.as_bytes());
    }

    pub fn load_category_overrides(&self) -> HashMap<String, (String, String)> {
        let mut m = HashMap::new();
        if let Ok(t) = fs::read_to_string(self.base.join("categories.txt")) {
            for l in t.lines() {
                let p: Vec<&str> = l.split('\t').collect();
                if p.len() == 3 {
                    m.insert(p[0].to_string(), (p[1].to_string(), p[2].to_string()));
                }
            }
        }
        m
    }

    pub fn set_mod_category(&mut self, key: &str, group: &str, cat: &str) {
        let mut map = self.load_category_overrides();
        map.insert(key.to_string(), (group.to_string(), cat.to_string()));
        let mut s = String::new();
        for (k, (g, c)) in &map {
            s.push_str(&format!("{}\t{}\t{}\n", k, g, c));
        }
        let _ = fsx::write_atomic(&self.base.join("categories.txt"), s.as_bytes());
        if let Some(m) = self.mods.iter_mut().find(|m| m.key == key) {
            m.cat_group = group.to_string();
            m.cat_name = cat.to_string();
            if !m.categories.iter().any(|c| c.eq_ignore_ascii_case(cat)) {
                m.categories.insert(0, cat.to_string());
            } else if let Some(pos) = m.categories.iter().position(|c| c.eq_ignore_ascii_case(cat)) {
                let val = m.categories.remove(pos);
                m.categories.insert(0, val);
            }
        }
    }
}

/// Classify a mod into `("CHARACTERS" | "OTHER", CategoryName, CleanTags)` using its `modinfo.ini`
/// categories (such as Fluffy Mod Manager's `category=!Other > Misc` or `Characters > Leon`),
/// its file paths (`reframework/`, `natives/stm/_chainsaw/character/...`, `.mesh.`, `.tex.`),
/// its title, or a user-selected category override.
fn classify_mod(
    game_id: &str,
    name: &str,
    _desc: &str,
    raw_cats: &[String],
    files: &[String],
    user_override: Option<&(String, String)>,
) -> (String, String, Vec<String>) {
    let chars = crate::games::character_categories(game_id);
    let mut parsed: Vec<(String, String)> = Vec::new();
    let mut extra_tags: Vec<String> = Vec::new();

    for raw in raw_cats {
        let cleaned = raw.trim().trim_start_matches('!').trim();
        if cleaned.is_empty() {
            continue;
        }
        let (grp_hint, leaf) = if let Some((l, r)) = cleaned.split_once('>') {
            (l.trim().trim_start_matches('!').trim().to_ascii_lowercase(), r.trim().trim_start_matches('!').trim())
        } else {
            (String::new(), cleaned)
        };
        if leaf.eq_ignore_ascii_case("Chubby Enhanced") {
            extra_tags.push("Chubby Enhanced".to_string());
            continue;
        }
        if let Some(&ch) = chars.iter().find(|&&c| c.eq_ignore_ascii_case(leaf) || leaf.to_ascii_lowercase().starts_with(&c.to_ascii_lowercase())) {
            parsed.push(("CHARACTERS".to_string(), ch.to_string()));
            continue;
        }
        let ll = leaf.to_ascii_lowercase();
        let mapped = match ll.as_str() {
            "co-op" | "coop" | "multiplayer" | "co-op & multiplayer" => Some(("OTHER", "Co-op & Multiplayer")),
            "re framework" | "reframework" | "framework" | "lua" | "script" | "scripts" | "plugin" | "plugins" => Some(("OTHER", "RE Framework")),
            "weapon" | "weapons" | "gun" | "guns" => Some(("OTHER", "Weapons")),
            "model" | "models" | "outfit" | "outfits" | "costume" | "costumes" | "skin" | "skins" | "hair" | "accessory" | "accessories" => Some(("OTHER", "Models")),
            "texture" | "textures" => Some(("OTHER", "Textures")),
            "animation" | "animations" | "motion" | "motions" => Some(("OTHER", "Animations")),
            "music" | "audio" | "sound" | "sounds" | "voice" | "sfx" => Some(("OTHER", "Music")),
            "text" | "ui" | "hud" | "gui" | "font" | "subtitles" => Some(("OTHER", "Text")),
            "misc" | "miscellaneous" | "other" | "stage" | "gameplay" | "overhaul" | "patch" => Some(("OTHER", "Misc")),
            "enemy" | "enemies" | "boss" | "bosses" | "creatures" => Some(("CHARACTERS", "Enemies")),
            "multiple" => Some(("CHARACTERS", "Multiple")),
            _ => None,
        };
        if let Some((g, c)) = mapped {
            parsed.push((g.to_string(), c.to_string()));
        } else if grp_hint.contains("char") {
            parsed.push(("CHARACTERS".to_string(), leaf.to_string()));
        } else {
            parsed.push(("OTHER".to_string(), leaf.to_string()));
        }
    }

    // Automatic inspection of file paths and mod name so mods without category= (or with generic
    // "!Other > Misc") still match character/weapon/framework categories accurately.
    let nl = name.to_ascii_lowercase();
    let fl_all: Vec<String> = files.iter().map(|f| f.to_ascii_lowercase()).collect();

    let mut auto_detected: Option<(String, String)> = None;
    let has_coop = fl_all.iter().any(|f| f.contains("re4-co-op.dll") || f.contains("chubby_coop_fix.lua")) || nl.contains("co-op") || nl.contains("multiplayer");
    let has_ref = fl_all.iter().any(|f| f == "dinput8.dll" || f.starts_with("reframework/"));

    if has_coop {
        auto_detected = Some(("OTHER".into(), "Co-op & Multiplayer".into()));
    } else if has_ref {
        auto_detected = Some(("OTHER".into(), "RE Framework".into()));
    }

    if auto_detected.is_none() {
        // Check RE4R / RE2R / RE3R character folder codes or character names in mod title.
        let char_match = if game_id == "re4r" {
            if fl_all.iter().any(|f| f.contains("/ch0") || f.contains("/cha0") || f.contains("/pl00")) || word_match(&nl, "leon") {
                Some("Leon")
            } else if fl_all.iter().any(|f| f.contains("/ch1") || f.contains("/cha1") || f.contains("/pl01")) || word_match(&nl, "ashley") {
                Some("Ashley")
            } else if fl_all.iter().any(|f| f.contains("/ch2") || f.contains("/cha2") || f.contains("/pl02")) || word_match(&nl, "ada") {
                Some("Ada")
            } else if fl_all.iter().any(|f| f.contains("/ch3") || f.contains("/cha3") || f.contains("/pl03")) || word_match(&nl, "luis") {
                Some("Luis")
            } else if fl_all.iter().any(|f| f.contains("/ch4") || f.contains("/cha4")) || word_match(&nl, "krauser") {
                Some("Krauser")
            } else if fl_all.iter().any(|f| f.contains("/ch5") || f.contains("/cha5")) || word_match(&nl, "wesker") {
                Some("Wesker")
            } else if fl_all.iter().any(|f| f.contains("/ch6") || f.contains("/cha6")) || word_match(&nl, "hunk") {
                Some("Hunk")
            } else {
                None
            }
        } else {
            chars.iter().copied().find(|&c| c != "Enemies" && c != "Multiple" && word_match(&nl, &c.to_ascii_lowercase()))
        };
        if let Some(ch) = char_match {
            auto_detected = Some(("CHARACTERS".into(), ch.into()));
        }
    }

    if auto_detected.is_none() {
        if fl_all.iter().any(|f| f.contains("/wp/") || f.contains("/weapon/") || f.contains("_chainsaw/character/wp"))
            || ["weapon", "handgun", "shotgun", "rifle", "knife", "magnum", "red9", "blacktail", "punisher", "killer7", "riot gun", "striker", "stingray"]
                .iter()
                .any(|w| nl.contains(w))
        {
            auto_detected = Some(("OTHER".into(), "Weapons".into()));
        } else if fl_all.iter().any(|f| f.contains(".mesh.") || f.contains(".mdf2.")) {
            auto_detected = Some(("OTHER".into(), "Models".into()));
        } else if fl_all.iter().any(|f| f.contains(".tex.")) {
            auto_detected = Some(("OTHER".into(), "Textures".into()));
        } else if fl_all.iter().any(|f| f.contains(".mot.") || f.contains(".motlist.") || f.contains(".motbank.")) {
            auto_detected = Some(("OTHER".into(), "Animations".into()));
        } else if fl_all.iter().any(|f| f.contains(".sbnk.") || f.contains(".pck.") || f.contains(".wem.") || f.contains(".bnk.") || f.contains("/sound/") || f.contains("/wwise/")) {
            auto_detected = Some(("OTHER".into(), "Music".into()));
        } else if fl_all.iter().any(|f| f.contains(".msg.") || f.contains(".gui.") || f.contains("/gui/") || f.contains("/font/")) {
            auto_detected = Some(("OTHER".into(), "Text".into()));
        }
    }

    let (primary_group, primary_cat) = if let Some((ug, uc)) = user_override {
        (ug.clone(), uc.clone())
    } else if let Some(first) = parsed.first() {
        first.clone()
    } else if let Some(ad) = auto_detected.clone() {
        ad
    } else {
        ("OTHER".to_string(), "Misc".to_string())
    };

    let mut clean_tags: Vec<String> = vec![primary_cat.clone()];
    for (_, c) in &parsed {
        if !clean_tags.iter().any(|t| t.eq_ignore_ascii_case(c)) {
            clean_tags.push(c.clone());
        }
    }
    if has_ref && !clean_tags.iter().any(|t| t.eq_ignore_ascii_case("RE Framework")) {
        clean_tags.push("RE Framework".to_string());
    }
    if let Some((_, ac)) = auto_detected {
        if !clean_tags.iter().any(|t| t.eq_ignore_ascii_case(&ac)) {
            clean_tags.push(ac);
        }
    }
    for et in extra_tags {
        if !clean_tags.iter().any(|t| t.eq_ignore_ascii_case(&et)) {
            clean_tags.push(et);
        }
    }

    (primary_group, primary_cat, clean_tags)
}

fn word_match(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case(needle))
}

fn plain_dest_static(re_engine: bool, rel: &str) -> Option<String> {
    let l = rel.to_ascii_lowercase();
    if re_engine && l.ends_with(".pak") && !l.starts_with("natives/") && !l.starts_with("reframework/") {
        None
    } else {
        Some(rel.to_string())
    }
}

const MARKER: &str = "ModdedByChubbyModManager.txt";
/// The original copies for at most 0.0333 s per frame (function 0x14009da10, constant at
/// 0x140243be8). Here copying runs on its own thread, so after each slice it briefly lets
/// the game and the window have the disk and CPU.
pub const COPY_TIME_SLICE: f64 = 0.0333;

fn copy_slice_yield(slice_start: &mut std::time::Instant) {
    if slice_start.elapsed().as_secs_f64() > COPY_TIME_SLICE {
        std::thread::sleep(std::time::Duration::from_millis(1));
        *slice_start = std::time::Instant::now();
    }
}
const CRASH_BACKUP: &str = "CrashReport_CMMBACKUP.exe";

/// Copy, and if that fails try once more by reading the whole file and writing it out
/// (the original falls back the same way when CopyFile fails, function 0x14009da10).
fn copy_retry(src: &Path, dst: &Path) -> std::io::Result<u32> {
    match fsx::copy_with_crc(src, dst) {
        Ok(c) => Ok(c),
        Err(_) => {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let bytes = fs::read(src)?;
            fsx::write_atomic(dst, &bytes)?;
            Ok(crc32fast::hash(&bytes))
        }
    }
}

fn cached_crc(cache: &mut HashMap<String, (u64, u64, u32)>, p: &Path) -> u32 {
    let name = p.file_name().unwrap().to_string_lossy().to_string();
    let size = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    let mt = mtime(p);
    if let Some((s, t, c)) = cache.get(&name) {
        if *s == size && *t == mt {
            return *c;
        }
    }
    let c = fsx::crc_file(p).unwrap_or(0);
    cache.insert(name, (size, mt, c));
    c
}

fn folder_crc(dir: &Path) -> u32 {
    let mut h = crc32fast::Hasher::new();
    for rel in fsx::walk_files(dir) {
        h.update(rel.as_bytes());
        h.update(&fs::metadata(dir.join(&rel)).map(|m| m.len()).unwrap_or(0).to_le_bytes());
    }
    h.finalize()
}

fn is_later_rar_part(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    if let Some(idx) = l.rfind(".part") {
        let num = &l[idx + 5..l.len().saturating_sub(4)];
        if let Ok(n) = num.parse::<u32>() {
            return n > 1;
        }
    }
    false
}

pub fn sanitize(s: &str) -> String {
    s.chars().map(|c| if "<>:\"/\\|?*\t".contains(c) { '_' } else { c }).collect()
}

fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
        p.is_file() && p.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case(name)).unwrap_or(false)
    })
}

/// Where the mod files start inside an unpacked archive or folder.
fn find_roots(dir: &Path) -> Vec<PathBuf> {
    let files = fsx::walk_files(dir);
    let infos: Vec<&String> = files
        .iter()
        .filter(|f| f.rsplit('/').next().unwrap_or("").eq_ignore_ascii_case("modinfo.ini"))
        .filter(|f| !f.to_lowercase().split('/').any(|c| SKIP_DIRS.contains(&c)))
        .collect();
    if !infos.is_empty() {
        let min = infos.iter().map(|f| f.matches('/').count()).min().unwrap();
        return infos
            .iter()
            .filter(|f| f.matches('/').count() == min)
            .map(|f| match f.rfind('/') {
                Some(i) => dir.join(&f[..i]),
                None => dir.to_path_buf(),
            })
            .collect();
    }
    // No modinfo.ini: skip single wrapper folders ("ModName\natives\..." -> "ModName").
    let mut cur = dir.to_path_buf();
    for _ in 0..6 {
        let entries: Vec<PathBuf> = match fs::read_dir(&cur) {
            Ok(rd) => rd
                .flatten()
                .map(|e| e.path())
                .filter(|p| {
                    let n = p.file_name().unwrap().to_string_lossy().to_lowercase();
                    if SKIP_DIRS.contains(&n.as_str()) || n == ".remh-stamp" {
                        return false;
                    }
                    if p.is_file() {
                        let e = n.rsplit('.').next().unwrap_or("");
                        if ["txt", "md", "url", "png", "jpg", "jpeg", "ini", "nfo", "pdf"].contains(&e) && n != "dinput8.ini" {
                            return false;
                        }
                    }
                    true
                })
                .collect(),
            Err(_) => break,
        };
        if entries.len() == 1 && entries[0].is_dir() {
            let n = entries[0].file_name().unwrap().to_string_lossy().to_lowercase();
            if GAME_TOP_DIRS.contains(&n.as_str()) {
                break;
            }
            cur = entries[0].clone();
        } else {
            break;
        }
    }
    vec![cur]
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    for rel in fsx::walk_files(from) {
        fsx::copy_with_crc(&from.join(&rel), &to.join(&rel))?;
    }
    Ok(())
}

/// Unpack an archive into `stage` unless it is already unpacked from the same file.
fn ensure_staged(archive: &Path, stage: &Path, crc: u32, rar_dirs: &[PathBuf]) -> Result<(), String> {
    let stamp_path = stage.join(".remh-stamp");
    let stamp = format!("{:08x}", crc);
    if fs::read_to_string(&stamp_path).map(|s| s == stamp).unwrap_or(false) {
        return Ok(());
    }
    let _ = fs::remove_dir_all(stage);
    fs::create_dir_all(stage).map_err(|e| e.to_string())?;
    let r = extract_archive(archive, stage, rar_dirs);
    if let Err(e) = r {
        let _ = fs::remove_dir_all(stage);
        return Err(e);
    }
    fs::write(&stamp_path, stamp).map_err(|e| e.to_string())
}

pub fn extract_archive(archive: &Path, dest: &Path, rar_dirs: &[PathBuf]) -> Result<(), String> {
    match ext_of(archive).as_str() {
        "zip" => {
            let f = fs::File::open(archive).map_err(|e| e.to_string())?;
            let mut z = zip::ZipArchive::new(std::io::BufReader::new(f)).map_err(|e| format!("not a valid ZIP ({})", e))?;
            z.extract(dest).map_err(|e| e.to_string())
        }
        "7z" | "7zip" => sevenz_rust2::decompress_file(archive, dest).map_err(|e| format!("7z: {}", e)),
        "rar" => crate::rar::extract(archive, dest, rar_dirs),
        other => Err(format!("unsupported archive type .{}", other)),
    }
}
