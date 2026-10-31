//! Drawing code, part 2: details, Live TV, AI concierge, settings and profiles.

use crate::app::*;
use crate::pages::*;
use crate::store::Card;
use crate::theme::*;
use crate::widgets::*;
use egui::{pos2, vec2, Align2, Color32, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui};
use moviebox_tui::providers::Release;

/// A pill-shaped text field.
pub fn input(ui: &mut Ui, rect: Rect, key: &str, text: &mut String, hint: &str, password: bool) -> Response {
    let p = ui.painter().clone();
    let id = Id::unique(("input", key));
    let focused = ui.memory(|m| m.has_focus(id));
    let f = ui.ctx().animate_bool_with_time(id.with("f"), focused, 0.15);
    p.rect_filled(rect, cr(rect.height() * 0.5), mix(alpha(Color32::WHITE, 0.07), Color32::from_rgb(22, 22, 32), f));
    p.rect_stroke(
        rect,
        cr(rect.height() * 0.5),
        Stroke::new(1.0 + f, mix(alpha(Color32::WHITE, 0.14), ACCENT, f)),
        StrokeKind::Inside,
    );
    let inner = Rect::from_min_max(pos2(rect.min.x + 18.0, rect.min.y + 3.0), pos2(rect.max.x - 18.0, rect.max.y - 3.0));
    let edit = egui::TextEdit::singleline(text)
        .id(id)
        .hint_text(hint)
        .font(font(14.5))
        .frame(egui::Frame::NONE)
        .password(password)
        .desired_width(inner.width())
        .margin(egui::Margin { left: 0, right: 0, top: ((inner.height() - 20.0) * 0.5).max(0.0) as i8, bottom: 0 })
        .align(egui::Align2::LEFT_CENTER);
    // A child ui keeps the parent layout cursor where it was (`ui.put` would move it back).
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    child.add(edit)
}

fn toggle(ui: &Ui, pos: egui::Pos2, key: &str, on: bool) -> Response {
    let rect = Rect::from_min_size(pos, vec2(46.0, 26.0));
    let resp = ui.interact(rect, Id::unique(("toggle", key)), Sense::click());
    pointer(ui, &resp);
    let t = ui.ctx().animate_bool_with_time(Id::unique(("toggle_t", key)), on, 0.15);
    let p = ui.painter();
    p.rect_filled(rect, cr(13.0), mix(Color32::from_rgb(58, 58, 78), ACCENT, t));
    p.circle_filled(pos2(rect.min.x + 13.0 + 20.0 * t, rect.center().y), 10.0, Color32::WHITE);
    resp
}

fn quality_color(r: &Release) -> Color32 {
    match r.resolution_u64() {
        h if h >= 2160 => GOLD,
        h if h >= 1080 => ACCENT,
        h if h >= 720 => VIOLET,
        _ => Color32::from_rgb(110, 120, 150),
    }
}

fn quality_label(r: &Release) -> String {
    match r.resolution_u64() {
        h if h >= 2160 => "4K".to_string(),
        h => format!("{h}p"),
    }
}

impl App {
    // ------------------------------------------------------------------ details

