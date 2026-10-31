//! Visual identity: a deep blue-black canvas, a rose accent and soft violet glow.

use egui::{
    epaint::Shadow, pos2, vec2, Color32, CornerRadius, Painter, Pos2, Shape, Stroke, Visuals,
};

pub const BG: Color32 = Color32::from_rgb(10, 10, 15);
pub const SURFACE: Color32 = Color32::from_rgb(21, 21, 31);
pub const SURFACE2: Color32 = Color32::from_rgb(31, 31, 44);
pub const BORDER: Color32 = Color32::from_rgb(48, 48, 66);
pub const TEXT: Color32 = Color32::from_rgb(244, 244, 250);
pub const DIM: Color32 = Color32::from_rgb(170, 170, 190);
pub const MUTED: Color32 = Color32::from_rgb(112, 112, 134);
pub const ACCENT: Color32 = Color32::from_rgb(255, 59, 92);
pub const VIOLET: Color32 = Color32::from_rgb(124, 92, 255);
pub const GOLD: Color32 = Color32::from_rgb(255, 200, 87);
pub const GREEN: Color32 = Color32::from_rgb(70, 214, 140);

pub fn cr(r: f32) -> CornerRadius {
    CornerRadius::same(r.round().clamp(0.0, 255.0) as u8)
}

/// Colour with an explicit (non-premultiplied) alpha 0..1.
pub fn alpha(c: Color32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (a.clamp(0.0, 1.0) * 255.0) as u8)
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

const HANKEN_MEDIUM: &[u8] = include_bytes!("../assets/fonts/HankenGrotesk-Medium.otf");
const HANKEN_BOLD: &[u8] = include_bytes!("../assets/fonts/HankenGrotesk-Bold.otf");

fn install_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};
    use std::sync::Arc;
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("hanken".to_owned(), Arc::new(FontData::from_static(HANKEN_MEDIUM)));
    fonts.font_data.insert("hanken_bold".to_owned(), Arc::new(FontData::from_static(HANKEN_BOLD)));
    let fallbacks: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut regular = vec!["hanken".to_owned()];
    regular.extend(fallbacks.iter().cloned());
    let mut bold = vec!["hanken_bold".to_owned(), "hanken".to_owned()];
    bold.extend(fallbacks);
    fonts.families.insert(FontFamily::Proportional, regular);
    fonts.families.insert(FontFamily::Name("bold".into()), bold);
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context) {
    install_fonts(ctx);
    let mut v = Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = SURFACE;
    v.extreme_bg_color = SURFACE;
    v.faint_bg_color = SURFACE;
    v.override_text_color = Some(TEXT);
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_shadow = Shadow {
        offset: [0, 8],
        blur: 24,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    v.selection.bg_fill = alpha(ACCENT, 0.45);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.text_cursor.stroke = Stroke::new(2.0, ACCENT);
    ctx.set_visuals(v);
    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.interaction.tooltip_delay = 0.4;
        s.spacing.scroll = egui::style::ScrollStyle::floating();
    });
}

// ------------------------------------------------------------------ icons

pub fn play_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let h = size * 0.5;
    let pts = vec![
        pos2(c.x - h * 0.55, c.y - h),
        pos2(c.x - h * 0.55, c.y + h),
        pos2(c.x + h * 0.95, c.y),
    ];
    p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

pub fn search_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let r = size * 0.32;
    let s = Stroke::new((size * 0.11).max(1.5), color);
    let cc = pos2(c.x - size * 0.08, c.y - size * 0.08);
    p.circle_stroke(cc, r, s);
    let d = r * 0.72;
    p.line_segment(
        [pos2(cc.x + d, cc.y + d), pos2(c.x + size * 0.38, c.y + size * 0.38)],
        s,
    );
}

/// dir: -1 = left, 1 = right, 2 = down
pub fn chevron(p: &Painter, c: Pos2, size: f32, dir: i32, color: Color32) {
    let s = Stroke::new((size * 0.13).max(1.8), color);
    let h = size * 0.32;
    let (a, b, m) = match dir {
        -1 => (pos2(c.x + h * 0.5, c.y - h), pos2(c.x + h * 0.5, c.y + h), pos2(c.x - h * 0.6, c.y)),
        1 => (pos2(c.x - h * 0.5, c.y - h), pos2(c.x - h * 0.5, c.y + h), pos2(c.x + h * 0.6, c.y)),
        _ => (pos2(c.x - h, c.y - h * 0.5), pos2(c.x + h, c.y - h * 0.5), pos2(c.x, c.y + h * 0.6)),
    };
    p.line_segment([a, m], s);
    p.line_segment([m, b], s);
}

