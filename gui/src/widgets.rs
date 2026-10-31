//! Small, stateless drawing helpers shared by every page.

use crate::theme::*;
use egui::{
    epaint::Mesh, load::SizedTexture, pos2, vec2, Align2, Color32, CursorIcon, FontId, Id, Image,
    Painter, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, TextureId, Ui, Vec2,
};
use std::sync::Arc;

pub type IconFn = fn(&Painter, Pos2, f32, Color32);

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn bfont(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("bold".into()))
}

pub fn text_w(p: &Painter, text: &str, size: f32) -> f32 {
    p.layout_no_wrap(text.to_string(), font(size), Color32::WHITE)
        .size()
        .x
}

pub fn text(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, color: Color32) -> Rect {
    p.text(pos, anchor, s, font(size), color)
}

/// Bold text (a real bold weight of the app typeface).
pub fn bold(p: &Painter, pos: Pos2, anchor: Align2, s: &str, size: f32, color: Color32) -> Rect {
    p.text(pos, anchor, s, bfont(size), color)
}

/// Like `wrapped`, in the bold weight.
pub fn wrapped_b(
    p: &Painter,
    s: &str,
    size: f32,
    color: Color32,
    width: f32,
    max_rows: usize,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(s.to_string(), bfont(size), color, width.max(1.0));
    job.wrap.max_rows = max_rows.max(1);
    job.wrap.overflow_character = Some('…');
    p.layout_job(job)
}

/// Lay out text wrapped to `width`, limited to `max_rows` (adds an ellipsis).
pub fn wrapped(
    p: &Painter,
    s: &str,
    size: f32,
    color: Color32,
    width: f32,
    max_rows: usize,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(s.to_string(), font(size), color, width.max(1.0));
    job.wrap.max_rows = max_rows.max(1);
    job.wrap.break_anywhere = false;
    job.wrap.overflow_character = Some('…');
    p.layout_job(job)
}

pub fn one_line(p: &Painter, s: &str, size: f32, color: Color32, width: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(s.to_string(), font(size), color, width.max(1.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    job.wrap.overflow_character = Some('…');
    p.layout_job(job)
}

/// Axis-aligned gradient rectangle with a colour per corner.
pub fn gradient_rect(p: &Painter, r: Rect, tl: Color32, tr: Color32, br: Color32, bl: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(r.left_top(), tl);
    m.colored_vertex(r.right_top(), tr);
    m.colored_vertex(r.right_bottom(), br);
    m.colored_vertex(r.left_bottom(), bl);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}

pub fn v_gradient(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    gradient_rect(p, r, top, top, bottom, bottom);
}

pub fn h_gradient(p: &Painter, r: Rect, left: Color32, right: Color32) {
    gradient_rect(p, r, left, right, right, left);
}

/// Radial glow made of a triangle fan.
pub fn glow(p: &Painter, center: Pos2, radius: f32, color: Color32) {
    let mut m = Mesh::default();
    m.colored_vertex(center, color);
    let n = 40;
    let edge = alpha(color, 0.0);
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        m.colored_vertex(pos2(center.x + a.cos() * radius, center.y + a.sin() * radius), edge);
    }
    for i in 0..n {
        m.add_triangle(0, 1 + i as u32, 1 + ((i + 1) % n) as u32);
    }
    p.add(Shape::mesh(m));
}

/// UV rect that crops an image of `img` size to cover `target`, biased vertically by `focus_y`.
pub fn cover_uv(img: Vec2, target: Vec2, focus_y: f32) -> Rect {
    if img.x <= 0.0 || img.y <= 0.0 || target.x <= 0.0 || target.y <= 0.0 {
        return Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    }
    let ir = img.x / img.y;
    let tr = target.x / target.y;
    if ir > tr {
        let w = tr / ir;
        let x0 = (1.0 - w) * 0.5;
        Rect::from_min_max(pos2(x0, 0.0), pos2(x0 + w, 1.0))
    } else {
        let h = ir / tr;
        let y0 = (1.0 - h) * focus_y.clamp(0.0, 1.0);
        Rect::from_min_max(pos2(0.0, y0), pos2(1.0, y0 + h))
    }
}

pub fn paint_tex(
    ui: &Ui,
    rect: Rect,
    id: TextureId,
    size: Vec2,
    radius: f32,
    tint: Color32,
    focus_y: f32,
) {
    let uv = cover_uv(size, rect.size(), focus_y);
    Image::new(SizedTexture::new(id, size))
        .uv(uv)
        .corner_radius(cr(radius))
        .tint(tint)
        .paint_at(ui, rect);
}

/// Soft drop shadow built from stacked translucent rounded rects.
pub fn shadow(p: &Painter, rect: Rect, radius: f32, spread: f32, strength: f32) {
    for i in 0..6 {
        let t = i as f32 / 6.0;
        let grow = spread * (1.0 - t);
        let a = strength * (0.07 + 0.05 * t) / 1.0;
        p.rect_filled(
            rect.expand(grow).translate(vec2(0.0, spread * 0.35)),
            cr(radius + grow),
            Color32::from_black_alpha((a * 255.0).clamp(0.0, 255.0) as u8),
        );
    }
}

pub fn pointer(ui: &Ui, r: &Response) {
    if r.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Ghost,
    Glass,
}

pub struct Btn<'a> {
    pub key: &'a str,
    pub label: &'a str,
    pub kind: Kind,
    pub icon: Option<IconFn>,
    pub height: f32,
    pub size: f32,
}

