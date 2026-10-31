//! Drawing code, part 1: frame, navigation, overlays, home, rails, cards and grids.

use crate::app::*;
use crate::data::DlState;
use crate::store::Card;
use crate::theme::*;
use crate::widgets::*;
use egui::{
    pos2, vec2, Align2, Color32, Id, Key, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, Vec2,
};
use moviebox_tui::providers::ProviderKind;

pub const NAV_H: f32 = 68.0;

/// Height of the top bar: slimmer on phones.
pub fn nav_h(w: f32) -> f32 {
    if w < COMPACT_W { 56.0 } else { NAV_H }
}

/// Windows narrower than this (in points) use the phone layout.
pub const COMPACT_W: f32 = 700.0;

pub fn pad(w: f32) -> f32 {
    if w < COMPACT_W {
        16.0
    } else {
        (w * 0.04).clamp(24.0, 60.0)
    }
}

pub fn card_dims(w: f32) -> (f32, f32) {
    if w < COMPACT_W {
        let cw = ((w - 16.0) / 2.6).clamp(104.0, 150.0).round();
        return (cw, (cw * 1.5).round());
    }
    let cw = if w >= 1500.0 {
        190.0
    } else if w >= 1250.0 {
        170.0
    } else {
        150.0
    };
    (cw, (cw * 1.5).round())
}

/// Allocate a full-width block of the given height and return its rect.
pub fn blk(ui: &mut Ui, w: f32, h: f32) -> Rect {
    ui.allocate_exact_size(vec2(w, h), Sense::hover()).0
}

pub fn skeleton(p: &egui::Painter, rect: Rect, radius: f32, t: f64) {
    let k = 0.5 + 0.5 * ((t as f32) * 2.6 + rect.min.x * 0.012).sin();
    p.rect_filled(rect, cr(radius), mix(SURFACE, SURFACE2, k));
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RailMode {
    Normal,
    Resume,
    Ai,
}

pub struct CardOpts<'a> {
    pub key: &'a str,
    pub resume: Option<String>,
    pub why: Option<&'a str>,
}

impl App {
    // ------------------------------------------------------------------ frame

    pub fn ui(&mut self, ui: &mut Ui) {
        let now = ui.input(|i| i.time);
        self.tick(now);
        self.drain();
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, BG);
        let w = full.width();
        self.form = if w < COMPACT_W { Form::Compact } else { Form::Regular };
        self.win_w = w;

