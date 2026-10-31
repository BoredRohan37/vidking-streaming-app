//! Touch-first UI: gestures, native text fields, phone navigation and phone-only screens.

use crate::app::*;
use crate::pages::*;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui};

/// Height of the phone bottom navigation bar.
pub const BAR_H: f32 = 60.0;

impl App {
    pub fn bar_visible(&self) -> bool {
        // Phones always; touch tablets too, where the top bar has no room for every tab.
        (self.compact() || (self.touch && self.win_w < 1100.0)) && !matches!(self.page, Page::Profiles | Page::ProfileEdit)
    }

    /// Space at the bottom of the window covered by the navigation bar.
    pub fn bottom_inset(&self) -> f32 {
        if self.bar_visible() { BAR_H } else { 0.0 }
    }

    /// Extra padding at the end of a page so content clears the bottom bar.
    pub fn bottom_pad(&self) -> f32 {
        if self.bar_visible() { BAR_H + 16.0 } else { 0.0 }
    }

    /// Horizontal scrolling for a strip: mouse wheel, touch drag with momentum, and vertical
    /// drags are forwarded to the page so scrolling the page never gets stuck on a row.
    /// Returns the current offset in points.
    pub fn hscroll(&mut self, ui: &Ui, key: &str, strip: Rect, content_w: f32) -> f32 {
        let max_off = (content_w - strip.width()).max(0.0);
        let rid = Id::unique(("hscroll", key));
        let resp = ui.interact(strip, rid, Sense::drag());
        let mut target = self.row_targets.get(key).copied().unwrap_or(0.0);

        if resp.hovered() {
            let dx = ui.input(|i| i.smooth_scroll_delta.x);
            if dx.abs() > 0.1 {
                target -= dx;
            }
        }
        if resp.drag_started() {
            self.gesture_axis = None;
        }
        let mut dragging_h = false;
        if resp.dragged() {
            if self.gesture_axis.is_none() {
                let (o, c) = ui.input(|i| (i.pointer.press_origin(), i.pointer.interact_pos()));
                if let (Some(o), Some(c)) = (o, c) {
                    let t = c - o;
                    if t.length() > 8.0 {
                        self.gesture_axis = Some(t.x.abs() >= t.y.abs());
                    }
                }
            }
            let d = resp.drag_delta();
            match self.gesture_axis {
                Some(true) => {
                    target -= d.x;
                    dragging_h = true;
                }
                Some(false) => ui.scroll_with_delta(vec2(0.0, d.y)),
                None => {}
            }
        }
        if resp.drag_stopped() && self.gesture_axis == Some(true) {
            let v = ui.input(|i| i.pointer.velocity().x);
            target -= v * 0.25;
        }
        target = target.clamp(0.0, max_off);
        self.row_targets.insert(key.to_string(), target);
        let dur = if dragging_h { 0.0 } else { 0.3 };
        ui.ctx().animate_value_with_time(rid, target, dur)
    }

    /// A text field. On touch devices it opens the native text dialog; on desktop it is a
    /// regular egui text edit. `lead` reserves space on the left (for an icon).
    pub fn field(&mut self, ui: &mut Ui, rect: Rect, key: &str, hint: &str, password: bool, lead: f32) -> Response {
        if self.touch {
            let p = ui.painter().clone();
            let id = Id::unique(("tfield", key));
            let resp = ui.interact(rect, id, Sense::click());
            let down = resp.is_pointer_button_down_on();
            p.rect_filled(rect, cr(rect.height() * 0.5), if down { Color32::from_rgb(28, 28, 40) } else { alpha(Color32::WHITE, 0.07) });
            p.rect_stroke(rect, cr(rect.height() * 0.5), Stroke::new(1.0, alpha(Color32::WHITE, 0.14)), StrokeKind::Inside);
            let value = self.field_value(key);
            let x = rect.min.x + 18.0 + lead;
            let width = rect.width() - 36.0 - lead;
            if value.is_empty() {
                let g = one_line(&p, hint, 14.5, MUTED, width);
                p.galley(pos2(x, rect.center().y - g.size().y * 0.5), g, MUTED);
            } else {
                let shown = if password { "•".repeat(value.chars().count().min(24)) } else { value.clone() };
                let g = one_line(&p, &shown, 14.5, TEXT, width);
                p.galley(pos2(x, rect.center().y - g.size().y * 0.5), g, TEXT);
            }
            if resp.clicked() {
                crate::platform::request_text(key, hint, &value, password);
            }
            resp
        } else {
            let mut s = self.field_value(key);
            let r = crate::pages2::input(ui, rect, key, &mut s, hint, password);
            self.set_field(key, s);
            r
        }
    }

