use std::sync::Arc;
use brain_common::{ResolutionResult, MatchMethod, Result};
use crate::traits::{IdentityResolver, KnowledgeStore, AiProvider, VectorStorage, EmbeddingProvider};
use strsim::jaro_winkler;

pub struct CascadedIdentityResolver {
    store: Arc<dyn KnowledgeStore>,
    ai: Arc<dyn AiProvider>,
    vector_store: Option<Arc<dyn VectorStorage>>,
    embeddings: Option<Arc<dyn EmbeddingProvider>>,
}

impl CascadedIdentityResolver {
    pub fn new(
        store: Arc<dyn KnowledgeStore>, 
        ai: Arc<dyn AiProvider>,
        vector_store: Option<Arc<dyn VectorStorage>>,
        embeddings: Option<Arc<dyn EmbeddingProvider>>,
    ) -> Self {
        Self { store, ai, vector_store, embeddings }
    }
}

#[async_trait::async_trait]
impl IdentityResolver for CascadedIdentityResolver {
    async fn resolve(&self, query: &str) -> Result<Option<ResolutionResult>> {
        let mut candidates = Vec::new();

        // Embed the query for semantic search if embeddings & vector store are available
        if let (Some(ref embeddings), Some(ref vector_store)) = (&self.embeddings, &self.vector_store) {
            if let Ok(query_embedding) = embeddings.embed(query).await {
                if let Ok(search_results) = vector_store.search(&query_embedding, 20).await {
                    for res in search_results {
                        if res.0.starts_with("entity:") {
                            let entity_name = res.0.trim_start_matches("entity:");
                            if let Ok(Some(entity)) = self.store.get_entity(entity_name).await {
                                candidates.push(entity);
                            }
                        }
                    }
                }
            }
        }

        let q_lower = query.to_lowercase();
        
        // 1. Exact match (by name)
        for e in &candidates {
            if e.name.to_lowercase() == q_lower {
                return Ok(Some(ResolutionResult {
                    entity: e.clone(),
                    confidence: 1.0,
                    matched_by: MatchMethod::Exact,
                }));
            }
        }
        
        // 2. Alias match
        for e in &candidates {
            for alias in &e.aliases {
                if alias.to_lowercase() == q_lower {
                    return Ok(Some(ResolutionResult {
                        entity: e.clone(),
                        confidence: 1.0,
                        matched_by: MatchMethod::Alias,
                    }));
                }
            }
        }
        
        // 3. Fuzzy match (Jaro-Winkler distance)
        let mut best_match = None;
        let mut best_score = 0.0;
        
        for e in &candidates {
            let mut max_score = jaro_winkler(&q_lower, &e.name.to_lowercase());
            
            for alias in &e.aliases {
                let score = jaro_winkler(&q_lower, &alias.to_lowercase());
                if score > max_score {
                    max_score = score;
                }
            }
            
            if max_score > best_score {
                best_score = max_score;
                best_match = Some(e.clone());
            }
        }
        
        if best_score > 0.85 {
            return Ok(Some(ResolutionResult {
                entity: best_match.unwrap(),
                confidence: best_score as f32,
                matched_by: MatchMethod::Fuzzy,
            }));
        }
        
        // 4. Semantic Match using LLM
        if !candidates.is_empty() {
            let candidate_names: Vec<String> = candidates.iter().map(|e| e.name.clone()).collect();
            let prompt = format!(
                "Do any of these existing entities semantically match the query '{}'?\n\
                 Candidates: {:?}\n\
                 If yes, reply with the EXACT name of the matching entity and nothing else.\n\
                 If no, reply with 'NONE'.",
                query, candidate_names
            );
            
            if let Ok(response) = self.ai.complete(&prompt).await {
                let resp_trim = response.trim();
                if resp_trim != "NONE" {
                    if let Some(matched_entity) = candidates.iter().find(|e| e.name == resp_trim) {
                        return Ok(Some(ResolutionResult {
                            entity: matched_entity.clone(),
                            confidence: 0.8,
                            matched_by: MatchMethod::Semantic,
                        }));
                    }
                }
            }
        }
        
        Ok(None)
    }

    async fn register_alias(&self, _canonical_id: &str, _alias: &str) -> Result<()> {
        Ok(())
    }
}

