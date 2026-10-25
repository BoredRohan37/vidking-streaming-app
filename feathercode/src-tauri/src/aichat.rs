use crate::db::{Db, Provider};
use futures_util::StreamExt;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

#[derive(Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ChunkEvt {
    request_id: String,
    delta: String,
    reasoning: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DoneEvt {
    request_id: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ErrEvt {
    request_id: String,
    error: String,
}

fn emit_err(app: &AppHandle, id: &str, msg: String) {
    let _ = app.emit("ai-error", ErrEvt { request_id: id.to_string(), error: msg });
}

/// Streaming AI chat. Runs in the background; frontend listens to:
///   ai-chunk  { requestId, delta, reasoning }
///   ai-done   { requestId }
///   ai-error  { requestId, error }
#[tauri::command]
pub async fn ai_chat(
    app: AppHandle,
    db: State<'_, Db>,
    request_id: String,
    provider_id: String,
    messages: Vec<ChatMessage>,
) -> Result<(), String> {
    let provider = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT kind, base_url, api_key, model FROM providers WHERE id = ?1",
            params![provider_id],
            |r| {
                Ok(Provider {
                    id: String::new(),
                    name: String::new(),
                    kind: r.get(0)?,
                    base_url: r.get(1)?,
                    api_key: r.get(2)?,
                    model: r.get(3)?,
                })
            },
        )
        .map_err(|e| format!("provider not found: {e}"))?
    };

    let app2 = app.clone();
    let req_id = request_id.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = stream_ai(&app2, provider, messages, &req_id).await {
            emit_err(&app2, &req_id, e);
        }
    });
    Ok(())
}

fn trim_base(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

async fn stream_ai(app: &AppHandle, p: Provider, messages: Vec<ChatMessage>, req_id: &str) -> Result<(), String> {
    let anthropic = p.kind == "anthropic";
    let base = if p.base_url.trim().is_empty() {
        match p.kind.as_str() {
            "glm" => "https://api.z.ai/api/paas/v4",
            "openai" => "https://api.openai.com/v1",
            "anthropic" => "https://api.anthropic.com",
            _ => return Err("base URL is required for this provider kind".into()),
        }
    } else {
        &p.base_url
    };

    let url = if anthropic {
        format!("{}/v1/messages", trim_base(base))
    } else {
        format!("{}/chat/completions", trim_base(base))
    };

    let client = reqwest::Client::new();
    let mut req = client.post(&url);
    if anthropic {
        req = req
            .header("x-api-key", &p.api_key)
            .header("anthropic-version", "2023-06-01");
    } else {
        req = req.header("Authorization", format!("Bearer {}", p.api_key));
    }
    let body = if anthropic {
        json!({ "model": p.model, "max_tokens": 8192, "stream": true, "messages": messages })
    } else {
        json!({ "model": p.model, "stream": true, "messages": messages })
    };

    let resp = req.json(&body).send().await.map_err(|e| format!("request failed: {e}"))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let snippet: String = text.chars().take(600).collect();
        return Err(format!("API error {status}: {snippet}"));
    }

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|e| format!("stream error: {e}"))?;
        buf.push_str(&String::from_utf8_lossy(&bytes));
        while let Some(pos) = buf.find('\n') {
            let line: String = buf.drain(..=pos).collect();
            let line = line.trim();
            let data = match line.strip_prefix("data:") {
                Some(d) => d.trim(),
                None => continue,
            };
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let v: serde_json::Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if anthropic {
                if v["type"] == "content_block_delta" {
                    if let Some(t) = v["delta"]["text"].as_str() {
                        if !t.is_empty() {
                            let _ = app.emit(
                                "ai-chunk",
                                ChunkEvt { request_id: req_id.into(), delta: t.into(), reasoning: false },
                            );
                        }
                    }
                }
            } else {
                if let Some(t) = v["choices"][0]["delta"]["content"].as_str() {
                    if !t.is_empty() {
                        let _ = app.emit(
                            "ai-chunk",
                            ChunkEvt { request_id: req_id.into(), delta: t.into(), reasoning: false },
                        );
                    }
                }
                // GLM-4.5+/deep-thinking models stream reasoning separately
                if let Some(t) = v["choices"][0]["delta"]["reasoning_content"].as_str() {
                    if !t.is_empty() {
                        let _ = app.emit(
                            "ai-chunk",
                            ChunkEvt { request_id: req_id.into(), delta: t.into(), reasoning: true },
                        );
                    }
                }
            }
        }
    }
    let _ = app.emit("ai-done", DoneEvt { request_id: req_id.into() });
    Ok(())
}
