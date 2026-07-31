//! Семантический поиск.
use brain_common::Result;
use brain_core::EmbeddingProvider;
use crate::store::VectorStore;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Семантический поиск через embeddings.
pub struct SemanticSearch {
    store: Arc<RwLock<VectorStore>>,
    provider: Arc<dyn EmbeddingProvider>,
    threshold: f32,
}

impl SemanticSearch {
    pub fn new(store: Arc<RwLock<VectorStore>>, provider: Arc<dyn EmbeddingProvider>, threshold: f32) -> Self {
        Self { store, provider, threshold }
    }

    /// Поиск похожих заметок по тексту.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<(String, f32)>> {
        let vector = self.provider.embed(query).await?;
        let store = self.store.read().await;
        let results = store.search(&vector, limit, self.threshold);
        Ok(results.into_iter().map(|(_, score, title)| (title, score)).collect())
    }

    /// Предложить связи для новой записи.
    pub async fn suggest_links(&self, text: &str, limit: usize) -> Result<Vec<String>> {
        let results = self.search(text, limit).await?;
        Ok(results.into_iter().map(|(title, _)| title).collect())
    }
}
