//! Offline demo data for the `--screenshot` developer mode (no network needed).

use crate::ai;
use crate::app::*;
use crate::store::*;
use moviebox_tui::providers::tv::models::Channel;
use moviebox_tui::providers::{
    Episode, MediaDetails, MediaType, ProviderKind, ProviderMediaId, Release, Season, SourceMirror,
};
use std::sync::Arc;

const TITLES: [&str; 20] = [
    "Neon Harbor", "The Last Lighthouse", "Crimson Tide Rising", "Midnight Orbit", "Paper Cities",
    "Echoes of Aurora", "Velvet Static", "Glass Kingdom", "Salt & Ember", "The Quiet Frontier",
    "Atlas Burning", "Northern Lights Club", "Parallel Summer", "Iron Lullaby", "Moon Over Marrakesh",
    "Sixth Street Saints", "Wildfire Season", "Cold Harbor Nights", "The Cartographer", "Brightwater",
];

fn cards() -> Vec<Card> {
    TITLES
        .iter()
        .enumerate()
        .map(|(i, t)| Card {
            provider: ProviderKind::MovieBox,
            id: format!("demo{i}"),
            title: t.to_string(),
            year: format!("{}", 2017 + (i % 9)),
            kind: if i % 3 == 1 { MediaType::Series } else { MediaType::Movie },
            poster: None,
            rating: Some(6.1 + ((i * 37) % 30) as f64 / 10.0),
        })
        .collect()
}

fn release(q: &str, size_mb: u64, codec: &str) -> Release {
    Release {
        provider: ProviderKind::MovieBox,
        filename: format!("Neon.Harbor.S01E02.{q}.WEB-DL.{codec}.mkv"),
        quality: Some(q.to_string()),
        codec: Some(codec.to_string()),
        language: Some("English".to_string()),
        size_bytes: Some(size_mb * 1024 * 1024),
        season: Some(1),
        episode: Some(2),
        mirrors: vec![SourceMirror {
            label: "Server A".into(),
            resolver_url: "https://example.com/video.mp4".into(),
            headers: vec![],
            direct_file: true,
        }],
        resource_id: None,
    }
}

