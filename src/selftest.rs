//! `--self-test <file.json>`: runs the real program on a throw-away sandbox (a fake game
//! folder in the temp directory; the real game is never touched) and drives it like a
//! player would: drop two zip files on it, move through the list, install and uninstall,
//! host a co-op session that a scripted friend joins, and check the idle mode and the
//! on-screen information.

use crate::app::{Action, App, Tab};
use crate::coop::{Event, Msg, Session};
use macroquad::prelude::get_time;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static BASE: OnceLock<PathBuf> = OnceLock::new();

fn base() -> &'static PathBuf {
    BASE.get_or_init(|| std::env::temp_dir().join(format!("chubby-mod-manager-selftest-{}", std::process::id())))
}

fn make_zip(path: &Path, files: &[(&str, &[u8])]) {
    let f = fs::File::create(path).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in files {
        z.start_file(*name, opts).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap();
}

/// Build the sandbox and return the data folder the program should use.
pub fn sandbox() -> PathBuf {
    let b = base();
    let _ = fs::remove_dir_all(b);
    let data = b.join("data");
    let game = b.join("game");
    let incoming = b.join("incoming");
    for d in [&data.join("Data"), &game.join("natives/stm"), &incoming, &b.join("friend")] {
        fs::create_dir_all(d).unwrap();
    }
    fs::write(game.join("re4.exe"), b"fake game program").unwrap();
    fs::write(game.join("dinput8.dll"), b"fake reframework").unwrap();
    fs::write(game.join("re2_fw_config.txt"), b"LooseFileLoader_Enabled=false\r\nREFrameworkConfig_MenuKey_V2=45\r\n").unwrap();
    fs::write(game.join("natives/stm/a.tex"), b"ORIGINAL").unwrap();
    fs::write(game.join("re_chunk_000.pak.patch_001.pak"), b"official patch").unwrap();
    fs::write(
        data.join("Data/config.cfg"),
        format!("current_game: re4r\nmodman_re4r_dir: {}\nplayer_name: Host\nWatchModsFolder: 0\n", game.display()),
    )
    .unwrap();
    make_zip(
        &incoming.join("Test Outfit.zip"),
        &[
            ("Test Outfit/modinfo.ini", b"name=Test Outfit\nauthor=Self Test\nversion=v1.0\ndescription=Replaces a.tex, adds new.tex and a PAK.\ncategory=Outfits\n"),
            ("Test Outfit/natives/stm/a.tex", b"MODDED"),
            ("Test Outfit/natives/stm/new.tex", b"NEW"),
            ("Test Outfit/outfit.pak", b"PAKDATA"),
        ],
    );
    make_zip(&incoming.join("Other Mod.zip"), &[("Other Mod/natives/stm/b.tex", b"OTHER")]);
    data
}

pub struct Tester {
    out: String,
    stage: u32,
    t0: f64,
    stage_t: f64,
    steps: Vec<(String, bool, String)>,
    friend: Option<Session>,
    friend_got: Option<Result<(), String>>,
    friend_asked: bool,
    jobs_at: u32,
    sel_before: Option<String>,
}

impl Tester {
    pub fn new(out: String) -> Tester {
        Tester { out, stage: 0, t0: get_time(), stage_t: get_time(), steps: Vec::new(), friend: None, friend_got: None, friend_asked: false, jobs_at: 0, sel_before: None }
    }

    fn next(&mut self) {
        self.stage += 1;
        self.stage_t = get_time();
    }

    fn rec(&mut self, step: &str, ok: bool, detail: String) {
        self.steps.push((step.to_string(), ok, detail));
    }

    fn timed_out(&self, secs: f64) -> bool {
        get_time() - self.stage_t > secs
    }

    fn game() -> PathBuf {
        base().join("game")
    }

    /// Called before the frame's update: inject input.
    pub fn before(&mut self, app: &mut App) {
        match self.stage {
            0 if app.frame > 3 && !app.is_busy() => {
                // Drop two zip files on the window, exactly like Explorer does.
                let inc = base().join("incoming");
                crate::sys::push_dropped(inc.join("Test Outfit.zip"));
                crate::sys::push_dropped(inc.join("Other Mod.zip"));
                self.jobs_at = app.jobs_done;
                self.next();
            }
            3 => {
                app.sel = 0;
                self.sel_before = app.selected_key();
                app.actions.push_back(Action::Down);
                self.next();
            }
            5 => {
                let vis = app.visible();
                if let Some(i) = vis.iter().position(|&i| app.lib.mods[i].name == "Test Outfit") {
                    app.sel = i;
                }
                self.jobs_at = app.jobs_done;
                app.actions.push_back(Action::Activate);
                self.next();
            }
            7 => {
                self.jobs_at = app.jobs_done;
                app.actions.push_back(Action::Activate);
                self.next();
            }
            _ => {}
        }
    }

    /// Called after the frame was drawn: measure. Returns true when finished.
    pub fn after(&mut self, app: &mut App) -> bool {
        let g = Self::game();
        match self.stage {
            1 => {
                if app.jobs_done > self.jobs_at && !app.is_busy() {
                    let names: Vec<String> = app.lib.mods.iter().map(|m| m.name.clone()).collect();
                    let ok = names.iter().any(|n| n == "Test Outfit") && names.iter().any(|n| n == "Other Mod");
                    let in_folder = app.lib.mods_dir().join("Test Outfit.zip").is_file();
                    self.rec("drop", ok && in_folder, format!("dropped 2 zip files; list now shows {:?}; copied into the Mods folder: {}", names, in_folder));
                    self.next();
                    self.next();
                } else if self.timed_out(20.0) {
                    self.rec("drop", false, "dropped zip files never appeared in the list".into());
                    self.stage = 3;
                }
            }
            4 => {
                let now = app.selected_key();
                let ok = app.sel == 1 && now.is_some() && now != self.sel_before;
                self.rec("move", ok, format!("Down arrow moved the selection from {:?} to {:?} (row {})", self.sel_before, now, app.sel));
                self.next();
            }
            6 => {
                if app.jobs_done > self.jobs_at && !app.is_busy() {
                    let a = fs::read(g.join("natives/stm/a.tex")).unwrap_or_default();
                    let new = g.join("natives/stm/new.tex").is_file();
                    let dummy_ok = (2..=6).all(|n| fs::metadata(g.join(format!("re_chunk_000.pak.patch_{:03}.pak", n))).map(|m| m.len() == 16).unwrap_or(false));
                    let pak = fs::read(g.join("re_chunk_000.pak.patch_007.pak")).unwrap_or_default();
                    let no_pak_mods = !g.join("pak_mods").exists();
                    let fw_cfg = fs::read_to_string(g.join("re2_fw_config.txt")).unwrap_or_default();
                    let loose_ok = fw_cfg.contains("LooseFileLoader_Enabled=true");
                    let backup = app.lib.base.join("Backup/natives/stm/a.tex").is_file();
                    let ok = a == b"MODDED" && new && dummy_ok && pak == b"PAKDATA" && no_pak_mods && loose_ok && backup;
                    self.rec(
                        "install",
                        ok,
                        format!("Enter installed it: a.tex is {:?}, new.tex added: {}, 5 dummy PAKs + patch_007: {}, loose loader enabled: {}, original backed up: {}", String::from_utf8_lossy(&a), new, dummy_ok && pak == b"PAKDATA" && no_pak_mods, loose_ok, backup),
                    );
                    self.next();
                } else if self.timed_out(20.0) {
                    self.rec("install", false, "install did not finish".into());
                    self.next();
                }
            }
            8 => {
                if app.jobs_done > self.jobs_at && !app.is_busy() {
                    let a = fs::read(g.join("natives/stm/a.tex")).unwrap_or_default();
                    let new_gone = !g.join("natives/stm/new.tex").exists();
                    let pak_gone = (2..=7).all(|n| !g.join(format!("re_chunk_000.pak.patch_{:03}.pak", n)).exists());
                    let official = g.join("re_chunk_000.pak.patch_001.pak").is_file();
                    let backup_gone = !app.lib.base.join("Backup/natives/stm/a.tex").exists();
                    let ok = a == b"ORIGINAL" && new_gone && pak_gone && official && backup_gone;
                    let install_ok = self.steps.iter().any(|s| s.0 == "install" && s.1);
                    self.rec(
                        "act",
                        ok && install_ok,
                        format!("installed then uninstalled 'Test Outfit': original a.tex restored: {}, added files + dummy PAKs removed: {}, game's own patch PAK untouched: {}, backup cleaned: {}", a == b"ORIGINAL", new_gone && pak_gone, official, backup_gone),
                    );
                    self.next();
                } else if self.timed_out(20.0) {
                    self.rec("act", false, "uninstall did not finish".into());
                    self.next();
                }
            }
            9 => {
                app.start_host(0);
                let port = app.coop.as_ref().map(|s| s.port).unwrap_or(0);
                match Session::join(&format!("127.0.0.1:{}", port), 9950, "Friend", "re4r", base().join("friend")) {
                    Ok(s) => {
                        s.send(0, &Msg::Mods(Default::default()));
                        self.friend = Some(s);
                    }
                    Err(e) => self.rec("coop", false, format!("friend could not connect: {}", e)),
                }
                self.next();
            }
            10 => {
                if let Some(f) = &self.friend {
                    while let Ok(e) = f.events.try_recv() {
                        match e {
                            Event::Msg(_, Msg::Mods(set)) if !self.friend_asked => {
                                if set.available.iter().any(|m| m.source_name == "Test Outfit.zip") {
                                    f.send(0, &Msg::Want("Test Outfit.zip".into()));
                                    self.friend_asked = true;
                                }
                            }
                            Event::FileDone(n, r) if n == "Test Outfit.zip" => self.friend_got = Some(r),
                            _ => {}
                        }
                    }
                }
                let got = base().join("friend/Test Outfit.zip");
                let same = got.is_file() && crate::fsx::crc_file(&got).ok() == crate::fsx::crc_file(&app.lib.mods_dir().join("Test Outfit.zip")).ok();
                let ping = app.players.first().and_then(|p| p.ping);
                if (self.friend_got.is_some() && ping.is_some() && !app.players.is_empty()) || self.timed_out(12.0) {
                    let ok = matches!(self.friend_got, Some(Ok(()))) && same && ping.is_some();
                    self.rec(
                        "coop",
                        ok,
                        format!(
                            "hosted a session, a friend joined ({} player(s)), ping {:?} ms, friend downloaded 'Test Outfit.zip' from the host: {} (checksum match: {})",
                            app.players.len(), ping, matches!(self.friend_got, Some(Ok(()))), same
                        ),
                    );
                    app.tab = Tab::Coop;
                    self.next();
                }
            }
            11 => {
                let coop_drawn = app.drawn.contains(&"coop");
                app.tab = Tab::Mods;
                // Idle mode + mouse: pretend the game is running and our window is behind it.
                app.fake_game_running = true;
                let fps_game = app.pacing(false);
                let fps_front = app.pacing(true);
                app.fake_game_running = false;
                let mut probe = crate::ui::Ui::new_light();
                probe.begin(false);
                let mouse_ignored = probe.mouse.x < 0.0 && !probe.pressed;
                self.rec(
                    "idle",
                    fps_game <= 2.0 && fps_front >= 30.0 && mouse_ignored && coop_drawn,
                    format!("while the game runs the window draws {} fps (vs {} in front) and ignores the mouse when behind the game: {}", fps_game, fps_front, mouse_ignored),
                );
                self.next();
            }
            12 => {
                let need = ["game", "status-chips", "nav", "modlist", "detail", "status"];
                let missing: Vec<&&str> = need.iter().filter(|n| !app.drawn.contains(n)).collect();
                self.rec(
                    "hud",
                    missing.is_empty(),
                    format!("drawn this frame: {:?}; game picker, status chips (game folder, game running, co-op), menu, mod list, mod details and status bar {}", app.drawn, if missing.is_empty() { "all shown".to_string() } else { format!("missing {:?}", missing) }),
                );
                self.write();
                return true;
            }
            _ => {}
        }
        if get_time() - self.t0 > 90.0 {
            self.rec("timeout", false, format!("stopped at stage {}", self.stage));
            self.write();
            return true;
        }
        false
    }

    fn write(&self) {
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ");
        let mut j = String::from("{\"version\": 1, \"steps\": [\n");
        for (i, (s, ok, d)) in self.steps.iter().enumerate() {
            j.push_str(&format!("  {{\"step\": \"{}\", \"passed\": {}, \"detail\": \"{}\"}}{}\n", esc(s), ok, esc(d), if i + 1 < self.steps.len() { "," } else { "" }));
        }
        j.push_str("]}\n");
        if let Some(p) = Path::new(&self.out).parent() {
            let _ = fs::create_dir_all(p);
        }
        let _ = fs::write(&self.out, j);
        let _ = fs::remove_dir_all(base());
    }
}