    pub fn page_details(&mut self, ui: &mut Ui, w: f32) {
        let Some(det) = self.det.take() else {
            self.empty_state_simple(ui, w, "Nothing selected");
            return;
        };
        let px = pad(w);
        let p = ui.painter().clone();
        if self.compact() {
            self.details_top_compact(ui, w, &det);
        } else {
        let top_h = 500.0;
        let rect = blk(ui, w, top_h);
        let p = ui.painter().clone();

        let meta = det.data.ready().cloned();
        let poster_url = meta.as_ref().and_then(|d| d.poster_url.clone()).or_else(|| det.card.poster.clone());
        let (tex, size, avg) = self.backdrop(poster_url.as_deref(), &det.card.title);
        paint_tex(ui, rect, tex, size, 0.0, Color32::WHITE, 0.3);
        if let Some(a) = avg {
            glow(&p, pos2(rect.max.x - w * 0.2, rect.min.y + 160.0), 420.0, alpha(a, 0.28));
        }
        h_gradient(&p, rect, alpha(BG, 0.9), alpha(BG, 0.25));
        v_gradient(&p, Rect::from_min_max(pos2(rect.min.x, rect.max.y - 260.0), rect.max), alpha(BG, 0.0), BG);
        v_gradient(&p, Rect::from_min_max(rect.min, pos2(rect.max.x, rect.min.y + 120.0)), alpha(BG, 0.7), alpha(BG, 0.0));

        // back
        let back = icon_button(ui, pos2(rect.min.x + px + 20.0, rect.min.y + nav_h(w) + 28.0), 20.0, "back", |p, c, s, col| chevron(p, c, s, -1, col), false);
        if back.clicked() {
            self.acts.push(Act::Back);
        }

        // poster
        let pr = Rect::from_min_size(pos2(rect.min.x + px, rect.min.y + 150.0), vec2(230.0, 345.0));
        shadow(&p, pr, 14.0, 26.0, 1.4);
        let (ptex, psize, preal) = match poster_url.as_deref() {
            Some(u) => self.poster_url(u, &det.card.key()),
            None => self.poster(&det.card),
        };
        paint_tex(ui, pr, ptex, psize, 14.0, Color32::WHITE, 0.2);
        if !preal {
            let g = wrapped_b(&p, &det.card.title, 22.0, Color32::WHITE, pr.width() - 36.0, 4);
            let gy = pr.max.y - 24.0 - g.size().y;
            v_gradient(&p, Rect::from_min_max(pos2(pr.min.x, gy - 50.0), pr.max), alpha(Color32::BLACK, 0.0), alpha(Color32::BLACK, 0.75));
            p.galley(pos2(pr.min.x + 18.0, gy), g, Color32::WHITE);
        }
        p.rect_stroke(pr, cr(14.0), Stroke::new(1.0, alpha(Color32::WHITE, 0.14)), StrokeKind::Inside);

        let x0 = pr.max.x + 44.0;
        let tw = (rect.max.x - x0 - px).max(240.0);
        let mut y = rect.min.y + 150.0;
        let title = meta.as_ref().map(|d| d.title.clone()).unwrap_or_else(|| det.card.title.clone());
        let g = wrapped_b(&p, &title, 42.0, TEXT, tw, 2);
        p.galley(pos2(x0, y), g.clone(), TEXT);
        y += g.size().y + 12.0;

        // meta row
        let mut mx = x0;
        let rating = meta.as_ref().and_then(|d| d.imdb_rating.clone()).or_else(|| det.card.rating.map(|r| format!("{r:.1}")));
        if let Some(r) = rating.filter(|r| !r.trim().is_empty()) {
            star_icon(&p, pos2(mx + 8.0, y + 11.0), 8.5, GOLD);
            text(&p, pos2(mx + 22.0, y + 11.0), Align2::LEFT_CENTER, &r, 15.0, TEXT);
            mx += 30.0 + text_w(&p, &r, 15.0) + 12.0;
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
        text(&p, pos2(mx, y + 11.0), Align2::LEFT_CENTER, &bits.join("  ·  "), 15.0, DIM);
        y += 38.0;

        // genres
        if let Some(d) = &meta {
            let mut gx = x0;
            for gname in d.genres.iter().take(5) {
                let r = tag(&p, pos2(gx, y), gname, 12.0, DIM, alpha(Color32::WHITE, 0.09));
                gx = r.max.x + 8.0;
            }
            if !d.genres.is_empty() {
                y += 36.0;
            }
        }

        // description
        match det.data {
            Load::Loading => {
                for (i, wd) in [1.0, 0.92, 0.6].iter().enumerate() {
                    skeleton(&p, Rect::from_min_size(pos2(x0, y + i as f32 * 22.0), vec2(tw * wd, 14.0)), 5.0, self.t);
                }
                y += 78.0;
            }
            Load::Failed(ref e) => {
                let g = wrapped(&p, &format!("Could not load details: {e}"), 14.5, ACCENT, tw, 3);
                p.galley(pos2(x0, y), g.clone(), ACCENT);
                y += g.size().y + 12.0;
            }
            _ => {
                let desc = meta
                    .as_ref()
                    .and_then(|d| d.description.clone().or_else(|| d.tagline.clone()))
                    .unwrap_or_default();
                if !desc.trim().is_empty() {
                    let g = wrapped(&p, desc.trim(), 15.0, DIM, tw.min(720.0), 5);
                    p.galley(pos2(x0, y), g.clone(), DIM);
                    y += g.size().y + 16.0;
                }
            }
        }

        // credits
        if let Some(d) = &meta {
            let mut line = Vec::new();
            if let Some(s) = d.director.as_ref().filter(|s| !s.trim().is_empty()) {
                line.push(format!("Director: {s}"));
            }
            if let Some(s) = d.stars.as_ref().filter(|s| !s.trim().is_empty()) {
                line.push(format!("Starring: {s}"));
            }
            if !line.is_empty() && y < rect.max.y - 110.0 {
                let g = one_line(&p, &line.join("   ·   "), 13.0, MUTED, tw);
                p.galley(pos2(x0, y), g, MUTED);
                y += 24.0;
            }
        }

        // actions
        let ay = (y + 6.0).min(rect.max.y - 58.0);
        let default_release = det.streams.ready().and_then(|l| pick_default(l, self.store.quality_cap)).cloned();
        let play_label = if det.card.is_series() && det.season > 0 {
            format!("Play S{} E{}", det.season, det.episode)
        } else {
            "Play".to_string()
        };
        let play = button(ui, pos2(x0, ay), Btn::new("d_play", &play_label, Kind::Primary).icon(play_icon).height(50.0));
        if play.clicked() {
            match &default_release {
                Some(r) => self.acts.push(Act::PlayRelease(r.clone())),
                None => {
                    if det.streams.is_loading() {
                        self.toast("Finding the best stream…", false);
                    } else {
                        self.toast("No playable stream yet. Try another source.", true);
                    }
                }
            }
        }
        let in_list = self.store.in_list(&det.card);
        let list_b = button(ui, pos2(play.rect.max.x + 10.0, ay), Btn::new("d_list", if in_list { "In My List" } else { "My List" }, Kind::Glass).icon(if in_list { check_icon } else { plus_icon }).height(50.0));
        if list_b.clicked() {
            self.acts.push(Act::ToggleList(det.card.clone()));
        }
        let dl = button(ui, pos2(list_b.rect.max.x + 10.0, ay), Btn::new("d_dl", "Download", Kind::Glass).icon(download_icon).height(50.0));
        if dl.clicked() {
            match &default_release {
                Some(r) => self.acts.push(Act::DownloadRelease(r.clone())),
                None => self.toast("Streams are still loading.", false),
            }
        }
        let ai_b = button(ui, pos2(dl.rect.max.x + 10.0, ay), Btn::new("d_ai", "Why watch?", Kind::Glass).icon(sparkle).height(50.0));
        if ai_b.clicked() {
            self.acts.push(Act::Insight);
        }
        let share = icon_button(ui, pos2(ai_b.rect.max.x + 34.0, ay + 25.0), 22.0, "d_share", |p, c, s, col| {
            // share glyph: three dots joined by lines
            let st = Stroke::new((s * 0.08).max(1.4), col);
            let (a, b, d) = (pos2(c.x - s * 0.28, c.y), pos2(c.x + s * 0.26, c.y - s * 0.26), pos2(c.x + s * 0.26, c.y + s * 0.26));
            p.line_segment([a, b], st);
            p.line_segment([a, d], st);
            for q in [a, b, d] {
                p.circle_filled(q, s * 0.11, col);
            }
        }, false);
        if share.clicked() {
            let t = if det.card.year.is_empty() { det.card.title.clone() } else { format!("{} ({})", det.card.title, det.card.year) };
            self.acts.push(Act::CopyText(t));
        }

        }

        // ---- AI insight
        match &det.insight {
            Load::Idle => {}
            state => {
                let r = blk(ui, w, 190.0);
                let card = Rect::from_min_max(pos2(r.min.x + px, r.min.y + 6.0), pos2(r.max.x - px, r.max.y - 12.0));
                p.rect_filled(card, cr(18.0), alpha(Color32::WHITE, 0.05));
                p.rect_stroke(card, cr(18.0), Stroke::new(1.0, alpha(VIOLET, 0.6)), StrokeKind::Inside);
                glow(&p, pos2(card.min.x + 60.0, card.min.y + 30.0), 160.0, alpha(VIOLET, 0.18));
                sparkle(&p, pos2(card.min.x + 30.0, card.min.y + 30.0), 20.0, ACCENT);
                bold(&p, pos2(card.min.x + 52.0, card.min.y + 30.0), Align2::LEFT_CENTER, "AI Insight", 16.0, TEXT);
                text(&p, pos2(card.max.x - 20.0, card.min.y + 30.0), Align2::RIGHT_CENTER, "Spoiler-free", 12.0, MUTED);
                match state {
                    Load::Loading => {
                        for (i, wd) in [0.9f32, 0.7, 0.5].iter().enumerate() {
                            skeleton(&p, Rect::from_min_size(pos2(card.min.x + 24.0, card.min.y + 58.0 + i as f32 * 26.0), vec2((card.width() - 48.0) * wd, 14.0)), 5.0, self.t);
                        }
                    }
                    Load::Failed(e) => {
                        let g = wrapped(&p, e, 14.0, ACCENT, card.width() - 48.0, 3);
                        p.galley(pos2(card.min.x + 24.0, card.min.y + 62.0), g, ACCENT);
                    }
                    Load::Ready(ins) => {
                        let g = wrapped(&p, &ins.pitch, 15.5, TEXT, card.width() - 48.0, 3);
                        p.galley(pos2(card.min.x + 24.0, card.min.y + 56.0), g.clone(), TEXT);
                        let ty = card.min.y + 56.0 + g.size().y + 14.0;
                        let mut tx = card.min.x + 24.0;
                        for m in &ins.mood {
                            let rr = tag(&p, pos2(tx, ty), m, 12.0, Color32::WHITE, alpha(VIOLET, 0.55));
                            tx = rr.max.x + 8.0;
                        }
                        if let Some(wi) = ins.watch_if.first() {
                            let t2 = if ins.watch_if.len() > 1 { format!("{}   ·   {}", wi, ins.watch_if[1]) } else { wi.clone() };
                            let g2 = one_line(&p, &t2, 13.0, DIM, card.width() - 48.0);
                            p.galley(pos2(card.min.x + 24.0, ty + 34.0), g2, DIM);
                        }
                        if !ins.note.is_empty() {
                            text(&p, pos2(card.min.x + 24.0, card.max.y - 18.0), Align2::LEFT_CENTER, &format!("Heads-up: {}", ins.note), 12.5, MUTED);
                        }
                    }
                    Load::Idle => {}
                }
            }
        }

        // ---- series: seasons + episodes
        if let Some(d) = det.data.ready() {
            if d.is_series() && !d.seasons.is_empty() {
                let r = blk(ui, w, 78.0);
                bold(&p, pos2(r.min.x + px, r.min.y + 28.0), Align2::LEFT_CENTER, "Episodes", 22.0, TEXT);
                let mut cx = r.min.x + px;
                let cy = r.min.y + 44.0;
                for s in &d.seasons {
                    let label = format!("Season {}", s.number);
                    let c = chip(ui, pos2(cx, cy), &format!("season{}", s.number), &label, s.number == det.season);
                    if c.clicked() && s.number != det.season {
                        self.acts.push(Act::PickSeason(s.number));
                    }
                    cx += c.rect.width() + 8.0;
                    if cx > r.max.x - px - 80.0 {
                        break;
                    }
                }
                if let Some(season) = d.seasons.iter().find(|s| s.number == det.season).or_else(|| d.seasons.first()) {
                    let row_h = 84.0;
                    let rr = blk(ui, w, season.episodes.len() as f32 * row_h + 16.0);
                    let clip = ui.clip_rect();
                    for (i, e) in season.episodes.iter().enumerate() {
                        let row = Rect::from_min_size(pos2(rr.min.x + px, rr.min.y + 6.0 + i as f32 * row_h), vec2(rr.width() - px * 2.0, row_h - 8.0));
                        if row.max.y < clip.min.y - 20.0 || row.min.y > clip.max.y + 20.0 {
                            continue;
                        }
                        let sel = e.number == det.episode;
                        let resp = ui.interact(row, Id::unique(("ep", season.number, e.number)), Sense::click());
                        pointer(ui, &resp);
                        let h = ui.ctx().animate_bool_with_time(Id::unique(("eph", season.number, e.number)), resp.hovered(), 0.12);
                        p.rect_filled(row, cr(14.0), if sel { alpha(ACCENT, 0.16) } else { mix(SURFACE, SURFACE2, h) });
                        p.rect_stroke(row, cr(14.0), Stroke::new(1.0, if sel { alpha(ACCENT, 0.7) } else { alpha(BORDER, 0.6) }), StrokeKind::Inside);
                        bold(&p, pos2(row.min.x + 34.0, row.center().y), Align2::CENTER_CENTER, &e.number.to_string(), 20.0, if sel { ACCENT } else { MUTED });
                        let name = e.title.clone().filter(|t| !t.trim().is_empty()).unwrap_or_else(|| format!("Episode {}", e.number));
                        let g = one_line(&p, &name, 15.0, TEXT, row.width() - 160.0);
                        p.galley(pos2(row.min.x + 70.0, row.min.y + 14.0), g, TEXT);
                        if let Some(ov) = e.overview.as_ref().filter(|s| !s.trim().is_empty()) {
                            let g = wrapped(&p, ov, 12.5, MUTED, row.width() - 160.0, 2);
                            p.galley(pos2(row.min.x + 70.0, row.min.y + 36.0), g, MUTED);
                        }
                        if sel {
                            tag(&p, pos2(row.max.x - 92.0, row.center().y - 11.0), "SELECTED", 10.0, Color32::WHITE, alpha(ACCENT, 0.8));
                        } else if resp.hovered() {
                            play_icon(&p, pos2(row.max.x - 34.0, row.center().y), 16.0, TEXT);
                        }
                        if resp.clicked() {
                            self.acts.push(Act::PickEpisode(season.number, e.number));
                        }
                    }
                }
            }
        }

        // ---- streams
        let hdr = blk(ui, w, 70.0);
        bold(&p, pos2(hdr.min.x + px, hdr.min.y + 36.0), Align2::LEFT_CENTER, "Streams", 22.0, TEXT);
        let sub = format!("via {}", self.provider.label());
        let src = chip(ui, pos2(hdr.min.x + px + 108.0, hdr.min.y + 19.0), "det_src", &sub, self.menu == Menu::Provider);
        self.provider_rect = src.rect;
        if src.clicked() {
            self.acts.push(Act::SetMenu(Menu::Provider));
        }
        match &det.streams {
            Load::Loading | Load::Idle => {
                let r = blk(ui, w, 3.0 * 70.0 + 10.0);
                for i in 0..3 {
                    skeleton(&p, Rect::from_min_size(pos2(r.min.x + px, r.min.y + i as f32 * 70.0), vec2(r.width() - px * 2.0, 60.0)), 14.0, self.t + i as f64 * 0.1);
                }
            }
            Load::Failed(e) => {
                let r = blk(ui, w, 90.0);
                let g = wrapped(&p, e, 14.0, ACCENT, r.width() - px * 2.0, 2);
                p.galley(pos2(r.min.x + px, r.min.y + 10.0), g, ACCENT);
            }
            Load::Ready(list) if list.is_empty() => {
                let r = blk(ui, w, 90.0);
                text(&p, pos2(r.min.x + px, r.min.y + 20.0), Align2::LEFT_CENTER, "No streams found for this title on this source. Try another source from the selector.", 14.0, MUTED);
            }
            Load::Ready(list) => {
                let rec = pick_default(list, self.store.quality_cap).map(|r| r.filename.clone());
                let r = blk(ui, w, list.len() as f32 * 72.0 + 12.0);
                let clip = ui.clip_rect();
                for (i, rel) in list.iter().enumerate() {
                    let row = Rect::from_min_size(pos2(r.min.x + px, r.min.y + i as f32 * 72.0), vec2(r.width() - px * 2.0, 62.0));
                    if row.max.y < clip.min.y - 20.0 || row.min.y > clip.max.y + 20.0 {
                        continue;
                    }
                    p.rect_filled(row, cr(14.0), SURFACE);
                    p.rect_stroke(row, cr(14.0), Stroke::new(1.0, alpha(BORDER, 0.7)), StrokeKind::Inside);
                    let q = quality_label(rel);
                    let qc = quality_color(rel);
                    let badge = Rect::from_center_size(pos2(row.min.x + 46.0, row.center().y), vec2(62.0, 28.0));
                    p.rect_filled(badge, cr(8.0), alpha(qc, 0.18));
                    p.rect_stroke(badge, cr(8.0), Stroke::new(1.0, alpha(qc, 0.8)), StrokeKind::Inside);
                    bold(&p, badge.center(), Align2::CENTER_CENTER, &q, 13.0, qc);
                    let name = if rel.filename.trim().is_empty() { rel.source_label().to_string() } else { rel.filename.clone() };
                    let compact = self.compact();
                    let name_w = if compact { row.width() - 200.0 } else { row.width() - 400.0 };
                    let g = one_line(&p, &name, 14.0, TEXT, name_w);
                    p.galley(pos2(row.min.x + 94.0, row.min.y + 10.0), g.clone(), TEXT);
                    if rec.as_deref() == Some(rel.filename.as_str()) && !compact {
                        tag(&p, pos2(row.min.x + 94.0 + g.size().x + 10.0, row.min.y + 9.0), "RECOMMENDED", 9.5, Color32::WHITE, alpha(GREEN, 0.55));
                    }
                    let mut bits: Vec<String> = Vec::new();
                    if let Some(c) = rel.codec.as_ref().filter(|s| !s.is_empty()) {
                        bits.push(c.clone());
                    }
                    if let Some(l) = rel.language.as_ref().filter(|s| !s.is_empty()) {
                        bits.push(l.clone());
                    }
                    if let Some(s) = rel.size_bytes {
                        bits.push(fmt_bytes(s));
                    }
                    bits.push(rel.source_label().to_string());
                    let g2 = one_line(&p, &bits.join("  ·  "), 12.5, MUTED, name_w);
                    p.galley(pos2(row.min.x + 94.0, row.min.y + 33.0), g2, MUTED);
                    let dlb = icon_button(ui, pos2(row.max.x - 34.0, row.center().y), 18.0, &format!("sdl{i}"), download_icon, false);
                    let pb = if compact {
                        icon_button(ui, pos2(row.max.x - 34.0 - 18.0 - 12.0 - 17.0, row.center().y), 17.0, &format!("spl{i}"), play_icon, true)
                    } else {
                        button(ui, pos2(row.max.x - 34.0 - 18.0 - 12.0 - 96.0, row.center().y - 18.0), Btn::new(&format!("spl{i}"), "Play", Kind::Primary).icon(play_icon).height(36.0))
                    };
                    if pb.clicked() {
                        self.acts.push(Act::PlayRelease(rel.clone()));
                    }
                    if dlb.clicked() {
                        self.acts.push(Act::DownloadRelease(rel.clone()));
                    }
                }
            }
        }
        blk(ui, w, 90.0);
        self.det = Some(det);
    }

    fn empty_state_simple(&mut self, ui: &mut Ui, w: f32, msg: &str) {
        let r = blk(ui, w, 400.0);
        text(ui.painter(), r.center(), Align2::CENTER_CENTER, msg, 18.0, MUTED);
    }

    // ------------------------------------------------------------------ live tv

    pub fn page_tv(&mut self, ui: &mut Ui, w: f32) {
        let px = pad(w);
        let head = blk(ui, w, nav_h(w) + 100.0);
        let p = ui.painter().clone();
        glow(&p, pos2(head.min.x + w * 0.2, head.min.y + 50.0), 360.0, alpha(Color32::from_rgb(40, 160, 255), 0.16));
        bold(&p, pos2(head.min.x + px, head.max.y - 52.0), Align2::LEFT_CENTER, "Live TV", 32.0, TEXT);
        let sub = format!("{} channels from {} playlist{}", self.tv.channels.len(), self.store.playlists.len(), if self.store.playlists.len() == 1 { "" } else { "s" });
        text(&p, pos2(head.min.x + px, head.max.y - 18.0), Align2::LEFT_CENTER, &sub, 14.0, DIM);

        // add playlist row
        let compact = self.compact();
        let r = blk(ui, w, if compact { 120.0 } else { 68.0 });
        let fw = if compact { w - px * 2.0 } else { (w - px * 2.0 - 140.0).min(640.0) };
        let field = Rect::from_min_size(pos2(r.min.x + px, r.min.y + 8.0), vec2(fw, 46.0));
        let resp = self.field(ui, field, "tv_url", "Paste an M3U / M3U8 playlist link", false, 0.0);
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let add_pos = if compact { pos2(r.min.x + px, r.min.y + 64.0) } else { pos2(field.max.x + 12.0, field.min.y) };
        let add = button(ui, add_pos, Btn::new("tv_add", "Add playlist", Kind::Primary).height(46.0));
        if add.clicked() || enter {
            let url = self.tv.url_input.clone();
            self.acts.push(Act::AddPlaylist(url));
        }

        // saved playlists
        if !self.store.playlists.is_empty() {
            let r = blk(ui, w, 46.0);
            let mut x = r.min.x + px;
            for (i, pl) in self.store.playlists.clone().iter().enumerate() {
                let label: String = pl.chars().rev().take(34).collect::<Vec<_>>().into_iter().rev().collect();
                let c = chip(ui, pos2(x, r.min.y + 4.0), &format!("pl{i}"), &format!("{label}  ×"), false);
                if c.clicked() {
                    self.acts.push(Act::RemovePlaylist(i));
                }
                x += c.rect.width() + 8.0;
                if x > r.max.x - px - 100.0 {
                    break;
                }
            }
        }
        if let Some(e) = self.tv.error.clone() {
            let r = blk(ui, w, 34.0);
            text(&p, pos2(r.min.x + px, r.center().y), Align2::LEFT_CENTER, &e, 13.5, ACCENT);
        }

        if self.tv.channels.is_empty() {
            if self.tv.loading > 0 {
                let (cw, _) = card_dims(w);
                let r = blk(ui, w, 300.0);
                let mut x = r.min.x + px;
                while x + cw < r.max.x - px {
                    skeleton(&p, Rect::from_min_size(pos2(x, r.min.y + 10.0), vec2(cw * 1.5, cw * 0.85)), 14.0, self.t + x as f64 * 0.001);
                    x += cw * 1.5 + 16.0;
                }
            } else {
                self.empty_state(ui, w, "Add your first playlist", "Live TV plays any IPTV playlist (M3U link) you provide. Paste a link above to load channels.", None);
            }
            blk(ui, w, 60.0);
            return;
        }

        // groups + filter
        let mut groups: Vec<(String, usize)> = Vec::new();
        for c in &self.tv.channels {
            let g = if c.group.trim().is_empty() { "Other".to_string() } else { c.group.clone() };
            match groups.iter_mut().find(|(n, _)| *n == g) {
                Some((_, n)) => *n += 1,
                None => groups.push((g, 1)),
            }
        }
        groups.sort_by_key(|g| std::cmp::Reverse(g.1));
        let r = blk(ui, w, 58.0);
        let filter_w = 200.0;
        let strip_right = if compact { r.max.x } else { r.max.x - px - filter_w - 8.0 };
        let strip = Rect::from_min_max(r.min, pos2(strip_right, r.max.y));
        let mut labels: Vec<(String, String, Option<String>)> = vec![("g_all".to_string(), "All".to_string(), None)];
        for (g, n) in groups.iter().take(14) {
            labels.push((format!("g_{g}"), format!("{g} · {n}"), Some(g.clone())));
        }
        let widths: Vec<f32> = labels.iter().map(|(_, l, _)| text_w(&p, l, 13.5) + 28.0).collect();
        let total = widths.iter().sum::<f32>() + (labels.len() as f32 - 1.0) * 8.0 + px * 2.0;
        let off = self.hscroll(ui, "tv_groups", strip, total);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(strip));
        child.set_clip_rect(strip.intersect(ui.clip_rect()));
        let mut x = strip.min.x + px - off;
        for ((key, label, group), cw) in labels.iter().zip(widths.iter()) {
            if x + cw > strip.min.x - 4.0 && x < strip.max.x + 4.0 {
                let active = self.tv.group == *group;
                let c = chip(&child, pos2(x, r.min.y + 10.0), key, label, active);
                if c.clicked() {
                    self.tv.group = group.clone();
                }
            }
            x += cw + 8.0;
        }
        let frect = if compact {
            let fr = blk(ui, w, 58.0);
            Rect::from_min_size(pos2(fr.min.x + px, fr.min.y + 4.0), vec2(w - px * 2.0, 46.0))
        } else {
            Rect::from_min_size(pos2(r.max.x - px - filter_w, r.min.y + 8.0), vec2(filter_w, 38.0))
        };
        self.field(ui, frect, "tv_filter", "Filter channels", false, 0.0);

        // channel grid
        let needle = self.tv.filter.trim().to_lowercase();
        let group = self.tv.group.clone();
        let list: Vec<_> = self
            .tv
            .channels
            .iter()
            .filter(|c| {
                let g = if c.group.trim().is_empty() { "Other" } else { c.group.as_str() };
                group.as_deref().is_none_or(|sel| sel == g) && (needle.is_empty() || c.name.to_lowercase().contains(&needle))
            })
            .take(600)
            .cloned()
            .collect();
        let gap = 16.0;
        let min_w = if compact { 150.0 } else { 230.0 };
        let cols = (((w - px * 2.0) + gap) / (min_w + gap)).floor().max(1.0) as usize;
        let cw = ((w - px * 2.0) - gap * (cols as f32 - 1.0)) / cols as f32;
        let ch = cw * 0.56;
        let row_h = ch + 52.0;
        let rows = list.len().div_ceil(cols);
        let grid = blk(ui, w, rows as f32 * row_h + 20.0);
        let clip = ui.clip_rect();
        for (i, c) in list.iter().enumerate() {
            let (cx, cy) = (i % cols, i / cols);
            let rect = Rect::from_min_size(pos2(grid.min.x + px + cx as f32 * (cw + gap), grid.min.y + cy as f32 * row_h + 6.0), vec2(cw, ch));
            if rect.max.y < clip.min.y - 30.0 || rect.min.y > clip.max.y + 30.0 {
                continue;
            }
            let id = Id::unique(("chan", &c.stream_url));
            let resp = ui.interact(rect.intersect(clip), id, Sense::click());
            pointer(ui, &resp);
            let h = ui.ctx().animate_bool_with_time(id.with("h"), resp.hovered(), 0.14);
            let rr = rect.expand(h * 4.0);
            shadow(&p, rr, 14.0, 8.0 + 10.0 * h, 0.7);
            let (tex, size, _) = self.placeholder_banner(&c.name);
            paint_tex(ui, rr, tex, size, 14.0, Color32::WHITE, 0.5);
            let mut has_logo = false;
            if !c.logo.trim().is_empty() {
                let (lt, ls, real) = self.poster_url(&c.logo, &c.name);
                if real {
                    has_logo = true;
                    let box_r = Rect::from_center_size(rr.center(), vec2(rr.width() * 0.5, rr.height() * 0.5));
                    p.rect_filled(box_r.expand(8.0), cr(10.0), alpha(Color32::WHITE, 0.92));
                    let fit = (box_r.width() / ls.x).min(box_r.height() / ls.y);
                    let img_r = Rect::from_center_size(box_r.center(), ls * fit);
                    paint_tex(ui, img_r, lt, ls, 4.0, Color32::WHITE, 0.5);
                }
            }
            if !has_logo {
                let initials: String = c.name.split_whitespace().filter_map(|w| w.chars().next()).take(2).flat_map(|ch| ch.to_uppercase()).collect();
                bold(&p, rr.center(), Align2::CENTER_CENTER, &initials, rr.height() * 0.36, alpha(Color32::WHITE, 0.88));
            }
            v_gradient(&p, Rect::from_min_max(pos2(rr.min.x, rr.max.y - 50.0), rr.max), alpha(Color32::BLACK, 0.0), alpha(Color32::BLACK, 0.55));
            tag(&p, rr.min + vec2(10.0, 10.0), "LIVE", 9.5, Color32::WHITE, alpha(ACCENT, 0.9));
            p.rect_stroke(rr, cr(14.0), Stroke::new(1.0 + h, mix(alpha(Color32::WHITE, 0.08), ACCENT, h)), StrokeKind::Inside);
            if h > 0.05 {
                p.circle_filled(rr.center(), 24.0 * h, alpha(Color32::BLACK, 0.55));
                play_icon(&p, rr.center() + vec2(1.5, 0.0), 22.0 * h, Color32::WHITE);
            }
            let g = one_line(&p, &c.name, 14.0, TEXT, cw - 4.0);
            p.galley(pos2(rect.min.x + 2.0, rect.max.y + 12.0), g, TEXT);
            let grp = if c.group.trim().is_empty() { "Live" } else { c.group.as_str() };
            let g2 = one_line(&p, grp, 12.0, MUTED, cw - 4.0);
            p.galley(pos2(rect.min.x + 2.0, rect.max.y + 32.0), g2, MUTED);
            if resp.clicked() {
                self.acts.push(Act::PlayChannel(c.clone()));
            }
        }
        blk(ui, w, 60.0);
    }