        let salt = (self.page as u8, self.nav_id);
        let out = egui::ScrollArea::vertical()
            .id_salt(salt)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                let inner_w = ui.available_width();
                match self.page {
                    Page::Home => self.page_home(ui, inner_w),
                    Page::Movies => self.page_browse(ui, inner_w, true),
                    Page::Series => self.page_browse(ui, inner_w, false),
                    Page::MyList => self.page_my_list(ui, inner_w),
                    Page::Search => self.page_search(ui, inner_w),
                    Page::Details => self.page_details(ui, inner_w),
                    Page::Tv => self.page_tv(ui, inner_w),
                    Page::Ai => self.page_ai(ui, inner_w),
                    Page::Settings => self.page_settings(ui, inner_w),
                    Page::Profiles => self.page_profiles(ui, inner_w),
                    Page::ProfileEdit => self.page_profile_edit(ui, inner_w),
                }
                blk(ui, inner_w, self.bottom_pad());
            });
        self.scroll_y = out.state.clamped_offset().y;

        if !matches!(self.page, Page::Profiles | Page::ProfileEdit) {
            if self.compact() {
                if self.page != Page::Details {
                    self.draw_nav_compact(ui, full);
                }
                self.draw_bottom_bar(ui, full);
            } else {
                self.draw_nav(ui, full);
                if self.bar_visible() {
                    self.draw_bottom_bar(ui, full);
                }
            }
        }
        self.draw_overlays(ui, full);

        if ui.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::BrowserBack)) {
            if self.menu != Menu::None {
                self.menu = Menu::None;
            } else if self.show_dls {
                self.show_dls = false;
            } else if self.page != Page::Home && !matches!(self.page, Page::Profiles) {
                self.acts.push(Act::Back);
            } else if self.touch {
                crate::platform::move_task_to_back();
            }
        }
        self.apply_acts();
        if let Some(d) = self.animating() {
            ui.ctx().request_repaint_after(d);
        }
    }

    // ------------------------------------------------------------------ nav

    fn tab(&mut self, ui: &Ui, x: f32, cy: f32, label: &str, page: Page, sparkle_icon: bool) -> f32 {
        let p = ui.painter();
        let size = 14.5;
        let icon_w = if sparkle_icon { 20.0 } else { 0.0 };
        let tw = text_w(p, label, size) + icon_w;
        let rect = Rect::from_min_size(pos2(x, cy - 20.0), vec2(tw + 24.0, 40.0));
        let resp = ui.interact(rect, Id::unique(("tab", label)), Sense::click());
        pointer(ui, &resp);
        let active = self.page == page || (page == Page::Home && self.page == Page::Search && false);
        let h = ui.ctx().animate_bool_with_time(Id::unique(("tabh", label)), resp.hovered() || active, 0.15);
        let color = mix(DIM, TEXT, h);
        let mut tx = rect.min.x + 12.0;
        if sparkle_icon {
            sparkle(p, pos2(tx + 8.0, cy), 15.0, mix(ACCENT, Color32::from_rgb(255, 150, 120), h));
            tx += 20.0;
        }
        if active {
            bold(p, pos2(tx, cy), Align2::LEFT_CENTER, label, size, TEXT);
            p.rect_filled(
                Rect::from_min_size(pos2(rect.min.x + 12.0, cy + 16.0), vec2(tw, 2.5)),
                cr(2.0),
                ACCENT,
            );
        } else {
            text(p, pos2(tx, cy), Align2::LEFT_CENTER, label, size, color);
        }
        if resp.clicked() {
            self.acts.push(Act::Go(page));
        }
        rect.width()
    }

    fn draw_nav(&mut self, ui: &mut Ui, full: Rect) {
        let w = full.width();
        let px = pad(w);
        let a = (self.scroll_y / 90.0).clamp(0.0, 1.0);
        let bar = Rect::from_min_size(full.min, vec2(w, NAV_H));
        let p = ui.painter().clone();
        v_gradient(&p, Rect::from_min_size(full.min, vec2(w, NAV_H + 26.0)), alpha(BG, 0.88 - 0.88 * a), alpha(BG, 0.0));
        p.rect_filled(bar, 0.0, alpha(BG, 0.93 * a));
        if a > 0.6 {
            p.line_segment(
                [pos2(bar.min.x, bar.max.y), pos2(bar.max.x, bar.max.y)],
                Stroke::new(1.0, alpha(BORDER, (a - 0.6) * 2.5)),
            );
        }
        let cy = bar.center().y;

        // Logo
        let logo = Rect::from_min_size(pos2(full.min.x + px, cy - 17.0), vec2(34.0, 34.0));
        let lresp = ui.interact(logo.expand2(vec2(112.0, 4.0)), Id::unique("logo"), Sense::click());
        pointer(ui, &lresp);
        if let Some((lt, ls)) = self.logo() {
            paint_tex(ui, logo, lt, ls, 0.0, Color32::WHITE, 0.5);
        }
        bold(&p, pos2(logo.max.x + 10.0, cy), Align2::LEFT_CENTER, "MovieBox", 21.0, TEXT);
        if lresp.clicked() {
            self.acts.push(Act::Go(Page::Home));
        }

        // Right cluster, laid out from the right edge.
        let mut rx = full.max.x - px;
        let prof = self.store.profile().clone();
        let av_c = pos2(rx - 18.0, cy);
        let av = ui.interact(Rect::from_center_size(av_c, vec2(40.0, 40.0)), Id::unique("avatar"), Sense::click());
        pointer(ui, &av);
        avatar(&p, av_c, 18.0, &prof.name, prof.hue);
        if av.hovered() || self.menu == Menu::Profile {
            p.circle_stroke(av_c, 20.0, Stroke::new(2.0, TEXT));
        }
        if av.clicked() {
            self.acts.push(Act::SetMenu(Menu::Profile));
        }
        rx -= 36.0 + 16.0;

        let dl_c = pos2(rx - 18.0, cy);
        let running = self.dls.iter().filter(|d| d.state == DlState::Running).count();
        let dlb = icon_button(ui, dl_c, 18.0, "dl", download_icon, self.show_dls);
        if running > 0 {
            p.circle_filled(dl_c + vec2(13.0, -13.0), 8.0, ACCENT);
            text(&p, dl_c + vec2(13.0, -13.0), Align2::CENTER_CENTER, &running.to_string(), 10.5, Color32::WHITE);
        }
        if dlb.clicked() {
            self.acts.push(Act::ToggleDownloads);
        }
        rx -= 36.0 + 10.0;

        let plabel = self.provider.label();
        let pw = text_w(&p, plabel, 13.5) + 52.0;
        let prect = Rect::from_min_size(pos2(rx - pw, cy - 18.0), vec2(pw, 36.0));
        let presp = ui.interact(prect, Id::unique("provider_chip"), Sense::click());
        pointer(ui, &presp);
        let ph = ui.ctx().animate_bool_with_time(Id::unique("provh"), presp.hovered() || self.menu == Menu::Provider, 0.12);
        p.rect_filled(prect, cr(18.0), mix(alpha(Color32::WHITE, 0.08), alpha(Color32::WHITE, 0.16), ph));
        p.rect_stroke(prect, cr(18.0), Stroke::new(1.0, alpha(Color32::WHITE, 0.14)), StrokeKind::Inside);
        p.circle_filled(pos2(prect.min.x + 16.0, cy), 4.0, GREEN);
        text(&p, pos2(prect.min.x + 28.0, cy), Align2::LEFT_CENTER, plabel, 13.5, TEXT);
        chevron(&p, pos2(prect.max.x - 16.0, cy), 14.0, 2, DIM);
        if presp.clicked() {
            self.acts.push(Act::SetMenu(Menu::Provider));
        }
        self.provider_rect = prect;
        self.avatar_rect = Rect::from_center_size(av_c, vec2(40.0, 40.0));
        rx -= pw + 12.0;

        // Search pill
        let sw = (w * 0.2).clamp(150.0, 320.0);
        let srect = Rect::from_min_size(pos2(rx - sw, cy - 19.0), vec2(sw, 38.0));
        if self.touch {
            self.field(ui, srect, "search", "Search titles", false, 24.0);
            search_icon(&p, pos2(srect.min.x + 22.0, cy), 17.0, DIM);
            self.search_rect = srect;
        } else {
        let focus_h = ui.ctx().animate_bool_with_time(Id::unique("srchf"), self.search_focused, 0.15);
        p.rect_filled(srect, cr(19.0), mix(alpha(Color32::WHITE, 0.08), Color32::from_rgb(22, 22, 32), focus_h));
        p.rect_stroke(
            srect,
            cr(19.0),
            Stroke::new(1.0 + focus_h, mix(alpha(Color32::WHITE, 0.14), ACCENT, focus_h)),
            StrokeKind::Inside,
        );
        search_icon(&p, pos2(srect.min.x + 20.0, cy), 17.0, mix(DIM, TEXT, focus_h));
        let input_rect = Rect::from_min_max(pos2(srect.min.x + 38.0, srect.min.y + 4.0), pos2(srect.max.x - 14.0, srect.max.y - 4.0));
        let edit = egui::TextEdit::singleline(&mut self.search_text)
            .id(Id::unique("search_input"))
            .hint_text("Search titles")
            .font(font(14.5))
            .frame(egui::Frame::NONE)
            .desired_width(input_rect.width())
            .margin(egui::Margin { left: 0, right: 0, top: ((input_rect.height() - 20.0) * 0.5).max(0.0) as i8, bottom: 0 })
            .align(egui::Align2::LEFT_CENTER);
        let resp = ui.put(input_rect, edit);
        self.search_focused = resp.has_focus();
        if resp.changed() {
            self.sug_changed = self.t;
        }
        if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            let q = self.search_text.clone();
            self.acts.push(Act::Search(q));
        }
        if !self.search_focused && ui.input(|i| i.key_pressed(Key::Slash)) && self.menu == Menu::None {
            ui.memory_mut(|m| m.request_focus(Id::unique("search_input")));
        }
        self.search_rect = srect;
        // debounce suggestions
        let q = self.search_text.trim().to_string();
        if self.search_focused && q.len() >= 2 && q != self.sug_for && self.t - self.sug_changed > 0.22 {
            self.sug_for = q.clone();
            self.sug_token = self.next_token();
            self.be.suggest(q.clone(), self.sug_token);
        }
        if q.len() < 2 {
            self.sug_for.clear();
            if !self.suggestions.is_empty() {
                self.suggestions.clear();
            }
        }
        }
        rx -= sw + 18.0;

        // Tabs
        let mut x = logo.max.x + 10.0 + text_w(&p, "MovieBox", 21.0) + 26.0;
        let family = self.store.profile().family;
        let tabs: Vec<(&str, Page, bool)> = if family {
            vec![("Home", Page::Home, false), ("Movies", Page::Movies, false), ("Series", Page::Series, false), ("My List", Page::MyList, false), ("Ask AI", Page::Ai, true)]
        } else {
            vec![("Home", Page::Home, false), ("Movies", Page::Movies, false), ("Series", Page::Series, false), ("Live TV", Page::Tv, false), ("My List", Page::MyList, false), ("Ask AI", Page::Ai, true)]
        };
        let tabs: Vec<(&str, Page, bool)> = if self.bar_visible() { Vec::new() } else { tabs };
        for (label, page, sp) in tabs {
            if x + 60.0 > rx {
                break;
            }
            x += self.tab(ui, x, cy, label, page, sp);
        }
    }

    // ------------------------------------------------------------------ overlays

    fn draw_overlays(&mut self, ui: &mut Ui, full: Rect) {
        let p = ui.painter().clone();
        let w = full.width();

        // click-away for menus
        let clicked_pos = ui.input(|i| if i.pointer.any_pressed() { i.pointer.interact_pos() } else { None });
        if let Some(pos) = clicked_pos {
            let inside = match self.menu {
                Menu::None => true,
                Menu::Provider => self.menu_rect.contains(pos) || self.provider_rect.contains(pos),
                Menu::Profile => self.menu_rect.contains(pos) || self.avatar_rect.contains(pos),
            };
            if !inside {
                self.menu = Menu::None;
            }
        }

        // Search suggestions dropdown
        let hover_prev = ui.input(|i| i.pointer.hover_pos()).is_some_and(|h| self.sug_rect.contains(h));
        let show_sug = (self.search_focused || hover_prev) && self.page != Page::Profiles;
        if show_sug {
            let q = self.search_text.trim().to_string();
            let mut items: Vec<(String, bool)> = Vec::new();
            if q.len() < 2 {
                for s in self.store.profile().recent_searches.iter().take(6) {
                    items.push((s.clone(), true));
                }
            } else {
                for s in self.suggestions.iter() {
                    items.push((s.clone(), false));
                }
            }
            let mut ask_ai = false;
            if q.split_whitespace().count() >= 4 && self.ai_ready() {
                ask_ai = true;
            }
            if !items.is_empty() || ask_ai {
                let row_h = 40.0;
                let n = items.len() + usize::from(ask_ai);
                let r = Rect::from_min_size(
                    pos2(self.search_rect.min.x, self.search_rect.max.y + 8.0),
                    vec2(self.search_rect.width().max(300.0), n as f32 * row_h + 12.0 + if q.len() < 2 { 26.0 } else { 0.0 }),
                );
                self.sug_rect = r.expand(6.0);
                shadow(&p, r, 14.0, 18.0, 1.0);
                p.rect_filled(r, cr(14.0), Color32::from_rgb(20, 20, 30));
                p.rect_stroke(r, cr(14.0), Stroke::new(1.0, BORDER), StrokeKind::Inside);
                let mut y = r.min.y + 6.0;
                if q.len() < 2 {
                    text(&p, pos2(r.min.x + 16.0, y + 12.0), Align2::LEFT_CENTER, "RECENT SEARCHES", 10.5, MUTED);
                    y += 26.0;
                }
                for (i, (s, recent)) in items.iter().enumerate() {
                    let rr = Rect::from_min_size(pos2(r.min.x + 6.0, y), vec2(r.width() - 12.0, row_h));
                    let resp = ui.interact(rr, Id::unique(("sug", i)), Sense::click());
                    pointer(ui, &resp);
                    if resp.hovered() {
                        p.rect_filled(rr, cr(10.0), alpha(Color32::WHITE, 0.07));
                    }
                    if *recent {
                        clock_icon(&p, pos2(rr.min.x + 20.0, rr.center().y), 15.0, MUTED);
                    } else {
                        search_icon(&p, pos2(rr.min.x + 20.0, rr.center().y), 14.0, MUTED);
                    }
                    let g = one_line(&p, s, 14.0, TEXT, rr.width() - 50.0);
                    p.galley(pos2(rr.min.x + 36.0, rr.center().y - g.size().y * 0.5), g, TEXT);
                    if resp.clicked() {
                        self.acts.push(Act::Search(s.clone()));
                        ui.memory_mut(|m| m.surrender_focus(Id::unique("search_input")));
                    }
                    y += row_h;
                }
                if ask_ai {
                    let rr = Rect::from_min_size(pos2(r.min.x + 6.0, y), vec2(r.width() - 12.0, row_h));
                    let resp = ui.interact(rr, Id::unique("sug_ai"), Sense::click());
                    pointer(ui, &resp);
                    p.rect_filled(rr, cr(10.0), alpha(ACCENT, if resp.hovered() { 0.25 } else { 0.14 }));
                    sparkle(&p, pos2(rr.min.x + 20.0, rr.center().y), 15.0, ACCENT);
                    text(&p, pos2(rr.min.x + 38.0, rr.center().y), Align2::LEFT_CENTER, "Ask the AI concierge instead", 13.5, TEXT);
                    if resp.clicked() {
                        self.acts.push(Act::Go(Page::Ai));
                        self.acts.push(Act::AskAi(q.clone()));
                        self.search_text.clear();
                        ui.memory_mut(|m| m.surrender_focus(Id::unique("search_input")));
                    }
                }
            } else {
                self.sug_rect = Rect::NOTHING;
            }
        } else {
            self.sug_rect = Rect::NOTHING;
        }

        // Menus
        match self.menu {
            Menu::Provider => {
                let items: Vec<ProviderKind> = ProviderKind::ENABLED.to_vec();
                let row_h = 46.0;
                let compact = self.compact();
                let r = if compact {
                    self.sheet(ui, full, items.len() as f32 * row_h + 64.0)
                } else {
                    Rect::from_min_size(
                        pos2((self.provider_rect.max.x - 280.0).max(8.0), self.provider_rect.max.y + 10.0),
                        vec2(280.0, items.len() as f32 * row_h + 48.0),
                    )
                };
                self.menu_rect = r;
                if !compact {
                    shadow(&p, r, 16.0, 20.0, 1.0);
                    p.rect_filled(r, cr(16.0), Color32::from_rgb(20, 20, 30));
                    p.rect_stroke(r, cr(16.0), Stroke::new(1.0, BORDER), StrokeKind::Inside);
                }
                text(&p, pos2(r.min.x + 18.0, r.min.y + if compact { 32.0 } else { 22.0 }), Align2::LEFT_CENTER, "SOURCE", 10.5, MUTED);
                let mut y = r.min.y + if compact { 50.0 } else { 40.0 };
                for k in items {
                    let rr = Rect::from_min_size(pos2(r.min.x + 6.0, y), vec2(r.width() - 12.0, row_h - 4.0));
                    let resp = ui.interact(rr, Id::unique(("prov", k.cache_key())), Sense::click());
                    pointer(ui, &resp);
                    if resp.hovered() {
                        p.rect_filled(rr, cr(10.0), alpha(Color32::WHITE, 0.07));
                    }
                    text(&p, pos2(rr.min.x + 14.0, rr.center().y), Align2::LEFT_CENTER, k.label(), 14.5, TEXT);
                    if k.is_bdix() {
                        tag(&p, pos2(rr.max.x - 76.0, rr.center().y - 11.0), "BDIX", 10.5, DIM, alpha(Color32::WHITE, 0.08));
                    }
                    if k == self.provider {
                        check_icon(&p, pos2(rr.max.x - 20.0, rr.center().y), 16.0, ACCENT);
                    }
                    if resp.clicked() {
                        self.acts.push(Act::SetProvider(k));
                    }
                    y += row_h;
                }
            }
            Menu::Profile => {
                let n = self.store.profiles.len();
                let row_h = 50.0;
                let compact = self.compact();
                let r = if compact {
                    self.sheet(ui, full, n as f32 * row_h + 3.0 * 44.0 + 40.0)
                } else {
                    Rect::from_min_size(
                        pos2((self.avatar_rect.max.x - 270.0).max(8.0), self.avatar_rect.max.y + 10.0),
                        vec2(270.0, n as f32 * row_h + 4.0 * 44.0 + 20.0),
                    )
                };
                self.menu_rect = r;
                if !compact {
                    shadow(&p, r, 16.0, 20.0, 1.0);
                    p.rect_filled(r, cr(16.0), Color32::from_rgb(20, 20, 30));
                    p.rect_stroke(r, cr(16.0), Stroke::new(1.0, BORDER), StrokeKind::Inside);
                }
                let mut y = r.min.y + if compact { 26.0 } else { 10.0 };
                for (i, pr) in self.store.profiles.clone().iter().enumerate() {
                    let rr = Rect::from_min_size(pos2(r.min.x + 6.0, y), vec2(r.width() - 12.0, row_h - 4.0));
                    let resp = ui.interact(rr, Id::unique(("pr", i)), Sense::click());
                    pointer(ui, &resp);
                    if resp.hovered() {
                        p.rect_filled(rr, cr(10.0), alpha(Color32::WHITE, 0.07));
                    }
                    avatar(&p, pos2(rr.min.x + 26.0, rr.center().y), 16.0, &pr.name, pr.hue);
                    text(&p, pos2(rr.min.x + 52.0, rr.center().y), Align2::LEFT_CENTER, &pr.name, 14.5, TEXT);
                    if i == self.store.active {
                        check_icon(&p, pos2(rr.max.x - 20.0, rr.center().y), 16.0, ACCENT);
                    }
                    if resp.clicked() {
                        self.acts.push(Act::SwitchProfile(i));
                    }
                    y += row_h;
                }
                p.line_segment([pos2(r.min.x + 14.0, y + 4.0), pos2(r.max.x - 14.0, y + 4.0)], Stroke::new(1.0, BORDER));
                y += 10.0;
                let entries: [(&str, Act); 3] = [
                    ("Add profile", Act::EditProfile(None)),
                    ("Settings", Act::Go(Page::Settings)),
                    ("Who’s watching?", Act::Go(Page::Profiles)),
                ];
                for (i, (label, act)) in entries.into_iter().enumerate() {
                    let rr = Rect::from_min_size(pos2(r.min.x + 6.0, y), vec2(r.width() - 12.0, 40.0));
                    let resp = ui.interact(rr, Id::unique(("pm", i)), Sense::click());
                    pointer(ui, &resp);
                    if resp.hovered() {
                        p.rect_filled(rr, cr(10.0), alpha(Color32::WHITE, 0.07));
                    }
                    text(&p, pos2(rr.min.x + 16.0, rr.center().y), Align2::LEFT_CENTER, label, 14.0, DIM);
                    if resp.clicked() {
                        self.acts.push(act);
                    }
                    y += 44.0;
                }
            }
            Menu::None => self.menu_rect = Rect::NOTHING,
        }

        // Downloads panel
        if self.show_dls {
            let n = self.dls.len().max(1);
            let pw = 400.0f32.min(w - 24.0);
            let r = Rect::from_min_size(
                pos2(full.max.x - pw - if self.compact() { 12.0 } else { pad(w) * 0.5 }, nav_h(w) + 6.0),
                vec2(pw, (n.min(6) as f32) * 64.0 + 62.0),
            );
            shadow(&p, r, 16.0, 20.0, 1.0);
            p.rect_filled(r, cr(16.0), Color32::from_rgb(20, 20, 30));
            p.rect_stroke(r, cr(16.0), Stroke::new(1.0, BORDER), StrokeKind::Inside);
            bold(&p, pos2(r.min.x + 18.0, r.min.y + 24.0), Align2::LEFT_CENTER, "Downloads", 16.0, TEXT);
            if self.dls.is_empty() {
                text(&p, pos2(r.min.x + 18.0, r.min.y + 62.0), Align2::LEFT_CENTER, "Nothing downloading. Use Download on a title.", 13.0, MUTED);
            }
            let mut y = r.min.y + 50.0;
            for d in self.dls.iter().rev().take(6) {
                let g = one_line(&p, &d.label, 13.5, TEXT, r.width() - 36.0);
                p.galley(pos2(r.min.x + 18.0, y), g, TEXT);
                let bar = Rect::from_min_size(pos2(r.min.x + 18.0, y + 24.0), vec2(r.width() - 36.0, 6.0));
                let (frac, status) = match &d.state {
                    DlState::Running => {
                        let f = d.total.map(|t| d.done as f32 / t.max(1) as f32).unwrap_or(0.0);
                        let s = match d.total {
                            Some(t) => format!("{} of {}", fmt_bytes(d.done), fmt_bytes(t)),
                            None => fmt_bytes(d.done),
                        };
                        (f, s)
                    }
                    DlState::Done(_) => (1.0, "Complete".to_string()),
                    DlState::Failed(e) => (0.0, e.chars().take(46).collect()),
                };
                let color = match &d.state {
                    DlState::Done(_) => GREEN,
                    DlState::Failed(_) => ACCENT,
                    DlState::Running => VIOLET,
                };
                progress_bar(&p, bar, frac, color);
                text(&p, pos2(r.min.x + 18.0, y + 42.0), Align2::LEFT_CENTER, &status, 11.5, MUTED);
                y += 64.0;
            }
        }

        // Up Next card
        if let Some(u) = &self.up_next {
            let remain = (u.deadline - self.t).max(0.0);
            let uw = 380.0f32.min(w - 24.0);
            let r = Rect::from_min_size(pos2(full.max.x - uw - if self.compact() { 12.0 } else { 28.0 }, full.max.y - self.bottom_inset() - 150.0 - 16.0), vec2(uw, 150.0));
            shadow(&p, r, 18.0, 24.0, 1.0);
            p.rect_filled(r, cr(18.0), Color32::from_rgb(22, 22, 33));
            p.rect_stroke(r, cr(18.0), Stroke::new(1.0, BORDER), StrokeKind::Inside);
            text(&p, pos2(r.min.x + 20.0, r.min.y + 24.0), Align2::LEFT_CENTER, "UP NEXT", 11.0, ACCENT);
            bold(&p, pos2(r.min.x + 20.0, r.min.y + 50.0), Align2::LEFT_CENTER, &u.card.title, 17.0, TEXT);
            text(&p, pos2(r.min.x + 20.0, r.min.y + 74.0), Align2::LEFT_CENTER, &format!("Season {} · Episode {}", u.season, u.episode), 13.0, DIM);
            progress_bar(&p, Rect::from_min_size(pos2(r.min.x + 20.0, r.min.y + 90.0), vec2(r.width() - 40.0, 4.0)), 1.0 - (remain / 12.0) as f32, ACCENT);
            let play = button(ui, pos2(r.min.x + 20.0, r.max.y - 52.0), Btn::new("up_play", "Play now", Kind::Primary).icon(play_icon).height(36.0));
            let skip = button(ui, pos2(r.min.x + 20.0 + 140.0, r.max.y - 52.0), Btn::new("up_skip", "Not now", Kind::Glass).height(36.0));
            if play.clicked() {
                self.acts.push(Act::PlayNext);
            }
            if skip.clicked() {
                self.acts.push(Act::DismissUpNext);
            }
        }

        // Toasts
        let mut y = full.max.y - self.bottom_inset() - 36.0;
        for t in self.toasts.iter().rev() {
            let age = (self.t - t.born) as f32;
            let fade = (age / 0.2).clamp(0.0, 1.0) * ((5.0 - age) / 0.4).clamp(0.0, 1.0);
            let g = one_line(&p, &t.text, 14.0, TEXT, (w * 0.6).min(620.0));
            let tw = g.size().x + 56.0;
            let r = Rect::from_center_size(pos2(full.center().x, y - 20.0 + (1.0 - fade) * 12.0), vec2(tw, 42.0));
            let bg = Color32::from_rgb(28, 28, 42);
            p.rect_filled(r, cr(21.0), alpha(bg, 0.96 * fade));
            p.rect_stroke(r, cr(21.0), Stroke::new(1.0, alpha(if t.error { ACCENT } else { BORDER }, fade)), StrokeKind::Inside);
            p.circle_filled(pos2(r.min.x + 20.0, r.center().y), 4.5, alpha(if t.error { ACCENT } else { GREEN }, fade));
            p.galley(pos2(r.min.x + 36.0, r.center().y - g.size().y * 0.5), g, alpha(TEXT, fade));
            y -= 52.0;
        }
    }

    // ------------------------------------------------------------------ cards

    /// Draw a poster card. Returns the click response when visible.
    pub fn card(&mut self, ui: &Ui, rect: Rect, clip: Rect, c: &Card, o: &CardOpts) -> Option<Response> {
        let vis = rect.expand(10.0).intersect(clip);
        if vis.width() < 2.0 || vis.height() < 2.0 {
            return None;
        }
        let p = ui.painter().clone();
        let id = Id::unique(("card", o.key, c.key()));
        let resp = ui.interact(rect.intersect(clip), id, Sense::click());
        pointer(ui, &resp);
        let hover = (resp.hovered() && !self.touch) || resp.is_pointer_button_down_on();
        let h = ui.ctx().animate_bool_with_time(id.with("h"), hover, 0.14);
        let poster_rect = Rect::from_min_size(rect.min, vec2(rect.width(), (rect.width() * 1.5).round()));
        #[cfg(test)]
        self.card_rects.push((format!("{}|{}", o.key, c.key()), poster_rect));
        let r = poster_rect.expand(h * 5.0);
        shadow(&p, r, 12.0, 8.0 + h * 12.0, 0.7 + h * 0.5);
        let (tex, size, real) = self.poster(c);
        paint_tex(ui, r, tex, size, 12.0, Color32::WHITE, 0.2);
        if !real {
            // placeholder carries the title since there is no key art
            let ta = 1.0 - h;
            let lift = if o.resume.is_some() { 30.0 } else { 0.0 };
            let g = wrapped(&p, &c.title, 15.0, alpha(Color32::WHITE, ta), r.width() - if self.touch { 54.0 } else { 28.0 }, 4);
            let gy = r.max.y - 18.0 - lift - g.size().y;
            v_gradient(&p, Rect::from_min_max(pos2(r.min.x, gy - 44.0), r.max), alpha(Color32::BLACK, 0.0), alpha(Color32::BLACK, 0.78 * ta));
            p.galley(pos2(r.min.x + 14.0, gy), g, alpha(Color32::WHITE, ta));
        }
        p.rect_stroke(r, cr(12.0), Stroke::new(1.0 + h, mix(alpha(Color32::WHITE, 0.08), ACCENT, h)), StrokeKind::Inside);

        if let Some(rt) = c.rating.filter(|r| *r > 0.0) {
            rating_badge(&p, r.min + vec2(8.0, 8.0), rt);
        }
        if c.is_series() {
            tag(&p, pos2(r.max.x - 52.0, r.min.y + 8.0), "SERIES", 9.5, Color32::WHITE, alpha(Color32::BLACK, 0.6));
        }
        if let Some(lbl) = &o.resume {
            let bar = Rect::from_min_size(pos2(r.min.x, r.max.y - 5.0), vec2(r.width(), 5.0));
            p.rect_filled(bar, 0.0, alpha(Color32::WHITE, 0.15));
            p.rect_filled(Rect::from_min_size(bar.min, vec2(r.width() * 0.0 + 18.0, 5.0)), 0.0, ACCENT);
            let pr = Rect::from_min_size(pos2(r.min.x + 8.0, r.max.y - 34.0), vec2(text_w(&p, lbl, 11.0) + 20.0, 22.0));
            p.rect_filled(pr, cr(11.0), alpha(Color32::BLACK, 0.7));
            text(&p, pr.center(), Align2::CENTER_CENTER, lbl, 11.0, Color32::WHITE);
        }

        // Touch: an always-visible add/remove button instead of hover actions
        let mut touch_handled = false;
        if self.touch {
            let in_list = self.store.in_list(c);
            let ac = pos2(r.max.x - 21.0, r.max.y - 21.0);
            let add = icon_button(ui, ac, 15.0, &format!("ta_{}_{}", o.key, c.key()), if in_list { check_icon } else { plus_icon }, in_list);
            if add.clicked() {
                self.acts.push(Act::ToggleList(c.clone()));
                touch_handled = true;
            }
        }
        // Hover actions
        if h > 0.01 && !self.touch {
            v_gradient(&p, Rect::from_min_max(pos2(r.min.x, r.max.y - 90.0), r.max), alpha(Color32::BLACK, 0.0), alpha(Color32::BLACK, 0.7 * h));
            let cy = r.max.y - 30.0;
            let play = icon_button(ui, pos2(r.min.x + 28.0, cy), 17.0, &format!("cp_{}_{}", o.key, c.key()), play_icon, false);
            let in_list = self.store.in_list(c);
            let add = icon_button(ui, pos2(r.min.x + 70.0, cy), 17.0, &format!("ca_{}_{}", o.key, c.key()), if in_list { check_icon } else { plus_icon }, in_list);
            if play.clicked() {
                self.acts.push(Act::QuickPlay(c.clone()));
            } else if add.clicked() {
                self.acts.push(Act::ToggleList(c.clone()));
            } else if resp.clicked() {
                self.acts.push(Act::OpenDetails(c.clone()));
            }
        } else if resp.clicked() && !touch_handled {
            self.acts.push(Act::OpenDetails(c.clone()));
        }

        // Title and meta
        let ty = poster_rect.max.y + 12.0;
        let g = one_line(&p, &c.title, 14.0, TEXT, rect.width() - 4.0);
        p.galley(pos2(rect.min.x + 2.0, ty), g, TEXT);
        let meta = if let Some(why) = o.why.filter(|w| !w.is_empty()) {
            why.to_string()
        } else {
            let kind = if c.is_series() { "Series" } else { "Movie" };
            if c.year.is_empty() { kind.to_string() } else { format!("{} · {kind}", c.year) }
        };
        let g2 = wrapped(&p, &meta, 12.0, MUTED, rect.width() - 4.0, if o.why.is_some() { 3 } else { 1 });
        p.galley(pos2(rect.min.x + 2.0, ty + 21.0), g2, MUTED);
        Some(resp)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rail(&mut self, ui: &mut Ui, w: f32, key: &str, title: &str, sub: Option<&str>, cards: &[Card], mode: RailMode, whys: &[String]) {
        let px = pad(w);
        let (cw, ch) = card_dims(w);
        let gap = 16.0;
        let head_h = 56.0;
        let meta_h = if mode == RailMode::Ai { 78.0 } else { 54.0 };
        let rect = blk(ui, w, head_h + ch + meta_h + 10.0);
        let p = ui.painter().clone();
        let tx = if mode == RailMode::Ai { px + 28.0 } else { px };
        if mode == RailMode::Ai {
            sparkle(&p, pos2(rect.min.x + px + 10.0, rect.min.y + 29.0), 20.0, ACCENT);
        }
        bold(&p, pos2(rect.min.x + tx, rect.min.y + 30.0), Align2::LEFT_CENTER, title, if w < COMPACT_W { 18.0 } else { 20.0 }, TEXT);
        if let Some(s) = sub {
            let tw = text_w(&p, title, 20.0);
            text(&p, pos2(rect.min.x + tx + tw + 14.0, rect.min.y + 32.0), Align2::LEFT_CENTER, s, 13.0, MUTED);
        }
        if key == "foryou" {
            let rb = button(ui, pos2(rect.max.x - px - 96.0, rect.min.y + 12.0), Btn::new("foryou_refresh", "Refresh", Kind::Ghost).icon(sparkle).height(34.0));
            if rb.clicked() {
                self.acts.push(Act::RefreshForYou);
            }
        }
        let strip = Rect::from_min_size(pos2(rect.min.x, rect.min.y + head_h), vec2(w, ch + meta_h));
        let total = cards.len() as f32 * (cw + gap) - gap + px * 2.0;
        let max_off = (total - w).max(0.0);
        let hovered = !self.touch && ui.input(|i| i.pointer.hover_pos()).is_some_and(|h| strip.contains(h));
        let off = self.hscroll(ui, key, strip, total);
        let target = self.row_targets.get(key).copied().unwrap_or(0.0);
        let arrows_h = ui.ctx().animate_bool_with_time(Id::unique(("rail_arrows", key)), hovered, 0.15);

        let child = ui.new_child(UiBuilder::new().max_rect(strip));
        {
            let ui = &child;
            let clip = strip.intersect(ui.clip_rect());
            for (i, c) in cards.iter().enumerate() {
                let x = rect.min.x + px + i as f32 * (cw + gap) - off;
                if x > strip.max.x + 20.0 || x + cw < strip.min.x - 20.0 {
                    continue;
                }
                let r = Rect::from_min_size(pos2(x, strip.min.y + 6.0), vec2(cw, ch));
                let resume = if mode == RailMode::Resume {
                    self.store.last_progress(c).map(|pg| {
                        if c.is_series() { format!("S{} E{} · Resume", pg.season, pg.episode) } else { "Resume".to_string() }
                    })
                } else {
                    None
                };
                let why = if mode == RailMode::Ai { whys.get(i).map(String::as_str) } else { None };
                self.card(ui, r, clip, c, &CardOpts { key, resume, why });
            }
        }

        // Edge arrows
        if arrows_h > 0.02 && max_off > 1.0 {
            let cy = strip.min.y + 6.0 + ch * 0.5;
            if off > 2.0 {
                let c = pos2(rect.min.x + 26.0, cy);
                v_gradient_h(&p, Rect::from_min_size(strip.min, vec2(70.0, strip.height())), alpha(BG, 0.85 * arrows_h), alpha(BG, 0.0));
                if icon_button(ui, c, 20.0, &format!("al_{key}"), |p, c, s, col| chevron(p, c, s, -1, col), false).clicked() {
                    self.row_targets.insert(key.to_string(), (target - w * 0.8).max(0.0));
                }
            }
            if off < max_off - 2.0 {
                let c = pos2(rect.max.x - 26.0, cy);
                v_gradient_h(&p, Rect::from_min_size(pos2(strip.max.x - 70.0, strip.min.y), vec2(70.0, strip.height())), alpha(BG, 0.0), alpha(BG, 0.85 * arrows_h));
                if icon_button(ui, c, 20.0, &format!("ar_{key}"), |p, c, s, col| chevron(p, c, s, 1, col), false).clicked() {
                    self.row_targets.insert(key.to_string(), (target + w * 0.8).min(max_off));
                }
            }
        }
    }

    pub fn skeleton_rail(&mut self, ui: &mut Ui, w: f32, title_w: f32) {
        let px = pad(w);
        let (cw, ch) = card_dims(w);
        let rect = blk(ui, w, 56.0 + ch + 54.0 + 10.0);
        let p = ui.painter().clone();
        skeleton(&p, Rect::from_min_size(pos2(rect.min.x + px, rect.min.y + 20.0), vec2(title_w, 20.0)), 6.0, self.t);
        let mut x = rect.min.x + px;
        while x < rect.max.x {
            let r = Rect::from_min_size(pos2(x, rect.min.y + 62.0), vec2(cw, ch));
            skeleton(&p, r, 12.0, self.t);
            skeleton(&p, Rect::from_min_size(pos2(x, r.max.y + 12.0), vec2(cw * 0.8, 12.0)), 4.0, self.t);
            skeleton(&p, Rect::from_min_size(pos2(x, r.max.y + 32.0), vec2(cw * 0.5, 10.0)), 4.0, self.t);
            x += cw + 16.0;
        }
    }

    // ------------------------------------------------------------------ hero

    fn hero(&mut self, ui: &mut Ui, w: f32) {
        if self.compact() {
            return self.hero_compact(ui, w);
        }
        let h = (w * 0.40).clamp(400.0, 560.0);
        let rect = blk(ui, w, h);
        let p = ui.painter().clone();
        let px = pad(w);
        let hero = self.hero.clone();
        let idx = if hero.is_empty() { 0 } else { ((self.t / 9.0) as usize) % hero.len() };
        let card = hero.get(idx).cloned();

        let (tex, size, avg) = match &card {
            Some(c) => {
                let url = c.poster.clone();
                self.backdrop(url.as_deref(), &c.title)
            }
            None => self.backdrop(None, "moviebox"),
        };
        let fade = ui.ctx().animate_bool_with_time(Id::unique(("herofade", idx)), true, 0.7);
        let tint = Color32::from_white_alpha((fade * 255.0) as u8);
        paint_tex(ui, rect, tex, size, 0.0, tint, 0.35);
        if let Some(a) = avg {
            glow(&p, pos2(rect.max.x - rect.width() * 0.18, rect.min.y + h * 0.35), h * 0.9, alpha(a, 0.25));
        }
        h_gradient(&p, rect, alpha(BG, 0.62), alpha(BG, 0.0));
        let left = Rect::from_min_max(rect.min, pos2(rect.min.x + rect.width() * 0.62, rect.max.y));
        h_gradient(&p, left, alpha(BG, 0.88), alpha(BG, 0.0));
        v_gradient(&p, Rect::from_min_max(pos2(rect.min.x, rect.max.y - h * 0.5), rect.max), alpha(BG, 0.0), BG);
        v_gradient(&p, Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + 110.0)), alpha(BG, 0.55), alpha(BG, 0.0));

        let x = rect.min.x + px;
        let mut y = rect.max.y - h * 0.62;
        match &card {
            Some(c) => {
                let slide = (1.0 - fade) * 14.0;
                tag(&p, pos2(x, y - slide), "FEATURED", 10.5, Color32::WHITE, alpha(ACCENT, 0.9 * fade));
                y += 36.0;
                let g = wrapped_b(&p, &c.title, 48.0, alpha(Color32::WHITE, fade), (rect.width() * 0.5).max(300.0), 2);
                p.galley(pos2(x, y - slide), g.clone(), alpha(Color32::WHITE, fade));
                y += g.size().y + 14.0;
                let mut mx = x;
                if let Some(rt) = c.rating.filter(|r| *r > 0.0) {
                    star_icon(&p, pos2(mx + 8.0, y + 10.0), 8.0, GOLD);
                    let s = format!("{rt:.1}");
                    text(&p, pos2(mx + 22.0, y + 10.0), Align2::LEFT_CENTER, &s, 15.0, TEXT);
                    mx += 28.0 + text_w(&p, &s, 15.0) + 12.0;
                }
                let meta = format!("{}{}", if c.year.is_empty() { String::new() } else { format!("{}  ·  ", c.year) }, if c.is_series() { "Series" } else { "Movie" });
                text(&p, pos2(mx, y + 10.0), Align2::LEFT_CENTER, &meta, 15.0, DIM);
                y += 44.0;
                let play = button(ui, pos2(x, y), Btn::new("hero_play", "Play", Kind::Primary).icon(play_icon).height(50.0));
                let info = button(ui, pos2(x + play.rect.width() + 12.0, y), Btn::new("hero_info", "More info", Kind::Glass).icon(info_icon).height(50.0));
                let in_list = self.store.in_list(c);
                let lst = icon_button(ui, pos2(info.rect.max.x + 36.0, y + 25.0), 24.0, "hero_list", if in_list { check_icon } else { plus_icon }, in_list);
                if play.clicked() {
                    self.acts.push(Act::QuickPlay(c.clone()));
                }
                if info.clicked() {
                    self.acts.push(Act::OpenDetails(c.clone()));
                }
                if lst.clicked() {
                    self.acts.push(Act::ToggleList(c.clone()));
                }
                // carousel dots
                if hero.len() > 1 {
                    let mut dx = rect.max.x - px - hero.len() as f32 * 22.0;
                    for i in 0..hero.len() {
                        let act = i == idx;
                        let r = Rect::from_min_size(pos2(dx, rect.max.y - 56.0), vec2(if act { 18.0 } else { 7.0 }, 7.0));
                        p.rect_filled(r, cr(3.5), if act { TEXT } else { alpha(Color32::WHITE, 0.3) });
                        dx += r.width() + 8.0;
                    }
                }
            }
            None => {
                if self.home.is_loading() {
                    skeleton(&p, Rect::from_min_size(pos2(x, y + 36.0), vec2(360.0, 46.0)), 8.0, self.t);
                    skeleton(&p, Rect::from_min_size(pos2(x, y + 100.0), vec2(220.0, 18.0)), 6.0, self.t);
                    skeleton(&p, Rect::from_min_size(pos2(x, y + 140.0), vec2(130.0, 50.0)), 25.0, self.t);
                } else {
                    bold(&p, pos2(x, y + 50.0), Align2::LEFT_CENTER, "Your next favourite is waiting", 36.0, TEXT);
                    let msg = match &self.home {
                        Load::Failed(e) => format!("Couldn’t reach the catalogue ({e}). Check your connection."),
                        _ => "Pick a source and search to get started.".to_string(),
                    };
                    let g = wrapped(&p, &msg, 15.0, DIM, (rect.width() * 0.5).max(300.0), 3);
                    p.galley(pos2(x, y + 90.0), g, DIM);
                    let retry = button(ui, pos2(x, y + 150.0), Btn::new("hero_retry", "Try again", Kind::Primary).height(46.0));
                    if retry.clicked() {
                        self.acts.push(Act::RetryHome);
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------ pages

    fn genre_chips(&mut self, ui: &mut Ui, w: f32) {
        let px = pad(w);
        let rect = blk(ui, w, 58.0);
        let genres = ["Action", "Comedy", "Drama", "Thriller", "Sci-Fi", "Horror", "Romance", "Animation", "Crime", "Family"];
        let widths: Vec<f32> = genres.iter().map(|g| text_w(ui.painter(), g, 13.5) + 28.0).collect();
        let total = widths.iter().sum::<f32>() + (genres.len() as f32 - 1.0) * 10.0 + px * 2.0;
        let off = self.hscroll(ui, "genre_chips", rect, total);
        let child = ui.new_child(UiBuilder::new().max_rect(rect));
        let mut x = rect.min.x + px - off;
        let y = rect.min.y + 12.0;
        for (g, tw) in genres.iter().zip(widths.iter()) {
            if x + tw > rect.min.x - 4.0 && x < rect.max.x + 4.0 {
                let r = chip(&child, pos2(x, y), g, g, false);
                if r.clicked() {
                    self.acts.push(Act::Search(g.to_string()));
                }
            }
            x += tw + 10.0;
        }
    }

    fn personal_rails(&mut self, ui: &mut Ui, w: f32) {
        let history: Vec<Card> = self.store.profile().history.iter().map(|h| h.card.clone()).collect();
        if !history.is_empty() {
            self.rail(ui, w, "continue", "Continue Watching", Some("Pick up where you left off"), &history, RailMode::Resume, &[]);
        }
        let list = self.store.profile().my_list.clone();
        if !list.is_empty() {
            self.rail(ui, w, "mylist", "My List", None, &list, RailMode::Normal, &[]);
        }
        match &self.ai.for_you {
            Load::Ready(r) => {
                let (cards, whys): (Vec<Card>, Vec<String>) = r
                    .picks
                    .iter()
                    .filter_map(|p| p.card.clone().map(|c| (c, p.why.clone())))
                    .unzip();
                if cards.len() >= 2 {
                    self.rail(ui, w, "foryou", "Picked for You", Some("AI-curated from your taste"), &cards, RailMode::Ai, &whys);
                }
            }
            Load::Loading => self.skeleton_rail(ui, w, 220.0),
            _ => {}
        }
    }

    pub fn page_home(&mut self, ui: &mut Ui, w: f32) {
        self.hero(ui, w);
        self.genre_chips(ui, w);
        self.personal_rails(ui, w);
        self.catalog_rails(ui, w, 0);
        blk(ui, w, 80.0);
    }

    fn catalog_rails(&mut self, ui: &mut Ui, w: f32, which: u8) {
        let rows = self.rows.clone();
        let list = match which {
            1 => &rows.movies,
            2 => &rows.series,
            _ => &rows.all,
        };
        if self.home.is_loading() {
            for tw in [200.0, 150.0, 230.0] {
                self.skeleton_rail(ui, w, tw);
            }
        } else if let Load::Failed(e) = &self.home {
            let r = blk(ui, w, 120.0);
            text(ui.painter(), pos2(r.min.x + pad(w), r.center().y), Align2::LEFT_CENTER, &format!("Could not load the catalogue: {e}"), 14.0, MUTED);
        }
        for (i, (title, cards)) in list.iter().enumerate() {
            self.rail(ui, w, &format!("cat{which}_{i}"), title, None, cards, RailMode::Normal, &[]);
        }
    }

    pub fn page_browse(&mut self, ui: &mut Ui, w: f32, movies: bool) {
        let px = pad(w);
        let head = blk(ui, w, nav_h(w) + 92.0);
        let p = ui.painter().clone();
        glow(&p, pos2(head.min.x + w * 0.15, head.min.y + 40.0), 340.0, alpha(if movies { ACCENT } else { VIOLET }, 0.16));
        bold(&p, pos2(head.min.x + px, head.max.y - 44.0), Align2::LEFT_CENTER, if movies { "Movies" } else { "Series" }, 34.0, TEXT);
        text(&p, pos2(head.min.x + px, head.max.y - 12.0), Align2::LEFT_CENTER, if movies { "Blockbusters, hidden gems and everything between." } else { "Stories worth staying up for." }, 14.0, DIM);
        self.genre_chips(ui, w);
        self.catalog_rails(ui, w, if movies { 1 } else { 2 });
        blk(ui, w, 80.0);
    }

    pub fn grid(&mut self, ui: &mut Ui, w: f32, key: &str, cards: &[Card]) {
        let px = pad(w);
        let compact = self.compact();
        let gap = if compact { 10.0 } else { 18.0 };
        let min_cw = if compact { 104.0 } else { card_dims(w).0 };
        let cols = (((w - px * 2.0) + gap) / (min_cw + gap)).floor().max(1.0) as usize;
        let cw = ((w - px * 2.0) - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = (cw * 1.5).round();
        let row_h = ch + 66.0;
        let rows = cards.len().div_ceil(cols);
        let rect = blk(ui, w, rows as f32 * row_h + 20.0);
        let clip = ui.clip_rect();
        for (i, c) in cards.iter().enumerate() {
            let (cx, cy) = (i % cols, i / cols);
            let r = Rect::from_min_size(pos2(rect.min.x + px + cx as f32 * (cw + gap), rect.min.y + cy as f32 * row_h + 8.0), vec2(cw, ch));
            if r.max.y < clip.min.y - 40.0 || r.min.y > clip.max.y + 40.0 {
                continue;
            }
            self.card(ui, r, clip, c, &CardOpts { key, resume: None, why: None });
        }
    }

    pub fn page_title(&self, ui: &mut Ui, w: f32, title: &str, sub: &str) {
        let px = pad(w);
        let head = blk(ui, w, nav_h(w) + 90.0);
        let p = ui.painter().clone();
        bold(&p, pos2(head.min.x + px, head.max.y - 44.0), Align2::LEFT_CENTER, title, 32.0, TEXT);
        text(&p, pos2(head.min.x + px, head.max.y - 12.0), Align2::LEFT_CENTER, sub, 14.0, DIM);
    }

    pub fn empty_state(&mut self, ui: &mut Ui, w: f32, title: &str, body: &str, cta: Option<(&str, Act)>) {
        let r = blk(ui, w, 360.0);
        let p = ui.painter().clone();
        let c = pos2(r.center().x, r.min.y + 110.0);
        glow(&p, c, 150.0, alpha(VIOLET, 0.22));
        p.circle_stroke(c, 46.0, Stroke::new(2.0, alpha(Color32::WHITE, 0.18)));
        play_icon(&p, c + vec2(2.0, 0.0), 30.0, alpha(Color32::WHITE, 0.55));
        bold(&p, pos2(c.x, c.y + 86.0), Align2::CENTER_CENTER, title, 22.0, TEXT);
        let g = wrapped(&p, body, 14.5, DIM, 460.0, 3);
        p.galley(pos2(c.x - g.size().x * 0.5, c.y + 112.0), g, DIM);
        if let Some((label, act)) = cta {
            let b = button(ui, pos2(c.x - 70.0, c.y + 170.0), Btn::new("empty_cta", label, Kind::Primary).height(44.0));
            if b.clicked() {
                self.acts.push(act);
            }
        }
    }

    pub fn page_search(&mut self, ui: &mut Ui, w: f32) {
        if self.compact() {
            return self.page_search_compact(ui, w);
        }
        let sub = format!("Searching {} for “{}”", self.provider.label(), self.last_query);
        self.page_title(ui, w, "Search results", &sub);
        match &self.search {
            Load::Loading => {
                let px = pad(w);
                let (cw, ch) = card_dims(w);
                let r = blk(ui, w, ch * 2.0 + 140.0);
                let p = ui.painter().clone();
                let mut i = 0;
                let mut y = r.min.y + 8.0;
                while y < r.max.y - ch {
                    let mut x = r.min.x + px;
                    while x + cw < r.max.x - px + 4.0 {
                        skeleton(&p, Rect::from_min_size(pos2(x, y), vec2(cw, ch)), 12.0, self.t + i as f64 * 0.05);
                        skeleton(&p, Rect::from_min_size(pos2(x, y + ch + 12.0), vec2(cw * 0.8, 12.0)), 4.0, self.t);
                        x += cw + 18.0;
                        i += 1;
                    }
                    y += ch + 66.0;
                }
            }
            Load::Failed(e) => {
                let msg = format!("{e}. Try another source from the selector at the top right.");
                let q = self.last_query.clone();
                self.empty_state(ui, w, "Search didn’t work", &msg, Some(("Try again", Act::Search(q))));
            }
            Load::Ready(cards) if cards.is_empty() => {
                let msg = format!("Nothing on {} matched “{}”. Try another spelling, or switch source.", self.provider.label(), self.last_query);
                self.empty_state(ui, w, "No results", &msg, Some(("Ask the AI concierge", Act::Go(Page::Ai))));
            }
            Load::Ready(cards) => {
                let cards = cards.clone();
                self.grid(ui, w, "search", &cards);
            }
            Load::Idle => self.empty_state(ui, w, "Search for something", "Type a title in the search bar. Press / to jump there.", None),
        }
        blk(ui, w, 60.0);
    }

    pub fn page_my_list(&mut self, ui: &mut Ui, w: f32) {
        self.page_title(ui, w, "My List", "Saved to watch later. Add titles with the + button.");
        let list = self.store.profile().my_list.clone();
        if list.is_empty() {
            self.empty_state(ui, w, "Your list is empty", "Hover a poster and tap + to save it here.", Some(("Browse titles", Act::Go(Page::Home))));
        } else {
            self.grid(ui, w, "mylist_grid", &list);
        }
        blk(ui, w, 60.0);
    }
}

fn v_gradient_h(p: &egui::Painter, r: Rect, left: Color32, right: Color32) {
    h_gradient(p, r, left, right);
}

#[allow(dead_code)]
fn _unused(_: Pos2, _: Vec2) {}
