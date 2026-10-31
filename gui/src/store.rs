//! Persistent user data: profiles, My List, watch history and app settings.
//! Stored as JSON next to the app's other config files.

use moviebox_tui::providers::{CatalogItem, MediaType, ProviderKind};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Card {
    pub provider: ProviderKind,
    pub id: String,
    pub title: String,
    pub year: String,
    pub kind: MediaType,
    pub poster: Option<String>,
    pub rating: Option<f64>,
}

impl Card {
    pub fn from_item(item: &CatalogItem, rating: Option<f64>) -> Self {
        Self {
            provider: item.id.provider,
            id: item.id.value.clone(),
            title: item.title.clone(),
            year: item.year.clone().unwrap_or_default(),
            kind: item.media_type,
            poster: item.poster_url.clone().filter(|u| !u.trim().is_empty()),
            rating,
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.provider.cache_key(), self.id)
    }

    pub fn is_series(&self) -> bool {
        self.kind == MediaType::Series
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Progress {
    pub card: Card,
    pub season: usize,
    pub episode: usize,
    pub ts: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub hue: f32,
    #[serde(default)]
    pub family: bool,
    #[serde(default)]
    pub my_list: Vec<Card>,
    #[serde(default)]
    pub history: Vec<Progress>,
    #[serde(default)]
    pub recent_searches: Vec<String>,
}

impl Profile {
    pub fn new(name: &str, hue: f32) -> Self {
        Self {
            name: name.to_string(),
            hue,
            family: false,
            my_list: Vec::new(),
            history: Vec::new(),
            recent_searches: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Store {
    pub profiles: Vec<Profile>,
    pub active: usize,
    pub provider: Option<String>,
    pub playlists: Vec<String>,
    /// 0 = automatic, otherwise the maximum playback height (e.g. 720).
    pub quality_cap: u32,
    pub ai_key: String,
    pub ai_model: String,
    pub ask_profile_on_start: bool,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            profiles: vec![Profile::new("You", 350.0)],
            active: 0,
            provider: None,
            playlists: Vec::new(),
            quality_cap: 0,
            ai_key: String::new(),
            ai_model: "claude-haiku-5-5".to_string(),
            ask_profile_on_start: true,
        }
    }
}

fn path() -> Option<PathBuf> {
    moviebox_tui::config::config_dir().map(|d| d.join("gui_store.json"))
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Store {
    pub fn load() -> Self {
        let mut s: Store = path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        if s.profiles.is_empty() {
            s.profiles.push(Profile::new("You", 350.0));
        }
        if s.active >= s.profiles.len() {
            s.active = 0;
        }
        if s.ai_model.trim().is_empty() {
            s.ai_model = "claude-haiku-5-5".to_string();
        }
        s
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &p);
            }
        }
    }

    pub fn profile(&self) -> &Profile {
        &self.profiles[self.active.min(self.profiles.len() - 1)]
    }

    pub fn profile_mut(&mut self) -> &mut Profile {
        let i = self.active.min(self.profiles.len() - 1);
        &mut self.profiles[i]
    }

    pub fn in_list(&self, card: &Card) -> bool {
        self.profile().my_list.iter().any(|c| c.key() == card.key())
    }

    /// Returns true when the card was added, false when removed.
    pub fn toggle_list(&mut self, card: &Card) -> bool {
        let key = card.key();
        let p = self.profile_mut();
        if let Some(i) = p.my_list.iter().position(|c| c.key() == key) {
            p.my_list.remove(i);
            false
        } else {
            p.my_list.insert(0, card.clone());
            true
        }
    }

    pub fn record_play(&mut self, card: &Card, season: usize, episode: usize) {
        let key = card.key();
        let p = self.profile_mut();
        p.history.retain(|h| h.card.key() != key);
        p.history.insert(
            0,
            Progress {
                card: card.clone(),
                season,
                episode,
                ts: now(),
            },
        );
        p.history.truncate(40);
    }

    pub fn last_progress(&self, card: &Card) -> Option<&Progress> {
        let key = card.key();
        self.profile().history.iter().find(|h| h.card.key() == key)
    }

    pub fn remember_search(&mut self, q: &str) {
        let q = q.trim();
        if q.is_empty() {
            return;
        }
        let p = self.profile_mut();
        p.recent_searches.retain(|s| !s.eq_ignore_ascii_case(q));
        p.recent_searches.insert(0, q.to_string());
        p.recent_searches.truncate(8);
    }

    pub fn provider_kind(&self) -> ProviderKind {
        self.provider
            .as_deref()
            .and_then(ProviderKind::parse)
            .filter(|p| ProviderKind::ENABLED.contains(p))
            .unwrap_or_default()
    }
}