    fn placeholder_banner(&mut self, seed: &str) -> (egui::TextureId, egui::Vec2, bool) {
        let (id, s) = self.aurora(&format!("tv:{seed}"));
        (id, s, false)
    }

    // ------------------------------------------------------------------ AI concierge

    pub fn page_ai(&mut self, ui: &mut Ui, w: f32) {
        let px = pad(w);
        let head = blk(ui, w, nav_h(w) + 150.0);
        let p = ui.painter().clone();
        glow(&p, pos2(head.center().x, head.min.y + 60.0), 460.0, alpha(VIOLET, 0.22));
        glow(&p, pos2(head.center().x + 220.0, head.min.y + 110.0), 320.0, alpha(ACCENT, 0.14));
        let c = pos2(head.center().x, head.min.y + nav_h(w) + 24.0);
        sparkle(&p, c, 34.0, ACCENT);
        bold(&p, pos2(c.x, c.y + 46.0), Align2::CENTER_CENTER, "Ask the AI concierge", 30.0, TEXT);
        if self.compact() {
            text(&p, pos2(c.x, c.y + 76.0), Align2::CENTER_CENTER, "Describe a mood, a movie you loved,", 14.0, DIM);
            text(&p, pos2(c.x, c.y + 96.0), Align2::CENTER_CENTER, "or a vibe. I’ll find what to watch.", 14.0, DIM);
        } else {
            text(&p, pos2(c.x, c.y + 78.0), Align2::CENTER_CENTER, "Describe a mood, a movie you loved, or a vibe. I’ll find what to watch.", 14.5, DIM);
        }

        let col_w = (w - px * 2.0).min(860.0);
        let cx0 = w * 0.5 - col_w * 0.5;

        if !self.ai_ready() {
            let r = blk(ui, w, 150.0);
            let card = Rect::from_min_size(pos2(cx0, r.min.y + 6.0), vec2(col_w, 120.0));
            p.rect_filled(card, cr(18.0), alpha(Color32::WHITE, 0.05));
            p.rect_stroke(card, cr(18.0), Stroke::new(1.0, alpha(VIOLET, 0.6)), StrokeKind::Inside);
            bold(&p, pos2(card.min.x + 26.0, card.min.y + 32.0), Align2::LEFT_CENTER, "Add your AI key to switch this on", 17.0, TEXT);
            let g = wrapped(&p, "AI features use your own Anthropic API key. It stays on this PC and requests go straight to the AI service, only when you ask.", 13.5, DIM, card.width() - 52.0, 2);
            p.galley(pos2(card.min.x + 26.0, card.min.y + 50.0), g, DIM);
            let b = button(ui, pos2(card.min.x + 26.0, card.max.y - 46.0), Btn::new("ai_key", "Open settings", Kind::Primary).height(36.0));
            if b.clicked() {
                self.acts.push(Act::Go(Page::Settings));
            }
        }

        // prompt box
        let compact = self.compact();
        let busy = self.ai.pending.is_some();
        let r = blk(ui, w, if compact { 130.0 } else { 76.0 });
        let fw = if compact { col_w } else { col_w - 130.0 };
        let field = Rect::from_min_size(pos2(cx0, r.min.y + 8.0), vec2(fw, 54.0));
        let resp = self.field(ui, field, "ai_in", "e.g. cozy rainy-day movies, or something like Inception but lighter", false, 0.0);
        let go_label = if busy { "Thinking…" } else { "Ask" };
        let go = if compact {
            button_in(ui, Rect::from_min_size(pos2(cx0, field.max.y + 10.0), vec2(col_w, 50.0)), Btn::new("ai_go", go_label, Kind::Primary).icon(sparkle).height(50.0))
        } else {
            button(ui, pos2(field.max.x + 12.0, field.min.y), Btn::new("ai_go", go_label, Kind::Primary).icon(sparkle).height(54.0))
        };
        let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (go.clicked() || enter) && !busy {
            let t = self.ai.input.clone();
            self.acts.push(Act::AskAi(t));
        }

        // example chips
        let r = blk(ui, w, 56.0);
        let ideas = ["Cozy rainy-day movies", "Mind-bending sci-fi", "Short series to binge this weekend", "Feel-good comedies", "Edge-of-your-seat thrillers"];
        let mut x = cx0;
        let mut y = r.min.y + 6.0;
        for s in ideas {
            let cw = text_w(&p, s, 13.5) + 28.0;
            if x + cw > cx0 + col_w {
                x = cx0;
                y += 42.0;
            }
            let c = chip(ui, pos2(x, y), s, s, false);
            if c.clicked() && !busy {
                self.acts.push(Act::AskAi(s.to_string()));
            }
            x += cw + 10.0;
        }
        blk(ui, w, (y - r.min.y - 6.0).max(0.0) + 8.0);

        // conversation
        let turns: Vec<(String, Result<crate::ai::Reply, String>)> = self.ai.turns.iter().map(|t| (t.prompt.clone(), t.result.clone())).collect();
        for (ti, (prompt, result)) in turns.iter().enumerate().rev() {
            // user bubble
            let g = wrapped(&p, prompt, 15.0, Color32::WHITE, col_w * 0.6, 3);
            let bh = g.size().y + 28.0;
            let r = blk(ui, w, bh + 18.0);
            let bw = g.size().x + 40.0;
            let br = Rect::from_min_size(pos2(cx0 + col_w - bw, r.min.y + 8.0), vec2(bw, bh));
            rounded_gradient(&p, br, 18.0, ACCENT, Color32::from_rgb(196, 64, 150));
            p.galley(pos2(br.min.x + 20.0, br.min.y + 14.0), g, Color32::WHITE);
            match result {
                Err(e) => {
                    let g = wrapped(&p, e, 14.5, ACCENT, col_w - 60.0, 4);
                    let r = blk(ui, w, g.size().y + 40.0);
                    p.galley(pos2(cx0 + 24.0, r.min.y + 12.0), g, ACCENT);
                }
                Ok(reply) => {
                    let g = wrapped(&p, &reply.text, 15.5, TEXT, col_w - 60.0, 4);
                    let r = blk(ui, w, g.size().y + 56.0);
                    let cardr = Rect::from_min_size(pos2(cx0, r.min.y + 4.0), vec2(col_w, g.size().y + 40.0));
                    p.rect_filled(cardr, cr(16.0), alpha(Color32::WHITE, 0.06));
                    sparkle(&p, pos2(cardr.min.x + 24.0, cardr.min.y + 24.0), 16.0, ACCENT);
                    p.galley(pos2(cardr.min.x + 46.0, cardr.min.y + 16.0), g, TEXT);
                    let found: Vec<(Card, String)> = reply.picks.iter().filter_map(|pk| pk.card.clone().map(|c| (c, pk.why.clone()))).collect();
                    if !found.is_empty() {
                        let (cards, whys): (Vec<Card>, Vec<String>) = found.into_iter().unzip();
                        self.rail(ui, w, &format!("ai_turn_{ti}"), "Picked for you", None, &cards, RailMode::Ai, &whys);
                    }
                    let missing: Vec<String> = reply.picks.iter().filter(|pk| pk.card.is_none()).map(|pk| pk.title.clone()).collect();
                    if !missing.is_empty() {
                        let r = blk(ui, w, 30.0);
                        let line = format!("Not on {}: {}", self.provider.label(), missing.join(", "));
                        let g = one_line(&p, &line, 12.5, MUTED, col_w);
                        p.galley(pos2(cx0 + 4.0, r.min.y + 6.0), g, MUTED);
                    }
                }
            }
            blk(ui, w, 16.0);
        }
        if let Some(q) = self.ai.pending.clone() {
            let g = wrapped(&p, &q, 15.0, Color32::WHITE, col_w * 0.6, 3);
            let bh = g.size().y + 28.0;
            let r = blk(ui, w, bh + 18.0);
            let bw = g.size().x + 40.0;
            let br = Rect::from_min_size(pos2(cx0 + col_w - bw, r.min.y + 8.0), vec2(bw, bh));
            rounded_gradient(&p, br, 18.0, ACCENT, Color32::from_rgb(196, 64, 150));
            p.galley(pos2(br.min.x + 20.0, br.min.y + 14.0), g, Color32::WHITE);
            let r = blk(ui, w, 70.0);
            let cardr = Rect::from_min_size(pos2(cx0, r.min.y + 4.0), vec2(150.0, 46.0));
            p.rect_filled(cardr, cr(23.0), alpha(Color32::WHITE, 0.06));
            for i in 0..3 {
                let ph = (self.t as f32 * 3.0 - i as f32 * 0.6).sin() * 0.5 + 0.5;
                p.circle_filled(pos2(cardr.min.x + 30.0 + i as f32 * 18.0, cardr.center().y - ph * 4.0 + 2.0), 4.5, alpha(ACCENT, 0.4 + 0.6 * ph));
            }
            text(&p, pos2(cardr.min.x + 90.0, cardr.center().y), Align2::LEFT_CENTER, "Thinking", 13.5, DIM);
        }
        blk(ui, w, 90.0);
    }

