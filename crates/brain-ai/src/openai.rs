use async_trait::async_trait;
use brain_common::{BrainError, Result};
use brain_core::{AiProvider, EmbeddingProvider};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    embedding_model: String,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

impl OpenAiProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        embedding_model: impl Into<String>,
    ) -> Self {
        Self {
            client: reqwest::Client::builder().timeout(Duration::from_secs(60)).build().unwrap(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            embedding_model: embedding_model.into(),
        }
    }

    async fn send_request(&self, req: &ChatRequest<'_>) -> Result<String> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        
        let mut builder = self.client.post(&url).json(req);
        if !self.api_key.is_empty() {
            builder = builder.bearer_auth(&self.api_key);
        }

        let resp = builder.send().await.map_err(|e| BrainError::Ai(e.to_string()))?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(BrainError::Ai(format!("OpenAI API error {}: {}", status, text)));
        }

        let body: ChatResponse = resp.json().await.map_err(|e| BrainError::Ai(e.to_string()))?;
        
        body.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| BrainError::Ai("Empty choices in response".into()))
    }
}

#[async_trait]
impl AiProvider for OpenAiProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let req = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage { role: "user", content: prompt }],
            response_format: None,
            temperature: 0.3,
        };
        self.send_request(&req).await
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        let req = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage { role: "user", content: prompt }],
            response_format: Some(ResponseFormat { format_type: "json_object".to_string() }),
            temperature: 0.1, // Lower temp for JSON
        };
        self.send_request(&req).await
    }

    async fn classify(&self, text: &str, categories: &[&str]) -> Result<(String, f32)> {
        let prompt = format!(
            "Classify the following text into one of these categories: {:?}\n\nText: {}\n\nReturn only the category name.",
            categories, text
        );
        let resp = self.complete(&prompt).await?;
        let resp = resp.trim().to_string();
        
        if categories.contains(&resp.as_str()) {
            Ok((resp, 0.9))
        } else {
            Ok(("Unknown".to_string(), 0.0))
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/v1/embeddings", self.base_url);
        let req = EmbedRequest { model: &self.embedding_model, input: text };
        
        let mut builder = self.client.post(&url).json(&req);
        if !self.api_key.is_empty() {
            builder = builder.bearer_auth(&self.api_key);
        }

        let resp = builder.send().await.map_err(|e| BrainError::Ai(e.to_string()))?;
        let body: EmbedResponse = resp.json().await.map_err(|e| BrainError::Ai(e.to_string()))?;
        
        body.data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| BrainError::Ai("No embedding returned".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            let vec = self.embed(text).await?;
            results.push(vec);
        }
        Ok(results)
    }
}
