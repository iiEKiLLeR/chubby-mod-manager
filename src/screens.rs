//! Everything that is drawn: top bar, side menu, Mods, Co-op, Presets, Options, Log, status bar.

use crate::app::{Action, App, Filter, Tab, TABS};
use crate::coop::{self, Msg};
use crate::fsx;
use crate::games;
use crate::sys;
use crate::ui::*;
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

impl App {
    pub fn draw(&mut self) {
        self.ui.gradient_bg();
        let (w, h) = (screen_width(), screen_height());
        self.draw_top(w);
        self.draw_nav(h);
        let nav_split_x = 10.0 + self.nav_w + 5.0;
        let main_h = h - 66.0 - 40.0;
        if let Some(mx) = self.ui.vsplitter(DragId::NavWidth, nav_split_x, 66.0, main_h) {
            let nw = (mx - 10.0).clamp(145.0, (w * 0.30).clamp(180.0, 320.0)).round();
            if (nw - self.nav_w).abs() >= 1.0 {
                self.nav_w = nw;
                self.cfg.set("layout_nav_w", nw as i64);
                self.layout_dirty = true;
            }
        }
        let main_x = 10.0 + self.nav_w + 10.0;
        let main = Rect::new(main_x, 66.0, (w - main_x - 10.0).max(400.0), main_h);
        match self.tab {
            Tab::Mods => self.draw_mods(main),
            Tab::Coop => self.draw_coop(main),
            Tab::Presets => self.draw_presets(main),
            Tab::Options => self.draw_options(main),
            Tab::Log => self.draw_log(main),
        }
        self.draw_status(w, h);
        if self.show_update_modal {
            self.draw_update_modal(w, h);
        }
        if self.ui.pressed && !self.ui.consumed {
            self.ui.focus = None;
        }
    }

