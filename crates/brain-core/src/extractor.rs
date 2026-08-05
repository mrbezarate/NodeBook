use std::sync::Arc;
use serde::{Deserialize, Serialize};
use brain_common::{BrainError, Result};
use crate::traits::AiProvider;

#[derive(Debug, Deserialize, Serialize)]
pub struct StructuredObservation {
    pub title: Option<String>,
    pub summary: String,
    pub enriched_text: Option<String>,
    pub entities: Vec<String>,
    pub tags: Vec<String>,
    pub confidence: f32,
}

pub struct Extractor {
    ai_provider: Arc<dyn AiProvider>,
}

impl Extractor {
    pub fn new(ai_provider: Arc<dyn AiProvider>) -> Self {
        Self { ai_provider }
    }

    pub async fn extract(&self, text: &str) -> Result<StructuredObservation> {
        let prompt_template = include_str!("../prompts/extractor_v1.md");
        let prompt = prompt_template.replace("{text}", text);

        let json_str = self.ai_provider.complete_json(&prompt).await?;
        
        let observation: StructuredObservation = serde_json::from_str(&json_str)
            .map_err(|e| BrainError::Parser(format!("Failed to parse LLM output: {}. Output was: {}", e, json_str)))?;

        Ok(observation)
    }
}
