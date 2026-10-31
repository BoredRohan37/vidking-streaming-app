//! Backend: runs network / player / download work on a tokio runtime and reports back to
//! the UI thread through a channel. Everything here reuses the `moviebox_tui` library.

use crate::art;
use moviebox_tui::models::BrowseMetrics;
use moviebox_tui::providers::tv::models::Channel;
use moviebox_tui::providers::tv::parser::M3UParser;
use moviebox_tui::providers::{
    CatalogItem, MediaDetails, PlaybackSource, ProviderKind, Release, ReleaseProvider,
    ResolutionIntent,
};
use moviebox_tui::service::MovieBoxService;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

pub type HomeData = (Vec<CatalogItem>, HashMap<String, BrowseMetrics>);

#[derive(Debug, Clone, PartialEq)]
pub enum DlState {
    Running,
    Done(String),
    Failed(String),
}

#[allow(clippy::large_enum_variant)]
pub enum Msg {
    Home(Result<HomeData, String>),
    Search {
        token: u64,
        result: Result<Vec<CatalogItem>, String>,
    },
    Details {
        token: u64,
        result: Result<MediaDetails, String>,
    },
    Streams {
        token: u64,
        season: usize,
        episode: usize,
        result: Result<Vec<Release>, String>,
    },
    Image {
        key: String,
        poster: Option<(Vec<u8>, usize, usize)>,
        backdrop: Option<art::Backdrop>,
    },
    Channels {
        url: String,
        result: Result<Vec<Channel>, String>,
    },
    Toast(String, bool),
    Suggest {
        token: u64,
        items: Vec<String>,
    },
    Ai {
        token: u64,
        purpose: crate::ai::Purpose,
        result: Result<crate::ai::Reply, String>,
    },
    Insight {
        token: u64,
        result: Result<crate::ai::Insight, String>,
    },
    PlayerExited {
        title: String,
    },
    Download {
        id: u64,
        label: String,
        done: u64,
        total: Option<u64>,
        state: DlState,
    },
}

#[derive(Clone)]
pub struct Backend {
    pub(crate) rt: Arc<tokio::runtime::Runtime>,
    pub(crate) svc: Arc<MovieBoxService>,
    pub(crate) tx: Sender<Msg>,
    pub(crate) wake: Arc<dyn Fn() + Send + Sync>,
    /// When false (screenshot/demo mode) no network or player calls are made.
    pub live: bool,
}

