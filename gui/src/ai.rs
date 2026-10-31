//! AI features, powered by the Anthropic Messages API with the viewer's own API key.
//! Nothing is sent unless the viewer asks for an AI feature and has added a key.

use crate::data::{Backend, Msg};
use crate::store::Card;
use moviebox_tui::providers::{MediaType, ProviderKind};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Purpose {
    Concierge,
    ForYou,
}

#[derive(Clone, Debug)]
pub struct Pick {
    pub title: String,
    pub why: String,
    pub card: Option<Card>,
}

#[derive(Clone, Debug)]
pub struct Reply {
    pub text: String,
    pub picks: Vec<Pick>,
}

#[derive(Clone, Debug, Default)]
pub struct Insight {
    pub pitch: String,
    pub watch_if: Vec<String>,
    pub mood: Vec<String>,
    pub note: String,
}

const API_URL: &str = "https://api.anthropic.com/v1/messages";

fn concierge_system(family: bool) -> String {
    let mut s = String::from(
        "You are the recommendation engine inside a streaming app called MovieBox. \
Respond with ONLY a JSON object, no markdown, in exactly this shape: \
{\"reply\": string, \"picks\": [{\"title\": string, \"year\": number or null, \"type\": \"movie\" or \"series\", \"why\": string}]}. \
\"reply\" is at most two warm, concise sentences with no spoilers. \
Give 6 to 8 picks. Use only real, well-known titles that exist; never invent titles. \
\"why\" is at most 16 words and spoiler-free. Do not repeat titles the viewer already watched or saved.",
    );
    if family {
        s.push_str(" The viewer profile is family-friendly: only suggest titles suitable for children and families.");
    }
    s
}

const INSIGHT_SYSTEM: &str = "You write short, spoiler-free viewing guides for a streaming app. \
Respond with ONLY a JSON object, no markdown: \
{\"pitch\": string (max 35 words, no spoilers), \"watch_if\": [string, string, string] (each max 9 words, start with 'You like'), \
\"mood\": [string, string, string] (single-word or two-word moods), \"note\": string (max 14 words, pacing or tone heads-up)}. \
Never reveal plot twists or endings.";

async fn call_claude(
    key: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let client = moviebox_tui::net::http_client_builder_base()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system,
        "messages": [{"role": "user", "content": user}],
    });
    let resp = client
        .post(API_URL)
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("Could not reach the AI service: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Could not read the AI response: {e}"))?;
    if !status.is_success() {
        let detail = serde_json::from_str::<Value>(&text)
            .ok()
            .and_then(|v| v["error"]["message"].as_str().map(str::to_string))
            .unwrap_or_default();
        return Err(match status.as_u16() {
            401 | 403 => "The AI key was rejected. Check it in Settings.".to_string(),
            429 => "The AI service is rate limited. Try again in a moment.".to_string(),
            _ => format!("AI request failed ({status}). {detail}"),
        });
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| format!("Bad AI response: {e}"))?;
    let out: String = v["content"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    if out.trim().is_empty() {
        Err("The AI returned an empty answer.".to_string())
    } else {
        Ok(out)
    }
}

/// Pull the first JSON object out of a model answer (tolerates code fences / chatter).
pub fn extract_json(s: &str) -> Option<Value> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str(&s[start..=end]).ok()
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

