use brain_common::{Result, BrainError, Entity, EntityType};
use brain_core::traits::{EmbeddingProvider, VectorStorage, EntityValidator};
use std::sync::Arc;
use async_trait::async_trait;

pub struct SemanticEntityValidator {
    embeddings: Arc<dyn EmbeddingProvider>,
    vector_store: Arc<dyn VectorStorage>,
    similarity_threshold: f32,
}

impl SemanticEntityValidator {
    pub fn new(
        embeddings: Arc<dyn EmbeddingProvider>,
        vector_store: Arc<dyn VectorStorage>,
        similarity_threshold: f32,
    ) -> Self {
        Self {
            embeddings,
            vector_store,
            similarity_threshold,
        }
    }
}

#[async_trait]
impl EntityValidator for SemanticEntityValidator {
    async fn validate_entity(&self, mut entity: Entity) -> Result<Entity> {
        // 1. Embed the entity name
        let vector = self.embeddings.embed(&entity.name).await?;
        
        // 2. Search for similar concepts in the VectorStore
        // Assuming we prefixed entities in vector store with "entity:" or they just exist.
        // For simplicity, we just search and look for a high match.
        let results = self.vector_store.search(&vector, 3).await?;
        
        // 3. If the best match is above our threshold (e.g. 0.90), we consider them the same concept
        if let Some((best_id, score)) = results.first() {
            if *score >= self.similarity_threshold {
                tracing::info!(
                    "Knowledge Validator: Merged entity '{}' with existing '{}' (score: {:.2})",
                    entity.name, best_id, score
                );
                
                // Extract just the name if it has an "entity:" prefix, or keep as is.
                let merged_name = if best_id.starts_with("entity:") {
                    best_id.strip_prefix("entity:").unwrap().to_string()
                } else {
                    best_id.clone()
                };
                
                entity.name = merged_name;
            }
        }
        
        Ok(entity)
    }
}

#[async_trait]
impl brain_core::traits::IdentityResolver for SemanticEntityValidator {
    async fn resolve(&self, query: &str) -> Result<Option<brain_common::ResolutionResult>> {
        let vector = self.embeddings.embed(query).await?;
        let results = self.vector_store.search(&vector, 3).await?;
        
        if let Some((best_id, score)) = results.first() {
            if *score >= self.similarity_threshold {
                let merged_name = if best_id.starts_with("entity:") {
                    best_id.strip_prefix("entity:").unwrap().to_string()
                } else {
                    best_id.clone()
                };
                
                return Ok(Some(brain_common::ResolutionResult {
                    entity: Entity::new(&merged_name, EntityType::Concept),
                    confidence: *score,
                    matched_by: brain_common::MatchMethod::Semantic,
                }));
            }
        }
        
        Ok(None)
    }
    
    async fn register_alias(&self, _canonical_id: &str, _alias: &str) -> Result<()> {
        // Here we could update a graph node or key-value store to register alias
        Ok(())
    }
}
