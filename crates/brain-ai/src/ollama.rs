//! Ollama HTTP client — локальный AI-провайдер.
use async_trait::async_trait;
use brain_common::{BrainError, Result};
use brain_core::{AiProvider, EmbeddingProvider};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Ollama AI-провайдер через HTTP API.
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    embedding_model: String,
}

#[derive(Serialize)]
struct GenerateRequest<'a> { 
    model: &'a str, 
    prompt: &'a str, 
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<&'a str>,
}

#[derive(Deserialize)]
struct GenerateResponse { response: String }

#[derive(Serialize)]
struct EmbedRequest<'a> { model: &'a str, input: &'a str }

#[derive(Deserialize)]
struct EmbedResponse { embeddings: Vec<Vec<f32>> }

impl OllamaProvider {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>, embedding_model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder().timeout(Duration::from_secs(30)).build().unwrap(),
            base_url: base_url.into(),
            model: model.into(),
            embedding_model: embedding_model.into(),
        }
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);
        let req = GenerateRequest { model: &self.model, prompt, stream: false, format: None };
        let resp = self.client.post(&url).json(&req).send().await
            .map_err(|e| BrainError::Ai(e.to_string()))?;
        let body: GenerateResponse = resp.json().await
            .map_err(|e| BrainError::Ai(e.to_string()))?;
        Ok(body.response)
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);
        let req = GenerateRequest { model: &self.model, prompt, stream: false, format: Some("json") };
        let resp = self.client.post(&url).json(&req).send().await
            .map_err(|e| BrainError::Ai(e.to_string()))?;
        let body: GenerateResponse = resp.json().await
            .map_err(|e| BrainError::Ai(e.to_string()))?;
        Ok(body.response)
    }

    async fn classify(&self, text: &str, categories: &[&str]) -> Result<(String, f32)> {
        let prompt = crate::prompt::PromptBuilder::classification_prompt(text, categories);
        let response = self.complete(&prompt).await?;
        crate::response::parse_classification(&response, categories)
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embed", self.base_url);
        let req = EmbedRequest { model: &self.embedding_model, input: text };
        let resp = self.client.post(&url).json(&req).send().await
            .map_err(|e| BrainError::Embedding(e.to_string()))?;
        let body: EmbedResponse = resp.json().await
            .map_err(|e| BrainError::Embedding(e.to_string()))?;
        body.embeddings.into_iter().next().ok_or_else(|| BrainError::Embedding("Empty embedding".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts { results.push(self.embed(text).await?); }
        Ok(results)
    }
}