impl Backend {
    /// Ask the AI for picks, then match each pick to a real catalogue entry.
    #[allow(clippy::too_many_arguments)]
    pub fn ai_ask(
        &self,
        token: u64,
        purpose: Purpose,
        key: String,
        model: String,
        prompt: String,
        context: String,
        provider: ProviderKind,
        family: bool,
    ) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let user = if context.trim().is_empty() {
                prompt.clone()
            } else {
                format!("{prompt}\n\nViewer context:\n{context}")
            };
            let result = async {
                let raw =
                    call_claude(&key, &model, &concierge_system(family), &user, 1200).await?;
                let v = extract_json(&raw).ok_or("The AI answer could not be understood.")?;
                let text = v["reply"].as_str().unwrap_or("Here are some ideas.").to_string();
                let mut wanted = Vec::new();
                for p in v["picks"].as_array().cloned().unwrap_or_default().into_iter().take(8) {
                    let title = p["title"].as_str().unwrap_or("").trim().to_string();
                    if title.is_empty() {
                        continue;
                    }
                    let why = p["why"].as_str().unwrap_or("").trim().to_string();
                    let kind = p["type"].as_str().unwrap_or("").to_string();
                    wanted.push((title, why, kind));
                }
                let mut handles = Vec::new();
                for (title, _, _) in &wanted {
                    let me2 = me.clone();
                    let q = title.clone();
                    handles.push(tokio::spawn(async move {
                        me2.svc.search_typed(provider, &q, 1).await.ok()
                    }));
                }
                let mut picks = Vec::new();
                for ((title, why, kind), h) in wanted.into_iter().zip(handles) {
                    let found = h.await.ok().flatten().unwrap_or_default();
                    let want = norm(&title);
                    let want_kind = match kind.as_str() {
                        "series" => Some(MediaType::Series),
                        "movie" => Some(MediaType::Movie),
                        _ => None,
                    };
                    let best = found
                        .iter()
                        .find(|c| norm(&c.title) == want && want_kind.is_none_or(|k| k == c.media_type))
                        .or_else(|| found.iter().find(|c| norm(&c.title) == want))
                        .or_else(|| {
                            found.iter().find(|c| {
                                let t = norm(&c.title);
                                !want.is_empty() && (t.starts_with(&want) || want.starts_with(&t))
                            })
                        });
                    picks.push(Pick {
                        title,
                        why,
                        card: best.map(|c| Card::from_item(c, None)),
                    });
                }
                Ok::<Reply, String>(Reply { text, picks })
            }
            .await;
            me.send(Msg::Ai {
                token,
                purpose,
                result,
            });
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ai_insight(
        &self,
        token: u64,
        key: String,
        model: String,
        title: String,
        year: String,
        genres: String,
        description: String,
    ) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let user = format!(
                "Title: {title}\nYear: {year}\nGenres: {genres}\nDescription: {description}"
            );
            let result = async {
                let raw = call_claude(&key, &model, INSIGHT_SYSTEM, &user, 500).await?;
                let v = extract_json(&raw).ok_or("The AI answer could not be understood.")?;
                let list = |k: &str| -> Vec<String> {
                    v[k].as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
                                .filter(|s| !s.is_empty())
                                .take(3)
                                .collect()
                        })
                        .unwrap_or_default()
                };
                Ok::<Insight, String>(Insight {
                    pitch: v["pitch"].as_str().unwrap_or("").trim().to_string(),
                    watch_if: list("watch_if"),
                    mood: list("mood"),
                    note: v["note"].as_str().unwrap_or("").trim().to_string(),
                })
            }
            .await;
            me.send(Msg::Insight { token, result });
        });
    }

    pub fn suggest(&self, query: String, token: u64) {
        if !self.live {
            return;
        }
        let me = self.clone();
        self.rt.spawn(async move {
            let items = me.svc.suggest(&query).await.unwrap_or_default();
            me.send(Msg::Suggest { token, items });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_json_from_fenced_answer() {
        let v = extract_json("Sure!\n```json\n{\"reply\":\"hi\",\"picks\":[]}\n```").unwrap();
        assert_eq!(v["reply"], "hi");
    }

    #[test]
    fn rejects_non_json() {
        assert!(extract_json("no braces here").is_none());
        assert!(extract_json("} backwards {").is_none());
    }

    #[test]
    fn normalises_titles() {
        assert_eq!(norm("The Dark Knight!"), "thedarkknight");
        assert_eq!(norm("  Spider-Man: Homecoming "), "spidermanhomecoming");
    }
}