    // ------------------------------------------------------------------ settings

    pub fn page_settings(&mut self, ui: &mut Ui, w: f32) {
        let px = pad(w);
        self.page_title_pub(ui, w, "Settings", "Make MovieBox yours.");
        let col_w = (w - px * 2.0).min(820.0);
        let x0 = w * 0.5 - col_w * 0.5;
        let p = ui.painter().clone();

        // section helper closure replaced by a small macro-like fn
        fn section(ui: &mut Ui, w: f32, x0: f32, col_w: f32, h: f32, title: &str) -> Rect {
            let r = blk(ui, w, h + 22.0);
            let card = Rect::from_min_size(pos2(r.min.x + x0, r.min.y + 4.0), vec2(col_w, h));
            let p = ui.painter();
            p.rect_filled(card, cr(18.0), SURFACE);
            p.rect_stroke(card, cr(18.0), Stroke::new(1.0, alpha(BORDER, 0.8)), StrokeKind::Inside);
            bold(p, pos2(card.min.x + 24.0, card.min.y + 28.0), Align2::LEFT_CENTER, title, 17.0, TEXT);
            card
        }

        // Profiles
        let n = self.store.profiles.len();
        let card = section(ui, w, x0, col_w, 120.0 + n as f32 * 0.0, "Profiles");
        let mut x = card.min.x + 24.0;
        for (i, pr) in self.store.profiles.clone().iter().enumerate() {
            let c = pos2(x + 28.0, card.min.y + 76.0);
            let resp = ui.interact(Rect::from_center_size(c, vec2(64.0, 64.0)), Id::unique(("setpr", i)), Sense::click());
            pointer(ui, &resp);
            avatar(&p, c, 26.0, &pr.name, pr.hue);
            if i == self.store.active {
                p.circle_stroke(c, 29.0, Stroke::new(2.0, ACCENT));
            }
            text(&p, pos2(c.x, c.y + 40.0), Align2::CENTER_CENTER, &pr.name, 12.0, DIM);
            if resp.clicked() {
                self.acts.push(Act::EditProfile(Some(i)));
            }
            x += 78.0;
        }
        if n < 5 {
            let c = pos2(x + 28.0, card.min.y + 76.0);
            let resp = icon_button(ui, c, 26.0, "setpr_add", plus_icon, false);
            if resp.clicked() {
                self.acts.push(Act::EditProfile(None));
            }
        }

        // Playback
        let card = section(ui, w, x0, col_w, 170.0, "Playback");
        text(&p, pos2(card.min.x + 24.0, card.min.y + 62.0), Align2::LEFT_CENTER, "Maximum video quality", 13.5, DIM);
        let opts = [("Auto", 0u32), ("480p", 480), ("720p", 720), ("1080p", 1080)];
        let mut cx = card.min.x + 24.0;
        for (label, v) in opts {
            let c = chip(ui, pos2(cx, card.min.y + 78.0), &format!("q{v}"), label, self.store.quality_cap == v);
            if c.clicked() {
                self.store.quality_cap = v;
                self.acts.push(Act::Persist);
            }
            cx += c.rect.width() + 8.0;
        }
        let players = self.players_text();
        let g = wrapped(&p, &players, 12.5, MUTED, card.width() - 48.0, 2);
        p.galley(pos2(card.min.x + 24.0, card.min.y + 128.0), g, MUTED);

        // AI
        let card = section(ui, w, x0, col_w, 266.0, "AI features");
        let g = wrapped(&p, "Powers the concierge, personal picks and “Why watch?”. Your key is stored only on this PC and is sent only to the AI service when you use an AI feature.", 13.0, DIM, card.width() - 48.0, 3);
        p.galley(pos2(card.min.x + 24.0, card.min.y + 50.0), g, DIM);
        let f1 = Rect::from_min_size(pos2(card.min.x + 24.0, card.min.y + 108.0), vec2(card.width() - 48.0, 44.0));
        let r1 = self.field(ui, f1, "ai_key_in", "Paste your Anthropic API key", true, 0.0);
        let f2 = Rect::from_min_size(pos2(card.min.x + 24.0, card.min.y + 172.0), vec2(card.width() - 48.0, 44.0));
        let r2 = self.field(ui, f2, "ai_model_in", "Model", false, 0.0);
        if r1.lost_focus() || r2.lost_focus() {
            self.acts.push(Act::Persist);
        }
        let status = if self.ai_ready() { "AI is on" } else { "AI is off. Add a key to enable it." };
        text(&p, pos2(card.max.x - 24.0, card.min.y + 28.0), Align2::RIGHT_CENTER, status, 12.5, if self.ai_ready() { GREEN } else { MUTED });

        // Shortcuts
        let card = section(ui, w, x0, col_w, 200.0, "Keyboard shortcuts");
        let keys = [("/", "Jump to search"), ("Esc", "Go back or close a menu"), ("Enter", "Run the search or AI request"), ("Click +", "Add a title to My List")];
        for (i, (k, d)) in keys.iter().enumerate() {
            let y = card.min.y + 64.0 + i as f32 * 32.0;
            let kr = tag(&p, pos2(card.min.x + 24.0, y - 12.0), k, 12.0, TEXT, alpha(Color32::WHITE, 0.1));
            text(&p, pos2(kr.max.x + 14.0, y), Align2::LEFT_CENTER, d, 13.5, DIM);
        }

        let r = blk(ui, w, 70.0);
        text(&p, pos2(r.center().x, r.center().y), Align2::CENTER_CENTER, "MovieBox · built on the open-source MovieBox-TUI engine", 12.0, MUTED);
        blk(ui, w, 40.0);
    }