impl Backend {
    pub fn new(tx: Sender<Msg>, wake: Arc<dyn Fn() + Send + Sync>, live: bool) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("moviebox-gui-io")
            .enable_all()
            .build()
            .expect("failed to start the async runtime");
        let svc = {
            let _guard = rt.enter();
            MovieBoxService::new()
        };
        Self {
            rt: Arc::new(rt),
            svc: Arc::new(svc),
            tx,
            wake,
            live,
        }
    }

    pub(crate) fn send(&self, m: Msg) {
        let _ = self.tx.send(m);
        (self.wake)();
    }

    pub fn toast(&self, text: impl Into<String>, error: bool) {
        self.send(Msg::Toast(text.into(), error));
    }

    // ---------------------------------------------------------------- browse

    pub fn load_home(&self) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let r = me.svc.homepage("2", 1).await;
            me.send(Msg::Home(r));
        });
    }

    pub fn search(&self, provider: ProviderKind, query: String, token: u64) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let r = me
                .svc
                .search_typed(provider, &query, 1)
                .await
                .map_err(|e| e.to_string());
            me.send(Msg::Search { token, result: r });
        });
    }

    pub fn details(&self, provider: ProviderKind, id: String, token: u64) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let r = me
                .svc
                .details_typed(provider, &id)
                .await
                .map_err(|e| e.to_string());
            me.send(Msg::Details { token, result: r });
        });
    }

    pub fn streams(
        &self,
        provider: ProviderKind,
        id: String,
        season: usize,
        episode: usize,
        token: u64,
    ) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let svc = &me.svc;
            let result = match provider {
                ProviderKind::MovieBox | ProviderKind::Addons => {
                    ReleaseProvider::episode_streams(&svc.client, &id, season, episode).await
                }
                ProviderKind::FourKHdHub => match svc.fourk_client.as_ref() {
                    Some(c) => ReleaseProvider::episode_streams(c, &id, season, episode).await,
                    None => Err(moviebox_tui::providers::ProviderError::Unavailable(
                        "4KHDHub is unavailable".to_string(),
                    )),
                },
                ProviderKind::Dramachi => {
                    ReleaseProvider::episode_streams(&svc.dramachi_client, &id, season, episode)
                        .await
                }
                ProviderKind::BdixCircleFtp => {
                    ReleaseProvider::episode_streams(&svc.circleftp_client, &id, season, episode)
                        .await
                }
                ProviderKind::BdixDhakaFlix => {
                    ReleaseProvider::episode_streams(&svc.dhakaflix_client, &id, season, episode)
                        .await
                }
            }
            .map_err(|e| e.to_string());
            me.send(Msg::Streams {
                token,
                season,
                episode,
                result,
            });
        });
    }

    // ---------------------------------------------------------------- images

    pub fn load_image(&self, key: String, url: String, want_backdrop: bool) {
        if !self.live {
            return;
        }
        let me = self.clone();
        let hq = key.starts_with("hq:");
        self.rt.spawn(async move {
            let id = format!("{:016x}", art::hash(&url));
            let img = me
                .svc
                .fetch_and_decode_cached_poster("gui", &id, &url)
                .await;
            let Some(img) = img else {
                me.send(Msg::Image {
                    key,
                    poster: None,
                    backdrop: None,
                });
                return;
            };
            let built = tokio::task::spawn_blocking(move || {
                let (mw, mh) = if hq { (720, 1080) } else { (320, 480) };
                let (px, w, h) = art::fit_rgba(&img, mw, mh);
                let backdrop = if want_backdrop {
                    art::backdrop_from_poster(&px, w, h)
                } else {
                    None
                };
                (px, w, h, backdrop)
            })
            .await;
            match built {
                Ok((px, w, h, backdrop)) => me.send(Msg::Image {
                    key,
                    poster: Some((px, w, h)),
                    backdrop,
                }),
                Err(_) => me.send(Msg::Image {
                    key,
                    poster: None,
                    backdrop: None,
                }),
            }
        });
    }

    // ---------------------------------------------------------------- playback

    /// Resolve a release into a playable source and open it in mpv/VLC.
    pub fn play_release(&self, provider: ProviderKind, release: Release, title: String, cap: u32) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            me.toast(format!("Preparing “{title}”…"), false);
            match me.resolve(provider, &release, ResolutionIntent::Playback).await {
                Ok(mut src) => {
                    if cap > 0 {
                        let cap = cap as u64;
                        src.max_height = Some(src.max_height.map_or(cap, |h| h.min(cap)));
                    }
                    me.launch(src, &title).await
                }
                Err(e) => me.toast(e, true),
            }
        });
    }

    pub fn play_url(&self, provider: ProviderKind, url: String, title: String) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let src = PlaybackSource {
                provider,
                url,
                headers: Vec::new(),
                subtitle: None,
                source_label: title.clone(),
                max_height: None,
            };
            me.launch(src, &title).await;
        });
    }

    async fn resolve(
        &self,
        provider: ProviderKind,
        release: &Release,
        intent: ResolutionIntent,
    ) -> Result<PlaybackSource, String> {
        let Some(mirror) = release.mirrors.first().cloned() else {
            return Err("No playable source was found for this stream.".to_string());
        };
        if provider == ProviderKind::FourKHdHub {
            let Some(client) = self.svc.fourk_client.clone() else {
                return Err("4KHDHub is unavailable right now.".to_string());
            };
            return match tokio::time::timeout(
                Duration::from_secs(18),
                client.resolve_release(release, intent),
            )
            .await
            {
                Ok(Ok(src)) => Ok(src),
                Ok(Err(e)) => Err(format!("4KHDHub: {e}")),
                Err(_) => Err("4KHDHub: timed out.".to_string()),
            };
        }
        Ok(PlaybackSource {
            provider,
            url: mirror.resolver_url,
            headers: mirror.headers,
            subtitle: None,
            source_label: mirror.label,
            max_height: Some(release.resolution_u64()).filter(|&h| h > 0),
        })
    }

    /// Open a resolved source: the in-app player on Android, mpv/VLC on desktop.
    async fn launch(&self, src: PlaybackSource, title: &str) {
        #[cfg(target_os = "android")]
        {
            match crate::platform::play_video(&src.url, &src.headers, title) {
                Ok(()) => self.toast(format!("Playing “{title}”"), false),
                Err(e) => self.toast(e, true),
            }
        }
        #[cfg(not(target_os = "android"))]
        self.launch_desktop(src, title).await;
    }

    #[cfg(not(target_os = "android"))]
    async fn launch_desktop(&self, src: PlaybackSource, title: &str) {
        let title = title.to_string();
        let me = self.clone();
        let me2 = self.clone();
        let title2 = title.clone();
        let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
            let kinds = moviebox_tui::player::detect();
            let kind = kinds
                .iter()
                .copied()
                .find(|k| moviebox_tui::player::supports_headers(*k, &src.headers))
                .or_else(|| kinds.first().copied())
                .ok_or_else(|| {
                    "No video player found. Put mpv.exe in the \"tools\" folder next to the app."
                        .to_string()
                })?;
            let mut cmd = moviebox_tui::player::command(
                kind,
                &src.url,
                src.subtitle.as_deref(),
                &src.headers,
                None,
                None,
                None,
                src.max_height,
            );
            moviebox_tui::player::configure_detached_process(&mut cmd);
            cmd.stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            let child = cmd
                .spawn()
                .map_err(|e| format!("Could not start {}: {e}", kind.label()))?;
            let watcher = me2.clone();
            let watched_title = title2.clone();
            std::thread::spawn(move || {
                let mut child = child;
                let _ = child.wait();
                watcher.send(Msg::PlayerExited {
                    title: watched_title,
                });
            });
            Ok(kind.label().to_string())
        })
        .await
        .unwrap_or_else(|e| Err(format!("Player task failed: {e}")));
        match result {
            Ok(label) => me.toast(format!("Playing “{title}” in {label}"), false),
            Err(e) => me.toast(e, true),
        }
    }

    // ---------------------------------------------------------------- downloads

    pub fn download(&self, provider: ProviderKind, release: Release, title: String, id: u64) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let label = title.clone();
            let fail = |msg: String| {
                me.send(Msg::Download {
                    id,
                    label: label.clone(),
                    done: 0,
                    total: None,
                    state: DlState::Failed(msg),
                });
            };
            let src = match me.resolve(provider, &release, ResolutionIntent::Download).await {
                Ok(s) => s,
                Err(e) => return fail(e),
            };
            if moviebox_tui::player::is_dash_url(&src.url) {
                return fail(
                    "This stream is adaptive (DASH). Play it instead, or use the terminal app for yt-dlp downloads."
                        .to_string(),
                );
            }
            let dir = moviebox_tui::service::resolve_download_dir(None);
            if let Err(e) = tokio::fs::create_dir_all(&dir).await {
                return fail(format!("Cannot create download folder: {e}"));
            }
            let ext = url_extension(&src.url).unwrap_or("mp4");
            let quality = release.quality.clone().unwrap_or_default();
            let mut name = sanitize(&title);
            if !quality.is_empty() {
                name = format!("{name} {}", sanitize(&quality));
            }
            let path = dir.join(format!("{name}.{ext}"));

            let client = match moviebox_tui::net::http_client_builder_base().build() {
                Ok(c) => c,
                Err(e) => return fail(format!("HTTP client error: {e}")),
            };
            let mut req = client
                .get(&src.url)
                .header("User-Agent", moviebox_tui::net::APP_HTTP_USER_AGENT);
            for (k, v) in &src.headers {
                req = req.header(k.as_str(), v.as_str());
            }
            let mut resp = match req.send().await.and_then(|r| r.error_for_status()) {
                Ok(r) => r,
                Err(e) => return fail(format!("Download failed: {e}")),
            };
            let total = resp.content_length();
            let mut file = match tokio::fs::File::create(&path).await {
                Ok(f) => f,
                Err(e) => return fail(format!("Cannot create file: {e}")),
            };
            use tokio::io::AsyncWriteExt;
            let mut done = 0u64;
            let mut last = std::time::Instant::now();
            loop {
                match resp.chunk().await {
                    Ok(Some(chunk)) => {
                        if let Err(e) = file.write_all(&chunk).await {
                            return fail(format!("Write failed: {e}"));
                        }
                        done += chunk.len() as u64;
                        if last.elapsed() > Duration::from_millis(300) {
                            last = std::time::Instant::now();
                            me.send(Msg::Download {
                                id,
                                label: label.clone(),
                                done,
                                total,
                                state: DlState::Running,
                            });
                        }
                    }
                    Ok(None) => break,
                    Err(e) => return fail(format!("Connection lost: {e}")),
                }
            }
            let _ = file.flush().await;
            me.send(Msg::Download {
                id,
                label,
                done,
                total,
                state: DlState::Done(path.display().to_string()),
            });
        });
    }

    // ---------------------------------------------------------------- live tv

    pub fn load_playlist(&self, url: String) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let parser = M3UParser::new();
            let result = parser
                .fetch_playlist(&url)
                .await
                .map_err(|e| e.to_string());
            me.send(Msg::Channels { url, result });
        });
    }
}

fn url_extension(url: &str) -> Option<&'static str> {
    let path = url.split(['?', '#']).next().unwrap_or(url).to_ascii_lowercase();
    for ext in ["mkv", "mp4", "webm", "avi", "mov", "m4v", "ts"] {
        if path.ends_with(&format!(".{ext}")) {
            return Some(ext);
        }
    }
    None
}

fn sanitize(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let cleaned = cleaned.trim_matches('.').trim().to_string();
    if cleaned.is_empty() {
        "video".to_string()
    } else {
        cleaned.chars().take(120).collect()
    }
}