    // ------------------------------------------------------------------ phone navigation

    pub fn draw_nav_compact(&mut self, ui: &mut Ui, full: Rect) {
        let w = full.width();
        let px = pad(w);
        let h = nav_h(w);
        let a = (self.scroll_y / 60.0).clamp(0.0, 1.0);
        let p = ui.painter().clone();
        let bar = Rect::from_min_size(full.min, vec2(w, h));
        v_gradient(&p, Rect::from_min_size(full.min, vec2(w, h + 18.0)), alpha(BG, 0.85 - 0.85 * a), alpha(BG, 0.0));
        p.rect_filled(bar, 0.0, alpha(BG, 0.94 * a));
        let cy = bar.center().y;

        let logo = Rect::from_min_size(pos2(full.min.x + px, cy - 15.0), vec2(30.0, 30.0));
        let lresp = ui.interact(logo.expand2(vec2(96.0, 8.0)), Id::unique("logo_compact"), Sense::click());
        if let Some((lt, ls)) = self.logo() {
            paint_tex(ui, logo, lt, ls, 0.0, Color32::WHITE, 0.5);
        }
        bold(&p, pos2(logo.max.x + 9.0, cy), Align2::LEFT_CENTER, "MovieBox", 19.0, TEXT);
        if lresp.clicked() {
            self.acts.push(Act::Go(Page::Home));
        }

        let mut rx = full.max.x - px;
        let prof = self.store.profile().clone();
        let av_c = pos2(rx - 17.0, cy);
        let av = ui.interact(Rect::from_center_size(av_c, vec2(44.0, 44.0)), Id::unique("avatar_compact"), Sense::click());
        avatar(&p, av_c, 16.0, &prof.name, prof.hue);
        if self.menu == Menu::Profile {
            p.circle_stroke(av_c, 18.5, Stroke::new(2.0, TEXT));
        }
        if av.clicked() {
            self.acts.push(Act::SetMenu(Menu::Profile));
        }
        self.avatar_rect = Rect::from_center_size(av_c, vec2(44.0, 44.0));
        rx -= 34.0 + 12.0;

        let ai_c = pos2(rx - 18.0, cy);
        if icon_button(ui, ai_c, 18.0, "c_ai", sparkle, self.page == Page::Ai).clicked() {
            self.acts.push(Act::Go(Page::Ai));
        }
        rx -= 36.0 + 10.0;

        let s_c = pos2(rx - 18.0, cy);
        if icon_button(ui, s_c, 18.0, "c_search", search_icon, self.page == Page::Search).clicked() {
            self.acts.push(Act::Go(Page::Search));
            let q = self.search_text.clone();
            crate::platform::request_text("search", "Search movies & series", &q, false);
        }
        rx -= 36.0 + 10.0;

        let running = self.dls.iter().filter(|d| d.state == crate::data::DlState::Running).count();
        if running > 0 || self.show_dls || !self.dls.is_empty() {
            let d_c = pos2(rx - 18.0, cy);
            if icon_button(ui, d_c, 18.0, "c_dl", download_icon, self.show_dls).clicked() {
                self.acts.push(Act::ToggleDownloads);
            }
            if running > 0 {
                p.circle_filled(d_c + vec2(13.0, -13.0), 8.0, ACCENT);
                text(&p, d_c + vec2(13.0, -13.0), Align2::CENTER_CENTER, &running.to_string(), 10.5, Color32::WHITE);
            }
        }
    }

