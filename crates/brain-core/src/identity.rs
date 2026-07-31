use std::sync::Arc;
use brain_common::{ResolutionResult, MatchMethod, Result};
use crate::traits::{IdentityResolver, KnowledgeStore, AiProvider};
use strsim::jaro_winkler;

pub struct CascadedIdentityResolver {
    store: Arc<dyn KnowledgeStore>,
    ai: Arc<dyn AiProvider>,
}

impl CascadedIdentityResolver {
    pub fn new(store: Arc<dyn KnowledgeStore>, ai: Arc<dyn AiProvider>) -> Self {
        Self { store, ai }
    }
}

#[async_trait::async_trait]
impl IdentityResolver for CascadedIdentityResolver {
    async fn resolve(&self, query: &str) -> Result<Option<ResolutionResult>> {
        let entities = self.store.list_entities(None).await?;
        let q_lower = query.to_lowercase();
        
        // 1. Exact match (by name)
        for e in &entities {
            if e.name.to_lowercase() == q_lower {
                return Ok(Some(ResolutionResult {
                    entity: e.clone(),
                    confidence: 1.0,
                    matched_by: MatchMethod::Exact,
                }));
            }
        }
        
        // 2. Alias match
        for e in &entities {
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
        
        for e in &entities {
            let score = jaro_winkler(&q_lower, &e.name.to_lowercase());
            if score > best_score {
                best_score = score;
                best_match = Some(e.clone());
            }
            
            for alias in &e.aliases {
                let score = jaro_winkler(&q_lower, &alias.to_lowercase());
                if score > best_score {
                    best_score = score;
                    best_match = Some(e.clone());
                }
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
        if !entities.is_empty() {
            // Ограничиваем список кандидатов для LLM
            let candidate_names: Vec<String> = entities.iter().map(|e| e.name.clone()).collect();
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
                    if let Some(matched_entity) = entities.iter().find(|e| e.name == resp_trim) {
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
        // Alias registration happens via DB, so we'll just ignore for now or implement if needed.
        Ok(())
    }
}
