use async_trait::async_trait;
use brain_common::{BrainError, Result};
use brain_core::{AiProvider, EmbeddingProvider};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Gemini AI-провайдер через Google API (OpenAI-совместимый и Vertex AI Enterprise Agent Platform).
pub struct GeminiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    embedding_model: String,
    is_enterprise: bool,
    project_id: String,
    location: String,
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

// ── Vertex AI / Enterprise Agent Platform Structs ────────────────────────────
#[derive(Serialize)]
struct VertexContent<'a> {
    role: &'a str,
    parts: Vec<VertexPart<'a>>,
}

#[derive(Serialize)]
struct VertexPart<'a> {
    text: &'a str,
}

#[derive(Serialize)]
struct VertexGenerationConfig {
    #[serde(rename = "responseMimeType", skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
    temperature: f32,
}

#[derive(Serialize)]
struct VertexGenerateRequest<'a> {
    contents: Vec<VertexContent<'a>>,
    #[serde(rename = "generationConfig")]
    generation_config: VertexGenerationConfig,
}

#[derive(Deserialize)]
struct VertexCandidate {
    content: VertexCandidateContent,
}

#[derive(Deserialize)]
struct VertexCandidateContent {
    parts: Vec<VertexCandidatePart>,
}

#[derive(Deserialize)]
struct VertexCandidatePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct VertexGenerateResponse {
    candidates: Option<Vec<VertexCandidate>>,
}

#[derive(Serialize)]
struct VertexEmbedInstance<'a> {
    content: &'a str,
}

#[derive(Serialize)]
struct VertexEmbedRequest<'a> {
    instances: Vec<VertexEmbedInstance<'a>>,
}

#[derive(Deserialize)]
struct VertexEmbedResponse {
    predictions: Option<Vec<VertexPrediction>>,
}

#[derive(Deserialize)]
struct VertexPrediction {
    embeddings: VertexEmbeddingValues,
}

#[derive(Deserialize)]
struct VertexEmbeddingValues {
    values: Vec<f32>,
}

impl GeminiProvider {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        embedding_model: impl Into<String>,
    ) -> Self {
        let mut url = base_url.into();
        let key = api_key.into();
        let is_enterprise = key.starts_with("AQ.") || url.contains("aiplatform.googleapis.com");

        if url.trim().is_empty() {
            if is_enterprise {
                url = "https://us-central1-aiplatform.googleapis.com".to_string();
            } else {
                url = "https://generativelanguage.googleapis.com/v1beta/openai".to_string();
            }
        }
        let url = url.trim_end_matches('/').to_string();

        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap(),
            base_url: url,
            api_key: key,
            model: model.into(),
            embedding_model: embedding_model.into(),
            is_enterprise,
            project_id: "568759207413".to_string(),
            location: "us-central1".to_string(),
        }
    }

    async fn send_vertex_generate(&self, prompt: &str, is_json: bool, temperature: f32) -> Result<String> {
        let endpoint = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent?key={}",
            self.location, self.project_id, self.location, self.model, self.api_key
        );

        let req = VertexGenerateRequest {
            contents: vec![VertexContent {
                role: "user",
                parts: vec![VertexPart { text: prompt }],
            }],
            generation_config: VertexGenerationConfig {
                response_mime_type: if is_json { Some("application/json".to_string()) } else { None },
                temperature,
            },
        };

        let resp = self.client.post(&endpoint)
            .json(&req)
            .send()
            .await
            .map_err(|e| BrainError::Ai(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(BrainError::Ai(format!("Vertex AI error {}: {}", status, text)));
        }

        let body: VertexGenerateResponse = resp.json().await.map_err(|e| BrainError::Ai(e.to_string()))?;
        
        let text = body.candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|cand| cand.content.parts.into_iter().next())
            .and_then(|p| p.text)
            .ok_or_else(|| BrainError::Ai("Empty candidates in Vertex AI response".into()))?;

        Ok(text)
    }

    async fn send_openai_request(&self, req: &ChatRequest<'_>) -> Result<String> {
        let endpoint = if self.base_url.ends_with("/openai") || self.base_url.ends_with("/v1") {
            format!("{}/chat/completions", self.base_url)
        } else {
            format!("{}/v1/chat/completions", self.base_url)
        };

        let mut builder = self.client.post(&endpoint).json(req);
        if !self.api_key.is_empty() {
            builder = builder.bearer_auth(&self.api_key);
        }

        let resp = builder.send().await.map_err(|e| BrainError::Ai(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(BrainError::Ai(format!("Gemini API error {}: {}", status, text)));
        }

        let body: ChatResponse = resp.json().await.map_err(|e| BrainError::Ai(e.to_string()))?;

        body.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| BrainError::Ai("Empty choices in Gemini response".into()))
    }
}