    pub fn draw_bottom_bar(&mut self, ui: &mut Ui, full: Rect) {
        let p = ui.painter().clone();
        let bar = Rect::from_min_max(pos2(full.min.x, full.max.y - BAR_H), full.max);
        v_gradient(&p, Rect::from_min_max(pos2(bar.min.x, bar.min.y - 22.0), pos2(bar.max.x, bar.min.y)), alpha(Color32::BLACK, 0.0), alpha(Color32::BLACK, 0.35));
        p.rect_filled(bar, 0.0, Color32::from_rgb(14, 14, 22));
        p.line_segment([bar.left_top(), bar.right_top()], Stroke::new(1.0, alpha(BORDER, 0.8)));
        let family = self.store.profile().family;
        let mut items: Vec<(&str, Page, IconFn)> = vec![("Home", Page::Home, home_icon), ("Movies", Page::Movies, film_icon), ("Series", Page::Series, series_icon)];
        if !family {
            items.push(("Live TV", Page::Tv, live_icon));
        }
        items.push(("My List", Page::MyList, bookmark_icon));
        if !self.compact() {
            items.push(("Ask AI", Page::Ai, sparkle));
        }
        let iw = full.width() / items.len() as f32;
        for (i, (label, page, icon)) in items.iter().enumerate() {
            let r = Rect::from_min_size(pos2(full.min.x + i as f32 * iw, bar.min.y), vec2(iw, BAR_H));
            let resp = ui.interact(r, Id::unique(("bb", *label)), Sense::click());
            let active = self.page == *page;
            let t = ui.ctx().animate_bool_with_time(Id::unique(("bbt", *label)), active, 0.15);
            icon(&p, pos2(r.center().x, r.min.y + 23.0), 23.0, mix(MUTED, ACCENT, t));
            text(&p, pos2(r.center().x, r.min.y + 45.0), Align2::CENTER_CENTER, label, 10.5, mix(MUTED, TEXT, t));
            if resp.clicked() {
                self.acts.push(Act::Go(*page));
            }
        }
    }

    /// Bottom sheet with a dimmed backdrop. Returns the sheet rect.
    pub fn sheet(&self, ui: &Ui, full: Rect, height: f32) -> Rect {
        let p = ui.painter();
        p.rect_filled(full, 0.0, alpha(Color32::BLACK, 0.55));
        let r = Rect::from_min_size(pos2(full.min.x + 10.0, full.max.y - self.bottom_inset() - height - 10.0), vec2(full.width() - 20.0, height));
        shadow(p, r, 22.0, 20.0, 1.0);
        p.rect_filled(r, cr(22.0), Color32::from_rgb(20, 20, 30));
        p.rect_stroke(r, cr(22.0), Stroke::new(1.0, BORDER), StrokeKind::Inside);
        p.rect_filled(Rect::from_center_size(pos2(r.center().x, r.min.y + 9.0), vec2(36.0, 4.0)), cr(2.0), alpha(Color32::WHITE, 0.25));
        r
    }

    // ------------------------------------------------------------------ phone hero

