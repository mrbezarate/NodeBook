//! BrainEngine — главный оркестратор системы.

use crate::agentic_pipeline::AgenticPipeline;
use crate::traits::{EmbeddingProvider, GraphStore, VaultStorage, VectorStorage};
use brain_common::{BrainEntry, BrainError, EntrySource, Result, SearchResult};
use brain_config::BrainConfig;
use std::collections::HashMap;
use std::sync::Arc;

/// Главный движок Brain — связывает все подсистемы.
pub struct BrainEngine {
    pub config: BrainConfig,
    pipeline: Arc<AgenticPipeline>,
    vault: Arc<dyn VaultStorage>,
    graph: Option<Arc<dyn GraphStore>>,
    embeddings: Option<Arc<dyn EmbeddingProvider>>,
    vector_store: Option<Arc<dyn crate::traits::VectorStorage>>,
    event_bus: Option<Arc<dyn brain_events::EventBus>>,
    context_manager: Option<Arc<dyn crate::traits::ContextManager>>,
    entity_validator: Option<Arc<dyn crate::traits::EntityValidator>>,
    identity_resolver: Option<Arc<dyn crate::traits::IdentityResolver>>,
    knowledge_store: Option<Arc<dyn crate::traits::KnowledgeStore>>,
    raw_event_store: Option<Arc<dyn crate::traits::RawEventStore>>,
}

impl BrainEngine {
    pub async fn ingest_raw_event(&self, text: &str, source: EntrySource) -> Result<String> {
        if let Some(ref store) = self.raw_event_store {
            let event_id = uuid::Uuid::new_v4().to_string();
            
            let (source_type, source_id, external_id) = match source {
                EntrySource::Telegram { user_id, message_id } => (
                    "telegram".to_string(), 
                    user_id.to_string(), 
                    Some(message_id.to_string())
                ),
                EntrySource::Cli => ("cli".to_string(), "local".to_string(), None),
                EntrySource::Web => ("web".to_string(), "browser".to_string(), None),
                EntrySource::Import => ("import".to_string(), "file".to_string(), None),
            };

            let payload = serde_json::to_string(&serde_json::json!({
                "source_type": source_type,
                "source_id": source_id,
                "external_id": external_id,
                "text": text,
            })).unwrap_or_default();

            let event = brain_common::RawEvent {
                id: event_id.clone(),
                source_type,
                source_id,
                external_id,
                payload,
                text: text.to_string(),
                status: "pending".to_string(),
            };

            store.save_raw_event(&event).await?;

            let job_id = uuid::Uuid::new_v4().to_string();
            let job = brain_common::Job {
                id: job_id.clone(),
                raw_event_id: event_id.clone(),
                job_type: "consolidate".to_string(),
                status: "pending".to_string(),
            };

            store.create_job(&job).await?;
            
            Ok(event_id)
        } else {
            Err(BrainError::Config("RawEventStore is not configured".to_string()))
        }
    }