#[async_trait]
impl AiProvider for GeminiProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        if self.is_enterprise {
            self.send_vertex_generate(prompt, false, 0.3).await
        } else {
            let req = ChatRequest {
                model: &self.model,
                messages: vec![ChatMessage { role: "user", content: prompt }],
                response_format: None,
                temperature: 0.3,
            };
            self.send_openai_request(&req).await
        }
    }

    async fn complete_json(&self, prompt: &str) -> Result<String> {
        let res = if self.is_enterprise {
            self.send_vertex_generate(prompt, true, 0.1).await?
        } else {
            let req = ChatRequest {
                model: &self.model,
                messages: vec![ChatMessage { role: "user", content: prompt }],
                response_format: Some(ResponseFormat { format_type: "json_object".to_string() }),
                temperature: 0.1,
            };
            self.send_openai_request(&req).await?
        };

        // Clean markdown backticks if model wrapped JSON in ```json ... ```
        let trimmed = res.trim();
        let cleaned = if trimmed.starts_with("```") {
            trimmed
                .strip_prefix("```json")
                .or_else(|| trimmed.strip_prefix("```"))
                .unwrap_or(trimmed)
                .strip_suffix("```")
                .unwrap_or(trimmed)
                .trim()
                .to_string()
        } else {
            trimmed.to_string()
        };
        Ok(cleaned)
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
impl EmbeddingProvider for GeminiProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if self.is_enterprise {
            let endpoint = format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predict?key={}",
                self.location, self.project_id, self.location, self.embedding_model, self.api_key
            );

            let req = VertexEmbedRequest {
                instances: vec![VertexEmbedInstance { content: text }],
            };

            let resp = self.client.post(&endpoint)
                .json(&req)
                .send()
                .await
                .map_err(|e| BrainError::Ai(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text_err = resp.text().await.unwrap_or_default();
                return Err(BrainError::Ai(format!("Vertex AI Embeddings error {}: {}", status, text_err)));
            }

            let body: VertexEmbedResponse = resp.json().await.map_err(|e| BrainError::Ai(e.to_string()))?;
            body.predictions
                .and_then(|p| p.into_iter().next())
                .map(|pred| pred.embeddings.values)
                .ok_or_else(|| BrainError::Ai("No embedding returned from Vertex AI".into()))
        } else {
            let endpoint = if self.base_url.ends_with("/openai") || self.base_url.ends_with("/v1") {
                format!("{}/embeddings", self.base_url)
            } else {
                format!("{}/v1/embeddings", self.base_url)
            };

            let req = EmbedRequest {
                model: &self.embedding_model,
                input: text,
            };

            let mut builder = self.client.post(&endpoint).json(&req);
            if !self.api_key.is_empty() {
                builder = builder.bearer_auth(&self.api_key);
            }

            let resp = builder.send().await.map_err(|e| BrainError::Ai(e.to_string()))?;
            if !resp.status().is_success() {
                let status = resp.status();
                let text_err = resp.text().await.unwrap_or_default();
                return Err(BrainError::Ai(format!("Gemini Embeddings API error {}: {}", status, text_err)));
            }

            let body: EmbedResponse = resp.json().await.map_err(|e| BrainError::Ai(e.to_string()))?;

            body.data
                .into_iter()
                .next()
                .map(|d| d.embedding)
                .ok_or_else(|| BrainError::Ai("No embedding returned from Gemini".into()))
        }
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
