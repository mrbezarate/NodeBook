//! Извлечение сущностей — regex + словарь + AI fallback.
use async_trait::async_trait;
use brain_common::{Entity, EntityType, Result};
use brain_core::EntityExtractor;

pub struct HybridEntityExtractor;
impl HybridEntityExtractor { pub fn new() -> Self { Self } }

static KNOWN_TECH: &[&str] = &["rust","python","javascript","typescript","docker","kubernetes","linux","git","react","vue","tokio","obsidian","telegram","openai","ollama","postgresql","sqlite","redis","nginx","aws"];

#[async_trait]
impl EntityExtractor for HybridEntityExtractor {
    async fn extract_entities(&self, text: &str) -> Result<Vec<Entity>> {
        let lower = text.to_lowercase();
        let mut entities = Vec::new();
        for &tech in KNOWN_TECH {
            if lower.contains(tech) {
                entities.push(Entity { name: tech.to_string(), entity_type: EntityType::Technology });
            }
        }
        // URL → Tool entity
        for word in text.split_whitespace() {
            if word.starts_with("http://") || word.starts_with("https://") {
                entities.push(Entity { name: word.to_string(), entity_type: EntityType::Tool });
            }
        }
        Ok(entities)
    }
}