    pub async fn ingest(&self, text: &str, source: EntrySource) -> Result<BrainEntry> {
        // 1. Обработать через пайплайн (Agentic Reasoner)
        let mut entry = self.pipeline.process(text, source.clone()).await?;

        // 1.5. Knowledge Validation (Validator)
        if let Some(ref validator) = self.entity_validator {
            let mut validated_entities = Vec::new();
            for entity in entry.classification.entities {
                if let Ok(valid_entity) = validator.validate_entity(entity).await {
                    validated_entities.push(valid_entity);
                }
            }
            entry.classification.entities = validated_entities;
        }

        // 1.5. Identity Resolution
        let mut resolved_entity = None;
        if let Some(ref resolver) = self.identity_resolver {
            if let Ok(Some(resolution)) = resolver.resolve(&entry.classification.suggested_title).await {
                // Детерминированное решение на основе confidence
                if resolution.confidence >= 0.85 {
                    resolved_entity = Some(resolution.entity);
                }
            }
        }

        // 2. Обновление Knowledge Store
        if let Some(ref store) = self.knowledge_store {
            let entity = if let Some(mut existing) = resolved_entity {
                // Сливаем новую информацию со старой
                if existing.summary.is_empty() {
                    existing.summary = entry.classification.summary.clone();
                } else {
                    existing.summary = format!("{}\n\nНовое обновление:\n{}", existing.summary, entry.classification.summary);
                }
                existing.sources.push(source.clone());
                for tag in &entry.classification.tags {
                    if !existing.tags.contains(tag) { existing.tags.push(tag.clone()); }
                }
                for link in &entry.classification.suggested_links {
                    existing.links.push(link.clone()); // TODO: check duplicates
                }
                existing
            } else {
                // Создаем новую сущность
                // Используем Project по умолчанию, если это не Concept
                let mut new_entity = brain_common::Entity::new(&entry.classification.suggested_title, brain_common::EntityType::Concept);
                new_entity.summary = entry.classification.summary.clone();
                new_entity.area = Some(entry.classification.area.clone());
                new_entity.tags = entry.classification.tags.clone();
                new_entity.links = entry.classification.suggested_links.clone();
                new_entity.sources.push(source.clone());
                new_entity
            };

            store.save_entity(&entity).await?;
            tracing::info!("Knowledge Entity saved: {}", entity.id);
        }

        // 3. Сохранить raw-событие в vault (Working Memory)
        let file_path = self.vault.write_entry(&entry).await?;
        tracing::info!("Raw Event saved to: {}", file_path);

        // 3. Обновить граф знаний
        if let Some(ref graph) = self.graph {
            let id = entry.id.as_str();
            let label = &entry.classification.suggested_title;
            let node_type = format!("{:?}", entry.classification.entry_type);
            graph.add_node(id, label, &node_type).await?;

            // Связи с сущностями
            for entity in &entry.classification.entities {
                let entity_id = format!("entity:{}", entity.name);
                graph.add_node(&entity_id, &entity.name, &format!("{:?}", entity.entity_type)).await?;
                graph.add_edge(id, &entity_id, "MentionsEntity").await?;
            }

            // Семантические связи (из RAG/LLM)
            for link in &entry.classification.suggested_links {
                let link_id = format!("concept:{}", link.target);
                graph.add_node(&link_id, &link.target, "Concept").await?;
                graph.add_edge(id, &link_id, &link.relation).await?;
            }
        }

        // 4. Сохранить embedding для семантического поиска
        if let Some(ref embeddings) = self.embeddings {
            match embeddings.embed(text).await {
                Ok(vector) => {
                    tracing::debug!("Embedding generated for entry {}", entry.id);
                    if let Some(ref vs) = self.vector_store {
                        if let Err(e) = vs.upsert(entry.id.as_str(), vector).await {
                            tracing::warn!("Failed to save vector to store: {}", e);
                        } else {
                            // Automatically save the vector store
                            let _ = vs.save().await;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to create embedding: {}", e);
                }
            }
        }

        if let Some(ref bus) = self.event_bus {
            let _ = bus.emit(brain_events::EventType::EntryCreated {
                entry_id: entry.id.to_string(),
                title: entry.classification.suggested_title.clone(),
                tags: entry.classification.tags.clone(),
            }).await;
            
            if !entry.classification.entities.is_empty() {
                let _ = bus.emit(brain_events::EventType::EntitiesExtracted {
                    entry_id: entry.id.to_string(),
                    entities: entry.classification.entities.iter().map(|e| e.name.clone()).collect(),
                }).await;
            }
        }
        
        Ok(entry)
    }

    /// Поиск по базе знаний.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let results = self.vault.search_by_text(query).await?;
        
        if let Some(ref bus) = self.event_bus {
            let _ = bus.emit(brain_events::EventType::SearchPerformed {
                query: query.to_string(),
                results_count: results.len(),
            }).await;
        }
        
        Ok(results)
    }

    /// Получить статистику системы.
    pub async fn get_stats(&self) -> Result<Stats> {
        let entries = self.vault.list_entries("").await?;
        Ok(Stats {
            total_entries: entries.len(),
            entries_by_type: HashMap::new(),
            entries_by_area: HashMap::new(),
        })
    }

    pub async fn get_debug_trace(&self, event_id: &str) -> Result<String> {
        if let Some(ref store) = self.raw_event_store {
            store.get_debug_trace(event_id).await
        } else {
            Err(BrainError::Config("RawEventStore is not configured".to_string()))
        }
    }

    pub async fn get_metrics_report(&self) -> Result<brain_common::SystemMetricsReport> {
        if let Some(ref store) = self.raw_event_store {
            store.get_metrics_report().await
        } else {
            Err(BrainError::Config("RawEventStore is not configured".to_string()))
        }
    }

    pub async fn rebuild_from_event(&self, event_id: &str) -> Result<()> {
        if let Some(ref store) = self.raw_event_store {
            store.reset_event_processing(event_id).await
        } else {
            Err(BrainError::Config("RawEventStore is not configured".to_string()))
        }
    }
}

/// Статистика системы.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    pub total_entries: usize,
    pub entries_by_type: HashMap<String, usize>,
    pub entries_by_area: HashMap<String, usize>,
}

