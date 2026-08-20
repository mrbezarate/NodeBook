use async_trait::async_trait;
use brain_common::{BrainError, Entity, EntityType, Result};
use brain_core::traits::KnowledgeStore;
use std::path::PathBuf;

pub struct EntityVault {
    root: PathBuf,
}

impl EntityVault {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn sanitize_id(id: &str) -> String {
        let safe: String = id
            .chars()
            .map(|c| if c == '/' || c == '\\' || c == ':' || c == '*' || c == '?' || c == '"' || c == '<' || c == '>' || c == '|' || c == '\0' { '_' } else { c })
            .collect();
        let safe = safe.trim_matches('.').trim().to_string();
        if safe.is_empty() {
            "unnamed_entity".to_string()
        } else {
            safe
        }
    }

    fn entity_path(&self, id: &str) -> PathBuf {
        let safe = Self::sanitize_id(id);
        self.root.join("Entities").join(format!("{}.md", safe))
    }
}

#[async_trait]
impl KnowledgeStore for EntityVault {
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        let path = self.entity_path(id);
        if !path.exists() {
            return Ok(None);
        }
        
        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| BrainError::Vault(e.to_string()))?;
        // We parse the file simply for now (a robust parser would handle edge cases)
        let mut entity = Entity::new(id, EntityType::Concept);
        entity.id = id.to_string();
        
        // Parse Title (e.g. "# Space Cowboy RPG")
        for line in content.lines() {
            if line.starts_with("# ") {
                entity.name = line.trim_start_matches("# ").to_string();
                break;
            }
        }
        
        // Parse Tags (e.g. "#gamedev")
        let mut tags = Vec::new();
        for word in content.split_whitespace() {
            if word.starts_with('#') && word.len() > 1 && !word.contains(' ') {
                tags.push(word.strip_prefix('#').unwrap().to_string());
            }
        }
        entity.tags = tags;
        
        // Extract Summary block
        if let Some(summary_start) = content.find("## Summary") {
            let rest = &content[summary_start + 10..];
            let end_idx = rest.find("## ").unwrap_or(rest.len());
            entity.summary = rest[..end_idx].trim().to_string();
        }

        Ok(Some(entity))
    }

    async fn save_entity(&self, entity: &Entity) -> Result<()> {
        let path = self.entity_path(&entity.id);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut md = String::new();
        md.push_str("---\n");
        md.push_str(&format!("id: {}\n", entity.id));
        if !entity.aliases.is_empty() {
            md.push_str("aliases:\n");
            for alias in &entity.aliases {
                md.push_str(&format!("  - {}\n", alias));
            }
        }
        md.push_str("---\n\n");

        md.push_str(&format!("# {}\n\n", entity.name));
        
        if !entity.summary.is_empty() {
            md.push_str("## Summary\n\n");
            md.push_str(&entity.summary);
            md.push_str("\n\n");
        }
        
        if !entity.links.is_empty() {
            md.push_str("## Knowledge Graph\n\n");
            for link in &entity.links {
                md.push_str(&format!("- {} -> [[{}]]\n", link.relation, link.target));
            }
            md.push('\n');
        }

        if !entity.sources.is_empty() {
            md.push_str("## Timeline\n\n");
            for source in &entity.sources {
                let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
                md.push_str(&format!("{}\nИзменено через: {:?}\n\n", date, source));
            }
        }

        if !entity.tags.is_empty() {
            md.push_str("## Tags\n\n");
            for tag in &entity.tags {
                md.push_str(&format!("#{}\n", tag));
            }
        }

        tokio::fs::write(&path, md).await?;
        tracing::info!("Saved entity {} to {:?}", entity.id, path);
        
        Ok(())
    }

    async fn list_entities(&self, filter_type: Option<EntityType>) -> Result<Vec<Entity>> {
        let dir = self.root.join("Entities");
        if !dir.exists() {
            return Ok(vec![]);
        }
        let mut entities = Vec::new();
        let mut entries = tokio::fs::read_dir(&dir).await.map_err(|e| BrainError::Vault(e.to_string()))?;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(Some(entity)) = self.get_entity(stem).await {
                        if let Some(ft) = &filter_type {
                            if &entity.entity_type == ft {
                                entities.push(entity);
                            }
                        } else {
                            entities.push(entity);
                        }
                    }
                }
            }
        }
        Ok(entities)
    }

    async fn delete_entity(&self, id: &str) -> Result<()> {
        let path = self.entity_path(id);
        if path.exists() {
            tokio::fs::remove_file(&path).await.map_err(|e| BrainError::Vault(e.to_string()))?;
        }
        Ok(())
    }
}
