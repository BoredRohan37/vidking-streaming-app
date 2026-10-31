//! Application state and behaviour. Drawing lives in `pages.rs`.

use crate::ai;
use crate::art;
use crate::data::*;
use crate::store::*;
use egui::{Color32, ColorImage, Context, Rect, TextureHandle, TextureId, TextureOptions, Vec2};
use moviebox_tui::models::BrowseMetrics;
use moviebox_tui::providers::tv::models::Channel;
use moviebox_tui::providers::{MediaDetails, MediaType, ProviderKind, Release};
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::time::Duration;

/// Layout class derived from the window width (in points).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    /// Phones in portrait: bottom navigation, single column.
    Compact,
    /// Tablets, landscape phones and desktop.
    Regular,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Home,
    Movies,
    Series,
    MyList,
    Search,
    Details,
    Tv,
    Ai,
    Settings,
    Profiles,
    ProfileEdit,
}

impl Page {
    pub fn is_tab(self) -> bool {
        matches!(
            self,
            Page::Home | Page::Movies | Page::Series | Page::MyList | Page::Tv | Page::Ai
        )
    }
}

pub enum Load<T> {
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}

impl<T> Load<T> {
    pub fn ready(&self) -> Option<&T> {
        match self {
            Load::Ready(v) => Some(v),
            _ => None,
        }
    }
    pub fn is_loading(&self) -> bool {
        matches!(self, Load::Loading)
    }
}

#[derive(Default)]
pub struct Rows {
    pub all: Vec<(String, Vec<Card>)>,
    pub movies: Vec<(String, Vec<Card>)>,
    pub series: Vec<(String, Vec<Card>)>,
}

pub struct Details {
    pub card: Card,
    pub token: u64,
    pub data: Load<MediaDetails>,
    pub season: usize,
    pub episode: usize,
    pub streams: Load<Vec<Release>>,
    pub stream_token: u64,
    pub insight: Load<ai::Insight>,
    pub insight_token: u64,
}

pub struct AiTurn {
    pub prompt: String,
    pub result: Result<ai::Reply, String>,
}

pub struct AiState {
    pub input: String,
    pub token: u64,
    pub pending: Option<String>,
    pub turns: Vec<AiTurn>,
    pub for_you: Load<ai::Reply>,
    pub for_you_token: u64,
    pub for_you_profile: usize,
}

pub struct Tv {
    pub channels: Vec<Channel>,
    pub loading: usize,
    pub error: Option<String>,
    pub group: Option<String>,
    pub filter: String,
    pub url_input: String,
    pub loaded_urls: Vec<String>,
}

pub struct Dl {
    pub id: u64,
    pub label: String,
    pub done: u64,
    pub total: Option<u64>,
    pub state: DlState,
}

pub struct Toast {
    pub text: String,
    pub error: bool,
    pub born: f64,
}

pub struct UpNext {
    pub card: Card,
    pub season: usize,
    pub episode: usize,
    pub deadline: f64,
}

pub struct NowPlaying {
    pub card: Card,
    pub season: usize,
    pub episode: usize,
}

pub struct AutoPlay {
    pub token: u64,
    pub card: Card,
    pub season: usize,
    pub episode: usize,
}

