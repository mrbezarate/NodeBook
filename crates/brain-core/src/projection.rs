use async_trait::async_trait;
use brain_common::{Area, Entity, EntityType, Result};
use std::sync::Arc;
use crate::traits::{ProjectionEngine, Renderer, RawEventStore};
use crate::extractor::StructuredObservation;
use std::fs;
use std::path::PathBuf;

pub struct SimpleProjectionEngine {
    raw_event_store: Arc<dyn RawEventStore>,
}

impl SimpleProjectionEngine {
    pub fn new(raw_event_store: Arc<dyn RawEventStore>) -> Self {
        Self { raw_event_store }
    }
}

#[async_trait]
impl ProjectionEngine for SimpleProjectionEngine {
    async fn project(&self, entity_id: &str) -> Result<Entity> {
        let observations = self.raw_event_store.get_observations(entity_id).await?;
        
        let mut aggregated_summary = String::new();
        let mut entity_name = format!("Generated Name for {}", entity_id);
        
        for obs in &observations {
            // Try to parse the fact as StructuredObservation.
            // If it fails, just append the raw fact text.
            if let Ok(structured) = serde_json::from_str::<StructuredObservation>(&obs.fact) {
                aggregated_summary.push_str(&structured.summary);
                aggregated_summary.push_str("\n");
                
                // Восстанавливаем каноническое имя сущности из самого первого наблюдения
                if entity_name.starts_with("Generated Name for") {
                    if let Some(title) = &structured.title {
                        entity_name = title.clone();
                    } else if let Some(name) = structured.entities.first() {
                        entity_name = name.clone();
                    }
                }
            } else {
                aggregated_summary.push_str(&obs.fact);
                aggregated_summary.push_str("\n");
            }
        }
        
        if aggregated_summary.is_empty() {
            aggregated_summary = "Нет наблюдений.".to_string();
        }

        let mut all_tags: Vec<String> = Vec::new();
        for obs in &observations {
            if let Ok(structured) = serde_json::from_str::<StructuredObservation>(&obs.fact) {
                for tag in structured.tags {
                    if !all_tags.contains(&tag) {
                        all_tags.push(tag);
                    }
                }
            }
        }

        let mut entity = Entity::new(&entity_name, EntityType::Concept);
        entity.id = entity_id.to_string();
        entity.area = Some(Area::Life);
        entity.summary = aggregated_summary.trim().to_string();
        entity.tags = all_tags;
        Ok(entity)
    }
}

pub struct ObsidianRenderer {
    pub base_path: PathBuf,
}

#[async_trait]
impl Renderer for ObsidianRenderer {
    async fn render(&self, entity: &Entity) -> Result<()> {
        let file_name = format!("{}.md", entity.name);
        let path = self.base_path.join(&file_name);
        
        let md = format!(
            "---\n\
             id: {}\n\
             type: {:?}\n\
             area: {:?}\n\
             ---\n\
             # {}\n\n\
             {}\n\n",
            entity.id,
            entity.entity_type,
            entity.area,
            entity.name,
            entity.summary
        );
        
        fs::write(&path, md).map_err(|e| brain_common::BrainError::Io(e))?;
        tracing::info!("Rendered to Obsidian: {:?}", path);
        Ok(())
    }
}