pub fn star_icon(p: &Painter, c: Pos2, r: f32, color: Color32) {
    let mut pts = Vec::with_capacity(10);
    for i in 0..10 {
        let ang = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let rad = if i % 2 == 0 { r } else { r * 0.45 };
        pts.push(pos2(c.x + ang.cos() * rad, c.y + ang.sin() * rad));
    }
    p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

pub fn download_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.12).max(1.6), color);
    let h = size * 0.34;
    p.line_segment([pos2(c.x, c.y - h), pos2(c.x, c.y + h * 0.55)], s);
    p.line_segment([pos2(c.x - h * 0.7, c.y + h * 0.0), pos2(c.x, c.y + h * 0.7)], s);
    p.line_segment([pos2(c.x + h * 0.7, c.y + h * 0.0), pos2(c.x, c.y + h * 0.7)], s);
    p.line_segment([pos2(c.x - h, c.y + h * 1.1), pos2(c.x + h, c.y + h * 1.1)], s);
}

pub fn info_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.09).max(1.5), color);
    p.circle_stroke(c, size * 0.42, s);
    p.line_segment([pos2(c.x, c.y - size * 0.02), pos2(c.x, c.y + size * 0.2)], s);
    p.circle_filled(pos2(c.x, c.y - size * 0.2), size * 0.05, color);
}

pub fn check_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.14).max(1.8), color);
    let h = size * 0.3;
    p.line_segment([pos2(c.x - h, c.y), pos2(c.x - h * 0.3, c.y + h * 0.7)], s);
    p.line_segment([pos2(c.x - h * 0.3, c.y + h * 0.7), pos2(c.x + h, c.y - h * 0.6)], s);
}

// ------------------------------------------------------------------ navigation icons

pub fn home_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.09).max(1.6), color);
    let u = size;
    let roof = [pos2(c.x - u * 0.42, c.y - u * 0.02), pos2(c.x, c.y - u * 0.40), pos2(c.x + u * 0.42, c.y - u * 0.02)];
    p.line_segment([roof[0], roof[1]], s);
    p.line_segment([roof[1], roof[2]], s);
    let body = [pos2(c.x - u * 0.30, c.y - u * 0.10), pos2(c.x - u * 0.30, c.y + u * 0.38), pos2(c.x + u * 0.30, c.y + u * 0.38), pos2(c.x + u * 0.30, c.y - u * 0.10)];
    for w in body.windows(2) {
        p.line_segment([w[0], w[1]], s);
    }
    p.line_segment([pos2(c.x - u * 0.09, c.y + u * 0.38), pos2(c.x - u * 0.09, c.y + u * 0.14)], s);
    p.line_segment([pos2(c.x + u * 0.09, c.y + u * 0.38), pos2(c.x + u * 0.09, c.y + u * 0.14)], s);
    p.line_segment([pos2(c.x - u * 0.09, c.y + u * 0.14), pos2(c.x + u * 0.09, c.y + u * 0.14)], s);
}

pub fn film_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.09).max(1.6), color);
    let r = egui::Rect::from_center_size(c, vec2(size * 0.80, size * 0.66));
    p.rect_stroke(r, cr(size * 0.1), s, egui::StrokeKind::Inside);
    p.line_segment([pos2(r.min.x + size * 0.17, r.min.y), pos2(r.min.x + size * 0.17, r.max.y)], s);
    p.line_segment([pos2(r.max.x - size * 0.17, r.min.y), pos2(r.max.x - size * 0.17, r.max.y)], s);
    for i in 0..3 {
        let y = r.min.y + size * (0.13 + 0.2 * i as f32);
        p.circle_filled(pos2(r.min.x + size * 0.085, y), size * 0.025, color);
        p.circle_filled(pos2(r.max.x - size * 0.085, y), size * 0.025, color);
    }
}

pub fn series_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.09).max(1.6), color);
    let back = egui::Rect::from_center_size(c + vec2(size * 0.07, -size * 0.09), vec2(size * 0.62, size * 0.46));
    let front = egui::Rect::from_center_size(c + vec2(-size * 0.05, size * 0.09), vec2(size * 0.62, size * 0.46));
    p.rect_stroke(back, cr(size * 0.08), Stroke::new(s.width, alpha(color, 0.55)), egui::StrokeKind::Inside);
    p.rect_stroke(front, cr(size * 0.08), s, egui::StrokeKind::Inside);
    play_icon(p, front.center() + vec2(size * 0.01, 0.0), size * 0.2, color);
}

pub fn live_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.08).max(1.5), color);
    p.circle_filled(c, size * 0.11, color);
    p.circle_stroke(c, size * 0.29, s);
    p.circle_stroke(c, size * 0.45, Stroke::new(s.width, alpha(color, 0.5)));
}

pub fn bookmark_icon(p: &Painter, c: Pos2, size: f32, color: Color32) {
    let s = Stroke::new((size * 0.09).max(1.6), color);
    let pts = vec![
        pos2(c.x - size * 0.25, c.y - size * 0.40),
        pos2(c.x + size * 0.25, c.y - size * 0.40),
        pos2(c.x + size * 0.25, c.y + size * 0.40),
        pos2(c.x, c.y + size * 0.20),
        pos2(c.x - size * 0.25, c.y + size * 0.40),
    ];
    p.add(Shape::closed_line(pts, s));
}