pub struct ProfileDraft {
    pub index: Option<usize>,
    pub name: String,
    pub hue: f32,
    pub family: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Menu {
    None,
    Provider,
    Profile,
}

pub enum Act {
    Go(Page),
    Back,
    OpenDetails(Card),
    QuickPlay(Card),
    PlayRelease(Release),
    DownloadRelease(Release),
    ToggleList(Card),
    Search(String),
    SetProvider(ProviderKind),
    PickSeason(usize),
    PickEpisode(usize, usize),
    AskAi(String),
    RefreshForYou,
    Insight,
    SwitchProfile(usize),
    EditProfile(Option<usize>),
    SaveProfile,
    DeleteProfile(usize),
    AddPlaylist(String),
    RemovePlaylist(usize),
    PlayChannel(Channel),
    RetryHome,
    Persist,
    PlayNext,
    DismissUpNext,
    SetMenu(Menu),
    CopyText(String),
    ToggleDownloads,
}

pub enum Slot {
    Pending,
    Ready(TextureHandle, Vec2),
    Failed,
}

pub struct BackSlot {
    pub tex: TextureHandle,
    pub size: Vec2,
    pub avg: Color32,
}

#[derive(Default)]
pub struct Images {
    pub posters: HashMap<String, Slot>,
    pub backdrops: HashMap<String, Option<BackSlot>>,
    pub placeholders: HashMap<String, (TextureHandle, Vec2)>,
    pub aurora: HashMap<String, (TextureHandle, Vec2)>,
    pub logo: Option<(TextureHandle, Vec2)>,
}

pub struct App {
    pub ctx: Context,
    pub be: Backend,
    pub rx: Receiver<Msg>,
    pub store: Store,
    pub page: Page,
    pub back: Vec<Page>,
    pub nav_id: u64,
    pub t: f64,
    pub scroll_y: f32,
    pub provider: ProviderKind,
    pub home: Load<()>,
    pub rows: Arc<Rows>,
    pub hero: Arc<Vec<Card>>,
    pub search_text: String,
    pub last_query: String,
    pub search: Load<Vec<Card>>,
    pub search_token: u64,
    pub suggestions: Vec<String>,
    pub sug_token: u64,
    pub sug_for: String,
    pub sug_changed: f64,
    pub search_focused: bool,
    pub det: Option<Details>,
    pub orders: HashMap<String, Vec<(usize, usize)>>,
    pub ai: AiState,
    pub tv: Tv,
    pub dls: Vec<Dl>,
    pub dl_next: u64,
    pub show_dls: bool,
    pub toasts: Vec<Toast>,
    pub menu: Menu,
    pub up_next: Option<UpNext>,
    pub now_playing: Option<NowPlaying>,
    pub auto: Option<AutoPlay>,
    pub draft: ProfileDraft,
    pub imgs: Images,
    pub acts: Vec<Act>,
    pub token: u64,
    pub row_targets: HashMap<String, f32>,
    pub started: f64,
    pub provider_rect: Rect,
    pub avatar_rect: Rect,
    pub search_rect: Rect,
    pub sug_rect: Rect,
    pub menu_rect: Rect,
    pub players: Option<String>,
    /// Touch-first behaviour (Android): no hover effects, native text input, drag scrolling.
    pub touch: bool,
    pub form: Form,
    /// Window width in points (set every frame).
    pub win_w: f32,
    /// Axis of the drag gesture in progress: Some(true) = horizontal, Some(false) = vertical.
    pub gesture_axis: Option<bool>,
    #[cfg(test)]
    pub card_rects: Vec<(String, Rect)>,
}

// ------------------------------------------------------------------ pure helpers

/// Next (season, episode) after `cur` in `order`.
pub fn next_episode(order: &[(usize, usize)], cur: (usize, usize)) -> Option<(usize, usize)> {
    let i = order.iter().position(|&x| x == cur)?;
    order.get(i + 1).copied()
}

/// Best release to auto-play: the highest quality not above the cap (1080p when automatic),
/// or the lowest available quality if everything is above it.
pub fn pick_default(releases: &[Release], cap: u32) -> Option<&Release> {
    let limit = if cap == 0 { 1080 } else { cap as u64 };
    let playable: Vec<&Release> = releases.iter().filter(|r| !r.mirrors.is_empty()).collect();
    playable
        .iter()
        .copied()
        .filter(|r| r.resolution_u64() <= limit)
        .max_by_key(|r| r.resolution_u64())
        .or_else(|| playable.iter().copied().min_by_key(|r| r.resolution_u64()))
}

pub fn episode_order(d: &MediaDetails) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = d
        .seasons
        .iter()
        .flat_map(|s| s.episodes.iter().map(move |e| (s.number, e.number)))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

pub fn fmt_bytes(b: u64) -> String {
    const U: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < U.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", U[i])
    }
}

