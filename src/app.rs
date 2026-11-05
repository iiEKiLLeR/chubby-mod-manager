//! Program state and behaviour (no drawing here; see screens.rs).

use crate::config::Config;
use crate::coop::{self, Event, ModSet, ModSig, Msg, Session};
use crate::games;
use crate::mods::{self, Library};
use crate::sys;
use crate::fsx;
use crate::ui::*;
use macroquad::prelude::*;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

pub const TITLE: &str = "Chubby Mod Manager";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUNDLED_COOP_ZIP_NAME: &str = "MULTIPLAYER_RE4R_Chubby_Enhanced.zip";
pub const BUNDLED_COOP_ZIP: &[u8] = include_bytes!("../assets/MULTIPLAYER_RE4R_Chubby_Enhanced.zip");
pub const BUNDLED_COOP_FIX_LUA: &[u8] = include_bytes!("../assets/chubby_coop_fix.lua");

/// Public GitHub repo that hosts installer downloads; every copy of the app checks it on startup.
pub const GITHUB_RELEASES_REPO: &str = "iiEKiLLeR/chubby-mod-manager-releases";
pub fn github_update_json_url() -> String {
    format!("https://github.com/{}/releases/latest/download/update.json", GITHUB_RELEASES_REPO)
}
pub fn github_installer_url(version: &str) -> String {
    format!("https://github.com/{}/releases/download/v{}/Chubby_Mod_Manager_Setup.exe", GITHUB_RELEASES_REPO, version)
}

pub const CURRENT_RELEASE_NOTES: &[(&str, &str)] = &[
    ("FIX", "v1.0.3 Smoother zombies on your partner's screen: enemy animation now syncs at 60 Hz (was 30 Hz)"),
    ("FIX", "Removed an older Chubby 'lag guard' that cached enemy data and capped it to 15 Hz when the game dipped below ~45 fps"),
    ("FIX", "Removed the old enemy position smoother that dragged enemies back on the host's side"),
    ("NEW", "Automatic updates from GitHub: the app checks for new versions every time it opens"),
    ("NEW", "One-click 'PLAY' button in the top bar and 'PLAY GAME' button in the left sidebar"),
    ("NEW", "REFramework menu matches your Chubby theme & opens with Fn+5 (F5)"),
];

#[derive(Clone, Debug)]
pub struct RemoteUpdate {
    pub version: String,
    pub title: String,
    pub download_url: String,
    pub notes: Vec<(String, String)>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Mods,
    Coop,
    Presets,
    Options,
    Log,
}
pub const TABS: &[(Tab, &str)] = &[(Tab::Mods, "Mods"), (Tab::Coop, "Co-op"), (Tab::Presets, "Presets"), (Tab::Options, "Options"), (Tab::Log, "Log")];

