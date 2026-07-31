use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::{stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{ChatMessage, ChatRole, LlmClient, TokenStream};
use crate::config::LlmConfig;

pub struct OllamaClient {
    base_url: String,
    chat_model: String,
    embed_model: String,
    http: reqwest::Client,
}

impl OllamaClient {
    pub fn new(config: &LlmConfig) -> Self {
        Self {
            base_url: config.ollama_base_url.clone(),
            chat_model: config.ollama_chat_model.clone(),
            embed_model: config.ollama_embed_model.clone(),
            http: reqwest::Client::new(),
        }
    }
}

#[derive(Deserialize)]
struct OllamaChatChunk {
    message: Option<OllamaMsg>,
    done: bool,
}

#[derive(Deserialize)]
struct OllamaMsg {
    content: String,
}

#[derive(Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl LlmClient for OllamaClient {
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
            "model": self.chat_model,
            "messages": msgs,
            "stream": true
        });

        let response = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Ollama API error: {}", response.status());
        }

        let byte_stream = response.bytes_stream();
        let token_stream = byte_stream.filter_map(|chunk| async move {
            let bytes = chunk.ok()?;
            let text = String::from_utf8_lossy(&bytes);
            // Each line is a JSON object
            let mut tokens = String::new();
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(chunk) = serde_json::from_str::<OllamaChatChunk>(line) {
                    if let Some(msg) = chunk.message {
                        tokens.push_str(&msg.content);
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
            "prompt": text
        });

        let response = self
            .http
            .post(format!("{}/api/embeddings", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            bail!("Ollama embed error: {}", response.status());
        }

        let data: OllamaEmbedResponse = response.json().await?;
        Ok(data.embedding)
    }
}
