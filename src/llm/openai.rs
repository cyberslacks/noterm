use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::json;

use super::{ChatMessage, ChatRole, LlmClient, TokenStream};
use crate::config::LlmConfig;

pub struct OpenAiClient {
    api_key: String,
    model: String,
    embed_model: String,
    base_url: String,
    http: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(config: &LlmConfig) -> Self {
        Self {
            api_key: config.openai_api_key.clone().unwrap_or_default(),
            model: config.openai_model.clone(),
            embed_model: config.openai_embed_model.clone(),
            base_url: config.openai_base_url.clone(),
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn chat_stream(&self, messages: Vec<ChatMessage>, system: &str) -> Result<TokenStream> {
        let mut msgs: Vec<serde_json::Value> = Vec::new();

        if !system.is_empty() {
            msgs.push(json!({ "role": "system", "content": system }));
        }

        for m in &messages {
            let role = match m.role {
                ChatRole::User => "user",
                ChatRole::Assistant => "assistant",
                ChatRole::System => "system",
            };
            msgs.push(json!({ "role": role, "content": m.content }));
        }

        let body = json!({
            "model": self.model,
            "messages": msgs,
            "stream": true
        });

        let mut req = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body);

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            bail!("OpenAI API error {status}: {text}");
        }

        let byte_stream = response.bytes_stream();
        let token_stream = byte_stream.filter_map(|chunk| async move {
            let bytes = chunk.ok()?;
            let text = String::from_utf8_lossy(&bytes);
            let mut tokens = String::new();
            for line in text.lines() {
                let line = line.trim();
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(text) = v["choices"][0]["delta"]["content"].as_str() {
                            tokens.push_str(text);
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

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let body = json!({
            "model": self.embed_model,
            "input": text
        });

        let mut req = self
            .http
            .post(format!("{}/embeddings", self.base_url))
            .json(&body);

        // Only attach the auth header when a key is actually configured —
        // sending "Bearer " (empty token) causes 401s on local OpenWebUI.
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!(
                "Embed API error {status} from {}/embeddings (model: {}): {body}",
                self.base_url,
                self.embed_model
            );
        }

        let data: serde_json::Value = response.json().await?;

        let embedding: Vec<f32> = data["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Unexpected embed response from {}/embeddings (model: {}). \
                     Expected {{\"data\":[{{\"embedding\":[...]}}]}}. Got: {}",
                    self.base_url,
                    self.embed_model,
                    data
                )
            })?
            .iter()
            .filter_map(|v| v.as_f64().map(|f| f as f32))
            .collect();

        if embedding.is_empty() {
            bail!(
                "Embed model '{}' at {} returned a zero-length vector",
                self.embed_model,
                self.base_url
            );
        }

        Ok(embedding)
    }
}