    pub fn hero_compact(&mut self, ui: &mut Ui, w: f32) {
        let h = (w * 1.26).clamp(420.0, 620.0);
        let rect = blk(ui, w, h);
        let p = ui.painter().clone();
        let px = pad(w);
        let hero = self.hero.clone();
        let idx = if hero.is_empty() { 0 } else { ((self.t / 9.0) as usize) % hero.len() };
        let card = hero.get(idx).cloned();

        match &card {
            Some(c) => {
                let (tex, size, _) = self.poster_hq(c);
                let fade = ui.ctx().animate_bool_with_time(Id::unique(("hero_fade_c", idx)), true, 0.6);
                paint_tex(ui, rect, tex, size, 0.0, Color32::from_white_alpha((fade * 255.0) as u8), 0.12);
            }
            None => {
                let (tex, size, _) = self.backdrop(None, "moviebox");
                paint_tex(ui, rect, tex, size, 0.0, Color32::WHITE, 0.5);
            }
        }
        v_gradient(&p, Rect::from_min_max(pos2(rect.min.x, rect.max.y - h * 0.62), rect.max), alpha(BG, 0.0), BG);
        v_gradient(&p, Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + 110.0)), alpha(BG, 0.65), alpha(BG, 0.0));

        let cx = rect.center().x;
        match &card {
            Some(c) => {
                let mut y = rect.max.y - 24.0;
                // dots
                if hero.len() > 1 {
                    let total = hero.len() as f32 * 14.0;
                    let mut dx = cx - total * 0.5;
                    for i in 0..hero.len() {
                        let act = i == idx;
                        let r = Rect::from_min_size(pos2(dx, y - 4.0), vec2(if act { 16.0 } else { 6.0 }, 6.0));
                        p.rect_filled(r, cr(3.0), if act { TEXT } else { alpha(Color32::WHITE, 0.3) });
                        dx += r.width() + 8.0;
                    }
                }
                y -= 24.0;
                // buttons
                let bh = 48.0;
                let side = bh;
                let play_w = w - px * 2.0 - side * 2.0 - 20.0;
                let play_rect = Rect::from_min_size(pos2(rect.min.x + px, y - bh), vec2(play_w, bh));
                if button_in(ui, play_rect, Btn::new("hc_play", "Play", Kind::Primary).icon(play_icon).height(bh)).clicked() {
                    self.acts.push(Act::QuickPlay(c.clone()));
                }
                let in_list = self.store.in_list(c);
                let lc = pos2(play_rect.max.x + 10.0 + side * 0.5, y - bh * 0.5);
                if icon_button(ui, lc, side * 0.5, "hc_list", if in_list { check_icon } else { plus_icon }, in_list).clicked() {
                    self.acts.push(Act::ToggleList(c.clone()));
                }
                let ic = pos2(lc.x + 10.0 + side, y - bh * 0.5);
                if icon_button(ui, ic, side * 0.5, "hc_info", info_icon, false).clicked() {
                    self.acts.push(Act::OpenDetails(c.clone()));
                }
                y -= bh + 16.0;
                // meta
                let kind = if c.is_series() { "Series" } else { "Movie" };
                let mut meta = String::new();
                if !c.year.is_empty() {
                    meta.push_str(&format!("{}  ·  ", c.year));
                }
                meta.push_str(kind);
                let rating = c.rating.filter(|r| *r > 0.0).map(|r| format!("{r:.1}"));
                let rating_w = rating.as_ref().map_or(0.0, |r| 22.0 + text_w(&p, r, 13.5) + 12.0);
                let mut mx = cx - (rating_w + text_w(&p, &meta, 13.5)) * 0.5;
                if let Some(r) = &rating {
                    star_icon(&p, pos2(mx + 7.0, y - 8.0), 7.5, GOLD);
                    text(&p, pos2(mx + 19.0, y - 8.0), Align2::LEFT_CENTER, r, 13.5, TEXT);
                    mx += rating_w;
                }
                text(&p, pos2(mx, y - 8.0), Align2::LEFT_CENTER, &meta, 13.5, DIM);
                y -= 28.0;
                let g = wrapped_b(&p, &c.title, 30.0, TEXT, w - px * 2.0, 2);
                p.galley(pos2(cx - g.size().x * 0.5, y - g.size().y), g.clone(), TEXT);
                y -= g.size().y + 10.0;
                tag(&p, pos2(cx - 34.0, y - 22.0), "FEATURED", 10.0, Color32::WHITE, alpha(ACCENT, 0.9));
            }
            None => {
                if self.home.is_loading() {
                    skeleton(&p, Rect::from_center_size(pos2(cx, rect.max.y - 160.0), vec2(w * 0.6, 34.0)), 8.0, self.t);
                    skeleton(&p, Rect::from_center_size(pos2(cx, rect.max.y - 100.0), vec2(w * 0.4, 16.0)), 6.0, self.t);
                    skeleton(&p, Rect::from_center_size(pos2(cx, rect.max.y - 50.0), vec2(w - px * 2.0, 48.0)), 24.0, self.t);
                } else {
                    let msg = match &self.home {
                        Load::Failed(e) => format!("Couldn’t reach the catalogue ({e})."),
                        _ => "Search to get started.".to_string(),
                    };
                    bold(&p, pos2(cx, rect.max.y - 170.0), Align2::CENTER_CENTER, "Your next favourite is waiting", 22.0, TEXT);
                    let g = wrapped(&p, &msg, 14.0, DIM, w - px * 2.0, 3);
                    p.galley(pos2(cx - g.size().x * 0.5, rect.max.y - 140.0), g, DIM);
                    let retry = Rect::from_center_size(pos2(cx, rect.max.y - 60.0), vec2(160.0, 46.0));
                    if button_in(ui, retry, Btn::new("hc_retry", "Try again", Kind::Primary).height(46.0)).clicked() {
                        self.acts.push(Act::RetryHome);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ phone search page

    pub fn page_search_compact(&mut self, ui: &mut Ui, w: f32) {
        let px = pad(w);
        let top = blk(ui, w, nav_h(w) + 10.0);
        let _ = top;
        let r = blk(ui, w, 64.0);
        let field_rect = Rect::from_min_size(pos2(r.min.x + px, r.min.y + 6.0), vec2(w - px * 2.0, 50.0));
        self.field(ui, field_rect, "search", "Search movies & series", false, 28.0);
        search_icon(ui.painter(), pos2(field_rect.min.x + 26.0, field_rect.center().y), 18.0, DIM);
        if !self.search_text.is_empty() {
            let clear = ui.interact(Rect::from_center_size(pos2(field_rect.max.x - 24.0, field_rect.center().y), vec2(40.0, 40.0)), Id::unique("search_clear"), Sense::click());
            let c = clear.rect.center();
            ui.painter().circle_filled(c, 9.0, alpha(Color32::WHITE, 0.18));
            let s = Stroke::new(1.6, TEXT);
            ui.painter().line_segment([c + vec2(-3.0, -3.0), c + vec2(3.0, 3.0)], s);
            ui.painter().line_segment([c + vec2(-3.0, 3.0), c + vec2(3.0, -3.0)], s);
            if clear.clicked() {
                self.search_text.clear();
                self.last_query.clear();
                self.search = Load::Idle;
            }
        }

        // source selector
        let r = blk(ui, w, 46.0);
        let label = format!("Source: {}", self.provider.label());
        let chip_r = chip(ui, pos2(r.min.x + px, r.min.y + 4.0), "src_chip", &label, self.menu == Menu::Provider);
        self.provider_rect = chip_r.rect;
        if chip_r.clicked() {
            self.acts.push(Act::SetMenu(Menu::Provider));
        }

        if self.last_query.is_empty() && !self.search.is_loading() && !matches!(self.search, Load::Ready(_) | Load::Failed(_)) {
            // recent searches
            let recents = self.store.profile().recent_searches.clone();
            if !recents.is_empty() {
                let r = blk(ui, w, 44.0);
                bold(ui.painter(), pos2(r.min.x + px, r.center().y + 6.0), Align2::LEFT_CENTER, "Recent searches", 17.0, TEXT);
                for (i, q) in recents.iter().take(6).enumerate() {
                    let r = blk(ui, w, 48.0);
                    let row = Rect::from_min_max(pos2(r.min.x + px, r.min.y), pos2(r.max.x - px, r.max.y));
                    let resp = ui.interact(row, Id::unique(("recent_c", i)), Sense::click());
                    let p = ui.painter();
                    if resp.is_pointer_button_down_on() {
                        p.rect_filled(row, cr(10.0), alpha(Color32::WHITE, 0.06));
                    }
                    clock_icon(p, pos2(row.min.x + 14.0, row.center().y), 17.0, MUTED);
                    let g = one_line(p, q, 15.0, TEXT, row.width() - 60.0);
                    p.galley(pos2(row.min.x + 38.0, row.center().y - g.size().y * 0.5), g, TEXT);
                    if resp.clicked() {
                        self.acts.push(Act::Search(q.clone()));
                    }
                }
            }
            // genres
            let r = blk(ui, w, 52.0);
            bold(ui.painter(), pos2(r.min.x + px, r.max.y - 18.0), Align2::LEFT_CENTER, "Browse by genre", 17.0, TEXT);
            let genres = ["Action", "Comedy", "Drama", "Thriller", "Sci-Fi", "Horror", "Romance", "Animation", "Crime", "Family", "Mystery", "Fantasy"];
            let mut x = px;
            let mut row_top = 0.0;
            let r = blk(ui, w, 3.0 * 46.0 + 8.0);
            for g in genres {
                let tw = text_w(ui.painter(), g, 13.5) + 28.0;
                if x + tw > w - px {
                    x = px;
                    row_top += 46.0;
                }
                if row_top > 2.0 * 46.0 {
                    break;
                }
                let c = chip(ui, pos2(r.min.x + x, r.min.y + row_top + 4.0), g, g, false);
                if c.clicked() {
                    self.acts.push(Act::Search(g.to_string()));
                }
                x += tw + 10.0;
            }
            return;
        }

        match &self.search {
            Load::Loading => {
                let (cw, ch) = (112.0f32, 168.0f32);
                let r = blk(ui, w, (ch + 60.0) * 2.0);
                let p = ui.painter().clone();
                let mut i = 0;
                for row in 0..2 {
                    let mut x = r.min.x + px;
                    for _ in 0..3 {
                        let y = r.min.y + row as f32 * (ch + 60.0) + 8.0;
                        skeleton(&p, Rect::from_min_size(pos2(x, y), vec2(cw.min((w - px * 2.0 - 20.0) / 3.0), ch)), 12.0, self.t + i as f64 * 0.05);
                        x += (w - px * 2.0 - 20.0) / 3.0 + 10.0;
                        i += 1;
                    }
                }
            }
            Load::Failed(e) => {
                let msg = format!("{e}. Try another source.");
                let q = self.last_query.clone();
                self.empty_state(ui, w, "Search didn’t work", &msg, Some(("Try again", Act::Search(q))));
            }
            Load::Ready(cards) if cards.is_empty() => {
                let msg = format!("Nothing on {} matched “{}”.", self.provider.label(), self.last_query);
                self.empty_state(ui, w, "No results", &msg, Some(("Ask the AI concierge", Act::Go(Page::Ai))));
            }
            Load::Ready(cards) => {
                let cards = cards.clone();
                let r = blk(ui, w, 40.0);
                text(ui.painter(), pos2(r.min.x + px, r.center().y), Align2::LEFT_CENTER, &format!("Results for “{}”", self.last_query), 14.0, DIM);
                self.grid(ui, w, "search", &cards);
            }
            Load::Idle => {}
        }
    }
}

impl App {
    // ------------------------------------------------------------------ phone details

    /// Top part of the details page on phones: full-bleed poster, title, actions, summary.
    pub fn details_top_compact(&mut self, ui: &mut Ui, w: f32, det: &Details) {
        let px = pad(w);
        let p = ui.painter().clone();
        let meta = det.data.ready().cloned();
        let hero_h = w.clamp(340.0, 520.0);
        let rect = blk(ui, w, hero_h);
        let (tex, size, _) = self.poster_hq(&det.card);
        paint_tex(ui, rect, tex, size, 0.0, Color32::WHITE, 0.15);
        v_gradient(&p, Rect::from_min_max(pos2(rect.min.x, rect.max.y - hero_h * 0.6), rect.max), alpha(BG, 0.0), BG);
        v_gradient(&p, Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + 100.0)), alpha(Color32::BLACK, 0.55), alpha(Color32::BLACK, 0.0));

        let back = icon_button(ui, pos2(rect.min.x + px + 20.0, rect.min.y + 30.0), 20.0, "back_c", |p, c, s, col| chevron(p, c, s, -1, col), false);
        if back.clicked() {
            self.acts.push(Act::Back);
        }
        let share = icon_button(ui, pos2(rect.max.x - px - 20.0, rect.min.y + 30.0), 20.0, "share_c", download_icon_share, false);
        if share.clicked() {
            let t = if det.card.year.is_empty() { det.card.title.clone() } else { format!("{} ({})", det.card.title, det.card.year) };
            self.acts.push(Act::CopyText(t));
        }

        let title = meta.as_ref().map(|d| d.title.clone()).unwrap_or_else(|| det.card.title.clone());
        let g = wrapped_b(&p, &title, 30.0, TEXT, w - px * 2.0, 2);
        p.galley(pos2(rect.min.x + px, rect.max.y - 14.0 - g.size().y), g, TEXT);

        // meta row
        let r = blk(ui, w, 34.0);
        let mut mx = r.min.x + px;
        let rating = meta.as_ref().and_then(|d| d.imdb_rating.clone()).or_else(|| det.card.rating.map(|x| format!("{x:.1}")));
        if let Some(rt) = rating.filter(|s| !s.trim().is_empty()) {
            star_icon(&p, pos2(mx + 7.0, r.center().y), 7.5, GOLD);
            text(&p, pos2(mx + 19.0, r.center().y), Align2::LEFT_CENTER, &rt, 14.0, TEXT);
            mx += 26.0 + text_w(&p, &rt, 14.0) + 10.0;
        }
        let year = meta.as_ref().and_then(|d| d.year.clone()).unwrap_or_else(|| det.card.year.clone());
        let mut bits: Vec<String> = Vec::new();
        if !year.is_empty() {
            bits.push(year);
        }
        bits.push(if det.card.is_series() { "Series".to_string() } else { "Movie".to_string() });
        if let Some(d) = meta.as_ref().and_then(|d| d.duration.clone()).filter(|s| !s.trim().is_empty()) {
            bits.push(d);
        }
        if let Some(d) = &meta {
            if d.is_series() {
                bits.push(format!("{} season{}", d.seasons.len(), if d.seasons.len() == 1 { "" } else { "s" }));
            }
        }
        text(&p, pos2(mx, r.center().y), Align2::LEFT_CENTER, &bits.join("  ·  "), 14.0, DIM);

        // primary action
        let default_release = det.streams.ready().and_then(|l| pick_default(l, self.store.quality_cap)).cloned();
        let label = if det.card.is_series() && det.season > 0 { format!("Play S{} E{}", det.season, det.episode) } else { "Play".to_string() };
        let r = blk(ui, w, 66.0);
        let play_rect = Rect::from_min_size(pos2(r.min.x + px, r.min.y + 8.0), vec2(w - px * 2.0, 50.0));
        if button_in(ui, play_rect, Btn::new("dc_play", &label, Kind::Primary).icon(play_icon).height(50.0)).clicked() {
            match &default_release {
                Some(rel) => self.acts.push(Act::PlayRelease(rel.clone())),
                None => {
                    if det.streams.is_loading() {
                        self.toast("Finding the best stream…", false);
                    } else {
                        self.toast("No playable stream yet. Try another source.", true);
                    }
                }
            }
        }

        // secondary actions with captions
        let r = blk(ui, w, 82.0);
        let in_list = self.store.in_list(&det.card);
        let cols = 3.0;
        let cw = (w - px * 2.0) / cols;
        let items: [(&str, &str, IconFn, bool); 3] = [
            ("dc_list", if in_list { "In My List" } else { "My List" }, if in_list { check_icon } else { plus_icon }, in_list),
            ("dc_dl", "Download", download_icon, false),
            ("dc_ai", "Why watch?", sparkle, false),
        ];
        for (i, (key, caption, icon, active)) in items.iter().enumerate() {
            let c = pos2(r.min.x + px + cw * (i as f32 + 0.5), r.min.y + 26.0);
            let resp = icon_button(ui, c, 22.0, key, *icon, *active);
            text(&p, pos2(c.x, c.y + 36.0), Align2::CENTER_CENTER, caption, 12.0, DIM);
            if resp.clicked() {
                match i {
                    0 => self.acts.push(Act::ToggleList(det.card.clone())),
                    1 => match &default_release {
                        Some(rel) => self.acts.push(Act::DownloadRelease(rel.clone())),
                        None => self.toast("Streams are still loading.", false),
                    },
                    _ => self.acts.push(Act::Insight),
                }
            }
        }

        // summary
        let desc = meta.as_ref().and_then(|d| d.description.clone().or_else(|| d.tagline.clone())).unwrap_or_default();
        match &det.data {
            Load::Loading => {
                let r = blk(ui, w, 90.0);
                for (i, wd) in [1.0f32, 0.92, 0.6].iter().enumerate() {
                    skeleton(&p, Rect::from_min_size(pos2(r.min.x + px, r.min.y + 8.0 + i as f32 * 24.0), vec2((w - px * 2.0) * wd, 14.0)), 5.0, self.t);
                }
            }
            Load::Failed(e) => {
                let g = wrapped(&p, &format!("Could not load details: {e}"), 14.0, ACCENT, w - px * 2.0, 4);
                let r = blk(ui, w, g.size().y + 16.0);
                p.galley(pos2(r.min.x + px, r.min.y + 4.0), g, ACCENT);
            }
            _ => {
                if !desc.trim().is_empty() {
                    let g = wrapped(&p, desc.trim(), 14.5, DIM, w - px * 2.0, 9);
                    let r = blk(ui, w, g.size().y + 18.0);
                    p.galley(pos2(r.min.x + px, r.min.y + 4.0), g, DIM);
                }
            }
        }
        if let Some(d) = &meta {
            if !d.genres.is_empty() {
                let r = blk(ui, w, 40.0);
                let mut gx = r.min.x + px;
                for gname in d.genres.iter().take(4) {
                    let rr = tag(&p, pos2(gx, r.min.y + 4.0), gname, 12.0, DIM, alpha(Color32::WHITE, 0.09));
                    gx = rr.max.x + 8.0;
                }
            }
            let mut line = Vec::new();
            if let Some(s) = d.director.as_ref().filter(|s| !s.trim().is_empty()) {
                line.push(format!("Director: {s}"));
            }
            if let Some(s) = d.stars.as_ref().filter(|s| !s.trim().is_empty()) {
                line.push(format!("Starring: {s}"));
            }
            if !line.is_empty() {
                let g = wrapped(&p, &line.join("\n"), 12.5, MUTED, w - px * 2.0, 4);
                let r = blk(ui, w, g.size().y + 14.0);
                p.galley(pos2(r.min.x + px, r.min.y + 2.0), g, MUTED);
            }
        }
        blk(ui, w, 8.0);
    }
}

/// Share glyph (three joined dots).
pub fn download_icon_share(p: &egui::Painter, c: egui::Pos2, s: f32, col: Color32) {
    let st = Stroke::new((s * 0.08).max(1.4), col);
    let (a, b, d) = (pos2(c.x - s * 0.28, c.y), pos2(c.x + s * 0.26, c.y - s * 0.26), pos2(c.x + s * 0.26, c.y + s * 0.26));
    p.line_segment([a, b], st);
    p.line_segment([a, d], st);
    for q in [a, b, d] {
        p.circle_filled(q, s * 0.11, col);
    }
}
