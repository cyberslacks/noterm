pub mod claude;
pub mod context;
pub mod ollama;
pub mod openai;
pub mod summarizer;

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use crate::config::{EmbedProvider, LlmConfig, LlmProvider};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
        }
    }
}

pub type TokenStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn chat_stream(&self, messages: Vec<ChatMessage>, system: &str) -> Result<TokenStream>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

pub fn make_client(config: &LlmConfig) -> Box<dyn LlmClient> {
    match config.provider {
        LlmProvider::Ollama => Box::new(ollama::OllamaClient::new(config)),
        LlmProvider::Claude => Box::new(claude::ClaudeClient::new(config)),
        LlmProvider::OpenAI => Box::new(openai::OpenAiClient::new(config)),
    }
}

/// Returns a client for the configured embed_provider.
/// Use this for embeddings instead of make_client() — Claude cannot embed.
pub fn make_embed_client(config: &LlmConfig) -> Box<dyn LlmClient> {
    match config.embed_provider {
        EmbedProvider::Ollama => Box::new(ollama::OllamaClient::new(config)),
        EmbedProvider::OpenAI => Box::new(openai::OpenAiClient::new(config)),
    }
}

/// Returns the model name string used for the current embed_provider.
/// This is stored in the DB as the embedding cache key.
pub fn embed_model_name(config: &LlmConfig) -> String {
    match config.embed_provider {
        EmbedProvider::Ollama => config.ollama_embed_model.clone(),
        EmbedProvider::OpenAI => config.openai_embed_model.clone(),
    }
}

// ── Model discovery ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Deserialize)]
struct OllamaModelEntry {
    name: String,
}

/// Fetch available model names from a running Ollama instance.
pub async fn list_ollama_models(base_url: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base_url}/api/tags"))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Ollama /api/tags returned {}", resp.status());
    }

    let data: OllamaTagsResponse = resp.json().await?;
    Ok(data.models.into_iter().map(|m| m.name).collect())
}

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModelEntry>,
}

#[derive(Deserialize)]
struct OpenAIModelEntry {
    id: String,
}

/// Fetch available model names from an OpenAI-compatible API (OpenAI, OpenWebUI, etc.).
pub async fn list_openai_models(base_url: &str, api_key: &str) -> Result<Vec<String>> {
    let client = reqwest::Client::new();
    let req = client
        .get(format!("{base_url}/models"))
        .timeout(std::time::Duration::from_secs(5))
        .bearer_auth(api_key);

    let resp = req.send().await?;

    if !resp.status().is_success() {
        anyhow::bail!("OpenAI /models returned {}", resp.status());
    }

    let data: OpenAIModelsResponse = resp.json().await?;
    let mut ids: Vec<String> = data.data.into_iter().map(|m| m.id).collect();
    ids.sort();
    Ok(ids)
}