#[derive(Clone, Debug)]
pub enum Action {
    Up,
    Down,
    Activate,
    Back,
    NextGame,
    PrevGame,
    Refresh,
    Drop(Vec<PathBuf>),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Filter {
    All,
    Installed,
    Available,
}

pub struct Player {
    pub id: u32,
    pub name: String,
    pub ping: Option<u32>,
    pub ready: bool,
    pub in_game: bool,
    pub set: Option<ModSet>,
}

pub struct JobDone {
    lib: Library,
    msgs: Vec<Result<String, String>>,
    then_apply: Option<Vec<String>>,
}

pub struct App {
    pub cfg: Config,
    pub data: PathBuf,
    pub games_root: PathBuf,
    pub game_idx: usize,
    pub lib: Library,
    pub job: Option<Receiver<JobDone>>,
    pub busy: String,
    pub job_t0: f64,
    pub job_progress: std::sync::Arc<(std::sync::atomic::AtomicUsize, std::sync::atomic::AtomicUsize)>,
    pub tab: Tab,
    pub sel: usize,
    pub scroll: f32,
    pub filter: Filter,
    pub fields: HashMap<Field, String>,
    pub log: Vec<(String, Color)>,
    pub toast: Option<(String, Color, f64)>,
    pub shots: HashMap<PathBuf, Option<Texture2D>>,
    pub ui: Ui,
    pub actions: VecDeque<Action>,
    pub drawn: Vec<&'static str>,
    pub frame: u64,
    pad_prev: u16,
    pub jobs_done: u32,
    // game process
    pub game_pid: Option<u32>,
    pub fake_game_running: bool,
    pub last_proc_check: f64,
    pub target_fps: f64,
    // co-op
    pub coop: Option<Session>,
    pub players: Vec<Player>,
    pub host_set: Option<ModSet>,
    pub host_name: String,
    pub my_ready: bool,
    pub chat: Vec<String>,
    pub transfers: BTreeMap<String, (u64, u64)>,
    pub sync_waiting: Vec<String>,
    pub pings_seen: u32,
    pub files_sent: u32,
    last_ping: f64,
    last_roster: f64,
    last_set_sent: Option<ModSet>,
    // folder watch
    mods_sig: u64,
    last_watch: f64,
    // update & patch notes popup
    pub show_update_modal: bool,
    pub remote_update: Option<RemoteUpdate>,
    pub update_status: String,
    pub update_rx: Option<Receiver<Result<Option<RemoteUpdate>, String>>>,
    pub headless: bool,
    pub quit: bool,
    log_file: Option<std::fs::File>,
}

impl App {
    pub fn new(data: PathBuf, headless: bool) -> App {
        let _ = std::fs::create_dir_all(data.join("Data"));
        let mut cfg = Config::load(&data.join("Data").join("config.cfg"));
        if cfg.get("FluffyFolder").trim().is_empty() && !headless {
            if let Some(f) = games::detect_fluffy() {
                cfg.set("FluffyFolder", f.to_string_lossy());
                cfg.save();
            }
        }
        let games_root = data.join("Games");
        if !headless && !cfg.flag("FluffyTakenOver") {
            let ff = cfg.get("FluffyFolder");
            if !ff.trim().is_empty() {
                take_over_from_fluffy(Path::new(ff.trim()), &mut cfg, &games_root);
            }
        }
        let game_idx = games::by_id(&cfg.get("current_game"));
        let log_file = std::fs::OpenOptions::new().create(true).append(true).open(data.join("Data").join("Log.txt")).ok();
        let lib = Library::new(games::GAMES[game_idx].clone(), None, &games_root);
        let mut fields = HashMap::new();
        fields.insert(Field::Name, cfg.get("player_name"));
        fields.insert(Field::Ip, cfg.get("network_ip"));
        fields.insert(Field::Port, cfg.get("network_port"));
        fields.insert(Field::Fluffy, cfg.get("FluffyFolder"));
        fields.insert(Field::UpdateUrl, cfg.get("update_url"));
        let show_update_modal = (!headless && cfg.get("last_seen_version").trim() != VERSION)
            || std::env::args().any(|a| a == "--show-update-popup");
        let mut app = App {
            cfg,
            data,
            games_root,
            game_idx,
            lib,
            job: None,
            busy: String::new(),
            job_t0: 0.0,
            job_progress: Default::default(),
            tab: Tab::Mods,
            sel: 0,
            scroll: 0.0,
            filter: Filter::All,
            fields,
            log: Vec::new(),
            toast: None,
            shots: HashMap::new(),
            ui: Ui::new(),
            actions: VecDeque::new(),
            drawn: Vec::new(),
            frame: 0,
            pad_prev: 0,
            jobs_done: 0,
            game_pid: None,
            fake_game_running: false,
            last_proc_check: -10.0,
            target_fps: 60.0,
            coop: None,
            players: Vec::new(),
            host_set: None,
            host_name: String::new(),
            my_ready: false,
            chat: Vec::new(),
            transfers: BTreeMap::new(),
            sync_waiting: Vec::new(),
            pings_seen: 0,
            files_sent: 0,
            last_ping: 0.0,
            last_roster: 0.0,
            last_set_sent: None,
            mods_sig: 0,
            last_watch: 0.0,
            show_update_modal,
            remote_update: None,
            update_status: format!("Running v{} (latest installed)", VERSION),
            update_rx: None,
            headless,
            quit: false,
            log_file,
        };
        app.ui.theme_idx = theme_index(&app.cfg.get("theme"));
        if crate::arg_value("--self-test").is_none() {
            app.provision_bundled_coop_mod();
            if !headless {
                app.check_for_updates(false);
            }
        }
        app.select_game(game_idx);
        app.sync_coop_mod_theme();
        if !headless {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let _ = std::fs::remove_file(dir.join("Chubby Mod Manager.exe.old"));
                }
            }
            app.check_for_updates(false);
        }
        app.say(Ok(format!("{} v{} ready. Data folder: {}", TITLE, VERSION, app.data.display())));
        app
    }

    pub fn dismiss_update_modal(&mut self) {
        self.show_update_modal = false;
        self.cfg.set("last_seen_version", VERSION);
        self.cfg.save();
    }

    pub fn check_for_updates(&mut self, manual: bool) {
        if self.update_rx.is_some() {
            return;
        }
        let url = self.fields.get(&Field::UpdateUrl).cloned().unwrap_or_else(|| self.cfg.get("update_url"));
        let url = url.trim().to_string();
        if !url.is_empty() {
            self.cfg.set("update_url", &url);
            self.cfg.save();
        }
        // GitHub Releases is always checked first; an optional custom URL and the
        // Co-op host's built-in update server are extra sources. The newest version wins.
        let mut candidates: Vec<String> = vec![github_update_json_url()];
        if !url.is_empty() && !candidates.contains(&url) {
            candidates.push(url.clone());
        }
        let net_ip = self.fields.get(&Field::Ip).cloned().unwrap_or_else(|| self.cfg.get("network_ip"));
        let net_ip = net_ip.trim();
        let port = self.port();
        if !net_ip.is_empty() && net_ip != "127.0.0.1" && !net_ip.eq_ignore_ascii_case("localhost") {
            let host_only = net_ip.split(':').next().unwrap_or(net_ip);
            let u1 = if net_ip.contains(':') {
                format!("http://{}/update.json", net_ip)
            } else {
                format!("http://{}:{}/update.json", host_only, port)
            };
            let u2 = format!("http://{}:9952/update.json", host_only);
            if !candidates.contains(&u1) {
                candidates.push(u1);
            }
            if !candidates.contains(&u2) {
                candidates.push(u2);
            }
        }
        let local_manifest = self.data.join("Data").join("update.json");
        self.update_status = "Checking for updates...".to_string();
        if manual {
            self.show_update_modal = true;
        }
        let (tx, rx) = channel();
        self.update_rx = Some(rx);
        std::thread::spawn(move || {
            let mut texts: Vec<String> = Vec::new();
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                for cand in &candidates {
                    let script = format!(
                        "$ProgressPreference = 'SilentlyContinue'; \
                         [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
                         $r = Invoke-WebRequest -Uri '{}' -UseBasicParsing -TimeoutSec 6 -Headers @{{'Cache-Control'='no-cache'}}; \
                         $c = $r.Content; if ($c -is [byte[]]) {{ $c = [Text.Encoding]::UTF8.GetString($c) }}; \
                         [Console]::Out.Write($c)",
                        cand.replace('\'', "''")
                    );
                    if let Some(txt) = std::process::Command::new("powershell.exe")
                        .creation_flags(0x0800_0000)
                        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
                        .output()
                        .ok()
                        .filter(|o| o.status.success())
                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                    {
                        if txt.contains("\"version\"") || txt.contains("\"tag_name\"") {
                            texts.push(txt);
                        }
                    }
                }
            }
            if let Ok(t) = std::fs::read_to_string(&local_manifest) {
                texts.push(t);
            }
            let mut best: Option<RemoteUpdate> = None;
            let mut any_ok = false;
            for t in &texts {
                if let Ok(found) = parse_update_manifest(t) {
                    any_ok = true;
                    if let Some(u) = found {
                        let better = best.as_ref().map(|b| is_newer_version(&u.version, &b.version)).unwrap_or(true);
                        if better {
                            best = Some(u);
                        }
                    }
                }
            }
            let res = if best.is_some() || any_ok {
                Ok(best)
            } else {
                Err("Could not reach the update server (check your internet connection).".to_string())
            };
            let _ = tx.send(res);
        });
    }

    pub fn poll_update(&mut self) {
        let Some(rx) = &self.update_rx else { return };
        let Ok(res) = rx.try_recv() else { return };
        self.update_rx = None;
        match res {
            Ok(Some(upd)) => {
                let ver = upd.version.clone();
                self.update_status = format!("New update v{} is available!", ver);
                self.remote_update = Some(upd);
                self.show_update_modal = true;
                self.say(Ok(format!("Update v{} available! Review the patch notes & hotfixes.", ver)));
            }
            Ok(None) => {
                self.update_status = format!("You are up to date (v{}).", VERSION);
            }
            Err(e) => {
                self.update_status = format!("Update check: {} (running v{})", e, VERSION);
            }
        }
    }

    pub fn apply_remote_update(&mut self) {
        let Some(upd) = self.remote_update.clone() else { return };
        if upd.download_url.trim().is_empty() {
            self.say(Err("No download_url provided in update manifest.".into()));
            return;
        }
        self.show_update_modal = false;
        let url = upd.download_url.trim().to_string();
        let ver = upd.version.clone();
        let data_dir = self.data.join("Data");
        let cur_exe = std::env::current_exe().ok();
        self.run_job(&format!("Downloading update v{}", ver), move |lib| {
            let tmp_installer = data_dir.join(format!("Chubby_Update_{}.exe", ver));
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let ok = if Path::new(&url).is_file() {
                    std::fs::copy(&url, &tmp_installer).is_ok()
                } else {
                    let script = format!(
                        "$ProgressPreference = 'SilentlyContinue'; \
                         [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
                         Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing -TimeoutSec 120",
                        url.replace('\'', "''"),
                        tmp_installer.display().to_string().replace('\'', "''")
                    );
                    std::process::Command::new("powershell.exe")
                        .creation_flags(0x0800_0000)
                        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false)
                };
                if !ok || !tmp_installer.is_file() {
                    return vec![Err(format!("Failed to download update v{} from {}", ver, url))];
                }
                let _ = std::process::Command::new(&tmp_installer)
                    .creation_flags(0x0800_0000)
                    .arg("--install-silent")
                    .status();
                if let Some(exe) = cur_exe {
                    let _ = std::process::Command::new(exe).spawn();
                    std::process::exit(0);
                }
            }
            let _ = lib;
            vec![Ok(format!("Updated to v{}!", ver))]
        }, None);
    }

    pub fn set_theme(&mut self, idx: usize) {
        let idx = idx % THEMES.len();
        self.ui.theme_idx = idx;
        self.cfg.set("theme", THEMES[idx].id);
        self.cfg.save();
        self.sync_coop_mod_theme();
        self.say(Ok(format!(
            "Theme set to {} (synced with REFramework & RE4R Co-op menu).",
            THEMES[idx].name
        )));
    }

    pub fn cycle_theme(&mut self) {
        self.set_theme((self.ui.theme_idx + 1) % THEMES.len());
    }

    /// Ensure the built-in enhanced RE4R Co-op mod is present and up-to-date in Games\RE4R\Mods.
    pub fn provision_bundled_coop_mod(&self) {
        let re4r_mods = self.games_root.join("RE4R").join("Mods");
        let _ = std::fs::create_dir_all(&re4r_mods);
        let zip_dest = re4r_mods.join(BUNDLED_COOP_ZIP_NAME);
        let needs_write = match std::fs::read(&zip_dest) {
            Ok(existing) => {
                existing.len() != BUNDLED_COOP_ZIP.len()
                    || crc32fast::hash(&existing) != crc32fast::hash(BUNDLED_COOP_ZIP)
            }
            Err(_) => true,
        };
        if needs_write {
            let _ = fsx::write_atomic(&zip_dest, BUNDLED_COOP_ZIP);
            let staged = self.games_root.join("RE4R").join("Staging").join(BUNDLED_COOP_ZIP_NAME);
            let _ = std::fs::remove_dir_all(&staged);
        }
    }

    /// Patch REFramework (dinput8.dll), Re4-Co-op.dll, and write chubby_theme.json + RE4MP_hz.json
    /// in staged/installed locations so both the REFramework menu and Co-op menu match Chubby Mod Manager's active theme.
    pub fn sync_coop_mod_theme(&mut self) {
        let t = *self.ui.theme();
        let mut roots = vec![
            self.games_root.join("RE4R").join("Staging").join(BUNDLED_COOP_ZIP_NAME).join("MULTIPLAYER RE4R"),
            self.games_root.join("RE4R").join("Mods").join("MULTIPLAYER RE4R"),
        ];
        if let Some(gd) = &self.lib.game_dir {
            roots.push(gd.clone());
        }
        for g in games::GAMES {
            let d = self.cfg.get(&format!("modman_{}_dir", g.id));
            if !d.is_empty() {
                let pb = PathBuf::from(d);
                if !roots.contains(&pb) {
                    roots.push(pb);
                }
            }
        }
        let abgr: u32 = 0xFF00_0000
            | (((t.accent_hi.b * 255.0).round() as u32 & 0xFF) << 16)
            | (((t.accent_hi.g * 255.0).round() as u32 & 0xFF) << 8)
            | ((t.accent_hi.r * 255.0).round() as u32 & 0xFF);
        let theme_json = format!(
            "{{\"id\":\"{}\",\"name\":\"{}\",\"accent_abgr\":{}}}\n",
            t.id, t.name, abgr
        );
        // Ensure 60 Hz enemy animation & partner animation so zombies don't look like 10-20 FPS on partner side
        let hz_60_json = "{\"master\":60,\"partner_pos\":60,\"partner_rot\":60,\"partner_anim\":60,\"partner_wpn\":30,\"enemy_pos\":60,\"enemy_anim\":60,\"enemy_vitals\":30,\"flag_gimmick\":15}\n";

        for root in &roots {
            let ref_dll = root.join("dinput8.dll");
            if ref_dll.is_file() {
                if let Ok(mut bytes) = std::fs::read(&ref_dll) {
                    if patch_reframework_dll_bytes(&mut bytes, &t) {
                        let _ = fsx::write_atomic(&ref_dll, &bytes);
                    }
                }
            }
            let dll_path = root.join("reframework").join("plugins").join("Re4-Co-op.dll");
            if dll_path.is_file() {
                if let Ok(mut bytes) = std::fs::read(&dll_path) {
                    if patch_coop_dll_bytes(&mut bytes, &t) {
                        let _ = fsx::write_atomic(&dll_path, &bytes);
                    }
                }
                let data_dir = root.join("reframework").join("data");
                let _ = std::fs::create_dir_all(&data_dir);
                let _ = fsx::write_atomic(&data_dir.join("chubby_theme.json"), theme_json.as_bytes());
                let hz_path = data_dir.join("RE4MP_hz.json");
                let needs_hz_write = match std::fs::read_to_string(&hz_path) {
                    Ok(existing) => existing.contains("\"enemy_anim\":30") || existing.contains("\"partner_anim\":30"),
                    Err(_) => true,
                };
                if needs_hz_write {
                    let _ = fsx::write_atomic(&hz_path, hz_60_json.as_bytes());
                }
                // Keep the Chubby Co-op fix script up to date wherever the mod is installed.
                let lua_path = root.join("reframework").join("autorun").join("!chubby_coop_fix.lua");
                if lua_path.is_file() {
                    let current = std::fs::read(&lua_path).unwrap_or_default();
                    if current != BUNDLED_COOP_FIX_LUA {
                        let _ = fsx::write_atomic(&lua_path, BUNDLED_COOP_FIX_LUA);
                    }
                }
            }
        }

        // Keep installed file checksums in sync when game_dir has dinput8.dll or Re4-Co-op.dll installed,
        // and ensure REFramework's menu keybind is set to F5 / Fn+5 (VK_F5 = 116).
        if let Some(gd) = &self.lib.game_dir {
            let fw_cfg = gd.join("re2_fw_config.txt");
            if fw_cfg.is_file() {
                if let Ok(txt) = std::fs::read_to_string(&fw_cfg) {
                    if txt.contains("REFrameworkConfig_MenuKey_V2=45") {
                        let new_txt = txt.replace("REFrameworkConfig_MenuKey_V2=45", "REFrameworkConfig_MenuKey_V2=116");
                        let _ = fsx::write_atomic(&fw_cfg, new_txt.as_bytes());
                    }
                }
            }
            let mut updated = false;
            for m in &mut self.lib.state.mods {
                for f in &mut m.files {
                    let dl = f.dest.to_ascii_lowercase();
                    if dl.ends_with("dinput8.dll") || dl.ends_with("re4-co-op.dll") || dl.ends_with("chubby_theme.json") || dl.ends_with("re4mp_hz.json") || dl.ends_with("!chubby_coop_fix.lua") {
                        if let Ok(crc) = fsx::crc_file(&gd.join(&f.dest)) {
                            if f.crc != crc {
                                f.crc = crc;
                                updated = true;
                            }
                        }
                    }
                }
            }
            if updated {
                self.lib.save_state_public();
            }
        }
    }

    // ---------- messages ----------

    pub fn say(&mut self, r: Result<String, String>) {
        let (s, c) = match r {
            Ok(s) => (s, TEXT),
            Err(s) => (s, BAD),
        };
        if let Some(f) = self.log_file.as_mut() {
            let _ = writeln!(f, "[{}] {}", fsx::now_stamp(), s);
        }
        self.toast = Some((s.clone(), c, get_time()));
        self.log.push((s, c));
        if self.log.len() > 500 {
            self.log.remove(0);
        }
    }

    // ---------- game selection ----------

    pub fn game(&self) -> &'static games::GameDef {
        &games::GAMES[self.game_idx]
    }

    fn game_dir_key(&self) -> String {
        format!("modman_{}_dir", self.game().id)
    }

    pub fn select_game(&mut self, idx: usize) {
        if self.job.is_some() {
            return;
        }
        self.game_idx = idx;
        self.cfg.set("current_game", self.game().id);
        let mut dir = self.cfg.get(&self.game_dir_key());
        if (dir.is_empty() || !Path::new(&dir).is_dir()) && !self.headless {
            if let Some(d) = games::detect_install(self.game(), &games::steam_common_dirs()) {
                dir = d.to_string_lossy().to_string();
                self.cfg.set(&self.game_dir_key(), &dir);
            }
        }
        self.fields.insert(Field::GameDir, dir.clone());
        let gd = if dir.is_empty() { None } else { Some(PathBuf::from(&dir)) };
        self.cfg.save();
        self.lib = Library::new(self.game().clone(), gd, &self.games_root);
        self.lib.auto_delete_backups = self.cfg.flag("AutoDeleteBackupFiles");
        self.sync_coop_mod_theme();
        let mut rar_dirs = vec![self.data.clone()];
        if let Ok(exe) = std::env::current_exe() {
            if let Some(d) = exe.parent() {
                rar_dirs.push(d.to_path_buf());
            }
        }
        let ff = self.cfg.get("FluffyFolder");
        if !ff.trim().is_empty() {
            rar_dirs.push(PathBuf::from(ff.trim()));
        }
        self.lib.rar_dirs = rar_dirs;
        self.sel = 0;
        self.scroll = 0.0;
        self.shots.clear();
        self.mods_sig = self.mods_signature();
        if let Some(s) = &self.coop {
            *s.incoming_dir.lock().unwrap() = self.lib.mods_dir();
        }
        self.run_job("Reading mods", |_| vec![], None);
    }

    pub fn set_game_dir(&mut self, dir: &str) {
        self.cfg.set(&self.game_dir_key(), dir.trim());
        self.cfg.save();
        self.select_game(self.game_idx);
    }

    // ---------- background jobs (scan, install, ...) ----------

    pub fn is_busy(&self) -> bool {
        self.job.is_some()
    }

    /// Run work on the library away from the window's thread; the list is rescanned afterwards.
    pub fn run_job<F>(&mut self, label: &str, f: F, then_apply: Option<Vec<String>>)
    where
        F: FnOnce(&mut Library) -> Vec<Result<String, String>> + Send + 'static,
    {
        if self.job.is_some() {
            self.say(Err("Busy, try again in a moment.".into()));
            return;
        }
        let mut lib = self.lib.clone();
        let (tx, rx) = channel();
        self.busy = label.to_string();
        self.job_t0 = get_time();
        self.job_progress = lib.progress.clone();
        std::thread::spawn(move || {
            if lib.mods.is_empty() {
                lib.scan();
            }
            let mut msgs = f(&mut lib);
            lib.scan();
            for w in &lib.warnings {
                msgs.push(Err(w.clone()));
            }
            let _ = tx.send(JobDone { lib, msgs, then_apply });
        });
        self.job = Some(rx);
    }

    fn poll_job(&mut self) {
        let Some(rx) = &self.job else { return };
        let Ok(done) = rx.try_recv() else { return };
        self.job = None;
        self.busy.clear();
        self.jobs_done += 1;
        self.lib = done.lib;
        for m in done.msgs {
            self.say(m);
        }
        if self.lib.game_updated {
            self.say(Err("The game seems to have been updated since mods were installed. Press 'Check files' on the Mods page.".into()));
        }
        self.sel = self.sel.min(self.visible().len().saturating_sub(1));
        self.mods_sig = self.mods_signature();
        self.sync_coop_mod_theme();
        if let Some(want) = done.then_apply {
            self.run_job("Matching the host's mods", move |l| vec![l.apply_set(&want)], None);
        }
        self.broadcast_my_set(false);
    }

    pub fn toggle_mod(&mut self, key: String) {
        let installed = self.lib.is_installed(&key);
        if !installed && self.game_running() {
            let m = self.lib.mods.iter().find(|m| m.key == key);
            if m.map(|m| m.files.iter().any(|f| !f.contains('/') && f.to_lowercase().ends_with(".pak"))).unwrap_or(false) {
                self.say(Err("Close the game before installing PAK mods.".into()));
                return;
            }
        }
        if installed {
            self.run_job("Uninstalling mod", move |l| vec![l.uninstall(&key)], None);
        } else {
            self.run_job("Installing mod", move |l| vec![l.install(&key)], None);
        }
    }

    pub fn import(&mut self, paths: Vec<PathBuf>, copy: bool) {
        if paths.is_empty() {
            return;
        }
        self.tab = Tab::Mods;
        let n = paths.len();
        self.run_job(&format!("Adding {} item(s)", n), move |l| l.import(&paths, copy), None);
    }

    // ---------- the mod list as shown ----------

    pub fn visible(&self) -> Vec<usize> {
        let q = self.fields.get(&Field::Search).cloned().unwrap_or_default().to_lowercase();
        self.lib
            .mods
            .iter()
            .enumerate()
            .filter(|(_, m)| match self.filter {
                Filter::All => true,
                Filter::Installed => m.installed,
                Filter::Available => !m.installed,
            })
            .filter(|(_, m)| {
                q.is_empty()
                    || m.name.to_lowercase().contains(&q)
                    || m.author.to_lowercase().contains(&q)
                    || m.categories.iter().any(|c| c.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn selected_key(&self) -> Option<String> {
        self.visible().get(self.sel).map(|&i| self.lib.mods[i].key.clone())
    }

    fn mods_signature(&self) -> u64 {
        let mut h = crc32fast::Hasher::new();
        if let Ok(rd) = std::fs::read_dir(self.lib.mods_dir()) {
            let mut v: Vec<String> = rd
                .flatten()
                .filter(|e| !e.file_name().to_string_lossy().ends_with(".part"))
                .map(|e| {
                    let m = e.metadata().ok();
                    format!(
                        "{}|{}|{:?}",
                        e.file_name().to_string_lossy(),
                        m.as_ref().map(|m| m.len()).unwrap_or(0),
                        m.and_then(|m| m.modified().ok())
                    )
                })
                .collect();
            v.sort();
            for s in v {
                h.update(s.as_bytes());
            }
        }
        h.finalize() as u64
    }

    // ---------- game process + power saving ----------

    pub fn game_running(&self) -> bool {
        self.game_pid.is_some() || self.fake_game_running
    }

    fn check_game(&mut self) {
        let now = get_time();
        if now - self.last_proc_check < 3.0 {
            return;
        }
        self.last_proc_check = now;
        let was = self.game_running();
        self.game_pid = sys::find_process(self.game().exe);
        let is = self.game_running();
        if is && !was {
            self.on_game_started();
        } else if !is && was {
            self.on_game_stopped();
        }
    }

    pub fn on_game_started(&mut self) {
        if let Some(pid) = self.game_pid {
            if self.cfg.flag("BoostGamePriority") && sys::set_priority(pid, false) {
                self.say(Ok("Gave the game a higher CPU priority.".into()));
            }
        }
        if !self.headless {
            sys::self_background(true);
            if self.cfg.flag("ExitWhenGameStarts") && self.coop.is_none() {
                self.quit = true;
            } else if self.cfg.flag("MinimizeWhenGameStarts") {
                sys::minimize(TITLE);
            }
        }
        self.ui.focus = None;
        self.say(Ok("Game is running: Chubby Mod Manager is sleeping so the game gets the computer.".into()));
        if let Some(s) = &self.coop {
            s.broadcast(&Msg::InGame(true));
        }
    }

    pub fn on_game_stopped(&mut self) {
        if !self.headless {
            sys::self_background(false);
        }
        self.say(Ok("Game closed. Back to full speed.".into()));
        if let Some(s) = &self.coop {
            s.broadcast(&Msg::InGame(false));
        }
    }

    /// Frames per second this window allows itself right now.
    pub fn pacing(&self, focused: bool) -> f64 {
        if self.game_running() && !focused && self.cfg.flag("SleepWhileGameRuns") {
            self.cfg.get_i("GameRunningFPS").max(1) as f64
        } else if !focused {
            self.cfg.get_i("NoFocusMaxFPS").max(1) as f64
        } else {
            self.cfg.get_i("MaxFPS").max(10) as f64
        }
    }

    // ---------- co-op ----------

    pub fn my_set(&self) -> ModSet {
        ModSet {
            game: self.game().id.to_string(),
            available: self
                .lib
                .mods
                .iter()
                .filter(|m| m.size > 0)
                .map(|m| ModSig { key: m.key.clone(), source_name: m.source_name.clone(), name: m.name.clone(), crc: m.crc, size: m.size })
                .collect(),
            installed: self.lib.state.mods.iter().map(|m| m.key.clone()).collect(),
        }
    }

    pub fn broadcast_my_set(&mut self, force: bool) {
        let set = self.my_set();
        if !force && self.last_set_sent.as_ref() == Some(&set) {
            return;
        }
        if let Some(s) = &self.coop {
            s.broadcast(&Msg::Mods(set.clone()));
            self.last_set_sent = Some(set);
        }
    }

    pub fn player_name(&self) -> String {
        let n = self.fields.get(&Field::Name).cloned().unwrap_or_default();
        if n.trim().is_empty() { "Player".into() } else { n.trim().to_string() }
    }

    pub fn port(&self) -> u16 {
        self.fields.get(&Field::Port).and_then(|p| p.trim().parse().ok()).unwrap_or(9950)
    }

    pub fn start_host(&mut self, port: u16) {
        self.leave();
        match Session::host(port, &self.player_name(), self.game().id, self.lib.mods_dir()) {
            Ok(s) => {
                let p = s.port;
                self.coop = Some(s);
                self.say(Ok(format!("Hosting on {}:{}. Friends join with this address.", coop::local_ip(), p)));
                self.last_set_sent = None;
            }
            Err(e) => self.say(Err(format!("Could not host on port {}: {}", port, e))),
        }
    }

    pub fn join(&mut self, addr: &str) {
        self.leave();
        match Session::join(addr, self.port(), &self.player_name(), self.game().id, self.lib.mods_dir()) {
            Ok(s) => {
                self.coop = Some(s);
                self.say(Ok(format!("Connected to {}.", addr)));
                self.last_set_sent = None;
                self.broadcast_my_set(true);
                self.check_for_updates(false);
            }
            Err(e) => self.say(Err(format!("Could not reach {}: {}. Check the address and that the host forwarded the port.", addr, e))),
        }
    }

    pub fn leave(&mut self) {
        if let Some(s) = self.coop.take() {
            s.close();
            self.say(Ok("Left the co-op session.".into()));
        }
        self.players.clear();
        self.host_set = None;
        self.host_name.clear();
        self.transfers.clear();
        self.sync_waiting.clear();
        self.my_ready = false;
    }

    pub fn hosting(&self) -> bool {
        self.coop.as_ref().map(|s| s.hosting).unwrap_or(false)
    }

    pub fn host_diff(&self) -> Option<coop::Diff> {
        self.host_set.as_ref().map(|h| coop::diff(h, &self.my_set()))
    }

    /// Joined player: fetch every mod the host has that I lack, then install exactly the host's set.
    pub fn sync_with_host(&mut self) {
        let Some(d) = self.host_diff() else { return };
        let Some(host) = self.host_set.clone() else { return };
        if let Some(s) = &self.coop {
            self.sync_waiting = d.missing.iter().map(|m| mods::sanitize(&m.source_name)).collect();
            for m in &d.missing {
                s.send(0, &Msg::Want(m.source_name.clone()));
            }
        }
        if self.sync_waiting.is_empty() {
            let want = host.installed.clone();
            self.run_job("Matching the host's mods", move |l| vec![l.apply_set(&want)], None);
        } else {
            self.say(Ok(format!("Downloading {} mod(s) from the host...", self.sync_waiting.len())));
        }
    }

    fn poll_coop(&mut self) {
        let Some(s) = &self.coop else { return };
        let mut evs = Vec::new();
        while let Ok(e) = s.events.try_recv() {
            evs.push(e);
        }
        let hosting = s.hosting;
        for e in evs {
            match e {
                Event::Joined(id, name, game) => {
                    if !self.players.iter().any(|p| p.id == id) {
                        self.players.push(Player { id, name: name.clone(), ping: None, ready: false, in_game: false, set: None });
                    }
                    if hosting {
                        self.say(Ok(format!("{} joined.", name)));
                        self.broadcast_my_set(true);
                    } else {
                        self.host_name = name.clone();
                        self.say(Ok(format!("Joined {}'s session.", name)));
                    }
                    if game != self.game().id {
                        self.say(Err(format!("{} has a different game selected ({}).", name, game)));
                    }
                }
                Event::Left(id, why) => {
                    self.players.retain(|p| p.id != id);
                    self.say(Ok(format!("Disconnected: {}", why)));
                    if !hosting {
                        if let Some(s) = self.coop.take() {
                            s.close();
                        }
                        self.host_set = None;
                        self.players.clear();
                        return;
                    }
                }
                Event::Ping(id, ms) => {
                    self.pings_seen += 1;
                    if let Some(p) = self.players.iter_mut().find(|p| p.id == id) {
                        p.ping = Some(match p.ping {
                            Some(old) => (old * 3 + ms) / 4,
                            None => ms,
                        });
                    }
                }
                Event::FileProgress(name, got, total) => {
                    self.transfers.insert(name, (got, total));
                }
                Event::FileDone(name, res) => {
                    self.transfers.remove(&name);
                    let sent = name.starts_with("sent ");
                    if sent {
                        self.transfers.remove(&format!("sending {}", &name[5..]));
                    }
                    match res {
                        Ok(()) if sent => self.files_sent += 1,
                        Ok(()) => self.say(Ok(format!("Received {} from the host.", name))),
                        Err(e) => self.say(Err(format!("Transfer of {} failed: {}", name, e))),
                    }
                    if let Some(i) = self.sync_waiting.iter().position(|n| *n == name) {
                        self.sync_waiting.remove(i);
                        if self.sync_waiting.is_empty() {
                            let want = self.host_set.as_ref().map(|h| h.installed.clone()).unwrap_or_default();
                            self.run_job("Reading new mods", |_| vec![], Some(want));
                        }
                    }
                }
                Event::Error(e) => self.say(Err(e)),
                Event::Msg(id, m) => self.on_msg(id, m),
            }
        }
        // Keep-alive pings: every second, every 5 s while the game runs.
        let now = get_time();
        let every = if self.game_running() { 5.0 } else { 1.0 };
        if now - self.last_ping > every {
            self.last_ping = now;
            if let Some(s) = &self.coop {
                s.broadcast(&Msg::Ping(coop::now_ms()));
            }
        }
        if hosting && now - self.last_roster > 2.0 {
            self.last_roster = now;
            let mine = self.my_set();
            let mut list = vec![coop::PlayerInfo { name: self.player_name(), ping: 0, ready: true, matches: true, in_game: self.game_running() }];
            for p in &self.players {
                let matches = p.set.as_ref().map(|ps| coop::diff(&mine, ps).matches()).unwrap_or(false);
                list.push(coop::PlayerInfo { name: p.name.clone(), ping: p.ping.unwrap_or(0), ready: p.ready, matches, in_game: p.in_game });
            }
            if let Some(s) = &self.coop {
                s.broadcast(&Msg::Roster(list));
            }
        }
    }

    fn on_msg(&mut self, id: u32, m: Msg) {
        let hosting = self.hosting();
        match m {
            Msg::Mods(set) => {
                if hosting {
                    if let Some(p) = self.players.iter_mut().find(|p| p.id == id) {
                        p.set = Some(set);
                    }
                } else {
                    self.host_set = Some(set);
                }
            }
            Msg::Want(name) => {
                if !hosting {
                    return;
                }
                if self.game_running() && self.cfg.flag("PauseTransfersDuringGame") {
                    if let Some(s) = &self.coop {
                        s.send(id, &Msg::Refuse("The host is playing right now; mod downloads are paused so the game stays smooth.".into()));
                    }
                    return;
                }
                let safe = mods::sanitize(&name);
                let p = self.lib.mods_dir().join(&safe);
                if p.is_file() {
                    if let Some(s) = &self.coop {
                        s.send_file(id, p);
                    }
                    self.say(Ok(format!("Sending {} to a friend.", safe)));
                } else if let Some(s) = &self.coop {
                    s.send(id, &Msg::Refuse(format!("{} is a folder mod on the host; ask them to zip it.", safe)));
                }
            }
            Msg::Refuse(why) => {
                self.sync_waiting.clear();
                self.say(Err(why));
            }
            Msg::Chat(from, text) => {
                self.chat.push(format!("{}: {}", from, text));
                if hosting {
                    if let Some(s) = &self.coop {
                        for pid in s.peer_ids() {
                            if pid != id {
                                s.send(pid, &Msg::Chat(from.clone(), text.clone()));
                            }
                        }
                    }
                }
            }
            Msg::Ready(r) => {
                if let Some(p) = self.players.iter_mut().find(|p| p.id == id) {
                    p.ready = r;
                }
            }
            Msg::InGame(r) => {
                if let Some(p) = self.players.iter_mut().find(|p| p.id == id) {
                    p.in_game = r;
                }
            }
            Msg::Roster(list) => {
                if !hosting {
                    let host_ping = self.players.iter().find(|x| x.id == 0).and_then(|x| x.ping);
                    self.players = list
                        .into_iter()
                        .enumerate()
                        .map(|(i, p)| Player {
                            id: if i == 0 { 0 } else { 1000 + i as u32 },
                            name: p.name,
                            ping: if i == 0 { host_ping } else { Some(p.ping) },
                            ready: p.ready,
                            in_game: p.in_game,
                            set: None,
                        })
                        .collect();
                }
            }
            Msg::Launch => {
                self.say(Ok("The host started the game.".into()));
                self.launch_game();
            }
            Msg::Welcome(my) => {
                if let Some(s) = self.coop.as_mut() {
                    s.my_id = my;
                }
            }
            _ => {}
        }
    }

    pub fn launch_game(&mut self) {
        if self.game_running() {
            self.say(Ok(format!("{} is already running.", self.game().name)));
            return;
        }
        if self.headless {
            return;
        }
        self.sync_coop_mod_theme();
        let g = self.game();
        if g.steam_app != 0 {
            sys::open(&format!("steam://rungameid/{}", g.steam_app));
            self.say(Ok(format!("Launching {}...", g.name)));
        } else if let Some(d) = &self.lib.game_dir {
            let exe = games::exe_path(g, d);
            let _ = std::process::Command::new(&exe).current_dir(d).spawn();
            self.say(Ok(format!("Launching {}...", g.name)));
        } else {
            self.say(Err("Set the game's install folder in Options first.".into()));
        }
    }

    // ---------- input ----------

    fn keyboard(&mut self) {
        if !self.ui.active || self.headless {
            return;
        }
        if self.ui.focus.is_some() {
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Tab) {
                self.ui.focus = None;
            }
            return;
        }
        // Controller (same layout as the original: D-pad moves, A selects, B goes back).
        let pad = sys::pad_buttons();
        let pressed = pad & !self.pad_prev;
        self.pad_prev = pad;
        for (bit, a) in [(0x0001u16, Action::Up), (0x0002, Action::Down), (0x0004, Action::PrevGame), (0x0008, Action::NextGame), (0x1000, Action::Activate), (0x2000, Action::Back)] {
            if pressed & bit != 0 {
                self.actions.push_back(a);
            }
        }
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let step = if shift { 5 } else { 1 };
        if is_key_pressed(KeyCode::Up) {
            for _ in 0..step {
                self.actions.push_back(Action::Up);
            }
        }
        if is_key_pressed(KeyCode::Down) {
            for _ in 0..step {
                self.actions.push_back(Action::Down);
            }
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            self.actions.push_back(Action::Activate);
        }
        if is_key_pressed(KeyCode::Backspace) || is_key_pressed(KeyCode::Escape) {
            self.actions.push_back(Action::Back);
        }
        if is_key_pressed(KeyCode::Right) {
            self.actions.push_back(Action::NextGame);
        }
        if is_key_pressed(KeyCode::Left) {
            self.actions.push_back(Action::PrevGame);
        }
        if is_key_pressed(KeyCode::F5) {
            self.actions.push_back(Action::Refresh);
        }
        if is_key_pressed(KeyCode::Tab) {
            let i = TABS.iter().position(|(t, _)| *t == self.tab).unwrap_or(0);
            self.tab = TABS[(i + 1) % TABS.len()].0;
        }
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if ctrl && is_key_pressed(KeyCode::F) {
            self.tab = Tab::Mods;
            self.ui.focus = Some(Field::Search);
        }
    }

    pub fn apply_action(&mut self, a: Action) {
        match a {
            Action::Up => {
                self.sel = self.sel.saturating_sub(1);
                self.keep_visible();
            }
            Action::Down => {
                let n = self.visible().len();
                if n > 0 {
                    self.sel = (self.sel + 1).min(n - 1);
                }
                self.keep_visible();
            }
            Action::Activate => {
                if self.tab == Tab::Mods && !self.is_busy() {
                    if let Some(k) = self.selected_key() {
                        self.toggle_mod(k);
                    }
                }
            }
            Action::Back => self.tab = Tab::Mods,
            Action::NextGame => self.select_game((self.game_idx + 1) % games::GAMES.len()),
            Action::PrevGame => self.select_game((self.game_idx + games::GAMES.len() - 1) % games::GAMES.len()),
            Action::Refresh => self.run_job("Refreshing", |_| vec![], None),
            Action::Drop(p) => self.import(p, false),
        }
    }

    fn keep_visible(&mut self) {
        let row = 40.0;
        let y = self.sel as f32 * row;
        let h = screen_height() - 66.0 - 40.0 - 52.0;
        if y < self.scroll {
            self.scroll = y;
        } else if y + row > self.scroll + h {
            self.scroll = y + row - h;
        }
    }

    // ---------- one frame ----------

    pub fn update(&mut self) {
        let focused = self.headless || sys::is_foreground(TITLE);
        self.ui.begin(focused);
        self.drawn.clear();
        self.frame += 1;
        if !self.headless {
            self.check_game();
        }
        self.poll_job();
        self.poll_coop();
        self.poll_update();
        let dropped = sys::take_dropped();
        if !dropped.is_empty() {
            self.actions.push_back(Action::Drop(dropped));
        }
        self.keyboard();
        // One queued action per frame so jobs started by one don't block the next.
        if !self.is_busy() || matches!(self.actions.front(), Some(Action::Up | Action::Down | Action::Back)) {
            if let Some(a) = self.actions.pop_front() {
                self.apply_action(a);
            }
        }
        // Watch the Mods folder: zips copied there in Explorer show up by themselves.
        let now = get_time();
        if self.cfg.flag("WatchModsFolder") && !self.game_running() && now - self.last_watch > 2.0 && !self.is_busy() {
            self.last_watch = now;
            let sig = self.mods_signature();
            if sig != self.mods_sig {
                self.mods_sig = sig;
                self.run_job("New files in the Mods folder", |_| vec![], None);
            }
        }
        self.target_fps = self.pacing(focused);
    }
}

/// Fluffy's own setting names for each game's install folder (strings near 0x140221360).
const FLUFFY_DIR_KEYS: &[(&str, &str)] = &[
    ("rehd", "modman_rehd_dir"),
    ("re0hd", "modman_re0hd_dir"),
    ("re2r", "modman_re2remake_dir"),
    ("re3r", "modman_re3remake_dir"),
    ("re4", "modman_re4_dir"),
    ("re4r", "modman_re4remake_dir"),
    ("re5", "modman_re5_dir"),
    ("re6", "modman_re6_dir"),
    ("re7", "modman_re7_dir"),
    ("re8", "modman_re8_dir"),
    ("rev", "modman_rev_dir"),
    ("rev2", "modman_rev2_dir"),
    ("resistance", "modman_resistance_dir"),
];

/// First start next to an existing Fluffy Mod Manager: take over its game folders, player
/// name, colour and port, and copy (not move) its mod files for every game. Runs once.
fn take_over_from_fluffy(fluffy: &Path, cfg: &mut Config, games_root: &Path) {
    let fcfg = Config::load(&fluffy.join("Data").join("config.cfg"));
    for (id, key) in FLUFFY_DIR_KEYS {
        let v = fcfg.get(key);
        let mine = format!("modman_{}_dir", id);
        if !v.trim().is_empty() && cfg.get(&mine).trim().is_empty() && Path::new(v.trim()).is_dir() {
            cfg.set(&mine, v.trim());
        }
    }
    for key in ["player_name", "network_port", "network_ip", "player_colorR", "player_colorG", "player_colorB"] {
        let v = fcfg.get(key);
        if !v.trim().is_empty() && v.trim() != "PlayerName" {
            cfg.set(key, v.trim());
        }
    }
    let cur = fcfg.get("modman_default_game");
    if games::GAMES.iter().any(|g| g.id == cur.trim()) {
        cfg.set("current_game", cur.trim());
    }
    let mut copied = 0;
    for g in games::GAMES {
        let src = fluffy.join("Games").join(g.folder).join("Mods");
        let Ok(rd) = std::fs::read_dir(&src) else { continue };
        let dst = games_root.join(g.folder).join("Mods");
        let _ = std::fs::create_dir_all(&dst);
        for e in rd.flatten() {
            let p = e.path();
            let to = dst.join(e.file_name());
            if p.is_file() && !to.exists() && fsx::copy_with_crc(&p, &to).is_ok() {
                copied += 1;
            }
        }
    }
    cfg.set("FluffyTakenOver", 1);
    cfg.set("FluffyTakeOverCopied", copied);
    cfg.save();
}

fn write_f32_le(buf: &mut [u8], off: usize, v: f32) {
    if off + 4 <= buf.len() {
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

fn write_i32_le(buf: &mut [u8], off: usize, v: i32) {
    if off + 4 <= buf.len() {
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }
}

fn write_vec4_le(buf: &mut [u8], off: usize, r: f32, g: f32, b: f32, a: f32) {
    write_f32_le(buf, off, r);
    write_f32_le(buf, off + 4, g);
    write_f32_le(buf, off + 8, b);
    write_f32_le(buf, off + 12, a);
}

fn write_imm_rgba_le(buf: &mut [u8], off_r: usize, r: f32, g: f32, b: f32, a: f32) {
    write_f32_le(buf, off_r, r);
    write_f32_le(buf, off_r + 10, g);
    write_f32_le(buf, off_r + 20, b);
    write_f32_le(buf, off_r + 30, a);
}

/// Binary-patch Re4-Co-op.dll (1,359,360 bytes) with:
/// 1. Single OS mouse cursor & reliable menu click fixes (FUN_1800653b0, FUN_180066970, FUN_180067180)
/// 2. Optimal default sync rates (60Hz partner/enemy pos & rot, 30Hz anim/vitals, 15Hz gimmick scan cap)
/// 3. Active Chubby Mod Manager theme palette (ImGuiStyle + .data / .rdata colors)
pub fn patch_coop_dll_bytes(buf: &mut [u8], t: &Theme) -> bool {
    if buf.len() != 1_359_360 || &buf[0..2] != b"MZ" {
        return false;
    }
    // 1. Set io.MouseDrawCursor = io.WantCaptureMouse (mov al, [rdi+0x394]; jmp 0x180065b8c at file offset 413567)
    // Shows cursor when hovering over the Co-op panel, and hides duplicate software cursor outside it.
    buf[413567..413578].copy_from_slice(&[0x8a, 0x87, 0x94, 0x03, 0x00, 0x00, 0xeb, 0x05, 0x90, 0xcd, 0x29]);

    // 2. Never teleport cursor to screen center on Co-op menu open (0x1800669f3 / file offset 417267)
    buf[417267] = 0xeb;

    // 3. Preserve OS GetCursorPos/ScreenToClient io.MousePos.x and io.MousePos.y (file offsets 417410, 417428)
    buf[417412] = 0x10;
    buf[417430] = 0x10;

    // 4. Eat WM_INPUT raw mouse messages while Co-op menu is open (0x180067239 / file offset 419385)
    buf[419385..419390].copy_from_slice(&[0xe9, 0xbd, 0x00, 0x00, 0x00]);

    // 5. Auto-hide upper-right "Co-op" button when not hovered (FUN_180061bc0)
    buf[397494..397502].copy_from_slice(&[0xf3, 0x0f, 0x59, 0x0d, 0x7a, 0x16, 0x0d, 0x00]); // speed 2.5f
    buf[397550..397559].copy_from_slice(&[0xf3, 0x44, 0x0f, 0x59, 0x15, 0x5d, 0x16, 0x0d, 0x00]); // * 3.0f
    buf[397660..397669].copy_from_slice(&[0xf3, 0x44, 0x0f, 0x5d, 0x15, 0xfb, 0x14, 0x0d, 0x00]); // min(alpha, 1.0f)
    write_f32_le(buf, 1319544, 0.0); // resting min alpha = 0.0 (completely invisible when not hovered)

    // 5. Optimal default sync rate indices ({15, 30, 60, 120})
    write_i32_le(buf, 1319404, 2); // master: 60Hz
    write_i32_le(buf, 1320576, 2); // partner_pos: 60Hz
    write_i32_le(buf, 1320580, 2); // partner_rot: 60Hz
    write_i32_le(buf, 1320584, 2); // partner_anim: 60Hz
    write_i32_le(buf, 1320588, 1); // partner_wpn: 30Hz
    write_i32_le(buf, 1320592, 2); // enemy_pos: 60Hz
    write_i32_le(buf, 1320596, 2); // enemy_anim: 60Hz (was 30Hz: enemies looked choppy on the partner's side)
    write_i32_le(buf, 1320600, 1); // enemy_vitals: 30Hz
    write_i32_le(buf, 1320604, 0); // flag_gimmick: 15Hz

    // 6. Active Chubby Mod Manager theme palette
    write_vec4_le(buf, 1133288, t.bg_top.r, t.bg_top.g, t.bg_top.b, 0.98);
    write_vec4_le(buf, 1133304, t.line_hi.r, t.line_hi.g, t.line_hi.b, 0.55);
    write_vec4_le(buf, 1319296, t.panel2.r, t.panel2.g, t.panel2.b, 1.0);
    write_vec4_le(buf, 1319312, t.muted.r, t.muted.g, t.muted.b, 1.0);
    write_vec4_le(buf, 1319328, t.accent.r, t.accent.g, t.accent.b, 1.0);
    write_vec4_le(buf, 1319344, t.text.r, t.text.g, t.text.b, 1.0);
    write_vec4_le(buf, 1320224, t.panel.r, t.panel.g, t.panel.b, 1.0);
    write_vec4_le(buf, 1320640, t.accent_hi.r, t.accent_hi.g, t.accent_hi.b, 1.0);
    write_vec4_le(buf, 1321072, t.muted.r * 0.78, t.muted.g * 0.78, t.muted.b * 0.78, 1.0);
    write_vec4_le(buf, 1321184, t.good.r, t.good.g, t.good.b, 1.0);

    write_imm_rgba_le(buf, 411947, t.field_bg.r, t.field_bg.g, t.field_bg.b, 1.0);
    write_imm_rgba_le(buf, 411987, t.panel2.r, t.panel2.g, t.panel2.b, 1.0);
    write_imm_rgba_le(buf, 412027, t.btn_hover.r, t.btn_hover.g, t.btn_hover.b, 1.0);
    write_imm_rgba_le(buf, 412067, t.panel2.r, t.panel2.g, t.panel2.b, 1.0);
    write_imm_rgba_le(buf, 412107, t.btn_hover.r, t.btn_hover.g, t.btn_hover.b, 1.0);
    write_imm_rgba_le(buf, 412147, t.accent.r, t.accent.g, t.accent.b, 1.0);
    write_imm_rgba_le(buf, 412187, t.panel2.r, t.panel2.g, t.panel2.b, 1.0);
    write_imm_rgba_le(buf, 412227, t.btn_hover.r, t.btn_hover.g, t.btn_hover.b, 1.0);
    write_imm_rgba_le(buf, 412267, t.accent.r, t.accent.g, t.accent.b, 1.0);
    true
}

/// Apply Chubby Mod Manager's active theme palette and modern 6px corner rounding to
/// `REFramework::set_imgui_style()` inside `dinput8.dll`.
pub fn patch_reframework_dll_bytes(buf: &mut [u8], t: &Theme) -> bool {
    if buf.len() < 500_000 || &buf[0..2] != b"MZ" {
        return false;
    }
    let matches_sig = |b: &[u8], pos: usize| -> bool {
        pos >= 42
            && pos + 950 <= b.len()
            && b[pos..pos + 6] == [0xc7, 0x80, 0x34, 0x01, 0x00, 0x00]
            && b[pos + 10..pos + 16] == [0xc7, 0x80, 0x38, 0x01, 0x00, 0x00]
            && b[pos + 40..pos + 46] == [0xc7, 0x80, 0x74, 0x04, 0x00, 0x00]
            && b[pos + 911..pos + 917] == [0xc7, 0x80, 0xd4, 0x01, 0x00, 0x00]
    };
    let base = if matches_sig(buf, 331521) {
        Some(331521)
    } else {
        (42..buf.len().saturating_sub(950)).find(|&i| matches_sig(buf, i))
    };
    let Some(base) = base else {
        return false;
    };

    // Modern 6.0px corner rounding (WindowRounding, ChildRounding, FrameRounding, GrabRounding, TabRounding)
    if buf[base - 42..base - 36] == [0xc7, 0x80, 0x88, 0x00, 0x00, 0x00] {
        write_f32_le(buf, base - 36, 6.0);
        write_f32_le(buf, base - 17, 6.0);
        write_f32_le(buf, base - 9, 6.0);
    }

    // ImGuiCol_WindowBg [rax+0x134]
    write_imm_rgba_le(buf, base + 6, t.bg_top.r, t.bg_top.g, t.bg_top.b, 0.96);
    // ImGuiCol_NavHighlight [rax+0x474]
    write_imm_rgba_le(buf, base + 46, t.accent_hi.r, t.accent_hi.g, t.accent_hi.b, 1.0);
    // ImGuiCol_PlotLines [rax+0x3c4]
    write_imm_rgba_le(buf, base + 86, t.line_hi.r, t.line_hi.g, t.line_hi.b, 1.0);
    // ImGuiCol_Header [rax+0x294]
    write_imm_rgba_le(buf, base + 126, t.panel2.r, t.panel2.g, t.panel2.b, 1.0);
    // ImGuiCol_HeaderHovered [rax+0x2a4]
    write_imm_rgba_le(buf, base + 166, t.btn_hover.r, t.btn_hover.g, t.btn_hover.b, 1.0);
    // ImGuiCol_HeaderActive [rax+0x2b4]
    write_imm_rgba_le(buf, base + 206, t.accent.r, t.accent.g, t.accent.b, 1.0);
    // ImGuiCol_Button [rax+0x264]
    write_imm_rgba_le(buf, base + 246, t.panel2.r, t.panel2.g, t.panel2.b, 1.0);
    // ImGuiCol_ButtonHovered [rax+0x274]
    write_imm_rgba_le(buf, base + 286, t.btn_hover.r, t.btn_hover.g, t.btn_hover.b, 1.0);
    // ImGuiCol_ButtonActive [rax+0x284]
    write_imm_rgba_le(buf, base + 326, t.accent.r, t.accent.g, t.accent.b, 1.0);
    // ImGuiCol_CheckMark [rax+0x234]
    write_imm_rgba_le(buf, base + 366, t.accent_hi.r, t.accent_hi.g, t.accent_hi.b, 1.0);
    // ImGuiCol_FrameBg [rax+0x184]
    write_imm_rgba_le(buf, base + 406, t.field_bg.r, t.field_bg.g, t.field_bg.b, 1.0);
    // ImGuiCol_FrameBgHovered [rax+0x194]
    write_imm_rgba_le(buf, base + 446, t.panel2.r, t.panel2.g, t.panel2.b, 1.0);
    // ImGuiCol_FrameBgActive [rax+0x1a4]
    write_imm_rgba_le(buf, base + 486, t.btn_hover.r, t.btn_hover.g, t.btn_hover.b, 1.0);
    // ImGuiCol_Tab [rax+0x344]
    write_imm_rgba_le(buf, base + 526, t.panel.r, t.panel.g, t.panel.b, 1.0);
    // ImGuiCol_TabHovered [rax+0x334]
    write_imm_rgba_le(buf, base + 566, t.btn_hover.r, t.btn_hover.g, t.btn_hover.b, 1.0);
    // ImGuiCol_TabActive [rax+0x354]
    write_imm_rgba_le(buf, base + 606, t.accent.r, t.accent.g, t.accent.b, 1.0);
    // ImGuiCol_TabUnfocused [rax+0x374]
    write_imm_rgba_le(buf, base + 646, t.panel.r, t.panel.g, t.panel.b, 1.0);
    // ImGuiCol_TabUnfocusedActive [rax+0x384]
    write_imm_rgba_le(buf, base + 686, t.panel2.r, t.panel2.g, t.panel2.b, 1.0);
    // ImGuiCol_ResizeGrip [rax+0x2f4] (3 floats: R at +726, G at +736, B at +747 due to REX.W prefix on [rax+0x2fc])
    write_f32_le(buf, base + 726, t.panel2.r);
    write_f32_le(buf, base + 736, t.panel2.g);
    write_f32_le(buf, base + 747, t.panel2.b);
    // ImGuiCol_ResizeGripHovered [rax+0x304]
    write_imm_rgba_le(buf, base + 757, t.btn_hover.r, t.btn_hover.g, t.btn_hover.b, 1.0);
    // ImGuiCol_ResizeGripActive [rax+0x314]
    write_imm_rgba_le(buf, base + 797, t.accent.r, t.accent.g, t.accent.b, 1.0);
    // ImGuiCol_TitleBg [rax+0x1b4]
    write_imm_rgba_le(buf, base + 837, t.top_bar.r, t.top_bar.g, t.top_bar.b, 1.0);
    // ImGuiCol_TitleBgActive [rax+0x1c4]
    write_imm_rgba_le(buf, base + 877, t.accent.r, t.accent.g, t.accent.b, 1.0);
    // ImGuiCol_TitleBgCollapsed [rax+0x1d4]
    write_imm_rgba_le(buf, base + 917, t.top_bar.r, t.top_bar.g, t.top_bar.b, 1.0);
    true
}

fn json_field(s: &str, key: &str) -> Option<String> {
    let pat = format!("\"{}\"", key);
    let idx = s.find(&pat)?;
    let rest = &s[idx + pat.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = after[1..].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

fn is_newer_version(remote: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.trim()
            .trim_start_matches('v')
            .split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .filter_map(|p| p.parse::<u32>().ok())
            .collect()
    };
    let r = parse(remote);
    let c = parse(current);
    if r.is_empty() {
        return false;
    }
    let len = r.len().max(c.len());
    for i in 0..len {
        let rv = r.get(i).copied().unwrap_or(0);
        let cv = c.get(i).copied().unwrap_or(0);
        if rv > cv {
            return true;
        }
        if rv < cv {
            return false;
        }
    }
    false
}

pub fn build_update_manifest_json(download_url: &str) -> String {
    let mut notes_json = String::new();
    for (i, (tag, body)) in CURRENT_RELEASE_NOTES.iter().enumerate() {
        if i > 0 {
            notes_json.push_str(",\n    ");
        }
        let escaped = body.replace('\\', "\\\\").replace('"', "\\\"");
        notes_json.push_str(&format!("\"[{}] {}\"", tag, escaped));
    }
    format!(
        "{{\n  \"version\": \"{}\",\n  \"title\": \"Chubby Mod Manager v{} Update\",\n  \"download_url\": \"{}\",\n  \"notes\": [\n    {}\n  ]\n}}\n",
        VERSION,
        VERSION,
        download_url.replace('\\', "\\\\").replace('"', "\\\""),
        notes_json
    )
}

pub fn run_self_silent_install() {
    let Ok(src_exe) = std::env::current_exe() else { return };
    let mut target_dir: Option<PathBuf> = None;
    if let Some(parent) = src_exe.parent() {
        if parent.file_name().map(|n| n.eq_ignore_ascii_case("Data")).unwrap_or(false) {
            if let Some(grand) = parent.parent() {
                target_dir = Some(grand.to_path_buf());
            }
        }
    }
    #[cfg(windows)]
    if target_dir.is_none() {
        use std::os::windows::process::CommandExt;
        if let Ok(out) = std::process::Command::new("powershell.exe")
            .creation_flags(0x0800_0000)
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
                target_dir = Some(PathBuf::from(s));
            }
        }
    }
    let Some(dir) = target_dir else { return };
    let _ = std::fs::create_dir_all(&dir);
    let exe_path = dir.join("Chubby Mod Manager.exe");
    let old_exe_path = dir.join("Chubby Mod Manager.exe.old");
    if exe_path.exists() && exe_path != src_exe {
        let _ = std::fs::remove_file(&old_exe_path);
        let _ = std::fs::rename(&exe_path, &old_exe_path);
    }
    if exe_path != src_exe {
        let _ = std::fs::copy(&src_exe, &exe_path);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let ps = format!(
            "$reg = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\ChubbyModManager'; \
             if (Test-Path $reg) {{ Set-ItemProperty -Path $reg -Name 'DisplayVersion' -Value '{}' -Force }}",
            VERSION
        );
        let _ = std::process::Command::new("powershell.exe")
            .creation_flags(0x0800_0000)
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &ps])
            .status();
    }
}

fn parse_update_manifest(text: &str) -> Result<Option<RemoteUpdate>, String> {
    let ver = json_field(text, "version")
        .or_else(|| json_field(text, "tag_name"))
        .ok_or_else(|| "Invalid update manifest (missing version)".to_string())?;
    let clean_ver = ver.trim().trim_start_matches('v').to_string();
    if !is_newer_version(&clean_ver, VERSION) {
        return Ok(None);
    }
    let title = json_field(text, "title")
        .or_else(|| json_field(text, "name"))
        .unwrap_or_else(|| format!("Chubby Mod Manager v{} Update", clean_ver));
    let download_url = json_field(text, "download_url")
        .or_else(|| json_field(text, "browser_download_url"))
        .unwrap_or_default();

    let mut notes = Vec::new();
    if let Some(nidx) = text.find("\"notes\"") {
        let rest = &text[nidx..];
        if let (Some(a), Some(b)) = (rest.find('['), rest.find(']')) {
            if a < b {
                let arr = &rest[a + 1..b];
                for part in arr.split('"') {
                    let t = part.trim();
                    if t.len() > 3 && !t.starts_with(',') {
                        let (tag, body) = if let Some(r) = t.strip_prefix("[HOTFIX]") {
                            ("HOTFIX", r.trim())
                        } else if let Some(r) = t.strip_prefix("[FIX]") {
                            ("FIX", r.trim())
                        } else if let Some(r) = t.strip_prefix("[NEW]") {
                            ("NEW", r.trim())
                        } else {
                            ("UPDATE", t)
                        };
                        notes.push((tag.to_string(), body.to_string()));
                    }
                }
            }
        }
    }
    if notes.is_empty() {
        if let Some(body) = json_field(text, "body") {
            for line in body.lines() {
                let l = line.trim().trim_start_matches('-').trim_start_matches('*').trim();
                if !l.is_empty() {
                    notes.push(("UPDATE".to_string(), l.to_string()));
                }
            }
        }
    }
    if notes.is_empty() {
        notes.push(("UPDATE".to_string(), format!("Update to version {} with latest fixes and improvements.", clean_ver)));
    }
    Ok(Some(RemoteUpdate {
        version: clean_ver,
        title,
        download_url,
        notes,
    }))
}