/// Builder для BrainEngine.
pub struct BrainEngineBuilder {
    config: Option<BrainConfig>,
    pipeline: Option<Arc<AgenticPipeline>>,
    vault: Option<Arc<dyn VaultStorage>>,
    graph: Option<Arc<dyn GraphStore>>,
    embeddings: Option<Arc<dyn EmbeddingProvider>>,
    vector_store: Option<Arc<dyn crate::traits::VectorStorage>>,
    event_bus: Option<Arc<dyn brain_events::EventBus>>,
    context_manager: Option<Arc<dyn crate::traits::ContextManager>>,
    entity_validator: Option<Arc<dyn crate::traits::EntityValidator>>,
    identity_resolver: Option<Arc<dyn crate::traits::IdentityResolver>>,
    knowledge_store: Option<Arc<dyn crate::traits::KnowledgeStore>>,
    raw_event_store: Option<Arc<dyn crate::traits::RawEventStore>>,
}

impl BrainEngineBuilder {
    pub fn new() -> Self {
        Self { 
            config: None, pipeline: None, vault: None, graph: None, embeddings: None, 
            vector_store: None, event_bus: None, context_manager: None, entity_validator: None,
            identity_resolver: None, knowledge_store: None, raw_event_store: None,
        }
    }

    pub fn config(mut self, c: BrainConfig) -> Self { self.config = Some(c); self }
    pub fn pipeline(mut self, p: Arc<AgenticPipeline>) -> Self { self.pipeline = Some(p); self }
    pub fn vault(mut self, v: Arc<dyn VaultStorage>) -> Self { self.vault = Some(v); self }
    pub fn graph(mut self, g: Arc<dyn GraphStore>) -> Self { self.graph = Some(g); self }
    pub fn embeddings(mut self, e: Arc<dyn EmbeddingProvider>) -> Self { self.embeddings = Some(e); self }
    pub fn vector_store(mut self, vs: Arc<dyn crate::traits::VectorStorage>) -> Self { self.vector_store = Some(vs); self }
    pub fn event_bus(mut self, eb: Arc<dyn brain_events::EventBus>) -> Self { self.event_bus = Some(eb); self }
    pub fn context_manager(mut self, cm: Arc<dyn crate::traits::ContextManager>) -> Self { self.context_manager = Some(cm); self }
    pub fn entity_validator(mut self, ev: Arc<dyn crate::traits::EntityValidator>) -> Self { self.entity_validator = Some(ev); self }
    pub fn identity_resolver(mut self, resolver: Arc<dyn crate::traits::IdentityResolver>) -> Self { 
        self.identity_resolver = Some(resolver); 
        self 
    }
    
    pub fn with_raw_event_store(mut self, store: Arc<dyn crate::traits::RawEventStore>) -> Self {
        self.raw_event_store = Some(store);
        self
    }

    pub fn with_knowledge_store(mut self, store: Arc<dyn crate::traits::KnowledgeStore>) -> Self { self.knowledge_store = Some(store); self }

    pub fn build(self) -> Result<BrainEngine> {
        Ok(BrainEngine {
            config: self.config.unwrap_or_default(),
            pipeline: self.pipeline.ok_or_else(|| BrainError::Config("Pipeline is required".into()))?,
            vault: self.vault.ok_or_else(|| BrainError::Config("VaultStorage is required".into()))?,
            graph: self.graph,
            embeddings: self.embeddings,
            vector_store: self.vector_store,
            event_bus: self.event_bus,
            context_manager: self.context_manager,
            entity_validator: self.entity_validator,
            identity_resolver: self.identity_resolver,
            knowledge_store: self.knowledge_store,
            raw_event_store: self.raw_event_store,
        })
    }
}

impl Default for BrainEngineBuilder {
    fn default() -> Self { Self::new() }
}