    fn players_text(&mut self) -> String {
        if self.touch {
            return "Videos play in MovieBox’s built-in player.".to_string();
        }
        if self.players.is_none() {
            let list = moviebox_tui::player::detect();
            self.players = Some(if list.is_empty() {
                "No video player found. Put mpv.exe in the “tools” folder next to this app.".to_string()
            } else {
                format!("Players found: {}", list.iter().map(|k| k.label()).collect::<Vec<_>>().join(", "))
            });
        }
        self.players.clone().unwrap_or_default()
    }

    pub fn page_title_pub(&self, ui: &mut Ui, w: f32, title: &str, sub: &str) {
        let px = pad(w);
        let head = blk(ui, w, nav_h(w) + 90.0);
        let p = ui.painter().clone();
        bold(&p, pos2(head.min.x + px, head.max.y - 44.0), Align2::LEFT_CENTER, title, 32.0, TEXT);
        text(&p, pos2(head.min.x + px, head.max.y - 12.0), Align2::LEFT_CENTER, sub, 14.0, DIM);
    }

    // ------------------------------------------------------------------ profiles

    pub fn page_profiles(&mut self, ui: &mut Ui, w: f32) {
        let h = ui.ctx().content_rect().height().max(560.0);
        let r = blk(ui, w, h);
        let p = ui.painter().clone();
        let compact = self.compact();
        glow(&p, pos2(r.center().x, r.min.y + h * 0.3), 520.0, alpha(VIOLET, 0.2));
        glow(&p, pos2(r.center().x + 260.0, r.min.y + h * 0.5), 360.0, alpha(ACCENT, 0.14));
        bold(&p, pos2(r.center().x, r.min.y + h * if compact { 0.18 } else { 0.26 }), Align2::CENTER_CENTER, "Who’s watching?", if compact { 30.0 } else { 38.0 }, TEXT);
        let profiles = self.store.profiles.clone();
        let count = profiles.len() + usize::from(profiles.len() < 5);
        let tile = if compact { 100.0 } else { 150.0 };
        let gap = if compact { 16.0 } else { 34.0 };
        let per_row = (((w - 32.0) + gap) / (tile + gap)).floor().max(1.0) as usize;
        let row_h = tile + if compact { 70.0 } else { 90.0 };
        let rows = count.div_ceil(per_row);
        let top = r.min.y + h * if compact { 0.30 } else { 0.5 } - (rows as f32 - 1.0) * row_h * 0.5;
        for idx in 0..count {
            let (row, col) = (idx / per_row, idx % per_row);
            let in_row = (count - row * per_row).min(per_row);
            let total = in_row as f32 * tile + (in_row as f32 - 1.0) * gap;
            let c = pos2(r.center().x - total * 0.5 + col as f32 * (tile + gap) + tile * 0.5, top + row as f32 * row_h);
            if let Some(pr) = profiles.get(idx) {
                let resp = ui.interact(Rect::from_center_size(c + vec2(0.0, 18.0), vec2(tile, tile + 40.0)), Id::unique(("who", idx)), Sense::click());
                pointer(ui, &resp);
                let hv = ui.ctx().animate_bool_with_time(Id::unique(("whoh", idx)), resp.hovered() || resp.is_pointer_button_down_on(), 0.14);
                let ar = tile * 0.4;
                glow(&p, c, ar * 1.7 * hv, alpha(ACCENT, 0.3 * hv));
                avatar(&p, c, ar + 4.0 * hv, &pr.name, pr.hue);
                p.circle_stroke(c, ar + 4.0 + 4.0 * hv, Stroke::new(2.0 * hv, TEXT));
                text(&p, pos2(c.x, c.y + ar + 30.0), Align2::CENTER_CENTER, &pr.name, 16.0, mix(DIM, TEXT, hv));
                if pr.family {
                    tag(&p, pos2(c.x - 28.0, c.y + ar + 46.0), "FAMILY", 9.5, Color32::WHITE, alpha(GREEN, 0.5));
                }
                if resp.clicked() {
                    self.acts.push(Act::SwitchProfile(idx));
                }
            } else {
                let b = icon_button(ui, c, tile * 0.36, "who_add", plus_icon, false);
                text(&p, pos2(c.x, c.y + tile * 0.4 + 30.0), Align2::CENTER_CENTER, "Add profile", 16.0, DIM);
                if b.clicked() {
                    self.acts.push(Act::EditProfile(None));
                }
            }
        }
        let by = top + (rows as f32 - 1.0) * row_h + tile + 70.0;
        let keep = button(ui, pos2(r.center().x - 80.0, by.min(r.max.y - 70.0)), Btn::new("who_manage", "Manage profiles", Kind::Glass).height(44.0));
        if keep.clicked() {
            self.acts.push(Act::Go(Page::Settings));
        }
    }