impl<'a> Btn<'a> {
    pub fn new(key: &'a str, label: &'a str, kind: Kind) -> Self {
        Self {
            key,
            label,
            kind,
            icon: None,
            height: 44.0,
            size: 15.0,
        }
    }
    pub fn icon(mut self, i: IconFn) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn height(mut self, h: f32) -> Self {
        self.height = h;
        self.size = (h * 0.34).clamp(12.0, 16.0);
        self
    }
}

pub fn btn_size(p: &Painter, b: &Btn) -> Vec2 {
    let tw = text_w(p, b.label, b.size);
    let icon_w = if b.icon.is_some() { b.height * 0.5 + 8.0 } else { 0.0 };
    vec2(tw + icon_w + b.height * 0.8, b.height)
}

/// Draw a button at `pos` (top-left). Returns its response.
pub fn button(ui: &Ui, pos: Pos2, b: Btn) -> Response {
    let p = ui.painter();
    let size = btn_size(p, &b);
    let rect = Rect::from_min_size(pos, size);
    button_in(ui, rect, b)
}

pub fn button_in(ui: &Ui, rect: Rect, b: Btn) -> Response {
    let p = ui.painter();
    let resp = ui.interact(rect, Id::unique(("btn", b.key)), Sense::click());
    pointer(ui, &resp);
    let h = ui.ctx().animate_bool_with_time(Id::unique(("btnh", b.key)), resp.hovered(), 0.12);
    let (fill, fg, stroke) = match b.kind {
        Kind::Primary => (mix(ACCENT, Color32::from_rgb(255, 110, 130), h * 0.5), Color32::WHITE, Stroke::NONE),
        Kind::Glass => (
            Color32::from_rgba_unmultiplied(255, 255, 255, (34.0 + h * 36.0) as u8),
            Color32::WHITE,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 46)),
        ),
        Kind::Ghost => (alpha(Color32::WHITE, 0.0 + h * 0.08), mix(DIM, TEXT, h), Stroke::NONE),
    };
    if b.kind == Kind::Primary {
        shadow(p, rect, rect.height() * 0.5, 10.0, 0.6 + h * 0.4);
    }
    p.rect_filled(rect, cr(rect.height() * 0.5), fill);
    if stroke != Stroke::NONE {
        p.rect_stroke(rect, cr(rect.height() * 0.5), stroke, StrokeKind::Inside);
    }
    let mut x = rect.min.x + b.height * 0.4;
    if let Some(icon) = b.icon {
        let isz = b.height * 0.42;
        icon(p, pos2(x + isz * 0.5, rect.center().y), isz, fg);
        x += isz + 8.0;
    }
    bold(p, pos2(x, rect.center().y), Align2::LEFT_CENTER, b.label, b.size, fg);
    resp
}

