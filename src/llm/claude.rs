use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::json;

use super::{ChatMessage, ChatRole, LlmClient, TokenStream};
use crate::config::LlmConfig;

pub struct ClaudeClient {
    api_key: String,
    model: String,
    http: reqwest::Client,
}

impl ClaudeClient {
    pub fn new(config: &LlmConfig) -> Self {
        Self {
            api_key: config.claude_api_key.clone().unwrap_or_default(),
            model: config.claude_model.clone(),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmClient for ClaudeClient {
    async fn chat_stream(&self, messages: Vec<ChatMessage>, system: &str) -> Result<TokenStream> {
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .map(|m| {
                let role = match m.role {
                    ChatRole::User => "user",
                    _ => "assistant",
                };
                json!({ "role": role, "content": m.content })
            })
            .collect();

        let mut body = json!({
            "model": self.model,
            "max_tokens": 4096,
            "messages": msgs,
            "stream": true
        });

        if !system.is_empty() {
            body["system"] = json!(system);
        }

        let response = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!("Claude API error {status}: {text}");
        }

        let byte_stream = response.bytes_stream();
        let token_stream = byte_stream.filter_map(|chunk| async move {
            let bytes = chunk.ok()?;
            let text = String::from_utf8_lossy(&bytes);
            let mut tokens = String::new();
            for line in text.lines() {
                let line = line.trim();
                // SSE format: "data: {...}"
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                        if v["type"] == "content_block_delta" {
                            if let Some(text) = v["delta"]["text"].as_str() {
                                tokens.push_str(text);
                            }
                        }
                    }
                }
            }
            if tokens.is_empty() {
                None
            } else {
                Some(Ok(tokens))
            }
        });

        Ok(Box::pin(token_stream))
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        bail!("Claude does not support embeddings; use Ollama for embedding generation")
    }
}