pub fn seed(app: &mut App, page: &str) {
    let all = cards();
    let movies: Vec<Card> = all.iter().filter(|c| !c.is_series()).cloned().collect();
    let series: Vec<Card> = all.iter().filter(|c| c.is_series()).cloned().collect();
    app.rows = Arc::new(Rows {
        all: vec![
            ("Trending Now".into(), all[..14].to_vec()),
            ("Popular Right Now".into(), all.iter().rev().take(14).cloned().collect()),
            ("Critically Acclaimed".into(), all[4..18].to_vec()),
        ],
        movies: vec![("Trending Movies".into(), movies.clone()), ("Top Rated Movies".into(), movies.iter().rev().cloned().collect())],
        series: vec![("Trending Series".into(), series.clone()), ("Binge-worthy".into(), series.iter().rev().cloned().collect())],
    });
    app.hero = Arc::new(all[..5].to_vec());
    app.home = Load::Ready(());

    {
        let p = app.store.profile_mut();
        p.name = "Rohan".into();
        p.hue = 350.0;
        p.my_list = all[6..11].to_vec();
        p.history = [3usize, 1, 8, 12]
            .iter()
            .map(|&i| Progress { card: all[i].clone(), season: 1 + i % 3, episode: 2 + i % 4, ts: 0 })
            .collect();
    }
    app.store.profiles.push(Profile::new("Guest", 200.0));
    app.ai.for_you = Load::Ready(ai::Reply {
        text: "Based on what you've been watching.".into(),
        picks: all[2..9]
            .iter()
            .map(|c| ai::Pick { title: c.title.clone(), why: "Moody, character-driven and beautifully shot.".into(), card: Some(c.clone()) })
            .collect(),
    });
    app.ai.for_you_profile = app.store.active;
    app.store.ai_key = "demo".into();

    app.search_text = String::new();
    match page {
        "profiles" => app.page = Page::Profiles,
        "edit" => {
            app.draft.name = "Sam".into();
            app.page = Page::ProfileEdit;
        }
        "movies" => app.page = Page::Movies,
        "series" => app.page = Page::Series,
        "mylist" => app.page = Page::MyList,
        "search" => {
            app.last_query = "neon".into();
            app.search = Load::Ready(all.clone());
            app.page = Page::Search;
        }
        "settings" => app.page = Page::Settings,
        "tv" => {
            app.tv.channels = (0..36)
                .map(|i| Channel {
                    id: format!("c{i}"),
                    name: format!("{} {}", ["Global News", "Sports One", "Cinema Gold", "Kids Zone", "Music Live", "Nature HD"][i % 6], i / 6 + 1),
                    logo: String::new(),
                    group: ["News", "Sports", "Movies", "Kids", "Music", "Documentary"][i % 6].to_string(),
                    stream_url: format!("http://example.com/{i}.m3u8"),
                })
                .collect();
            app.store.playlists = vec!["https://example.com/playlist.m3u".into()];
            app.page = Page::Tv;
        }
        "ai" => {
            let mut picks: Vec<ai::Pick> = all[3..8]
                .iter()
                .map(|c| ai::Pick { title: c.title.clone(), why: "Slow-burn mystery with a stunning score.".into(), card: Some(c.clone()) })
                .collect();
            picks.push(ai::Pick { title: "Some Unlisted Title".into(), why: String::new(), card: None });
            app.ai.turns.push(AiTurn {
                prompt: "Something cozy but mysterious for a rainy night".into(),
                result: Ok(ai::Reply { text: "Here are atmospheric picks that keep you guessing without getting heavy.".into(), picks }),
            });
            app.page = Page::Ai;
        }
        "details" | "details-ai" => {
            let id = ProviderMediaId { provider: ProviderKind::MovieBox, value: "demo1".into() };
            let seasons = vec![
                Season { number: 1, episodes: (1..=8).map(|n| Episode { season: 1, number: n, title: Some(["Arrival", "The Signal", "Low Tide", "Static", "Undertow", "Harbor Lights", "Blackout", "Dawn"][n - 1].to_string()), overview: Some("A quiet port town starts picking up a signal nobody can explain, and old friends are forced to trust each other again.".into()) }).collect() },
                Season { number: 2, episodes: (1..=6).map(|n| Episode { season: 2, number: n, title: None, overview: None }).collect() },
            ];
            let data = MediaDetails {
                id,
                title: "Neon Harbor".into(),
                media_type: MediaType::Series,
                year: Some("2024".into()),
                description: Some("When a dormant radio tower hums back to life in a fading coastal town, a salvage diver and a disgraced engineer follow the signal into the bay. What waits below could rewrite everything the town believes about its own past.".into()),
                tagline: None,
                imdb_rating: Some("8.4".into()),
                director: Some("Mira Okonkwo".into()),
                stars: Some("Lena Vargas, Idris Hale, Tomás Reyes".into()),
                prints: None,
                audios: None,
                poster_url: None,
                duration: Some("48m".into()),
                genres: vec!["Mystery".into(), "Sci-Fi".into(), "Drama".into()],
                seasons,
                dubs: vec![],
            };
            app.orders.insert("demo1".into(), episode_order(&data));
            app.det = Some(Details {
                card: all[1].clone(),
                token: 1,
                data: Load::Ready(data),
                season: 1,
                episode: 2,
                streams: Load::Ready(vec![release("2160p", 5200, "HEVC"), release("1080p", 2100, "H.264"), release("720p", 900, "H.264"), release("480p", 420, "H.264")]),
                stream_token: 1,
                insight: if page == "details-ai" {
                    Load::Ready(ai::Insight {
                        pitch: "A patient, atmospheric mystery where the sea feels like a character, and every answer opens a stranger question.".into(),
                        watch_if: vec!["You like slow-burn sci-fi".into(), "You enjoyed small-town secrets".into()],
                        mood: vec!["Moody".into(), "Haunting".into(), "Cerebral".into()],
                        note: "Slow first episode, big payoff.".into(),
                    })
                } else {
                    Load::Idle
                },
                insight_token: 0,
            });
            app.page = Page::Details;
        }
        _ => app.page = Page::Home,
    }
    app.toast("Added “Paper Cities” to My List", false);
}