pub fn icon_button(ui: &Ui, center: Pos2, r: f32, key: &str, icon: IconFn, active: bool) -> Response {
    let p = ui.painter();
    let rect = Rect::from_center_size(center, vec2(r * 2.0, r * 2.0));
    let resp = ui.interact(rect, Id::unique(("ibtn", key)), Sense::click());
    pointer(ui, &resp);
    let h = ui.ctx().animate_bool_with_time(Id::unique(("ibtnh", key)), resp.hovered(), 0.12);
    let base = if active { alpha(ACCENT, 0.9) } else { alpha(Color32::WHITE, 0.10 + 0.10 * h) };
    p.circle_filled(center, r, base);
    p.circle_stroke(center, r, Stroke::new(1.0, alpha(Color32::WHITE, 0.12 + 0.2 * h)));
    icon(p, center, r * 1.05, if active { Color32::WHITE } else { mix(DIM, TEXT, h) });
    resp
}

pub fn chip(ui: &Ui, pos: Pos2, key: &str, label: &str, active: bool) -> Response {
    let p = ui.painter();
    let tw = text_w(p, label, 13.5);
    let rect = Rect::from_min_size(pos, vec2(tw + 28.0, 34.0));
    let resp = ui.interact(rect, Id::unique(("chip", key)), Sense::click());
    pointer(ui, &resp);
    let h = ui.ctx().animate_bool_with_time(Id::unique(("chiph", key)), resp.hovered(), 0.12);
    let (fill, fg, st) = if active {
        (TEXT, BG, Stroke::NONE)
    } else {
        (
            mix(SURFACE, SURFACE2, h),
            mix(DIM, TEXT, h),
            Stroke::new(1.0, mix(BORDER, Color32::from_rgb(90, 90, 120), h)),
        )
    };
    p.rect_filled(rect, cr(17.0), fill);
    if st != Stroke::NONE {
        p.rect_stroke(rect, cr(17.0), st, StrokeKind::Inside);
    }
    text(p, rect.center(), Align2::CENTER_CENTER, label, 13.5, fg);
    resp
}

/// A static, non-interactive tag.
pub fn tag(p: &Painter, pos: Pos2, label: &str, size: f32, fg: Color32, bg: Color32) -> Rect {
    let tw = text_w(p, label, size);
    let rect = Rect::from_min_size(pos, vec2(tw + size * 1.2, size + 10.0));
    p.rect_filled(rect, cr(6.0), bg);
    text(p, rect.center(), Align2::CENTER_CENTER, label, size, fg);
    rect
}

pub fn rating_badge(p: &Painter, pos: Pos2, rating: f64) -> Rect {
    let label = format!("{rating:.1}");
    let tw = text_w(p, &label, 12.0);
    let rect = Rect::from_min_size(pos, vec2(tw + 28.0, 22.0));
    p.rect_filled(rect, cr(11.0), Color32::from_black_alpha(165));
    star_icon(p, pos2(rect.min.x + 12.0, rect.center().y), 5.5, GOLD);
    text(p, pos2(rect.min.x + 21.0, rect.center().y), Align2::LEFT_CENTER, &label, 12.0, TEXT);
    rect
}

pub fn avatar(p: &Painter, center: Pos2, r: f32, name: &str, hue: f32) {
    let a = crate::art::hsv(hue, 0.62, 0.95);
    let b = crate::art::hsv(hue + 40.0, 0.7, 0.62);
    let ca = Color32::from_rgb(a[0] as u8, a[1] as u8, a[2] as u8);
    let cb = Color32::from_rgb(b[0] as u8, b[1] as u8, b[2] as u8);
    p.circle_filled(center, r, cb);
    p.circle_filled(center - vec2(0.0, r * 0.12), r * 0.88, ca);
    let initial: String = name.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
    bold(p, center, Align2::CENTER_CENTER, &initial, r * 1.0, Color32::WHITE);
}