    pub fn page_profile_edit(&mut self, ui: &mut Ui, w: f32) {
        let h = ui.ctx().content_rect().height().max(560.0);
        let r = blk(ui, w, h);
        let p = ui.painter().clone();
        glow(&p, pos2(r.center().x, r.min.y + 160.0), 460.0, alpha(VIOLET, 0.18));
        let col_w = (w - 40.0).min(520.0);
        let x0 = r.center().x - col_w * 0.5;
        let editing = self.draft.index.is_some();
        bold(&p, pos2(r.center().x, r.min.y + 90.0), Align2::CENTER_CENTER, if editing { "Edit profile" } else { "New profile" }, 32.0, TEXT);
        avatar(&p, pos2(r.center().x, r.min.y + 190.0), 54.0, if self.draft.name.is_empty() { "?" } else { &self.draft.name }, self.draft.hue);
        let f = Rect::from_min_size(pos2(x0, r.min.y + 270.0), vec2(col_w, 50.0));
        self.field(ui, f, "pe_name", "Profile name", false, 0.0);

        text(&p, pos2(x0, r.min.y + 354.0), Align2::LEFT_CENTER, "Colour", 13.5, DIM);
        for i in 0..8 {
            let hue = i as f32 * 45.0 + 350.0;
            let c = pos2(x0 + 18.0 + i as f32 * ((col_w - 36.0) / 7.0), r.min.y + 390.0);
            let resp = ui.interact(Rect::from_center_size(c, vec2(38.0, 38.0)), Id::unique(("hue", i)), Sense::click());
            pointer(ui, &resp);
            avatar(&p, c, 15.0, "", hue);
            if (self.draft.hue - hue).abs() < 1.0 {
                p.circle_stroke(c, 19.0, Stroke::new(2.0, TEXT));
            }
            if resp.clicked() {
                self.draft.hue = hue;
            }
        }
        text(&p, pos2(x0, r.min.y + 448.0), Align2::LEFT_CENTER, "Family-friendly picks", 15.0, TEXT);
        text(&p, pos2(x0, r.min.y + 470.0), Align2::LEFT_CENTER, "AI suggestions for this profile stay suitable for children.", 12.5, MUTED);
        let t = toggle(ui, pos2(x0 + col_w - 46.0, r.min.y + 448.0 - 13.0), "pe_family", self.draft.family);
        if t.clicked() {
            self.draft.family = !self.draft.family;
        }
        let save = button(ui, pos2(x0, r.min.y + 520.0), Btn::new("pe_save", "Save", Kind::Primary).height(46.0));
        let cancel = button(ui, pos2(x0 + 110.0, r.min.y + 520.0), Btn::new("pe_cancel", "Cancel", Kind::Glass).height(46.0));
        if save.clicked() {
            self.acts.push(Act::SaveProfile);
        }
        if cancel.clicked() {
            self.acts.push(Act::Back);
        }
        if let Some(i) = self.draft.index {
            if self.store.profiles.len() > 1 {
                let del = button(ui, pos2(x0 + col_w - 130.0, r.min.y + 520.0), Btn::new("pe_del", "Delete", Kind::Ghost).height(46.0));
                if del.clicked() {
                    self.acts.push(Act::DeleteProfile(i));
                }
            }
        }
    }
}
