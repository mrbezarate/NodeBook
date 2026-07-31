//! NoopAiProvider — система работает без AI, на чистых алгоритмах.
use async_trait::async_trait;
use brain_common::{BrainError, Result};
use brain_core::{AiProvider, EmbeddingProvider};

/// AI-провайдер «заглушка» — используется когда AI отключён.
pub struct NoopAiProvider;

#[async_trait]
impl AiProvider for NoopAiProvider {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Err(BrainError::Ai("AI disabled (NoopProvider)".into()))
    }
    async fn classify(&self, _text: &str, _categories: &[&str]) -> Result<(String, f32)> {
        Err(BrainError::Ai("AI disabled (NoopProvider)".into()))
    }
}

#[async_trait]
impl EmbeddingProvider for NoopAiProvider {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        Err(BrainError::Ai("Embeddings disabled (NoopProvider)".into()))
    }
    async fn embed_batch(&self, _texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Err(BrainError::Ai("Embeddings disabled (NoopProvider)".into()))
    }
}