fn sort_desc(cards: &mut [(Card, BrowseMetrics)], f: impl Fn(&BrowseMetrics) -> Option<f64>) {
    cards.sort_by(|a, b| {
        f(&b.1)
            .unwrap_or(f64::MIN)
            .partial_cmp(&f(&a.1).unwrap_or(f64::MIN))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub fn build_rows(items: &[(Card, BrowseMetrics)]) -> Rows {
    fn rail(
        title: &str,
        list: Vec<Card>,
        out: &mut Vec<(String, Vec<Card>)>,
    ) {
        if list.len() >= 3 && !out.iter().any(|(_, c)| c.first() == list.first() && c.len() == list.len()) {
            out.push((title.to_string(), list));
        }
    }
    let make = |filter: Option<MediaType>| {
        let mut base: Vec<(Card, BrowseMetrics)> = items
            .iter()
            .filter(|(c, _)| filter.is_none_or(|k| k == c.kind))
            .cloned()
            .collect();
        let mut out = Vec::new();
        let label = match filter {
            None => ("Trending Now", "Popular Right Now", "Critically Acclaimed"),
            Some(MediaType::Movie) => ("Trending Movies", "Popular Movies", "Top Rated Movies"),
            Some(MediaType::Series) => ("Trending Series", "Popular Series", "Top Rated Series"),
        };
        let has = |base: &[(Card, BrowseMetrics)], f: &dyn Fn(&BrowseMetrics) -> Option<f64>| base.iter().any(|(_, m)| f(m).is_some());
        if has(&base, &|m| m.trending) {
            sort_desc(&mut base, |m| m.trending);
        }
        rail(label.0, base.iter().take(20).map(|(c, _)| c.clone()).collect(), &mut out);
        if has(&base, &|m| m.popularity) {
            let mut b = base.clone();
            sort_desc(&mut b, |m| m.popularity);
            rail(label.1, b.into_iter().take(20).map(|(c, _)| c).collect(), &mut out);
        }
        if has(&base, &|m| m.rating.or(m.recent_rating)) {
            let mut b = base.clone();
            sort_desc(&mut b, |m| m.rating.or(m.recent_rating));
            rail(label.2, b.into_iter().take(20).map(|(c, _)| c).collect(), &mut out);
        }
        out
    };
    let mut all = make(None);
    let movies_only: Vec<Card> = items.iter().filter(|(c, _)| c.kind == MediaType::Movie).map(|(c, _)| c.clone()).collect();
    let series_only: Vec<Card> = items.iter().filter(|(c, _)| c.kind == MediaType::Series).map(|(c, _)| c.clone()).collect();
    rail("Movies for Tonight", movies_only, &mut all);
    rail("Series to Binge", series_only, &mut all);
    if all.is_empty() {
        let list: Vec<Card> = items.iter().map(|(c, _)| c.clone()).collect();
        if !list.is_empty() {
            all.push(("Discover".to_string(), list));
        }
    }
    let mut movies = make(Some(MediaType::Movie));
    let mut series = make(Some(MediaType::Series));
    if movies.is_empty() {
        let l: Vec<Card> = items.iter().filter(|(c, _)| c.kind == MediaType::Movie).map(|(c, _)| c.clone()).collect();
        if !l.is_empty() {
            movies.push(("Movies".to_string(), l));
        }
    }
    if series.is_empty() {
        let l: Vec<Card> = items.iter().filter(|(c, _)| c.kind == MediaType::Series).map(|(c, _)| c.clone()).collect();
        if !l.is_empty() {
            series.push(("Series".to_string(), l));
        }
    }
    Rows { all, movies, series }
}

impl App {
    pub fn new(ctx: Context, be: Backend, rx: Receiver<Msg>, store: Store) -> Self {
        let provider = store.provider_kind();
        let start_page = if store.profiles.len() > 1 && store.ask_profile_on_start {
            Page::Profiles
        } else {
            Page::Home
        };
        let playlists = store.playlists.clone();
        let mut app = Self {
            ctx,
            be,
            rx,
            store,
            page: start_page,
            back: Vec::new(),
            nav_id: 0,
            t: 0.0,
            scroll_y: 0.0,
            provider,
            home: Load::Idle,
            rows: Arc::new(Rows::default()),
            hero: Arc::new(Vec::new()),
            search_text: String::new(),
            last_query: String::new(),
            search: Load::Idle,
            search_token: 0,
            suggestions: Vec::new(),
            sug_token: 0,
            sug_for: String::new(),
            sug_changed: 0.0,
            search_focused: false,
            det: None,
            orders: HashMap::new(),
            ai: AiState {
                input: String::new(),
                token: 0,
                pending: None,
                turns: Vec::new(),
                for_you: Load::Idle,
                for_you_token: 0,
                for_you_profile: usize::MAX,
            },
            tv: Tv {
                channels: Vec::new(),
                loading: 0,
                error: None,
                group: None,
                filter: String::new(),
                url_input: String::new(),
                loaded_urls: Vec::new(),
            },
            dls: Vec::new(),
            dl_next: 1,
            show_dls: false,
            toasts: Vec::new(),
            menu: Menu::None,
            up_next: None,
            now_playing: None,
            auto: None,
            draft: ProfileDraft {
                index: None,
                name: String::new(),
                hue: 200.0,
                family: false,
            },
            imgs: Images::default(),
            acts: Vec::new(),
            token: 1,
            row_targets: HashMap::new(),
            started: 0.0,
            provider_rect: Rect::NOTHING,
            avatar_rect: Rect::NOTHING,
            search_rect: Rect::NOTHING,
            sug_rect: Rect::NOTHING,
            menu_rect: Rect::NOTHING,
            players: None,
            touch: crate::platform::is_touch_platform(),
            form: Form::Regular,
            win_w: 0.0,
            gesture_axis: None,
            #[cfg(test)]
            card_rects: Vec::new(),
        };
        app.home = Load::Loading;
        app.be.load_home();
        for url in playlists {
            app.tv.loading += 1;
            app.be.load_playlist(url);
        }
        app
    }

    pub fn next_token(&mut self) -> u64 {
        self.token += 1;
        self.token
    }

    pub fn toast(&mut self, text: impl Into<String>, error: bool) {
        self.toasts.push(Toast {
            text: text.into(),
            error,
            born: self.t,
        });
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn persist(&self) {
        self.store.save();
    }

    // ------------------------------------------------------------ images

    fn make_tex(&self, name: String, rgba: &[u8], w: usize, h: usize) -> TextureHandle {
        let img = ColorImage::from_rgba_unmultiplied([w, h], rgba);
        self.ctx.load_texture(name, img, TextureOptions::LINEAR)
    }

    pub fn placeholder(&mut self, seed: &str) -> (TextureId, Vec2) {
        if !self.imgs.placeholders.contains_key(seed) {
            let (w, h) = (200usize, 300usize);
            let px = art::placeholder_poster(seed, w, h);
            let tex = self.make_tex(format!("ph:{seed}"), &px, w, h);
            self.imgs.placeholders.insert(seed.to_string(), (tex, Vec2::new(w as f32, h as f32)));
        }
        let (t, s) = &self.imgs.placeholders[seed];
        (t.id(), *s)
    }

    /// The app icon, used as the brand mark in the navigation bar.
    pub fn logo(&mut self) -> Option<(TextureId, Vec2)> {
        if self.imgs.logo.is_none() {
            let bytes: &[u8] = include_bytes!("../assets/icon-256.png");
            let img = image::load_from_memory(bytes).ok()?.to_rgba8();
            let (w, h) = (img.width() as usize, img.height() as usize);
            let tex = self.make_tex("brand-logo".to_string(), img.as_raw(), w, h);
            self.imgs.logo = Some((tex, Vec2::new(w as f32, h as f32)));
        }
        self.imgs.logo.as_ref().map(|(t, s)| (t.id(), *s))
    }

    pub fn aurora(&mut self, seed: &str) -> (TextureId, Vec2) {
        if !self.imgs.aurora.contains_key(seed) {
            let (w, h) = (480usize, 270usize);
            let px = art::aurora(w, h, seed);
            let tex = self.make_tex(format!("au:{seed}"), &px, w, h);
            self.imgs.aurora.insert(seed.to_string(), (tex, Vec2::new(w as f32, h as f32)));
        }
        let (t, s) = &self.imgs.aurora[seed];
        (t.id(), *s)
    }

    /// Poster texture for a card; a generated placeholder until the real one arrives.
    pub fn poster(&mut self, card: &Card) -> (TextureId, Vec2, bool) {
        if let Some(url) = card.poster.clone() {
            let key = format!("p:{url}");
            match self.imgs.posters.get(&key) {
                Some(Slot::Ready(t, s)) => return (t.id(), *s, true),
                Some(Slot::Pending) | Some(Slot::Failed) => {}
                None => {
                    self.imgs.posters.insert(key.clone(), Slot::Pending);
                    self.be.load_image(key, url, false);
                }
            }
        }
        let (id, s) = self.placeholder(&card.key());
        (id, s, false)
    }

    /// High-resolution poster for full-bleed use (phone hero, details).
    pub fn poster_hq(&mut self, card: &Card) -> (TextureId, Vec2, bool) {
        if let Some(url) = card.poster.clone() {
            let key = format!("hq:{url}");
            match self.imgs.posters.get(&key) {
                Some(Slot::Ready(t, s)) => return (t.id(), *s, true),
                Some(Slot::Pending) => {}
                Some(Slot::Failed) => {}
                None => {
                    self.imgs.posters.insert(key.clone(), Slot::Pending);
                    self.be.load_image(key, url.clone(), false);
                }
            }
            // show the small poster while the big one loads
            if let Some(Slot::Ready(t, s)) = self.imgs.posters.get(&format!("p:{url}")) {
                return (t.id(), *s, true);
            }
        }
        let (id, s) = self.placeholder(&card.key());
        (id, s, false)
    }

    pub fn poster_url(&mut self, url: &str, seed: &str) -> (TextureId, Vec2, bool) {
        if !url.trim().is_empty() {
            let key = format!("p:{url}");
            match self.imgs.posters.get(&key) {
                Some(Slot::Ready(t, s)) => return (t.id(), *s, true),
                Some(Slot::Pending) | Some(Slot::Failed) => {}
                None => {
                    self.imgs.posters.insert(key.clone(), Slot::Pending);
                    self.be.load_image(key, url.to_string(), false);
                }
            }
        }
        let (id, s) = self.placeholder(seed);
        (id, s, false)
    }

    /// Soft backdrop for hero/details; an aurora when there is no artwork.
    pub fn backdrop(&mut self, url: Option<&str>, seed: &str) -> (TextureId, Vec2, Option<Color32>) {
        if let Some(url) = url.filter(|u| !u.trim().is_empty()) {
            let key = format!("b:{url}");
            match self.imgs.backdrops.get(&key) {
                Some(Some(b)) => return (b.tex.id(), b.size, Some(b.avg)),
                Some(None) => {}
                None => {
                    self.imgs.backdrops.insert(key.clone(), None);
                    self.be.load_image(key, url.to_string(), true);
                }
            }
        }
        let (id, s) = self.aurora(seed);
        (id, s, None)
    }

    // ------------------------------------------------------------ messages

    pub fn drain(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            self.handle(msg);
        }
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Home(res) => match res {
                Ok((items, metrics)) => {
                    let list: Vec<(Card, BrowseMetrics)> = items
                        .iter()
                        .map(|i| {
                            let m = metrics.get(&i.id.value).copied().unwrap_or_default();
                            (Card::from_item(i, m.rating.or(m.recent_rating)), m)
                        })
                        .collect();
                    let rows = build_rows(&list);
                    let mut hero: Vec<Card> = rows
                        .all
                        .first()
                        .map(|(_, c)| c.iter().filter(|c| c.poster.is_some()).take(6).cloned().collect())
                        .unwrap_or_default();
                    if hero.is_empty() {
                        hero = list.iter().filter(|(c, _)| c.poster.is_some()).take(6).map(|(c, _)| c.clone()).collect();
                    }
                    self.rows = Arc::new(rows);
                    self.hero = Arc::new(hero);
                    self.home = if list.is_empty() {
                        Load::Failed("The catalogue came back empty.".to_string())
                    } else {
                        Load::Ready(())
                    };
                }
                Err(e) => self.home = Load::Failed(e),
            },
            Msg::Search { token, result } => {
                if token == self.search_token {
                    self.search = match result {
                        Ok(items) => Load::Ready(items.iter().map(|i| Card::from_item(i, None)).collect()),
                        Err(e) => Load::Failed(e),
                    };
                }
            }
            Msg::Details { token, result } => match result {
                Ok(d) => {
                    self.orders.insert(d.id.value.clone(), episode_order(&d));
                    let mut request: Option<(ProviderKind, String, usize, usize, u64)> = None;
                    let mut new_token = 0;
                    if self.det.as_ref().is_some_and(|x| x.token == token) {
                        new_token = self.next_token();
                        if let Some(det) = self.det.as_mut() {
                            let (s, e) = if d.is_series() {
                                let order = episode_order(&d);
                                let start = self
                                    .store
                                    .last_progress(&det.card)
                                    .map(|p| (p.season, p.episode))
                                    .filter(|x| order.contains(x))
                                    .or_else(|| order.first().copied())
                                    .unwrap_or((1, 1));
                                start
                            } else {
                                (0, 0)
                            };
                            det.season = s;
                            det.episode = e;
                            det.stream_token = new_token;
                            det.streams = Load::Loading;
                            request = Some((det.card.provider, det.card.id.clone(), s, e, new_token));
                            det.data = Load::Ready(d);
                        }
                    }
                    if let Some((p, id, s, e, t)) = request {
                        let _ = new_token;
                        self.be.streams(p, id, s, e, t);
                    }
                }
                Err(e) => {
                    if let Some(det) = self.det.as_mut().filter(|x| x.token == token) {
                        det.data = Load::Failed(e.clone());
                        det.streams = Load::Failed(e);
                    }
                }
            },
            Msg::Streams {
                token,
                season,
                episode,
                result,
            } => {
                if self.auto.as_ref().is_some_and(|a| a.token == token) {
                    let a = self.auto.take().unwrap();
                    match result {
                        Ok(list) => match pick_default(&list, self.store.quality_cap).cloned() {
                            Some(r) => self.play_now(&a.card, a.season, a.episode, r),
                            None => self.toast("No playable streams were found. Try another provider.", true),
                        },
                        Err(e) => self.toast(format!("Could not load streams: {e}"), true),
                    }
                } else if let Some(det) = self.det.as_mut().filter(|d| d.stream_token == token) {
                    let _ = (season, episode);
                    det.streams = match result {
                        Ok(list) => Load::Ready(list),
                        Err(e) => Load::Failed(e),
                    };
                }
            }
            Msg::Image { key, poster, backdrop } => {
                if let Some(rest) = key.strip_prefix("b:") {
                    let _ = rest;
                    let slot = backdrop.map(|b| {
                        let tex = self.make_tex(key.clone(), &b.rgba, b.w, b.h);
                        BackSlot {
                            tex,
                            size: Vec2::new(b.w as f32, b.h as f32),
                            avg: Color32::from_rgb(b.avg[0], b.avg[1], b.avg[2]),
                        }
                    });
                    self.imgs.backdrops.insert(key, slot);
                } else {
                    let slot = match poster {
                        Some((px, w, h)) => {
                            let tex = self.make_tex(key.clone(), &px, w, h);
                            Slot::Ready(tex, Vec2::new(w as f32, h as f32))
                        }
                        None => Slot::Failed,
                    };
                    self.imgs.posters.insert(key, slot);
                }
            }
            Msg::Channels { url, result } => {
                self.tv.loading = self.tv.loading.saturating_sub(1);
                match result {
                    Ok(list) => {
                        self.tv.error = None;
                        self.tv.loaded_urls.push(url);
                        for c in list {
                            if !self.tv.channels.iter().any(|x| x.stream_url == c.stream_url) {
                                self.tv.channels.push(c);
                            }
                        }
                    }
                    Err(e) => self.tv.error = Some(format!("Could not load playlist: {e}")),
                }
            }
            Msg::Toast(text, error) => self.toast(text, error),
            Msg::Suggest { token, items } => {
                if token == self.sug_token {
                    self.suggestions = items.into_iter().take(6).collect();
                }
            }
            Msg::Ai { token, purpose, result } => match purpose {
                ai::Purpose::Concierge => {
                    if token == self.ai.token {
                        let prompt = self.ai.pending.take().unwrap_or_default();
                        self.ai.turns.push(AiTurn { prompt, result });
                    }
                }
                ai::Purpose::ForYou => {
                    if token == self.ai.for_you_token {
                        self.ai.for_you = match result {
                            Ok(r) => Load::Ready(r),
                            Err(e) => Load::Failed(e),
                        };
                    }
                }
            },
            Msg::Insight { token, result } => {
                if let Some(det) = self.det.as_mut().filter(|d| d.insight_token == token) {
                    det.insight = match result {
                        Ok(i) => Load::Ready(i),
                        Err(e) => Load::Failed(e),
                    };
                }
            }
            Msg::Download { id, label, done, total, state } => {
                if let DlState::Done(path) = &state {
                    self.toast(format!("Downloaded “{label}” to {path}"), false);
                } else if let DlState::Failed(e) = &state {
                    self.toast(e.clone(), true);
                }
                if let Some(d) = self.dls.iter_mut().find(|d| d.id == id) {
                    d.done = done;
                    d.total = total;
                    d.state = state;
                } else {
                    self.dls.push(Dl { id, label, done, total, state });
                }
            }
            Msg::PlayerExited { title } => {
                let _ = title;
                if let Some(np) = self.now_playing.take() {
                    if np.card.is_series() {
                        if let Some(order) = self.orders.get(&np.card.id) {
                            if let Some((s, e)) = next_episode(order, (np.season, np.episode)) {
                                self.up_next = Some(UpNext {
                                    card: np.card,
                                    season: s,
                                    episode: e,
                                    deadline: self.t + 12.0,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ playback

    pub fn play_now(&mut self, card: &Card, season: usize, episode: usize, release: Release) {
        self.store.record_play(card, season, episode);
        self.persist();
        let title = if card.is_series() {
            format!("{} · S{season} E{episode}", card.title)
        } else {
            card.title.clone()
        };
        self.now_playing = Some(NowPlaying {
            card: card.clone(),
            season,
            episode,
        });
        self.up_next = None;
        self.be
            .play_release(card.provider, release, title, self.store.quality_cap);
    }

    fn quick_play_episode(&mut self, card: Card, season: usize, episode: usize) {
        let token = self.next_token();
        self.auto = Some(AutoPlay {
            token,
            card: card.clone(),
            season,
            episode,
        });
        self.toast(format!("Starting “{}”…", card.title), false);
        self.be.streams(card.provider, card.id.clone(), season, episode, token);
        if card.is_series() && !self.orders.contains_key(&card.id) {
            let t = self.next_token();
            self.be.details(card.provider, card.id.clone(), t);
        }
    }

    // ------------------------------------------------------------ navigation

    pub fn go(&mut self, page: Page) {
        if page == self.page {
            return;
        }
        if page.is_tab() || page == Page::Profiles {
            self.back.clear();
        } else {
            self.back.push(self.page);
            if self.back.len() > 24 {
                self.back.remove(0);
            }
        }
        self.page = page;
        self.nav_id += 1;
        self.menu = Menu::None;
    }

    pub fn open_details(&mut self, card: Card) {
        let token = self.next_token();
        self.be.details(card.provider, card.id.clone(), token);
        self.det = Some(Details {
            card,
            token,
            data: Load::Loading,
            season: 0,
            episode: 0,
            streams: Load::Idle,
            stream_token: 0,
            insight: Load::Idle,
            insight_token: 0,
        });
        if self.page == Page::Details {
            self.nav_id += 1;
        } else {
            self.go(Page::Details);
        }
    }

    fn start_search(&mut self, q: String) {
        let q = q.trim().to_string();
        if q.is_empty() {
            return;
        }
        self.store.remember_search(&q);
        self.persist();
        self.search_text = q.clone();
        self.last_query = q.clone();
        self.search = Load::Loading;
        self.search_token = self.next_token();
        self.suggestions.clear();
        self.sug_for = q.clone();
        self.be.search(self.provider, q, self.search_token);
        if self.page == Page::Search {
            self.nav_id += 1;
        } else {
            self.go(Page::Search);
        }
    }

    pub fn ai_context(&self) -> String {
        let p = self.store.profile();
        let watched: Vec<String> = p.history.iter().take(12).map(|h| h.card.title.clone()).collect();
        let saved: Vec<String> = p.my_list.iter().take(12).map(|c| c.title.clone()).collect();
        let mut s = String::new();
        if !watched.is_empty() {
            s.push_str(&format!("Recently watched: {}.\n", watched.join(", ")));
        }
        if !saved.is_empty() {
            s.push_str(&format!("Saved in My List: {}.\n", saved.join(", ")));
        }
        s
    }

    pub fn ai_ready(&self) -> bool {
        !self.store.ai_key.trim().is_empty()
    }

    pub fn maybe_for_you(&mut self) {
        if !self.ai_ready() || !self.be.live {
            return;
        }
        let p = self.store.profile();
        if p.history.is_empty() && p.my_list.is_empty() {
            return;
        }
        if matches!(self.ai.for_you, Load::Loading) {
            return;
        }
        if self.ai.for_you_profile == self.store.active && !matches!(self.ai.for_you, Load::Idle) {
            return;
        }
        self.request_for_you();
    }

    fn request_for_you(&mut self) {
        self.ai.for_you_profile = self.store.active;
        self.ai.for_you = Load::Loading;
        self.ai.for_you_token = self.next_token();
        let family = self.store.profile().family;
        self.be.ai_ask(
            self.ai.for_you_token,
            ai::Purpose::ForYou,
            self.store.ai_key.trim().to_string(),
            self.store.ai_model.clone(),
            "Recommend titles this viewer will love next, based on their taste.".to_string(),
            self.ai_context(),
            self.provider,
            family,
        );
    }

    // ------------------------------------------------------------ actions

    pub fn apply_acts(&mut self) {
        let acts = std::mem::take(&mut self.acts);
        for act in acts {
            self.apply(act);
        }
    }

    fn apply(&mut self, act: Act) {
        match act {
            Act::Go(p) => self.go(p),
            Act::Back => {
                if let Some(p) = self.back.pop() {
                    self.page = p;
                    self.nav_id += 1;
                    self.menu = Menu::None;
                } else if self.page != Page::Home {
                    self.page = Page::Home;
                    self.nav_id += 1;
                }
            }
            Act::OpenDetails(c) => self.open_details(c),
            Act::QuickPlay(c) => {
                if c.is_series() {
                    let (s, e) = self
                        .store
                        .last_progress(&c)
                        .map(|p| (p.season, p.episode))
                        .unwrap_or((1, 1));
                    self.quick_play_episode(c, s, e);
                } else {
                    self.quick_play_episode(c, 0, 0);
                }
            }
            Act::PlayRelease(r) => {
                if let Some(det) = self.det.as_ref() {
                    let (card, s, e) = (det.card.clone(), det.season, det.episode);
                    self.play_now(&card, s, e, r);
                }
            }
            Act::DownloadRelease(r) => {
                if let Some(det) = self.det.as_ref() {
                    let title = if det.card.is_series() {
                        format!("{} S{:02}E{:02}", det.card.title, det.season, det.episode)
                    } else {
                        det.card.title.clone()
                    };
                    let id = self.dl_next;
                    self.dl_next += 1;
                    self.dls.push(Dl {
                        id,
                        label: title.clone(),
                        done: 0,
                        total: None,
                        state: DlState::Running,
                    });
                    self.show_dls = true;
                    self.be.download(det.card.provider, r, title, id);
                }
            }
            Act::ToggleList(c) => {
                let added = self.store.toggle_list(&c);
                self.persist();
                self.toast(
                    if added {
                        format!("Added “{}” to My List", c.title)
                    } else {
                        format!("Removed “{}” from My List", c.title)
                    },
                    false,
                );
            }
            Act::Search(q) => self.start_search(q),
            Act::SetProvider(p) => {
                self.provider = p;
                self.store.provider = Some(p.cache_key().to_string());
                self.persist();
                self.menu = Menu::None;
                if self.page == Page::Search && !self.last_query.is_empty() {
                    let q = self.last_query.clone();
                    self.start_search(q);
                }
                self.toast(format!("Provider: {}", p.label()), false);
            }
            Act::PickSeason(s) => {
                let first = self
                    .det
                    .as_ref()
                    .and_then(|d| d.data.ready())
                    .and_then(|d| d.seasons.iter().find(|x| x.number == s))
                    .and_then(|x| x.episodes.first().map(|e| e.number));
                if let Some(e) = first {
                    self.pick_episode(s, e);
                }
            }
            Act::PickEpisode(s, e) => self.pick_episode(s, e),
            Act::AskAi(q) => {
                let q = q.trim().to_string();
                if q.is_empty() || self.ai.pending.is_some() {
                    return;
                }
                if !self.ai_ready() {
                    self.toast("Add your AI key in Settings to use the AI concierge.", true);
                    return;
                }
                self.ai.token = self.next_token();
                self.ai.pending = Some(q.clone());
                self.ai.input.clear();
                let family = self.store.profile().family;
                self.be.ai_ask(
                    self.ai.token,
                    ai::Purpose::Concierge,
                    self.store.ai_key.trim().to_string(),
                    self.store.ai_model.clone(),
                    q,
                    self.ai_context(),
                    self.provider,
                    family,
                );
            }
            Act::RefreshForYou => {
                if self.ai_ready() {
                    self.request_for_you();
                } else {
                    self.toast("Add your AI key in Settings to get personal picks.", true);
                }
            }
            Act::Insight => {
                if !self.ai_ready() {
                    self.toast("Add your AI key in Settings to use AI insights.", true);
                    return;
                }
                let token = self.next_token();
                let (key, model) = (self.store.ai_key.trim().to_string(), self.store.ai_model.clone());
                if let Some(det) = self.det.as_mut() {
                    det.insight_token = token;
                    det.insight = Load::Loading;
                    let (genres, desc) = det
                        .data
                        .ready()
                        .map(|d| (d.genres.join(", "), d.description.clone().unwrap_or_default()))
                        .unwrap_or_default();
                    self.be.ai_insight(
                        token,
                        key,
                        model,
                        det.card.title.clone(),
                        det.card.year.clone(),
                        genres,
                        desc,
                    );
                }
            }
            Act::SwitchProfile(i) => {
                if i < self.store.profiles.len() {
                    self.store.active = i;
                    self.persist();
                    self.menu = Menu::None;
                    self.ai.turns.clear();
                    self.ai.for_you = Load::Idle;
                    self.page = Page::Home;
                    self.back.clear();
                    self.nav_id += 1;
                }
            }
            Act::EditProfile(idx) => {
                self.draft = match idx.and_then(|i| self.store.profiles.get(i).map(|p| (i, p))) {
                    Some((i, p)) => ProfileDraft {
                        index: Some(i),
                        name: p.name.clone(),
                        hue: p.hue,
                        family: p.family,
                    },
                    None => ProfileDraft {
                        index: None,
                        name: String::new(),
                        hue: (self.store.profiles.len() as f32 * 67.0 + 20.0) % 360.0,
                        family: false,
                    },
                };
                self.menu = Menu::None;
                self.go(Page::ProfileEdit);
            }
            Act::SaveProfile => {
                let name = self.draft.name.trim().to_string();
                if name.is_empty() {
                    self.toast("Give the profile a name.", true);
                    return;
                }
                match self.draft.index {
                    Some(i) if i < self.store.profiles.len() => {
                        let p = &mut self.store.profiles[i];
                        p.name = name;
                        p.hue = self.draft.hue;
                        p.family = self.draft.family;
                    }
                    _ => {
                        if self.store.profiles.len() >= 5 {
                            self.toast("You can have up to five profiles.", true);
                            return;
                        }
                        let mut p = Profile::new(&name, self.draft.hue);
                        p.family = self.draft.family;
                        self.store.profiles.push(p);
                    }
                }
                self.persist();
                self.back.pop();
                self.page = if self.store.profiles.len() > 1 { Page::Profiles } else { Page::Settings };
                self.nav_id += 1;
            }
            Act::DeleteProfile(i) => {
                if self.store.profiles.len() > 1 && i < self.store.profiles.len() {
                    self.store.profiles.remove(i);
                    if self.store.active >= self.store.profiles.len() {
                        self.store.active = 0;
                    }
                    self.persist();
                    self.back.clear();
                    self.page = Page::Settings;
                    self.nav_id += 1;
                } else {
                    self.toast("At least one profile is required.", true);
                }
            }
            Act::AddPlaylist(url) => {
                let url = url.trim().to_string();
                if url.is_empty() {
                    return;
                }
                if self.store.playlists.contains(&url) {
                    self.toast("That playlist is already added.", true);
                    return;
                }
                self.store.playlists.push(url.clone());
                self.persist();
                self.tv.loading += 1;
                self.tv.url_input.clear();
                self.be.load_playlist(url);
            }
            Act::RemovePlaylist(i) => {
                if i < self.store.playlists.len() {
                    self.store.playlists.remove(i);
                    self.persist();
                    self.tv.channels.clear();
                    self.tv.loaded_urls.clear();
                    for url in self.store.playlists.clone() {
                        self.tv.loading += 1;
                        self.be.load_playlist(url);
                    }
                }
            }
            Act::PlayChannel(ch) => {
                self.toast(format!("Tuning to {}…", ch.name), false);
                self.be.play_url(self.provider, ch.stream_url.clone(), ch.name.clone());
            }
            Act::RetryHome => {
                self.home = Load::Loading;
                self.be.load_home();
            }
            Act::Persist => self.persist(),
            Act::PlayNext => {
                if let Some(u) = self.up_next.take() {
                    self.quick_play_episode(u.card, u.season, u.episode);
                }
            }
            Act::DismissUpNext => self.up_next = None,
            Act::SetMenu(m) => self.menu = if self.menu == m { Menu::None } else { m },
            Act::CopyText(s) => {
                crate::platform::copy_text(&s);
                self.ctx.copy_text(s);
                self.toast("Copied to clipboard", false);
            }
            Act::ToggleDownloads => self.show_dls = !self.show_dls,
        }
    }

    fn pick_episode(&mut self, s: usize, e: usize) {
        let token = self.next_token();
        if let Some(det) = self.det.as_mut() {
            det.season = s;
            det.episode = e;
            det.streams = Load::Loading;
            det.stream_token = token;
            self.be.streams(det.card.provider, det.card.id.clone(), s, e, token);
        }
    }

    // ------------------------------------------------------------ per-frame upkeep

    pub fn compact(&self) -> bool {
        self.form == Form::Compact
    }

    /// The text behind a named input field (used by the native text dialog).
    fn field_string(&mut self, key: &str) -> Option<&mut String> {
        Some(match key {
            "search" => &mut self.search_text,
            "ai_in" => &mut self.ai.input,
            "tv_url" => &mut self.tv.url_input,
            "tv_filter" => &mut self.tv.filter,
            "ai_key_in" => &mut self.store.ai_key,
            "ai_model_in" => &mut self.store.ai_model,
            "pe_name" => &mut self.draft.name,
            _ => return None,
        })
    }

    pub fn field_value(&mut self, key: &str) -> String {
        self.field_string(key).map(|s| s.clone()).unwrap_or_default()
    }

    pub fn set_field(&mut self, key: &str, value: String) {
        let value = if key == "pe_name" { value.chars().take(18).collect() } else { value };
        if let Some(s) = self.field_string(key) {
            *s = value;
        }
    }

    fn apply_text_result(&mut self, r: crate::platform::TextResult) {
        self.set_field(&r.id, r.text.clone());
        match r.id.as_str() {
            "search" if r.submitted => self.acts.push(Act::Search(r.text)),
            "ai_in" if r.submitted => self.acts.push(Act::AskAi(r.text)),
            "tv_url" if r.submitted => self.acts.push(Act::AddPlaylist(r.text)),
            "ai_key_in" | "ai_model_in" => self.acts.push(Act::Persist),
            _ => {}
        }
    }

    pub fn tick(&mut self, now: f64) {
        self.t = now;
        #[cfg(test)]
        self.card_rects.clear();
        for r in crate::platform::poll_text() {
            self.apply_text_result(r);
        }
        if crate::platform::take_player_closed() {
            self.handle(Msg::PlayerExited { title: String::new() });
        }
        if self.started == 0.0 {
            self.started = now;
        }
        self.toasts.retain(|t| now - t.born < 5.0);
        if let Some(u) = &self.up_next {
            if now >= u.deadline {
                self.acts.push(Act::PlayNext);
            }
        }
        if self.page == Page::Home {
            self.maybe_for_you();
        }
    }

    /// Whether something on screen is animating and needs steady repaints.
    pub fn animating(&self) -> Option<Duration> {
        let busy = self.home.is_loading()
            || self.search.is_loading()
            || self.ai.pending.is_some()
            || self.ai.for_you.is_loading()
            || self.tv.loading > 0
            || self.up_next.is_some()
            || !self.toasts.is_empty()
            || self.dls.iter().any(|d| d.state == DlState::Running)
            || self.det.as_ref().is_some_and(|d| {
                d.data.is_loading() || d.streams.is_loading() || d.insight.is_loading()
            })
            || self.imgs.posters.values().any(|s| matches!(s, Slot::Pending));
        if busy {
            Some(Duration::from_millis(33))
        } else if self.page == Page::Home && self.hero.len() > 1 {
            Some(Duration::from_millis(500))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moviebox_tui::providers::SourceMirror;

    fn rel(q: &str) -> Release {
        Release {
            provider: ProviderKind::MovieBox,
            filename: format!("file-{q}"),
            quality: Some(q.to_string()),
            codec: None,
            language: None,
            size_bytes: None,
            season: None,
            episode: None,
            mirrors: vec![SourceMirror {
                label: "m".into(),
                resolver_url: "https://example.com/a.mp4".into(),
                headers: vec![],
                direct_file: true,
            }],
            resource_id: None,
        }
    }

    #[test]
    fn next_episode_walks_across_seasons() {
        let order = vec![(1, 1), (1, 2), (2, 1)];
        assert_eq!(next_episode(&order, (1, 1)), Some((1, 2)));
        assert_eq!(next_episode(&order, (1, 2)), Some((2, 1)));
        assert_eq!(next_episode(&order, (2, 1)), None);
        assert_eq!(next_episode(&order, (9, 9)), None);
    }

    #[test]
    fn default_release_respects_cap() {
        let list = vec![rel("480p"), rel("2160p"), rel("1080p"), rel("720p")];
        assert_eq!(pick_default(&list, 0).unwrap().quality.as_deref(), Some("1080p"));
        assert_eq!(pick_default(&list, 720).unwrap().quality.as_deref(), Some("720p"));
        assert_eq!(pick_default(&list, 360).unwrap().quality.as_deref(), Some("480p"));
        assert!(pick_default(&[], 0).is_none());
    }

    #[test]
    fn bytes_are_human_readable() {
        assert_eq!(fmt_bytes(512), "512 B");
        assert_eq!(fmt_bytes(1536), "1.5 KB");
        assert_eq!(fmt_bytes(5 * 1024 * 1024 * 1024), "5.0 GB");
    }
}