pub fn progress_bar(p: &Painter, rect: Rect, frac: f32, fill: Color32) {
    p.rect_filled(rect, cr(rect.height() * 0.5), alpha(Color32::WHITE, 0.12));
    let w = (rect.width() * frac.clamp(0.0, 1.0)).max(if frac > 0.0 { rect.height() } else { 0.0 });
    if w > 0.0 {
        p.rect_filled(
            Rect::from_min_size(rect.min, vec2(w.min(rect.width()), rect.height())),
            cr(rect.height() * 0.5),
            fill,
        );
    }
}

/// Four-point sparkle used for everything AI.
pub fn sparkle(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let r = size * 0.5;
    let k = r * 0.22;
    let pts = vec![
        pos2(c.x, c.y - r),
        pos2(c.x + k, c.y - k),
        pos2(c.x + r, c.y),
        pos2(c.x + k, c.y + k),
        pos2(c.x, c.y + r),
        pos2(c.x - k, c.y + k),
        pos2(c.x - r, c.y),
        pos2(c.x - k, c.y - k),
    ];
    p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
    let s = r * 0.42;
    let off = pos2(c.x + r * 0.78, c.y - r * 0.78);
    let k2 = s * 0.25;
    p.add(Shape::convex_polygon(
        vec![
            pos2(off.x, off.y - s),
            pos2(off.x + k2, off.y - k2),
            pos2(off.x + s, off.y),
            pos2(off.x + k2, off.y + k2),
            pos2(off.x, off.y + s),
            pos2(off.x - k2, off.y + k2),
            pos2(off.x - s, off.y),
            pos2(off.x - k2, off.y - k2),
        ],
        color,
        Stroke::NONE,
    ));
}

pub fn plus_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.13).max(1.8), color);
    let h = size * 0.3;
    p.line_segment([pos2(c.x - h, c.y), pos2(c.x + h, c.y)], s);
    p.line_segment([pos2(c.x, c.y - h), pos2(c.x, c.y + h)], s);
}


/// Rounded rectangle filled with a diagonal gradient (triangle fan, anti-aliased by an outline).
pub fn rounded_gradient(p: &Painter, r: Rect, radius: f32, c0: Color32, c1: Color32) {
    let radius = radius.min(r.width() * 0.5).min(r.height() * 0.5);
    let segs = 8;
    let corners = [
        (r.max.x - radius, r.min.y + radius, -90.0f32),
        (r.max.x - radius, r.max.y - radius, 0.0),
        (r.min.x + radius, r.max.y - radius, 90.0),
        (r.min.x + radius, r.min.y + radius, 180.0),
    ];
    let mut pts: Vec<Pos2> = Vec::with_capacity(4 * (segs + 1));
    for (cx, cy, start) in corners {
        for i in 0..=segs {
            let a = (start + 90.0 * i as f32 / segs as f32).to_radians();
            pts.push(pos2(cx + a.cos() * radius, cy + a.sin() * radius));
        }
    }
    let color_at = |q: Pos2| {
        let t = ((q.x - r.min.x) / r.width().max(1.0)) * 0.75 + ((q.y - r.min.y) / r.height().max(1.0)) * 0.25;
        mix(c0, c1, t)
    };
    let mut m = Mesh::default();
    m.colored_vertex(r.center(), color_at(r.center()));
    for q in &pts {
        m.colored_vertex(*q, color_at(*q));
    }
    let n = pts.len() as u32;
    for i in 0..n {
        m.add_triangle(0, 1 + i, 1 + ((i + 1) % n));
    }
    p.add(Shape::mesh(m));
    p.rect_stroke(r, cr(radius), Stroke::new(1.0, c0), StrokeKind::Inside);
}

pub fn clock_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.1).max(1.4), color);
    p.circle_stroke(c, size * 0.38, s);
    p.line_segment([c, pos2(c.x, c.y - size * 0.22)], s);
    p.line_segment([c, pos2(c.x + size * 0.16, c.y + size * 0.08)], s);
}
