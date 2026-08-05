use async_trait::async_trait;
use brain_common::{BrainError, Result, SearchResult};
use brain_core::traits::{ContextManager, ActiveContext, VaultStorage, VectorStorage, EmbeddingProvider};
use std::sync::Arc;


pub struct BrainMemory {
    embeddings: Arc<dyn EmbeddingProvider>,
    vector_store: Arc<dyn VectorStorage>,
    vault: Arc<dyn VaultStorage>,
}

impl BrainMemory {
    pub fn new(
        embeddings: Arc<dyn EmbeddingProvider>,
        vector_store: Arc<dyn VectorStorage>,
        vault: Arc<dyn VaultStorage>,
    ) -> Self {
        Self {
            embeddings,
            vector_store,
            vault,
        }
    }
}

#[async_trait]
impl ContextManager for BrainMemory {
    async fn gather_context(&self, text: &str) -> Result<ActiveContext> {
        let mut context = ActiveContext::default();

        // 1. Semantic Search
        if let Ok(vector) = self.embeddings.embed(text).await {
            if let Ok(results) = self.vector_store.search(&vector, 3).await {
                context.semantic_results = results;
            }
        }

        // 2. Keyword Search (Extract a naive keyword from the text for searching)
        // Just search the raw text directly. If it's too long, truncate it.
        let query_str = if text.len() > 100 {
            text.chars().take(100).collect::<String>()
        } else {
            text.to_string()
        };
        if let Ok(results) = self.vault.search_by_text(&query_str).await {
            context.keyword_results = results.into_iter().take(3).collect();
        }

        Ok(context)
    }
}