    fn draw_top(&mut self, w: f32) {
        let t = *self.ui.theme();
        draw_rectangle(0.0, 0.0, w, 56.0, t.top_bar);
        draw_rectangle(0.0, 54.0, w, 2.0, t.accent);
        draw_line(0.0, 54.0, w * 0.45, 54.0, 1.0, t.accent_hi);

        // Modern rounded brand emblem + title.
        let badge = Rect::new(14.0, 12.0, 32.0, 32.0);
        fill_rrect(badge, 6.0, t.accent_soft);
        if let Some(ic) = &self.ui.icon {
            draw_texture_ex(
                ic,
                badge.x + 1.0,
                badge.y + 1.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(badge.w - 2.0, badge.h - 2.0)),
                    ..Default::default()
                },
            );
        } else {
            fill_rrect(Rect::new(badge.x + 8.0, badge.y + 7.0, 5.0, 16.0), 2.0, t.accent_hi);
            fill_rrect(Rect::new(badge.x + 16.0, badge.y + 11.0, 6.0, 12.0), 2.0, t.text);
        }
        stroke_rrect(badge, 6.0, 1.5, t.accent);
        self.ui.btext("CHUBBY MOD MANAGER", 54.0, 28.0, 18.5, t.text);
        let ver_badge = if let Some(upd) = &self.remote_update {
            format!("v{} -> UPDATE v{}!", crate::app::VERSION, upd.version)
        } else {
            format!("VERSION {}  |  PATCH NOTES", crate::app::VERSION)
        };
        let vrect = Rect::new(52.0, 31.0, 240.0, 19.0);
        let vhov = self.ui.hover(vrect);
        let vcol = if self.remote_update.is_some() {
            t.good
        } else if vhov {
            t.text
        } else {
            t.accent_hi
        };
        self.ui.text(&ver_badge, 54.0, 44.0, 12.0, vcol);
        if self.ui.click(vrect) {
            self.show_update_modal = true;
        }

        let gx = 314.0;
        let busy = self.is_busy();
        if self.ui.button(Rect::new(gx, 12.0, 32.0, 32.0), "<", 2, !busy) {
            self.actions.push_back(Action::PrevGame);
        }
        let name = self.game().name;
        let game_box = Rect::new(gx + 36.0, 12.0, 246.0, 32.0);
        fill_rrect(game_box, 6.0, t.panel2);
        stroke_rrect(game_box, 6.0, 1.0, t.line_hi);
        let tw = self.ui.width(name, 16.0);
        self.ui.btext(name, game_box.x + (game_box.w - tw) / 2.0, 33.5, 16.0, t.text);
        if self.ui.button(Rect::new(gx + 286.0, 12.0, 32.0, 32.0), ">", 2, !busy) {
            self.actions.push_back(Action::NextGame);
        }
        self.drawn.push("game");

        // Play button right beside the game selector.
        let running = self.game_running();
        let play_r = Rect::new(gx + 324.0, 12.0, 92.0, 32.0);
        let play_hov = self.ui.hover(play_r) && !running && !busy;
        let play_bg = if running {
            Color::new(t.good.r, t.good.g, t.good.b, 0.22)
        } else if play_hov {
            t.accent_hi
        } else {
            t.accent
        };
        fill_rrect(play_r, 6.0, play_bg);
        stroke_rrect(
            play_r,
            6.0,
            1.2,
            if running { t.good } else if play_hov { WHITE } else { t.accent_hi },
        );
        if running {
            draw_circle(play_r.x + 15.0, play_r.y + 16.0, 4.5, t.good);
            self.ui.btext("PLAYING", play_r.x + 25.0, play_r.y + 21.5, 13.5, t.good);
        } else {
            let tri_col = if play_hov { Color::new(0.05, 0.06, 0.08, 1.0) } else { WHITE };
            self.ui.play_icon(play_r.x + 15.0, play_r.y + 10.0, 12.0, tri_col);
            self.ui.btext("PLAY", play_r.x + 32.0, play_r.y + 22.0, 15.0, tri_col);
            if !busy && self.ui.click(play_r) {
                self.launch_game();
            }
        }

        // Quick theme switcher pill right beside the Play button.
        let theme_label = format!("Theme: {}", t.name);
        let tlw = self.ui.width(&theme_label, 13.0) + 28.0;
        let tr = Rect::new(gx + 424.0, 14.0, tlw, 28.0);
        let thov = self.ui.hover(tr);
        fill_rrect(tr, 14.0, if thov { t.btn_hover } else { t.panel });
        stroke_rrect(tr, 14.0, 1.0, if thov { t.accent_hi } else { t.line });
        draw_circle(tr.x + 12.0, tr.y + 14.0, 4.5, t.accent_hi);
        self.ui.text(&theme_label, tr.x + 21.0, tr.y + 18.5, 13.0, t.text);
        if self.ui.click(tr) {
            self.cycle_theme();
        }

        let mut x = w - 14.0;
        let mut chips: Vec<(String, Color)> = vec![
            match &self.lib.game_dir {
                Some(_) => ("Game folder found".to_string(), t.good),
                None => ("Game folder not set".to_string(), t.warn),
            },
        ];
        if running {
            chips.push(("Game running: sleeping".to_string(), t.warn));
        }
        chips.push(match &self.coop {
            Some(s) if s.hosting => (format!("Hosting: {} friend(s)", self.players.len()), t.good),
            Some(_) => (format!("In {}'s session", if self.host_name.is_empty() { "a" } else { &self.host_name }), t.good),
            None => ("Co-op: off".to_string(), t.muted),
        });
        for (s, c) in chips.iter().rev() {
            let cw = self.ui.width(s, 13.0) + 26.0;
            x -= cw;
            let r = Rect::new(x, 15.0, cw, 26.0);
            fill_rrect(r, 13.0, Color::new(c.r, c.g, c.b, 0.13));
            stroke_rrect(r, 13.0, 1.0, Color::new(c.r, c.g, c.b, 0.30));
            draw_circle(r.x + 11.0, r.y + 13.0, 4.0, *c);
            self.ui.text(s, r.x + 19.0, r.y + 17.5, 13.0, t.text);
            x -= 8.0;
        }
        self.drawn.push("status-chips");
    }

    fn draw_nav(&mut self, h: f32) {
        let t = *self.ui.theme();
        let r = Rect::new(10.0, 66.0, self.nav_w, h - 66.0 - 40.0);
        self.ui.panel(r, t.panel);
        let mut y = r.y + 10.0;
        for (tab_id, label) in TABS {
            let br = Rect::new(r.x + 8.0, y, r.w - 16.0, 38.0);
            let on = self.tab == *tab_id;
            if on {
                fill_rrect(br, 6.0, t.accent_soft);
                stroke_rrect(br, 6.0, 1.0, Color::new(t.accent.r, t.accent.g, t.accent.b, 0.40));
                fill_rrect(Rect::new(br.x + 2.0, br.y + 7.0, 3.5, br.h - 14.0), 1.5, t.accent_hi);
            } else if self.ui.hover(br) {
                fill_rrect(br, 6.0, Color::new(1.0, 1.0, 1.0, 0.045));
            }
            let extra = match tab_id {
                Tab::Mods => format!("{}", self.lib.mods.len()),
                Tab::Coop if self.coop.is_some() => format!("{}", self.players.len() + if self.hosting() { 1 } else { 0 }),
                _ => String::new(),
            };
            if on {
                self.ui.btext(label, br.x + 14.0, br.y + 25.0, 17.0, t.text);
            } else {
                self.ui.text(label, br.x + 14.0, br.y + 25.0, 17.0, t.muted);
            }
            if !extra.is_empty() {
                let ew = self.ui.width(&extra, 13.0) + 12.0;
                let pill = Rect::new(br.x + br.w - ew - 8.0, br.y + 10.0, ew, 18.0);
                fill_rrect(pill, 9.0, if on { Color::new(t.accent.r, t.accent.g, t.accent.b, 0.28) } else { t.field_bg });
                self.ui.text(&extra, pill.x + 6.0, pill.y + 13.5, 13.0, if on { t.text } else { t.muted });
            }
            if self.ui.click(br) {
                self.tab = *tab_id;
            }
            y += 42.0;
        }

        // Prominent sidebar Play Game button.
        let running = self.game_running();
        let busy = self.is_busy();
        let play_nav = Rect::new(r.x + 10.0, y + 8.0, r.w - 20.0, 44.0);
        let pnhov = self.ui.hover(play_nav) && !running && !busy;
        let pn_bg = if running {
            Color::new(t.good.r, t.good.g, t.good.b, 0.20)
        } else if pnhov {
            t.accent_hi
        } else {
            t.accent
        };
        fill_rrect(play_nav, 7.0, pn_bg);
        stroke_rrect(
            play_nav,
            7.0,
            1.4,
            if running { t.good } else if pnhov { WHITE } else { t.accent_hi },
        );
        if running {
            draw_circle(play_nav.x + 22.0, play_nav.y + 22.0, 5.0, t.good);
            self.ui.btext("GAME RUNNING", play_nav.x + 36.0, play_nav.y + 27.5, 15.0, t.good);
        } else {
            let fg = if pnhov { Color::new(0.05, 0.06, 0.08, 1.0) } else { WHITE };
            self.ui.play_icon(play_nav.x + 22.0, play_nav.y + 14.0, 16.0, fg);
            self.ui.btext("PLAY GAME", play_nav.x + 44.0, play_nav.y + 28.0, 16.5, fg);
            if !busy && self.ui.click(play_nav) {
                self.launch_game();
            }
        }
        let dz = Rect::new(r.x + 10.0, r.y + r.h - 154.0, r.w - 20.0, 94.0);
        fill_rrect(dz, 7.0, Color::new(t.accent.r, t.accent.g, t.accent.b, 0.07));
        stroke_rrect(dz, 7.0, 1.2, Color::new(t.accent.r, t.accent.g, t.accent.b, 0.48));
        self.ui.btext("Drop mods here", dz.x + 12.0, dz.y + 24.0, 15.0, t.text);
        self.ui.text("Drag .zip .7z .rar", dz.x + 12.0, dz.y + 46.0, 13.0, t.muted);
        self.ui.text("or .pak files onto", dz.x + 12.0, dz.y + 63.0, 13.0, t.muted);
        self.ui.text("this window.", dz.x + 12.0, dz.y + 80.0, 13.0, t.muted);
        let inst = self.lib.state.mods.len();
        let avail = self.lib.mods.iter().filter(|m| !m.installed).count();
        draw_circle(r.x + 18.0, r.y + r.h - 36.0, 3.5, if inst > 0 { t.good } else { t.muted });
        self.ui.text(&format!("{} installed", inst), r.x + 27.0, r.y + r.h - 32.0, 14.0, if inst > 0 { t.good } else { t.muted });
        draw_circle(r.x + 18.0, r.y + r.h - 18.0, 3.5, t.muted);
        self.ui.text(&format!("{} not installed", avail), r.x + 27.0, r.y + r.h - 14.0, 14.0, t.muted);
        self.drawn.push("nav");
    }

    fn draw_status(&mut self, w: f32, h: f32) {
        let t = *self.ui.theme();
        let r = Rect::new(0.0, h - 30.0, w, 30.0);
        draw_rectangle(r.x, r.y, r.w, r.h, t.status_bar);
        draw_line(0.0, r.y, w, r.y, 1.0, t.line);
        let mut x = 14.0;
        if self.is_busy() {
            let phase = get_time() as f32 * 6.0;
            draw_circle(x + 6.0 + phase.sin() * 3.0, r.y + 15.0, 4.0, t.accent_hi);
            // After 1 s the original shows "Installing mod! (%i/%i)" (function 0x14011d9e0).
            let (done, total) = (
                self.job_progress.0.load(std::sync::atomic::Ordering::Relaxed),
                self.job_progress.1.load(std::sync::atomic::Ordering::Relaxed),
            );
            let b = if get_time() - self.job_t0 > 1.0 && total > 0 {
                format!("{}! ({}/{})", self.busy, done.min(total), total)
            } else {
                format!("{}...", self.busy)
            };
            self.ui.text(&b, x + 18.0, r.y + 20.0, 14.0, t.text);
            x += 38.0 + self.ui.width(&b, 14.0);
        }
        if let Some((s, c, ts)) = &self.toast {
            if get_time() - ts < 12.0 || self.headless {
                let col = if *c == BAD { t.bad } else { t.text };
                let s = self.ui.fit(s, 14.0, (w - x - 340.0).max(50.0));
                self.ui.text(&s, x, r.y + 20.0, 14.0, col);
            }
        }
        let mode = if self.game_running() && self.target_fps <= 5.0 {
            format!("v{}  |  Power: {:.0} fps (the game has priority)", crate::app::VERSION, self.target_fps)
        } else {
            format!("v{}  |  Power: {:.0} fps max{}", crate::app::VERSION, self.target_fps, if self.target_fps < 30.0 { " (in background)" } else { "" })
        };
        let mw = self.ui.width(&mode, 13.0);
        self.ui.text(&mode, w - mw - 14.0, r.y + 20.0, 13.0, t.muted);
        self.drawn.push("status");
    }

    fn draw_mods(&mut self, area: Rect) {
        let t = *self.ui.theme();
        let mut search = self.fields.remove(&Field::Search).unwrap_or_default();
        let before = search.clone();
        self.ui.field(Rect::new(area.x, area.y, 250.0, 34.0), Field::Search, &mut search, "Search mods, authors, tags");
        if search != before {
            self.sel = 0;
            self.scroll = 0.0;
        }
        self.fields.insert(Field::Search, search);
        let mut bx = area.x + 260.0;
        for (f, label) in [(Filter::All, "All"), (Filter::Installed, "Installed"), (Filter::Available, "Not installed")] {
            let bw = self.ui.width(label, 15.0) + 24.0;
            if self.ui.button(Rect::new(bx, area.y, bw, 34.0), label, if self.filter == f { 1 } else { 0 }, true) {
                self.filter = f;
                self.sel = 0;
                self.scroll = 0.0;
            }
            bx += bw + 6.0;
        }
        let busy = self.is_busy();
        let vis = self.visible();
        let mut rx = area.x + area.w;
        let mut clicked = None;
        for label in ["Turn All OFF", "Turn All ON", "Check files", "Open folder", "Refresh"] {
            let bw = self.ui.width(label, 14.5) + 18.0;
            rx -= bw;
            if self.ui.button(Rect::new(rx, area.y, bw, 34.0), label, 0, !busy || label == "Open folder") {
                clicked = Some(label);
            }
            rx -= 6.0;
        }
        match clicked {
            Some("Turn All OFF") => {
                if self.selected_category.is_none() && self.filter == Filter::All && self.fields.get(&Field::Search).map(|s| s.is_empty()).unwrap_or(true) {
                    self.run_job("Turning all mods OFF", |l| vec![l.uninstall_all()], None);
                } else {
                    let keys: Vec<String> = vis.iter().map(|&i| self.lib.mods[i].key.clone()).collect();
                    self.set_mods_enabled(keys, false);
                }
            }
            Some("Turn All ON") => {
                let keys: Vec<String> = vis.iter().filter(|&&i| self.lib.mods[i].size > 0).map(|&i| self.lib.mods[i].key.clone()).collect();
                self.set_mods_enabled(keys, true);
            }
            Some("Check files") => self.run_job(
                "Checking installed files",
                |l| {
                    let r = l.verify();
                    if r.missing.is_empty() && r.changed.is_empty() {
                        vec![Ok(format!("All {} installed mod files are intact.", r.checked))]
                    } else {
                        vec![Err(format!("{} missing and {} changed file(s); repairing...", r.missing.len(), r.changed.len())), l.repair()]
                    }
                },
                None,
            ),
            Some("Open folder") => sys::open(&self.lib.mods_dir().to_string_lossy()),
            Some("Refresh") => self.actions.push_back(Action::Refresh),
            _ => {}
        }

        let max_cat_w = (area.w * 0.36).clamp(180.0, 380.0);
        self.cat_w = self.cat_w.clamp(145.0, max_cat_w);
        let cat_r = Rect::new(area.x, area.y + 44.0, self.cat_w, area.h - 44.0);
        let cat_split_x = cat_r.x + cat_r.w + 5.0;
        if let Some(mx) = self.ui.vsplitter(DragId::CatWidth, cat_split_x, cat_r.y, cat_r.h) {
            let cw = (mx - area.x).clamp(145.0, max_cat_w).round();
            if (cw - self.cat_w).abs() >= 1.0 {
                self.cat_w = cw;
                self.cfg.set("layout_cat_w", cw as i64);
                self.layout_dirty = true;
            }
        }

        let rem_w = (area.w - self.cat_w - 20.0).max(320.0);
        let list_w = (rem_w * (self.modlist_pct / 100.0)).floor().clamp(220.0, rem_w - 200.0);
        let list_r = Rect::new(cat_r.x + cat_r.w + 10.0, cat_r.y, list_w, cat_r.h);
        let list_split_x = list_r.x + list_r.w + 5.0;
        if let Some(mx) = self.ui.vsplitter(DragId::ModListSplit, list_split_x, list_r.y, list_r.h) {
            let pct = (((mx - list_r.x) / rem_w) * 100.0).clamp(28.0, 74.0).round();
            if (pct - self.modlist_pct).abs() >= 1.0 {
                self.modlist_pct = pct;
                self.cfg.set("layout_modlist_pct", pct as i64);
                self.layout_dirty = true;
            }
        }
        let det_r = Rect::new(list_r.x + list_r.w + 10.0, list_r.y, (rem_w - list_w).max(180.0), list_r.h);

        self.draw_categories_panel(cat_r);
        self.ui.panel(list_r, t.panel);

        // Top context header inside the mod list so it's always obvious which category is shown.
        let hdr_r = Rect::new(list_r.x + 6.0, list_r.y + 6.0, list_r.w - 12.0, 32.0);
        fill_rrect(hdr_r, 6.0, t.panel2);
        stroke_rrect(hdr_r, 6.0, 1.0, t.line);
        if let Some((grp, cat)) = self.selected_category.clone() {
            let title = format!("{}  >  {} ({})", grp, cat.to_uppercase(), vis.len());
            let fit_t = self.ui.bfit(&title, 14.0, (hdr_r.w - 116.0).max(40.0));
            self.ui.btext(&fit_t, hdr_r.x + 12.0, hdr_r.y + 21.0, 14.0, t.accent_hi);
            let all_btn = Rect::new(hdr_r.x + hdr_r.w - 102.0, hdr_r.y + 4.0, 96.0, 24.0);
            if self.ui.button(all_btn, "Show All Mods", 1, true) {
                self.selected_category = None;
                self.sel = 0;
                self.scroll = 0.0;
            }
        } else {
            let title = format!("ALL MODS ({})", vis.len());
            self.ui.btext(&title, hdr_r.x + 12.0, hdr_r.y + 21.0, 14.0, t.text);
            let grp_label = if self.group_by_category { "Category Headers: ON" } else { "Category Headers: OFF" };
            let gw = self.ui.width(grp_label, 12.5) + 20.0;
            let grp_btn = Rect::new(hdr_r.x + hdr_r.w - gw - 6.0, hdr_r.y + 4.0, gw, 24.0);
            let ghov = self.ui.hover(grp_btn);
            fill_rrect(grp_btn, 5.0, if self.group_by_category { t.accent_soft } else if ghov { t.btn_hover } else { t.field_bg });
            stroke_rrect(grp_btn, 5.0, 1.0, if self.group_by_category { t.accent } else { t.line });
            self.ui.text(grp_label, grp_btn.x + 10.0, grp_btn.y + 16.5, 12.5, if self.group_by_category { t.text } else { t.muted });
            if self.ui.click(grp_btn) {
                self.group_by_category = !self.group_by_category;
                self.cfg.set("group_by_category", if self.group_by_category { 1 } else { 0 });
                self.layout_dirty = true;
            }
        }

        let body_r = Rect::new(list_r.x, list_r.y + 42.0, list_r.w, list_r.h - 42.0);
        if self.lib.mods.is_empty() {
            self.draw_empty(body_r);
        } else {
            enum DisplayItem {
                Header(String, String, usize, usize),
                Mod(usize, usize),
            }
            let mut items: Vec<DisplayItem> = Vec::new();
            let show_headers = self.group_by_category && self.selected_category.is_none();
            let mut last_cat: Option<(String, String)> = None;
            for (vi, &mi) in vis.iter().enumerate() {
                if show_headers {
                    let m = &self.lib.mods[mi];
                    let key = (m.cat_group.clone(), m.cat_name.clone());
                    if last_cat.as_ref() != Some(&key) {
                        let cnt = vis
                            .iter()
                            .filter(|&&idx| {
                                let om = &self.lib.mods[idx];
                                om.cat_group == key.0 && om.cat_name == key.1
                            })
                            .count();
                        let icnt = vis
                            .iter()
                            .filter(|&&idx| {
                                let om = &self.lib.mods[idx];
                                om.cat_group == key.0 && om.cat_name == key.1 && om.installed
                            })
                            .count();
                        items.push(DisplayItem::Header(key.0.clone(), key.1.clone(), cnt, icnt));
                        last_cat = Some(key);
                    }
                }
                items.push(DisplayItem::Mod(vi, mi));
            }

            let total_h: f32 = items
                .iter()
                .map(|it| match it {
                    DisplayItem::Header(..) => 30.0,
                    DisplayItem::Mod(..) => 44.0,
                })
                .sum();

            if self.ui.hover(body_r) && self.ui.wheel != 0.0 {
                self.scroll -= self.ui.wheel.signum() * 44.0 * 2.0;
            }
            let max_scroll = (total_h - (body_r.h - 10.0)).max(0.0);
            self.scroll = self.scroll.clamp(0.0, max_scroll);
            let mut cur_y = body_r.y + 4.0 - self.scroll;
            let mut toggle = None;
            let mut pick_cat = None;
            let mut bulk_cat_toggle: Option<(String, String, bool)> = None;

            for it in &items {
                match it {
                    DisplayItem::Header(grp, cat, cnt, icnt) => {
                        let h = 28.0;
                        if cur_y >= body_r.y - 4.0 && cur_y + h <= body_r.y + body_r.h {
                            let hr = Rect::new(body_r.x + 8.0, cur_y + 2.0, body_r.w - 18.0, h - 4.0);
                            let all_active = *icnt == *cnt && *cnt > 0;
                            let sw_label = if all_active { "All OFF" } else { "All ON" };
                            let sw_w = self.ui.width(sw_label, 11.5) + 16.0;
                            let sw_btn = Rect::new(hr.x + hr.w - sw_w - 4.0, hr.y + 3.0, sw_w, hr.h - 6.0);
                            let swhov = self.ui.hover(sw_btn) && !busy;
                            let hhov = self.ui.hover(hr) && !self.ui.hover(sw_btn);

                            fill_rrect(hr, 5.0, if hhov { t.accent_soft } else { Color::new(t.accent.r, t.accent.g, t.accent.b, 0.13) });
                            stroke_rrect(hr, 5.0, 1.0, Color::new(t.accent.r, t.accent.g, t.accent.b, if hhov { 0.55 } else { 0.28 }));
                            fill_rrect(Rect::new(hr.x + 2.0, hr.y + 4.0, 3.0, hr.h - 8.0), 1.5, t.accent_hi);
                            let hlabel = format!("{}  >  {}", grp, cat.to_uppercase());
                            self.ui.btext(&hlabel, hr.x + 12.0, hr.y + 17.0, 13.0, t.accent_hi);
                            let right_txt = format!("{} / {} ON", icnt, cnt);
                            let rw = self.ui.width(&right_txt, 12.0);
                            self.ui.text(&right_txt, sw_btn.x - rw - 10.0, hr.y + 16.5, 12.0, if *icnt > 0 { t.good } else { t.muted });

                            fill_rrect(
                                sw_btn,
                                4.0,
                                if all_active {
                                    Color::new(t.good.r, t.good.g, t.good.b, if swhov { 0.34 } else { 0.20 })
                                } else if swhov {
                                    t.accent_hi
                                } else {
                                    t.field_bg
                                },
                            );
                            stroke_rrect(sw_btn, 4.0, 1.0, if all_active { t.good } else { t.line_hi });
                            self.ui.btext(
                                sw_label,
                                sw_btn.x + 8.0,
                                sw_btn.y + 13.5,
                                11.5,
                                if all_active {
                                    t.good
                                } else if swhov {
                                    Color::new(0.05, 0.06, 0.08, 1.0)
                                } else {
                                    t.text
                                },
                            );

                            if !busy && self.ui.click(sw_btn) {
                                bulk_cat_toggle = Some((grp.clone(), cat.clone(), !all_active));
                            } else if self.ui.click(hr) {
                                pick_cat = Some((grp.clone(), cat.clone()));
                            }
                        }
                        cur_y += 30.0;
                    }
                    DisplayItem::Mod(vi, mi) => {
                        let row = 44.0;
                        if cur_y >= body_r.y - 4.0 && cur_y + row <= body_r.y + body_r.h + 2.0 {
                            let m = self.lib.mods[*mi].clone();
                            let rr = Rect::new(body_r.x + 6.0, cur_y, body_r.w - 14.0, row - 3.0);
                            if *vi == self.sel {
                                fill_rrect(rr, 6.0, t.accent_soft);
                                stroke_rrect(rr, 6.0, 1.0, Color::new(t.accent.r, t.accent.g, t.accent.b, 0.35));
                                fill_rrect(Rect::new(rr.x + 2.0, rr.y + 6.0, 3.5, rr.h - 12.0), 1.5, t.accent_hi);
                            } else if self.ui.hover(rr) {
                                fill_rrect(rr, 6.0, Color::new(1.0, 1.0, 1.0, 0.04));
                            }

                            // Single interactive ON / OFF pill switch on the left side of the mod row.
                            let sw_r = Rect::new(rr.x + 10.0, rr.y + 9.0, 42.0, 22.0);
                            let sw_hov = self.ui.hover(sw_r) && !busy && (m.size > 0 || m.installed);
                            let sw_bg = if m.installed {
                                Color::new(t.good.r, t.good.g, t.good.b, if sw_hov { 0.42 } else { 0.28 })
                            } else if sw_hov {
                                t.btn_hover
                            } else {
                                t.field_bg
                            };
                            fill_rrect(sw_r, 11.0, sw_bg);
                            stroke_rrect(sw_r, 11.0, 1.2, if m.installed { t.good } else if sw_hov { t.accent_hi } else { t.line_hi });
                            let knob_x = if m.installed { sw_r.x + sw_r.w - 11.0 } else { sw_r.x + 11.0 };
                            draw_circle(knob_x, sw_r.y + sw_r.h / 2.0, 7.0, if m.installed { t.good } else { t.muted });

                            // Right-aligned tags with exact 12.0pt measurement and 6px spacing so tags never touch.
                            let tag_y = rr.y + (rr.h - 19.0) * 0.5;
                            let mut tx = rr.x + rr.w - 10.0;
                            if !m.version.is_empty() {
                                let v = self.ui.fit(&m.version, 12.0, 68.0);
                                let tw = self.ui.width(&v, 12.0) + 12.0;
                                tx -= tw;
                                self.ui.tag(tx, tag_y, &v, t.muted);
                                tx -= 6.0;
                            }
                            if !m.cat_name.is_empty() {
                                let ctag = self.ui.fit(&m.cat_name, 12.0, 108.0);
                                let tw = self.ui.width(&ctag, 12.0) + 12.0;
                                tx -= tw;
                                self.ui.tag(tx, tag_y, &ctag, t.accent_hi);
                                tx -= 6.0;
                            }
                            if !m.conflicts.is_empty() {
                                let tw = self.ui.width("shares files", 12.0) + 12.0;
                                tx -= tw;
                                self.ui.tag(tx, tag_y, "shares files", t.warn);
                                tx -= 6.0;
                            }

                            let name_x = sw_r.x + sw_r.w + 12.0;
                            let avail_w = (tx - name_x - 10.0).max(40.0);
                            if *vi == self.sel {
                                let nm = self.ui.bfit(&m.name, 15.5, avail_w);
                                self.ui.btext(&nm, name_x, rr.y + 25.5, 15.5, t.text);
                            } else {
                                let nm = self.ui.fit(&m.name, 15.5, avail_w);
                                self.ui.text(&nm, name_x, rr.y + 25.5, 15.5, t.text);
                            }

                            let can_toggle = !busy && (m.size > 0 || m.installed);
                            if can_toggle && self.ui.click(sw_r) {
                                self.sel = *vi;
                                toggle = Some(m.key.clone());
                            } else if self.ui.click(rr) {
                                self.sel = *vi;
                            }
                        }
                        cur_y += row;
                    }
                }
            }
            if let Some((grp, cat, enable)) = bulk_cat_toggle {
                let keys: Vec<String> = self
                    .lib
                    .mods
                    .iter()
                    .filter(|m| m.cat_group.eq_ignore_ascii_case(&grp) && m.cat_name.eq_ignore_ascii_case(&cat) && (m.size > 0 || m.installed))
                    .map(|m| m.key.clone())
                    .collect();
                self.set_mods_enabled(keys, enable);
            }
            if let Some(cat_pair) = pick_cat {
                self.selected_category = Some(cat_pair);
                self.sel = 0;
                self.scroll = 0.0;
            }
            if let Some(k) = toggle {
                self.toggle_mod(k);
            }
            if vis.is_empty() {
                if let Some((grp, cat)) = self.selected_category.clone() {
                    let msg1 = format!("No mods in {} > {} yet.", grp, cat);
                    let msg2 = "Drop a .zip, .rar, .7z or .pak mod onto this window, or view all mods.";
                    self.ui.btext(&msg1, body_r.x + 22.0, body_r.y + 46.0, 17.0, t.text);
                    self.ui.text(msg2, body_r.x + 22.0, body_r.y + 72.0, 14.0, t.muted);
                    if self.ui.button(Rect::new(body_r.x + 22.0, body_r.y + 92.0, 140.0, 34.0), "Show All Mods", 1, true) {
                        self.selected_category = None;
                        self.sel = 0;
                        self.scroll = 0.0;
                    }
                } else {
                    self.ui.text("No mods match this filter.", body_r.x + 20.0, body_r.y + 40.0, 16.0, t.muted);
                }
            }
            if max_scroll > 0.0 {
                let frac = (body_r.h - 10.0) / total_h;
                let bh = (body_r.h * frac).max(30.0);
                let by = body_r.y + 4.0 + (body_r.h - 8.0 - bh) * (self.scroll / max_scroll);
                fill_rrect(Rect::new(body_r.x + body_r.w - 6.0, by, 3.5, bh), 1.5, t.accent_hi);
            }
        }
        self.drawn.push("modlist");
        self.draw_detail(det_r);
    }

    /// Left-hand Category Navigator inspired by Fluffy Mod Manager's CHARACTERS & OTHER
    /// category list, redesigned with clear status indicators, counts, and one-click filtering.
    fn draw_categories_panel(&mut self, r: Rect) {
        let t = *self.ui.theme();
        self.ui.panel(r, t.panel);

        let total_mods = self.lib.mods.len();
        let total_inst = self.lib.state.mods.len();
        let all_on = self.selected_category.is_none();
        let all_r = Rect::new(r.x + 8.0, r.y + 8.0, r.w - 16.0, 32.0);
        let all_hov = self.ui.hover(all_r);
        fill_rrect(
            all_r,
            6.0,
            if all_on {
                t.accent_soft
            } else if all_hov {
                t.btn_hover
            } else {
                t.panel2
            },
        );
        stroke_rrect(all_r, 6.0, 1.2, if all_on { t.accent_hi } else { t.line_hi });
        let lbl = format!("All mods ({})", total_mods);
        let lw = self.ui.width(&lbl, 14.5);
        if total_inst > 0 {
            draw_circle(all_r.x + 14.0, all_r.y + 16.0, 4.5, t.good);
        }
        self.ui.btext(&lbl, all_r.x + (all_r.w - lw) / 2.0, all_r.y + 21.0, 14.5, t.text);
        if self.ui.click(all_r) {
            self.selected_category = None;
            self.sel = 0;
            self.scroll = 0.0;
        }

        let (chars, others) = self.category_tree();
        let list_top = r.y + 46.0;
        let list_h = r.h - 52.0;
        let body = Rect::new(r.x, list_top, r.w, list_h);

        let sec_h = 26.0;
        let row_h = 29.0;
        let content_h = sec_h * 2.0 + (chars.len() + others.len()) as f32 * row_h + 10.0;
        if self.ui.hover(body) && self.ui.wheel != 0.0 {
            self.cat_scroll -= self.ui.wheel.signum() * row_h * 2.0;
        }
        let max_scroll = (content_h - list_h).max(0.0);
        self.cat_scroll = self.cat_scroll.clamp(0.0, max_scroll);

        let mut y = list_top - self.cat_scroll;
        let mut clicked_cat: Option<(String, String)> = None;
        let mut toggle_cat_sq: Option<(String, String, bool)> = None;
        let busy = self.is_busy();

        for (section_title, group_key, entries) in [("CHARACTERS", "CHARACTERS", &chars), ("OTHER", "OTHER", &others)] {
            if y >= list_top - 4.0 && y + sec_h <= list_top + list_h + 2.0 {
                let sr = Rect::new(r.x + 8.0, y + 2.0, r.w - 16.0, sec_h - 4.0);
                // Crisp high-contrast section banner like Fluffy's CHARACTERS / OTHER bar
                fill_rrect(sr, 4.0, Color::new(t.text.r, t.text.g, t.text.b, 0.92));
                let sw = self.ui.width(section_title, 13.0);
                self.ui.btext(section_title, sr.x + (sr.w - sw) / 2.0, sr.y + 16.0, 13.0, Color::new(0.06, 0.07, 0.09, 1.0));
            }
            y += sec_h;

            for (cname, count, inst_count) in entries {
                if y >= list_top - 4.0 && y + row_h <= list_top + list_h + 2.0 {
                    let is_sel = self
                        .selected_category
                        .as_ref()
                        .map(|(g, c)| g.eq_ignore_ascii_case(group_key) && c.eq_ignore_ascii_case(cname))
                        .unwrap_or(false);
                    let sq = Rect::new(r.x + 8.0, y + 2.0, 22.0, row_h - 4.0);
                    let bar = Rect::new(r.x + 33.0, y + 2.0, r.w - 41.0, row_h - 4.0);
                    let sq_hov = self.ui.hover(sq) && *count > 0 && !busy;
                    let hov = self.ui.hover(bar);

                    // Left status checkbox square (click to toggle all mods in this category ON/OFF)
                    fill_rrect(
                        sq,
                        4.0,
                        if *inst_count > 0 {
                            Color::new(t.good.r, t.good.g, t.good.b, if sq_hov { 0.38 } else { 0.22 })
                        } else if *count > 0 {
                            if sq_hov { t.btn_hover } else { t.accent_soft }
                        } else {
                            t.field_bg
                        },
                    );
                    stroke_rrect(
                        sq,
                        4.0,
                        1.0,
                        if *inst_count > 0 {
                            t.good
                        } else if sq_hov || is_sel {
                            t.accent_hi
                        } else {
                            t.line
                        },
                    );
                    if *inst_count > 0 {
                        draw_circle(sq.x + sq.w / 2.0, sq.y + sq.h / 2.0, 4.5, t.good);
                    } else if *count > 0 {
                        draw_circle(sq.x + sq.w / 2.0, sq.y + sq.h / 2.0, 3.0, t.accent_hi);
                    }

                    // Category button bar
                    fill_rrect(
                        bar,
                        4.0,
                        if is_sel {
                            t.accent_soft
                        } else if hov {
                            t.btn_hover
                        } else {
                            t.panel2
                        },
                    );
                    stroke_rrect(
                        bar,
                        4.0,
                        if is_sel { 1.4 } else { 1.0 },
                        if is_sel {
                            t.accent_hi
                        } else if hov {
                            t.line_hi
                        } else {
                            t.line
                        },
                    );

                    let label = format!("{} ({})", cname, count);
                    let fit_lbl = self.ui.fit(&label, 13.5, bar.w - 12.0);
                    let lw = self.ui.width(&fit_lbl, 13.5);
                    let txt_col = if is_sel {
                        t.accent_hi
                    } else if *count > 0 {
                        t.text
                    } else {
                        t.muted
                    };
                    if *count > 0 || is_sel {
                        self.ui.btext(&fit_lbl, bar.x + (bar.w - lw) / 2.0, bar.y + 17.5, 13.5, txt_col);
                    } else {
                        self.ui.text(&fit_lbl, bar.x + (bar.w - lw) / 2.0, bar.y + 17.5, 13.5, txt_col);
                    }

                    if *count > 0 && !busy && self.ui.click(sq) {
                        toggle_cat_sq = Some((group_key.to_string(), cname.clone(), *inst_count < *count));
                    } else if self.ui.click(bar) || ( *count == 0 && self.ui.click(sq) ) {
                        clicked_cat = Some((group_key.to_string(), cname.clone()));
                    }
                }
                y += row_h;
            }
            y += 4.0;
        }

        if let Some((grp, cat, enable)) = toggle_cat_sq {
            let keys: Vec<String> = self
                .lib
                .mods
                .iter()
                .filter(|m| {
                    ((m.cat_group.eq_ignore_ascii_case(&grp) && m.cat_name.eq_ignore_ascii_case(&cat))
                        || (grp == "OTHER" && m.categories.iter().any(|c| c.eq_ignore_ascii_case(&cat))))
                        && (m.size > 0 || m.installed)
                })
                .map(|m| m.key.clone())
                .collect();
            self.set_mods_enabled(keys, enable);
        }

        if let Some((g, c)) = clicked_cat {
            let same = self
                .selected_category
                .as_ref()
                .map(|( sg, sc )| sg.eq_ignore_ascii_case(&g) && sc.eq_ignore_ascii_case(&c))
                .unwrap_or(false);
            if same {
                self.selected_category = None;
            } else {
                self.selected_category = Some((g, c));
            }
            self.sel = 0;
            self.scroll = 0.0;
        }

        if max_scroll > 0.0 {
            let frac = list_h / content_h;
            let bh = (list_h * frac).max(24.0);
            let by = list_top + (list_h - bh) * (self.cat_scroll / max_scroll);
            fill_rrect(Rect::new(r.x + r.w - 5.0, by, 3.0, bh), 1.5, t.accent_hi);
        }
    }

    fn draw_empty(&mut self, r: Rect) {
        let t = *self.ui.theme();
        let dz = Rect::new(r.x + 24.0, r.y + 24.0, r.w - 48.0, r.h - 48.0);
        let pulse = (get_time() * 2.0).sin() as f32 * 0.12 + 0.52;
        let col = Color::new(t.accent.r, t.accent.g, t.accent.b, pulse);
        fill_rrect(dz, 10.0, Color::new(t.accent.r, t.accent.g, t.accent.b, 0.05));
        let dash = 14.0;
        let mut x = dz.x + 10.0;
        while x < dz.x + dz.w - 10.0 {
            let x2 = (x + dash * 0.6).min(dz.x + dz.w - 10.0);
            draw_line(x, dz.y, x2, dz.y, 1.8, col);
            draw_line(x, dz.y + dz.h, x2, dz.y + dz.h, 1.8, col);
            x += dash;
        }
        let mut y = dz.y + 10.0;
        while y < dz.y + dz.h - 10.0 {
            let y2 = (y + dash * 0.6).min(dz.y + dz.h - 10.0);
            draw_line(dz.x, y, dz.x, y2, 1.8, col);
            draw_line(dz.x + dz.w, y, dz.x + dz.w, y2, 1.8, col);
            y += dash;
        }
        let cx = dz.x + dz.w / 2.0;
        let cy = dz.y + dz.h / 2.0 - 44.0;
        draw_circle(cx, cy - 4.0, 56.0, Color::new(t.accent.r, t.accent.g, t.accent.b, 0.08));
        let tray = Rect::new(cx - 42.0, cy - 10.0, 84.0, 54.0);
        fill_rrect(tray, 10.0, t.accent_soft);
        stroke_rrect(tray, 10.0, 2.0, t.accent_hi);
        let tab_r = Rect::new(cx - 28.0, cy - 18.0, 28.0, 12.0);
        fill_rrect(tab_r, 4.0, t.accent);
        let bob = (get_time() * 2.5).sin() as f32 * 3.0;
        let ay = cy - 48.0 + bob;
        fill_rrect(Rect::new(cx - 5.0, ay, 10.0, 26.0), 3.0, t.text);
        draw_triangle(vec2(cx - 15.0, ay + 22.0), vec2(cx + 15.0, ay + 22.0), vec2(cx, ay + 40.0), t.text);
        let l1 = "Drop mod files here";
        self.ui.btext(l1, cx - self.ui.width(l1, 25.0) / 2.0 - 4.0, cy + 92.0, 25.0, t.text);
        let lines = [
            "Drag .zip, .7z, .rar or .pak files (or mod folders) onto this window.".to_string(),
            format!("They are copied into the {} Mods folder and listed right away.", self.game().folder),
            "Mods copied into that folder with Explorer also appear by themselves.".to_string(),
        ];
        for (i, l) in lines.iter().enumerate() {
            let l = self.ui.fit(l, 14.0, dz.w - 24.0);
            self.ui.text(&l, cx - self.ui.width(&l, 14.0) / 2.0, cy + 124.0 + i as f32 * 21.0, 14.0, t.muted);
        }
        self.drawn.push("dropzone");
    }

    fn shot(&mut self, p: &Path) -> Option<Texture2D> {
        if let Some(t) = self.shots.get(p) {
            return t.clone();
        }
        let tex = std::fs::read(p).ok().and_then(|b| image::load_from_memory(&b).ok()).map(|img| {
            let img = img.thumbnail(960, 540).to_rgba8();
            let t = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, img.as_raw());
            t.set_filter(FilterMode::Linear);
            t
        });
        self.shots.insert(p.to_path_buf(), tex.clone());
        tex
    }

    fn draw_detail(&mut self, r: Rect) {
        let t = *self.ui.theme();
        self.ui.panel(r, t.panel);
        let vis = self.visible();
        let Some(&mi) = vis.get(self.sel) else {
            self.ui.btext("How it works", r.x + 18.0, r.y + 36.0, 20.0, t.text);
            draw_line(r.x + 18.0, r.y + 46.0, r.x + r.w - 18.0, r.y + 46.0, 1.0, t.line);
            let tips = [
                "Install copies a mod's files into the game and backs up every original file first.",
                "Uninstall puts the originals back exactly, even when two mods changed the same file.",
                "Check files finds mod files a game update overwrote and repairs them.",
                "Keys: Up/Down move, Enter installs or uninstalls, Left/Right changes game, F5 refreshes, Tab switches page.",
            ];
            let mut y = r.y + 72.0;
            for tip in tips {
                draw_circle(r.x + 24.0, y - 5.0, 3.5, t.accent_hi);
                y = self.ui.wrap(tip, r.x + 36.0, y, r.w - 54.0, 15.0, t.muted, 4) + 10.0;
            }
            self.drawn.push("detail");
            return;
        };
        let m = self.lib.mods[mi].clone();
        let pw = r.w - 24.0;
        let ph = (r.h * (self.preview_pct / 100.0)).clamp(100.0, (r.h - 210.0).max(120.0));
        let pr = Rect::new(r.x + 12.0, r.y + 12.0, pw, ph);
        let split_y = pr.y + pr.h + 5.0;
        if let Some(my) = self.ui.hsplitter(DragId::PreviewHeight, pr.x, split_y, pr.w) {
            let pct = (((my - (r.y + 12.0)) / r.h) * 100.0).clamp(18.0, 68.0).round();
            if (pct - self.preview_pct).abs() >= 1.0 {
                self.preview_pct = pct;
                self.cfg.set("layout_preview_pct", pct as i64);
                self.layout_dirty = true;
            }
        }
        let tex = if self.cfg.flag("ShowModPreviewScreenshots") { m.screenshot.as_ref().and_then(|p| self.shot(p)) } else { None };
        match tex {
            Some(tex_img) => {
                let s = (pr.w / tex_img.width()).min(pr.h / tex_img.height());
                let (dw, dh) = (tex_img.width() * s, tex_img.height() * s);
                fill_rrect(pr, 6.0, t.field_bg);
                draw_texture_ex(&tex_img, pr.x + (pr.w - dw) / 2.0, pr.y + (pr.h - dh) / 2.0, WHITE, DrawTextureParams { dest_size: Some(vec2(dw, dh)), ..Default::default() });
                stroke_rrect(pr, 6.0, 1.0, t.line_hi);
            }
            None => {
                let hue = (crc32fast::hash(m.name.as_bytes()) % 360) as f32 / 360.0;
                let c1 = macroquad::color::hsl_to_rgb(hue, 0.48, 0.22);
                let c2 = macroquad::color::hsl_to_rgb((hue + 0.08) % 1.0, 0.52, 0.11);
                for i in 0..24 {
                    let k = i as f32 / 23.0;
                    let c = Color::new(c1.r + (c2.r - c1.r) * k, c1.g + (c2.g - c1.g) * k, c1.b + (c2.b - c1.b) * k, 1.0);
                    draw_rectangle(pr.x, pr.y + pr.h * i as f32 / 24.0, pr.w, pr.h / 24.0 + 1.0, c);
                }
                stroke_rrect(pr, 6.0, 1.0, t.line_hi);
                let initial: String = m.name.chars().take(1).collect::<String>().to_uppercase();
                let iw = self.ui.width(&initial, 64.0);
                self.ui.btext(&initial, pr.x + (pr.w - iw) / 2.0, pr.y + pr.h / 2.0 + 22.0, 64.0, Color::new(1.0, 1.0, 1.0, 0.35));
                self.ui.text("No screenshot", pr.x + 10.0, pr.y + pr.h - 10.0, 12.0, Color::new(1.0, 1.0, 1.0, 0.45));
            }
        }
        let mut y = pr.y + pr.h + 28.0;
        let title = self.ui.bfit(&m.name, 20.0, r.w - 36.0);
        self.ui.btext(&title, r.x + 12.0, y, 20.0, t.text);
        y += 22.0;
        let mut sub = Vec::new();
        if !m.author.is_empty() {
            sub.push(format!("by {}", m.author));
        }
        if !m.version.is_empty() {
            sub.push(m.version.clone());
        }
        if m.size > 0 {
            sub.push(fsx::human_size(m.size));
        }
        sub.push(format!("{} file(s)", m.files.len()));
        let subline = self.ui.fit(&sub.join("  |  "), 14.0, r.w - 24.0);
        self.ui.text(&subline, r.x + 12.0, y, 14.0, t.muted);
        y += 10.0;
        let mut tx = r.x + 12.0;
        if m.installed {
            tx += self.ui.tag(tx, y, "INSTALLED", t.good) + 6.0;
        }
        if !m.cat_name.is_empty() {
            let cat_badge = format!("{} > {}", m.cat_group, m.cat_name);
            tx += self.ui.tag(tx, y, &cat_badge, t.accent_hi) + 6.0;
        }
        let chg_w = self.ui.width("Change Category", 11.5) + 14.0;
        if tx + chg_w < r.x + r.w - 10.0 {
            let chg_r = Rect::new(tx, y, chg_w, 20.0);
            let chov = self.ui.hover(chg_r);
            fill_rrect(chg_r, 4.0, if chov { t.btn_hover } else { t.field_bg });
            stroke_rrect(chg_r, 4.0, 1.0, if chov { t.accent_hi } else { t.line_hi });
            self.ui.text("Change Category", chg_r.x + 7.0, chg_r.y + 14.5, 11.5, if chov { t.text } else { t.muted });
            if self.ui.click(chg_r) {
                self.cycle_mod_category(&m.key);
            }
        }
        y += 34.0;
        let desc = if m.description.is_empty() { "No description came with this mod.".to_string() } else { m.description.clone() };
        let btn_y = r.y + r.h - 46.0;
        let room = (((btn_y - 60.0 - y) / 19.5).max(1.0)) as usize;
        y = self.ui.wrap(&desc, r.x + 12.0, y, r.w - 24.0, 15.0, t.text, room);
        y += 4.0;
        // Same wording as the original (function 0x14011d9e0) for installed mods it would overwrite.
        let installed_over: Vec<String> = if m.installed {
            Vec::new()
        } else {
            m.conflicts.iter().filter(|c| self.lib.mods.iter().any(|o| &o.name == *c && o.installed)).cloned().collect()
        };
        if !installed_over.is_empty() && y < btn_y - 40.0 {
            let s = if installed_over.len() == 1 {
                format!("Selected mod would overwrite files from this installed mod: {}", installed_over[0])
            } else {
                format!("Selected mod would overwrite files from {} installed mods, including this one: {}", installed_over.len(), installed_over[0])
            };
            y = self.ui.wrap(&s, r.x + 12.0, y, r.w - 24.0, 13.0, t.warn, 2);
        }
        if !m.conflicts.is_empty() && y < btn_y - 40.0 {
            let s = format!("Shares files with: {}. The one installed last wins; uninstalling puts the other one back.", m.conflicts.join(", "));
            y = self.ui.wrap(&s, r.x + 12.0, y, r.w - 24.0, 13.0, t.warn, 3);
        }
        if !m.requires.is_empty() && y < btn_y - 30.0 {
            y = self.ui.wrap(&format!("Requires: {}", m.requires), r.x + 12.0, y, r.w - 24.0, 13.0, t.muted, 2);
        }
        if !m.homepage.is_empty() && y < btn_y - 30.0 {
            y = self.ui.wrap(&format!("Home page: {}", m.homepage), r.x + 12.0, y, r.w - 24.0, 13.0, t.muted, 1);
        }
        if m.needs_reframework && y < btn_y - 20.0 {
            self.ui.wrap("This mod uses loose files: the game needs REFramework (dinput8.dll) to load them.", r.x + 12.0, y, r.w - 24.0, 13.0, t.warn, 2);
        }
        let busy = self.is_busy();
        let bw = (r.w - 24.0 - 12.0) / 3.0;
        let label = if m.installed { "Turn OFF" } else { "Turn ON" };
        if self.ui.button(Rect::new(r.x + 12.0, btn_y, bw, 34.0), label, 1, !busy && (m.size > 0 || m.installed)) {
            self.toggle_mod(m.key.clone());
        }
        if self.ui.button(Rect::new(r.x + 18.0 + bw, btn_y, bw, 34.0), "Show file", 0, m.size > 0) {
            sys::open(&self.lib.mods_dir().to_string_lossy());
        }
        if self.ui.button(Rect::new(r.x + 24.0 + bw * 2.0, btn_y, bw, 34.0), "Delete", 0, !busy && !m.installed && m.size > 0) {
            let key = m.key.clone();
            self.run_job("Deleting", move |l| vec![l.delete_source(&key)], None);
        }
        self.drawn.push("detail");
    }

    fn draw_coop(&mut self, area: Rect) {
        let t = *self.ui.theme();
        self.drawn.push("coop");
        let busy = self.is_busy();
        if self.coop.is_none() {
            let top_h = self.coop_top_h.clamp(220.0, (area.h - 140.0).max(240.0));
            let cw = ((area.w - 10.0) * (self.coop_top_pct / 100.0)).floor().clamp(240.0, area.w - 250.0);
            let host_r = Rect::new(area.x, area.y, cw, top_h);
            let join_r = Rect::new(area.x + cw + 10.0, area.y, area.w - cw - 10.0, top_h);
            if let Some(mx) = self.ui.vsplitter(DragId::CoopTopSplit, area.x + cw + 5.0, area.y, top_h) {
                let pct = (((mx - area.x) / (area.w - 10.0)) * 100.0).clamp(25.0, 75.0).round();
                if (pct - self.coop_top_pct).abs() >= 1.0 {
                    self.coop_top_pct = pct;
                    self.cfg.set("layout_coop_top_pct", pct as i64);
                    self.layout_dirty = true;
                }
            }
            if let Some(my) = self.ui.hsplitter(DragId::CoopTopHeight, area.x, area.y + top_h + 5.0, area.w) {
                let nh = (my - area.y).clamp(220.0, (area.h - 140.0).max(240.0)).round();
                if (nh - self.coop_top_h).abs() >= 1.0 {
                    self.coop_top_h = nh;
                    self.cfg.set("layout_coop_top_h", nh as i64);
                    self.layout_dirty = true;
                }
            }
            self.ui.panel(host_r, t.panel);
            self.ui.panel(join_r, t.panel);
            self.ui.btext("Host a session", host_r.x + 18.0, host_r.y + 34.0, 22.0, t.text);
            self.ui.wrap("Friends join you. Your mod list becomes the one everybody plays with: they download what they are missing from you in one click.", host_r.x + 18.0, host_r.y + 62.0, host_r.w - 36.0, 14.0, t.muted, 4);
            self.ui.text("Your name", host_r.x + 18.0, host_r.y + 132.0, 14.0, t.muted);
            let mut name = self.fields.remove(&Field::Name).unwrap_or_default();
            self.ui.field(Rect::new(host_r.x + 18.0, host_r.y + 140.0, host_r.w - 36.0, 32.0), Field::Name, &mut name, "Player");
            self.fields.insert(Field::Name, name);
            self.ui.text("Port (TCP)", host_r.x + 18.0, host_r.y + 188.0, 14.0, t.muted);
            let mut port = self.fields.remove(&Field::Port).unwrap_or_default();
            self.ui.field(Rect::new(host_r.x + 18.0, host_r.y + 196.0, 120.0, 32.0), Field::Port, &mut port, "9950");
            self.fields.insert(Field::Port, port);
            let lip = format!("This PC: {}", coop::local_ip());
            self.ui.text(&lip, host_r.x + 150.0, host_r.y + 217.0, 14.0, t.muted);
            let btn_y = (host_r.y + host_r.h - 48.0).max(host_r.y + 236.0);
            if self.ui.button(Rect::new(host_r.x + 18.0, btn_y, host_r.w - 36.0, 36.0), "Start hosting", 1, !busy) {
                self.cfg.set("network_port", self.port());
                self.cfg.set("player_name", self.player_name());
                self.cfg.save();
                self.start_host(self.port());
            }
            self.ui.btext("Join a friend", join_r.x + 18.0, join_r.y + 34.0, 22.0, t.text);
            self.ui.wrap("Type the address your host shows (for example 192.168.1.20 or 26.12.3.4:9950) and press Join.", join_r.x + 18.0, join_r.y + 62.0, join_r.w - 36.0, 14.0, t.muted, 4);
            self.ui.text("Host address", join_r.x + 18.0, join_r.y + 132.0, 14.0, t.muted);
            let mut ip = self.fields.remove(&Field::Ip).unwrap_or_default();
            let enter = self.ui.field(Rect::new(join_r.x + 18.0, join_r.y + 140.0, join_r.w - 36.0, 32.0), Field::Ip, &mut ip, "IP address");
            self.fields.insert(Field::Ip, ip.clone());
            if (self.ui.button(Rect::new(join_r.x + 18.0, btn_y, join_r.w - 36.0, 36.0), "Join", 1, !busy && !ip.trim().is_empty()) || enter) && !ip.trim().is_empty() {
                self.cfg.set("network_ip", ip.trim());
                self.cfg.set("player_name", self.player_name());
                self.cfg.save();
                self.ui.focus = None;
                self.join(ip.trim());
            }
            let tips = Rect::new(area.x, area.y + top_h + 10.0, area.w, area.h - top_h - 10.0);
            self.ui.panel(tips, t.panel);
            self.ui.btext("Built-in RE4R Co-op Mod (Chubby Enhanced) & Smooth Checklist", tips.x + 18.0, tips.y + 30.0, 18.0, t.text);
            let sync_lbl = format!("Sync Theme ({})", t.name);
            let sbw = self.ui.width(&sync_lbl, 14.0) + 24.0;
            if self.ui.button(Rect::new(tips.x + tips.w - sbw - 16.0, tips.y + 10.0, sbw, 28.0), &sync_lbl, 0, !busy) {
                self.sync_coop_mod_theme();
                self.say(Ok(format!("Synced '{}' theme & fixes to RE4R Co-op mod.", t.name)));
            }
            let lines = [
                "Injected Multiplayer RE4R v4.2-ChubbyFix: bundled in Mods with single OS mouse cursor fix (no double cursor on game start) and live theme sync.",
                "Known Issues Fixed: (1) 15Hz gimmick scan cap + SDK/disk cache eliminates area scan lag; (2) 60Hz enemy pose lerp/nlerp fixes choppy enemy movement; (3) Pause Menu & Attache Case inventory no longer freeze Co-op.",
                "Same mods, same files: friends who join your session here can download the enhanced Co-op mod and your active mod list in one click.",
                "While the game runs this window sleeps, lowers its own priority, raises the game's, and pauses mod downloads so nothing competes with the game.",
            ];
            let mut y = tips.y + 58.0;
            for l in lines {
                draw_circle(tips.x + 24.0, y - 5.0, 3.5, t.accent_hi);
                y = self.ui.wrap(l, tips.x + 36.0, y, tips.w - 60.0, 14.0, t.text, 3) + 6.0;
            }
            return;
        }
        let hosting = self.hosting();
        let left_w = ((area.w - 10.0) * (self.coop_session_pct / 100.0)).floor().clamp(300.0, area.w - 240.0);
        let left = Rect::new(area.x, area.y, left_w, area.h);
        let right = Rect::new(left.x + left.w + 10.0, area.y, area.w - left.w - 10.0, area.h);
        if let Some(mx) = self.ui.vsplitter(DragId::CoopSessionSplit, left.x + left.w + 5.0, area.y, area.h) {
            let pct = (((mx - area.x) / (area.w - 10.0)) * 100.0).clamp(30.0, 78.0).round();
            if (pct - self.coop_session_pct).abs() >= 1.0 {
                self.coop_session_pct = pct;
                self.cfg.set("layout_coop_session_pct", pct as i64);
                self.layout_dirty = true;
            }
        }
        self.ui.panel(left, t.panel);
        self.ui.panel(right, t.panel);
        let title = if hosting {
            format!("Hosting at {}:{}", coop::local_ip(), self.coop.as_ref().map(|s| s.port).unwrap_or(0))
        } else {
            format!("{}'s session", if self.host_name.is_empty() { "Host" } else { &self.host_name })
        };
        self.ui.btext(&title, left.x + 16.0, left.y + 32.0, 20.0, t.text);
        if self.ui.button(Rect::new(left.x + left.w - 116.0, left.y + 12.0, 100.0, 30.0), "Leave", 0, true) {
            self.leave();
            return;
        }
        let cols = [0.0, 220.0, 320.0, 420.0, 500.0];
        let mut y = left.y + 62.0;
        for (i, h) in ["Player", "Ping", "Mods", "Ready", "Playing"].iter().enumerate() {
            self.ui.text(h, left.x + 16.0 + cols[i], y, 13.0, t.muted);
        }
        y += 8.0;
        draw_line(left.x + 12.0, y, left.x + left.w - 12.0, y, 1.0, t.line);
        let mine = self.my_set();
        let mut rows: Vec<(String, Option<u32>, Option<bool>, bool, bool)> = Vec::new();
        if hosting {
            rows.push((format!("{} (you, host)", self.player_name()), None, Some(true), true, self.game_running()));
            for p in &self.players {
                let m = p.set.as_ref().map(|s| coop::diff(&mine, s).matches());
                rows.push((p.name.clone(), p.ping, m, p.ready, p.in_game));
            }
        } else {
            let my_match = self.host_diff().map(|d| d.matches());
            if self.players.is_empty() {
                let host_ping = None;
                rows.push((format!("{} (host)", self.host_name), host_ping, Some(true), true, false));
            }
            let me = self.player_name();
            for (i, p) in self.players.iter().enumerate() {
                let is_me = i > 0 && p.name == me;
                let name = if i == 0 { format!("{} (host)", p.name) } else if is_me { format!("{} (you)", p.name) } else { p.name.clone() };
                let m = if is_me { my_match } else { Some(true) };
                rows.push((name, p.ping, m, if is_me { self.my_ready } else { p.ready }, p.in_game));
            }
        }
        for (name, ping, matches, ready, playing) in rows {
            y += 32.0;
            let n = self.ui.fit(&name, 16.0, 200.0);
            self.ui.text(&n, left.x + 16.0, y, 16.0, t.text);
            match ping {
                Some(ms) => {
                    let (q, c) = coop::quality(ms);
                    let col = Color::new(c[0], c[1], c[2], 1.0);
                    self.ui.text(&format!("{} ms", ms), left.x + 16.0 + cols[1], y, 15.0, col);
                    self.ui.text(q, left.x + 16.0 + cols[1] + 52.0, y, 11.0, col);
                }
                None => self.ui.text("-", left.x + 16.0 + cols[1], y, 15.0, t.muted),
            }
            match matches {
                Some(true) => self.ui.text("Match", left.x + 16.0 + cols[2], y, 15.0, t.good),
                Some(false) => self.ui.text("Differ", left.x + 16.0 + cols[2], y, 15.0, t.bad),
                None => self.ui.text("...", left.x + 16.0 + cols[2], y, 15.0, t.muted),
            }
            self.ui.text(if ready { "Yes" } else { "No" }, left.x + 16.0 + cols[3], y, 15.0, if ready { t.good } else { t.muted });
            self.ui.text(if playing { "In game" } else { "-" }, left.x + 16.0 + cols[4], y, 15.0, if playing { t.warn } else { t.muted });
        }
        let ay = left.y + left.h - 56.0;
        if hosting {
            if self.ui.button(Rect::new(left.x + 16.0, ay, 240.0, 40.0), "Start game for everyone", 1, !self.game_running()) {
                if let Some(s) = &self.coop {
                    s.broadcast(&Msg::Launch);
                }
                self.launch_game();
            }
            if self.ui.button(Rect::new(left.x + 266.0, ay, 210.0, 40.0), "Send my mod list again", 0, true) {
                self.broadcast_my_set(true);
            }
        } else {
            let d = self.host_diff();
            let ok = d.as_ref().map(|d| d.matches()).unwrap_or(false);
            let can = !ok && !busy && self.sync_waiting.is_empty() && d.is_some();
            if self.ui.button(Rect::new(left.x + 16.0, ay, 240.0, 40.0), if ok { "Mods match the host" } else { "Sync my mods with host" }, 1, can) {
                self.sync_with_host();
            }
            let rl = if self.my_ready { "Ready (click to undo)" } else { "Mark me ready" };
            if self.ui.button(Rect::new(left.x + 266.0, ay, 210.0, 40.0), rl, 0, true) {
                self.my_ready = !self.my_ready;
                if let Some(s) = &self.coop {
                    s.send(0, &Msg::Ready(self.my_ready));
                }
            }
            if let Some(d) = d {
                let yy = ay - 70.0;
                self.ui.btext("Your mods compared with the host's", left.x + 16.0, yy, 15.0, t.text);
                let items = [
                    (format!("{} mod file(s) to download", d.missing.len()), d.missing.is_empty()),
                    (format!("{} mod(s) to install", d.to_install.len()), d.to_install.is_empty()),
                    (format!("{} mod(s) to remove", d.to_remove.len()), d.to_remove.is_empty()),
                    (if d.order_differs { "Install order differs".to_string() } else { "Same install order".to_string() }, !d.order_differs),
                ];
                for (i, (s, good)) in items.iter().enumerate() {
                    let x = left.x + 16.0 + (i % 2) as f32 * 260.0;
                    let y2 = yy + 24.0 + (i / 2) as f32 * 22.0;
                    draw_circle(x + 5.0, y2 - 5.0, 4.0, if *good { t.good } else { t.warn });
                    self.ui.text(s, x + 16.0, y2, 14.0, t.text);
                }
            }
        }
        let mut ty = left.y + left.h - 150.0;
        let list: Vec<(String, (u64, u64))> = self.transfers.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (name, (got, total)) in list.iter().take(3) {
            let frac = if *total > 0 { *got as f32 / *total as f32 } else { 0.0 };
            let bw = left.w - 32.0;
            let bar = Rect::new(left.x + 16.0, ty, bw, 18.0);
            fill_rrect(bar, 5.0, t.field_bg);
            fill_rrect(Rect::new(bar.x, bar.y, bw * frac, 18.0), 5.0, Color::new(t.accent.r, t.accent.g, t.accent.b, 0.65));
            stroke_rrect(bar, 5.0, 1.0, t.line);
            let s = format!("{}  {} / {}", self.ui.fit(name, 13.0, bw - 160.0), fsx::human_size(*got), fsx::human_size(*total));
            self.ui.text(&s, left.x + 22.0, ty + 14.0, 13.0, t.text);
            ty -= 24.0;
        }
        self.ui.btext("Chat", right.x + 16.0, right.y + 30.0, 18.0, t.text);
        let mut cy = right.y + right.h - 60.0;
        let n = ((right.h - 110.0) / 20.0).max(1.0) as usize;
        for line in self.chat.iter().rev().take(n) {
            self.ui.text(&self.ui.fit(line, 14.0, right.w - 32.0), right.x + 16.0, cy, 14.0, t.text);
            cy -= 20.0;
        }
        let mut msg = self.fields.remove(&Field::Chat).unwrap_or_default();
        let sent = self.ui.field(Rect::new(right.x + 12.0, right.y + right.h - 44.0, right.w - 24.0, 32.0), Field::Chat, &mut msg, "Say something, press Enter");
        if sent && !msg.trim().is_empty() {
            let me = self.player_name();
            self.chat.push(format!("{}: {}", me, msg.trim()));
            if let Some(s) = &self.coop {
                s.broadcast(&Msg::Chat(me, msg.trim().to_string()));
            }
            msg.clear();
        }
        self.fields.insert(Field::Chat, msg);
    }

    fn draw_presets(&mut self, area: Rect) {
        let t = *self.ui.theme();
        self.ui.panel(area, t.panel);
        self.ui.btext("Presets", area.x + 18.0, area.y + 34.0, 22.0, t.text);
        self.ui.wrap("A preset remembers exactly which mods are installed and in what order. Apply one to switch setups in one click, for example a co-op set and a solo set.", area.x + 18.0, area.y + 62.0, area.w - 36.0, 14.0, t.muted, 3);
        let mut name = self.fields.remove(&Field::Preset).unwrap_or_default();
        let enter = self.ui.field(Rect::new(area.x + 18.0, area.y + 104.0, 300.0, 34.0), Field::Preset, &mut name, "Preset name");
        let busy = self.is_busy();
        let clicked = self.ui.button(Rect::new(area.x + 328.0, area.y + 104.0, 260.0, 34.0), "Save installed mods as preset", 1, !name.trim().is_empty());
        if (clicked || enter) && !name.trim().is_empty() {
            let r = self.lib.save_preset(name.trim());
            self.say(r);
            name.clear();
        }
        self.fields.insert(Field::Preset, name);
        let mut y = area.y + 160.0;
        let presets = self.lib.list_presets();
        if presets.is_empty() {
            self.ui.text("No presets yet.", area.x + 18.0, y + 20.0, 15.0, t.muted);
        }
        for p in presets {
            let keys = self.lib.read_preset(&p);
            let rr = Rect::new(area.x + 12.0, y, area.w - 24.0, 44.0);
            fill_rrect(rr, 6.0, if self.ui.hover(rr) { t.panel2 } else { Color::new(1.0, 1.0, 1.0, 0.02) });
            stroke_rrect(rr, 6.0, 1.0, t.line);
            self.ui.btext(&p, rr.x + 14.0, rr.y + 27.0, 17.0, t.text);
            self.ui.text(&format!("{} mod(s)", keys.len()), rr.x + 300.0, rr.y + 27.0, 14.0, t.muted);
            if self.ui.button(Rect::new(rr.x + rr.w - 210.0, rr.y + 6.0, 100.0, 32.0), "Apply", 1, !busy) {
                self.run_job(&format!("Applying preset {}", p), move |l| vec![l.apply_set(&keys)], None);
            }
            if self.ui.button(Rect::new(rr.x + rr.w - 104.0, rr.y + 6.0, 96.0, 32.0), "Delete", 0, true) {
                let _ = std::fs::remove_file(self.lib.presets_dir().join(format!("{}.txt", p)));
                self.say(Ok(format!("Deleted preset {}.", p)));
            }
            y += 48.0;
        }
        self.drawn.push("presets");
    }

    fn draw_options(&mut self, area: Rect) {
        let t = *self.ui.theme();
        let left_w = ((area.w - 10.0) * (self.options_pct / 100.0)).floor().clamp(260.0, area.w - 260.0);
        let left = Rect::new(area.x, area.y, left_w, area.h);
        let right = Rect::new(left.x + left.w + 10.0, area.y, area.w - left.w - 10.0, area.h);
        if let Some(mx) = self.ui.vsplitter(DragId::OptionsSplit, left.x + left.w + 5.0, area.y, area.h) {
            let pct = (((mx - area.x) / (area.w - 10.0)) * 100.0).clamp(30.0, 70.0).round();
            if (pct - self.options_pct).abs() >= 1.0 {
                self.options_pct = pct;
                self.cfg.set("layout_options_pct", pct as i64);
                self.layout_dirty = true;
            }
        }
        self.ui.panel(left, t.panel);
        self.ui.panel(right, t.panel);
        let busy = self.is_busy();
        self.ui.btext(&format!("{} install folder", self.game().folder), left.x + 18.0, left.y + 32.0, 18.0, t.text);
        let mut gd = self.fields.remove(&Field::GameDir).unwrap_or_default();
        let enter = self.ui.field(Rect::new(left.x + 18.0, left.y + 46.0, left.w - 36.0, 32.0), Field::GameDir, &mut gd, "Folder that contains the game's .exe");
        self.fields.insert(Field::GameDir, gd.clone());
        let exe_ok = !gd.trim().is_empty() && games::exe_path(self.game(), Path::new(gd.trim())).is_file();
        let status = if gd.trim().is_empty() { "Not set" } else if exe_ok { "Game found in this folder." } else { "The game's program is not in this folder." };
        self.ui.text(status, left.x + 18.0, left.y + 98.0, 14.0, if exe_ok { t.good } else { t.warn });
        if self.ui.button(Rect::new(left.x + 18.0, left.y + 110.0, 120.0, 32.0), "Use folder", 1, !busy) || enter {
            self.ui.focus = None;
            self.set_game_dir(&gd);
        }
        if self.ui.button(Rect::new(left.x + 146.0, left.y + 110.0, 120.0, 32.0), "Auto-detect", 0, !busy) {
            match games::detect_install(self.game(), &games::steam_common_dirs()) {
                Some(d) => self.set_game_dir(&d.to_string_lossy()),
                None => self.say(Err(format!("Could not find {} in your Steam libraries.", self.game().exe))),
            }
        }
        if self.ui.button(Rect::new(left.x + 274.0, left.y + 110.0, 120.0, 32.0), "Open", 0, exe_ok) {
            sys::open(gd.trim());
        }
        self.ui.btext("Bring mods over from Fluffy Mod Manager", left.x + 18.0, left.y + 180.0, 18.0, t.text);
        let mut ff = self.fields.remove(&Field::Fluffy).unwrap_or_default();
        self.ui.field(Rect::new(left.x + 18.0, left.y + 194.0, left.w - 36.0, 32.0), Field::Fluffy, &mut ff, "Fluffy Mod Manager folder (has Modmanager.exe)");
        self.fields.insert(Field::Fluffy, ff.clone());
        let src = Path::new(ff.trim()).join("Games").join(self.game().folder).join("Mods");
        let has_src = !ff.trim().is_empty() && src.is_dir();
        if self.ui.button(Rect::new(left.x + 18.0, left.y + 236.0, 220.0, 32.0), "Import its mod files", 0, !busy && has_src) {
            self.cfg.set("FluffyFolder", ff.trim());
            self.cfg.save();
            let list: Vec<PathBuf> = std::fs::read_dir(&src).map(|rd| rd.flatten().map(|e| e.path()).collect()).unwrap_or_default();
            self.import(list, true);
        }
        let backup = Path::new(ff.trim()).join("Games").join(self.game().folder).join("Backup");
        if !ff.trim().is_empty() && !fsx::walk_files(&backup).is_empty() {
            self.ui.wrap("Fluffy still has mods installed for this game. Uninstall them in Fluffy first so the two managers don't step on each other.", left.x + 18.0, left.y + 292.0, left.w - 36.0, 13.0, t.warn, 3);
        } else {
            self.ui.wrap("Copies the archives from Fluffy's Mods folder for this game. Your game files are not touched until you press Install.", left.x + 18.0, left.y + 292.0, left.w - 36.0, 13.0, t.muted, 3);
        }

        // Theme picker section on the left panel.
        self.ui.btext("Color theme", left.x + 18.0, left.y + 382.0, 18.0, t.text);
        let mut tx = left.x + 18.0;
        let mut ty = left.y + 396.0;
        for (idx, th) in THEMES.iter().enumerate() {
            let bw = self.ui.width(th.name, 14.0) + 32.0;
            if tx + bw > left.x + left.w - 18.0 {
                tx = left.x + 18.0;
                ty += 38.0;
            }
            let br = Rect::new(tx, ty, bw, 32.0);
            let sel = self.ui.theme_idx == idx;
            let hov = self.ui.hover(br);
            fill_rrect(br, 6.0, if sel { th.accent_soft } else if hov { t.btn_hover } else { t.field_bg });
            stroke_rrect(br, 6.0, if sel { 1.5 } else { 1.0 }, if sel { th.accent_hi } else { t.line });
            draw_circle(br.x + 13.0, br.y + 16.0, 5.0, th.accent_hi);
            self.ui.text(th.name, br.x + 24.0, br.y + 21.0, 14.0, t.text);
            if self.ui.click(br) {
                self.set_theme(idx);
            }
            tx += bw + 8.0;
        }

        self.ui.btext("Window Behaviour", right.x + 18.0, right.y + 32.0, 18.0, t.text);
        let mut y = right.y + 48.0;
        if self.ui.toggle(
            Rect::new(right.x + 10.0, y, right.w - 20.0, 34.0),
            "Minimize this window when the game starts",
            self.cfg.flag("MinimizeWhenGameStarts"),
        ) {
            self.cfg.toggle("MinimizeWhenGameStarts");
            self.cfg.save();
        }
        y += 60.0;

        self.ui.btext("Updates & Patch Notes", right.x + 18.0, y + 18.0, 18.0, t.text);
        y += 36.0;
        if self.ui.button(Rect::new(right.x + 18.0, y, 180.0, 36.0), "Check for Updates", 1, true) {
            self.check_for_updates(true);
        }
        if self.ui.button(Rect::new(right.x + 208.0, y, 180.0, 36.0), "View Patch Notes", 2, true) {
            self.show_update_modal = true;
        }
        y += 48.0;
        let st = self.ui.fit(&self.update_status, 14.0, right.w - 36.0);
        self.ui.text(&st, right.x + 18.0, y, 14.0, t.muted);
        self.drawn.push("options");
    }

    fn draw_update_modal(&mut self, w: f32, h: f32) {
        let t = *self.ui.theme();
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.72));

        let mw = 760.0f32.min(w - 40.0);
        let mh = 470.0f32.min(h - 40.0);
        let mx = (w - mw) * 0.5;
        let my = (h - mh) * 0.5;
        let card = Rect::new(mx, my, mw, mh);

        fill_rrect(card, 12.0, t.panel);
        stroke_rrect(card, 12.0, 1.8, t.accent);

        // Modal top banner
        let banner = Rect::new(mx + 2.0, my + 2.0, mw - 4.0, 74.0);
        fill_rrect(banner, 10.0, t.top_bar);
        draw_line(mx + 2.0, my + 76.0, mx + mw - 2.0, my + 76.0, 1.5, t.accent);

        if let Some(ic) = &self.ui.icon {
            draw_texture_ex(
                ic,
                mx + 22.0,
                my + 17.0,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(44.0, 44.0)),
                    ..Default::default()
                },
            );
        }

        let (headline, subline) = if let Some(upd) = &self.remote_update {
            (
                format!("NEW UPDATE AVAILABLE: v{}  (Current: v{})", upd.version, crate::app::VERSION),
                upd.title.clone(),
            )
        } else {
            (
                format!("WHAT'S NEW IN VERSION {}", crate::app::VERSION),
                "LATEST VERSION PATCH NOTES & FIXES".to_string(),
            )
        };

        self.ui.btext(&headline, mx + 78.0, my + 38.0, 21.0, t.text);
        self.ui.text(&subline, mx + 78.0, my + 60.0, 14.0, t.accent_hi);

        // Close [X] button in top-right of modal
        if self.ui.button(Rect::new(mx + mw - 48.0, my + 18.0, 32.0, 32.0), "X", 0, true) {
            self.dismiss_update_modal();
            return;
        }

        // Release notes / hotfixes list box
        let list_r = Rect::new(mx + 22.0, my + 92.0, mw - 44.0, mh - 166.0);
        fill_rrect(list_r, 8.0, t.field_bg);
        stroke_rrect(list_r, 8.0, 1.0, t.line_hi);

        let items: Vec<(String, String)> = if let Some(upd) = &self.remote_update {
            upd.notes.clone()
        } else {
            crate::app::CURRENT_RELEASE_NOTES
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect()
        };

        let mut ry = list_r.y + 16.0;
        for (tag, desc) in items {
            if ry + 36.0 > list_r.y + list_r.h {
                break;
            }
            let row_r = Rect::new(list_r.x + 12.0, ry, list_r.w - 24.0, 38.0);
            fill_rrect(row_r, 6.0, t.panel2);
            let tag_col = match tag.as_str() {
                "HOTFIX" => t.warn,
                "FIX" => t.good,
                _ => t.accent_hi,
            };
            let pill = Rect::new(row_r.x + 10.0, row_r.y + 8.0, 66.0, 22.0);
            fill_rrect(pill, 5.0, Color::new(tag_col.r, tag_col.g, tag_col.b, 0.20));
            stroke_rrect(pill, 5.0, 1.0, tag_col);
            let tw = self.ui.width(&tag, 12.0);
            self.ui.btext(&tag, pill.x + (pill.w - tw) * 0.5, pill.y + 15.5, 12.0, tag_col);

            let fitted = self.ui.fit(&desc, 14.5, row_r.w - 94.0);
            self.ui.text(&fitted, row_r.x + 86.0, row_r.y + 24.0, 14.5, t.text);
            ry += 44.0;
        }

        // Bottom status + action buttons
        let st = self.ui.fit(&self.update_status, 14.0, mw - 420.0);
        self.ui.text(&st, mx + 24.0, my + mh - 28.0, 14.0, t.muted);

        if self.remote_update.is_some() {
            if self.ui.button(Rect::new(mx + mw - 386.0, my + mh - 54.0, 164.0, 38.0), "Remind Me Later", 0, true) {
                self.show_update_modal = false;
            }
            if self.ui.button(Rect::new(mx + mw - 212.0, my + mh - 54.0, 190.0, 38.0), "Install Update Now", 1, true) {
                self.apply_remote_update();
            }
        } else {
            if self.ui.button(Rect::new(mx + mw - 366.0, my + mh - 54.0, 168.0, 38.0), "Check for Updates", 2, true) {
                self.check_for_updates(true);
            }
            if self.ui.button(Rect::new(mx + mw - 188.0, my + mh - 54.0, 166.0, 38.0), "Got It - Close", 1, true) {
                self.dismiss_update_modal();
            }
        }
        // Consume any click inside the modal backdrop so underlying widgets don't fire
        self.ui.consumed = true;
        self.drawn.push("update-modal");
    }

    fn draw_log(&mut self, area: Rect) {
        let t = *self.ui.theme();
        self.ui.panel(area, t.panel);
        let mut y = area.y + area.h - 16.0;
        let lines: Vec<(String, Color)> = self.log.iter().rev().cloned().collect();
        for (s, c) in lines {
            if y < area.y + 20.0 {
                break;
            }
            let col = if c == BAD { t.bad } else { t.text };
            let s = self.ui.fit(&s, 14.0, area.w - 30.0);
            self.ui.text(&s, area.x + 14.0, y, 14.0, col);
            y -= 20.0;
        }
        self.drawn.push("log");
    }
}
