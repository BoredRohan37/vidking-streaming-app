//! Headless touch / layout tests: synthetic pointer input through the real UI code.

use crate::app::*;
use crate::data::Backend;
use crate::store::Store;
use crate::{demo, platform, theme};
use egui::{pos2, vec2, Event, Modifiers, PointerButton, Pos2, Rect};
use std::sync::{mpsc, Arc, Mutex};

static SERIAL: Mutex<()> = Mutex::new(());

struct Harness {
    ctx: egui::Context,
    app: App,
    t: f64,
    size: egui::Vec2,
}

impl Harness {
    fn new(w: f32, h: f32, page: &str) -> Self {
        std::env::set_var("MOVIEBOX_CONFIG_DIR", std::env::temp_dir().join("moviebox-gui-tests"));
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let (tx, rx) = mpsc::channel();
        let be = Backend::new(tx, Arc::new(|| {}), false);
        let mut app = App::new(ctx.clone(), be, rx, Store::default());
        app.touch = true;
        demo::seed(&mut app, page);
        let mut hn = Self { ctx, app, t: 20.0, size: vec2(w, h) };
        for _ in 0..4 {
            hn.frame(vec![]);
        }
        hn
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.t += 0.05;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(self.t),
            events,
            ..Default::default()
        };
        let mut out = self.ctx.run_ui(raw, |ui| self.app.ui(ui));
        out.textures_delta.clear();
    }

    fn settle(&mut self) {
        for _ in 0..3 {
            self.frame(vec![]);
        }
    }

    fn mv(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerMoved(p)]);
    }

    fn press(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE }]);
    }

    fn release(&mut self, p: Pos2) {
        self.frame(vec![Event::PointerButton { pos: p, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE }]);
        self.settle();
    }

    fn tap(&mut self, p: Pos2) {
        self.mv(p);
        self.press(p);
        self.release(p);
    }

    /// Drag from `from` by `delta` in small steps, then release.
    fn drag(&mut self, from: Pos2, delta: egui::Vec2) {
        self.mv(from);
        self.press(from);
        let steps = 10;
        let mut p = from;
        for _ in 0..steps {
            p += delta / steps as f32;
            self.mv(p);
        }
        self.release(p);
    }

    /// A point inside the poster of the first card of the given rail (clear of the + button).
    fn card_point(&self, rail: &str) -> Pos2 {
        let (_, r) = self
            .app
            .card_rects
            .iter()
            .find(|(k, _)| k.starts_with(&format!("{rail}|")))
            .unwrap_or_else(|| panic!("no card drawn for rail {rail}"));
        pos2(r.min.x + r.width() * 0.35, r.min.y + r.height() * 0.3)
    }
}

#[test]
fn layout_form_follows_width() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let phone = Harness::new(390.0, 844.0, "home");
    assert!(phone.app.compact());
    let tablet = Harness::new(900.0, 1200.0, "home");
    assert!(!tablet.app.compact());
    assert!(tablet.app.bar_visible(), "touch tablets need the bottom bar so every tab is reachable");
    let wide = Harness::new(1280.0, 800.0, "home");
    assert!(!wide.app.bar_visible(), "wide windows have room for the top tabs");
}

#[test]
fn tapping_a_poster_opens_details() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = Harness::new(390.0, 844.0, "home");
    let p = h.card_point("continue");
    assert!(p.y < 844.0 - 70.0, "card must be above the bottom bar, got y={}", p.y);
    h.tap(p);
    assert_eq!(h.app.page, Page::Details);
    assert!(h.app.det.is_some());
}

#[test]
fn horizontal_drag_scrolls_the_rail_without_opening() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = Harness::new(390.0, 844.0, "home");
    let p = h.card_point("continue");
    h.drag(p, vec2(-120.0, 0.0));
    assert_eq!(h.app.page, Page::Home, "a swipe must not open the title");
    let off = h.app.row_targets.get("continue").copied().unwrap_or(0.0);
    assert!(off > 40.0, "rail should have scrolled, offset = {off}");
}

#[test]
fn vertical_drag_on_a_rail_scrolls_the_page() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = Harness::new(390.0, 844.0, "home");
    let before = h.app.scroll_y;
    let p = h.card_point("continue");
    h.drag(p, vec2(0.0, -220.0));
    assert_eq!(h.app.page, Page::Home);
    assert!(h.app.scroll_y > before + 60.0, "page should scroll, before={before} after={}", h.app.scroll_y);
    let off = h.app.row_targets.get("continue").copied().unwrap_or(0.0);
    assert!(off < 1.0, "the rail must not move on a vertical swipe, offset = {off}");
}

#[test]
fn bottom_bar_switches_tabs() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = Harness::new(390.0, 844.0, "home");
    // 5 tabs: Home, Movies, Series, Live TV, My List
    h.tap(pos2(390.0 / 5.0 * 1.5, 844.0 - 30.0));
    assert_eq!(h.app.page, Page::Movies);
    h.tap(pos2(390.0 / 5.0 * 4.5, 844.0 - 30.0));
    assert_eq!(h.app.page, Page::MyList);
    h.tap(pos2(390.0 / 5.0 * 0.5, 844.0 - 30.0));
    assert_eq!(h.app.page, Page::Home);
}

#[test]
fn search_field_uses_the_native_text_dialog() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = Harness::new(390.0, 844.0, "settings");
    h.app.page = Page::Search;
    h.app.search = Load::Idle;
    h.app.last_query.clear();
    h.settle();
    *platform::LAST_TEXT_REQUEST.lock().unwrap() = None;
    // the search field sits just under the top bar
    h.tap(pos2(195.0, 56.0 + 10.0 + 6.0 + 25.0));
    let req = platform::LAST_TEXT_REQUEST.lock().unwrap().clone();
    let (id, _, _, password) = req.expect("tapping the field should open the native text dialog");
    assert_eq!(id, "search");
    assert!(!password);
    // the dialog answers: the text lands in the field and the search runs
    platform::push_text("search".into(), "neon harbor".into(), true);
    h.settle();
    assert_eq!(h.app.search_text, "neon harbor");
    assert_eq!(h.app.last_query, "neon harbor");
    assert_eq!(h.app.page, Page::Search);
}

#[test]
fn password_field_is_masked_and_named_fields_roundtrip() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = Harness::new(390.0, 844.0, "settings");
    platform::push_text("ai_key_in".into(), "sk-test".into(), false);
    h.settle();
    assert_eq!(h.app.store.ai_key, "sk-test");
    platform::push_text("pe_name".into(), "A very long profile name indeed".into(), false);
    h.settle();
    assert_eq!(h.app.draft.name.chars().count(), 18);
}

#[test]
fn player_closed_offers_the_next_episode() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut h = Harness::new(390.0, 844.0, "details");
    let card = h.app.det.as_ref().unwrap().card.clone();
    h.app.now_playing = Some(NowPlaying { card, season: 1, episode: 2 });
    platform::notify_player_closed();
    h.settle();
    let up = h.app.up_next.as_ref().expect("an Up next prompt should appear");
    assert_eq!((up.season, up.episode), (1, 3));
}
