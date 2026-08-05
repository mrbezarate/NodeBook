//! BrainEngine — главный оркестратор системы.

use crate::agentic_pipeline::AgenticPipeline;
use crate::traits::{EmbeddingProvider, GraphStore, VaultStorage};
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
            
            let (source_type, source_id, external_id, proc_id) = match source {
                EntrySource::Telegram { user_id, message_id, processing_msg_id } => (
                    "telegram".to_string(), 
                    user_id.to_string(), 
                    Some(message_id.to_string()),
                    processing_msg_id
                ),
                EntrySource::Cli => ("cli".to_string(), "local".to_string(), None, None),
                EntrySource::Web => ("web".to_string(), "browser".to_string(), None, None),
                EntrySource::Import => ("import".to_string(), "file".to_string(), None, None),
            };

            let payload = serde_json::to_string(&serde_json::json!({
                "source_type": source_type,
                "source_id": source_id,
                "external_id": external_id,
                "text": text,
                "processing_msg_id": proc_id,
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

    pub async fn ingest(&self, text: &str, source: EntrySource) -> Result<(BrainEntry, String)> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let id_str = match &source {
            EntrySource::Telegram { message_id, .. } => message_id.to_string(),
            _ => hasher.finish().to_string(),
        };
        let hash_key = format!("{}_{}", id_str, hasher.finish());

        if let Some(ref store) = self.raw_event_store {
            let (source_type, source_id, external_id) = match &source {
                EntrySource::Telegram { user_id, message_id, .. } => (
                    "telegram".to_string(), 
                    user_id.to_string(), 
                    Some(message_id.to_string()),
                ),
                _ => ("other".to_string(), "local".to_string(), None),
            };
            let event = brain_common::RawEvent {
                id: hash_key.clone(),
                source_type,
                source_id,
                external_id,
                payload: "".to_string(),
                text: text.to_string(),
                status: "processed".to_string(),
            };
            if let Err(e) = store.save_raw_event(&event).await {
                tracing::warn!("Idempotency hit or DB error! Skipping duplicate ingestion for {}: {}", hash_key, e);
                return Err(brain_common::BrainError::Validation("Duplicate message (idempotency hit)".into()));
            }
        }
        let span = tracing::info_span!("ingest", message_id = %hash_key);
        let _enter = span.enter();

        let log_event = |aggregate_id: String, event: brain_common::SourcingEvent| {
            if let Some(ref store) = self.raw_event_store {
                let store = store.clone();
                let record = brain_common::SourcingEventRecord {
                    id: uuid::Uuid::new_v4().to_string(),
                    aggregate_id,
                    event,
                    created_at: chrono::Utc::now(),
                };
                tokio::spawn(async move {
                    if let Err(e) = store.append_audit_event(&record).await {
                        tracing::error!("Event append failed: {:?}", e);
                    }
                });
            }
        };

        log_event(hash_key.clone(), brain_common::SourcingEvent::MessageIngested { text: text.to_string(), source: source.clone() });
        log_event(hash_key.clone(), brain_common::SourcingEvent::LlmProcessRequested { text: text.to_string(), source: source.clone() });
        log_event(hash_key.clone(), brain_common::SourcingEvent::EmbeddingProcessRequested { text: text.to_string() });
        
        tracing::info!("Ingest async accepted: {}", text);
        
        // Return minimal entry for now to satisfy current handlers.
        Ok((brain_common::BrainEntry::fallback(text, source), hash_key))
    }

    /// Запуск асинхронных воркеров пайплайна
    pub fn start_workers(self: &Arc<Self>) {
        let engine = self.clone();
        tokio::spawn(async move {
            engine.worker_llm_processor().await;
        });

        let engine2 = self.clone();
        tokio::spawn(async move {
            engine2.worker_embeddings().await;
        });
        
        let engine3 = self.clone();
        tokio::spawn(async move {
            engine3.worker_storage().await;
        });

        let engine4 = self.clone();
        tokio::spawn(async move {
            engine4.worker_projection().await;
        });
    }

    async fn worker_llm_processor(&self) {
        loop {
            if let Some(ref store) = self.raw_event_store {
                match store.next_unprocessed_event("LlmProcessRequested").await {
                    Ok(Some(record)) => {
                        if let brain_common::SourcingEvent::LlmProcessRequested { text, source } = record.event {
                            let request_id = uuid::Uuid::new_v4().to_string();
                            tracing::info!(request_id = %request_id, aggregate_id = %record.aggregate_id, "start LLM request");
                            
                            // Context Builder: Production-grade Hybrid Retrieval
                            let mut prompt = text.clone();
                            let mut candidates = vec![];
                            if let Some(ref store) = self.raw_event_store {
                                let retriever = crate::retrieval::HybridRetriever::new(
                                    store.clone(),
                                    self.vector_store.clone(),
                                    self.embeddings.clone(),
                                );
                                
                                let (context_text, retrieved_cands) = retriever.retrieve_context(&text).await;
                                candidates = retrieved_cands;
                                if !context_text.is_empty() {
                                    prompt = format!("Context (Related Knowledge):\n{}\n\nUser Input:\n{}", context_text, prompt);
                                }
                            }

                            // Re-use pipeline
                            match tokio::time::timeout(std::time::Duration::from_secs(60), self.pipeline.process(&prompt, source)).await {
                                Ok(Ok(entry)) => {
                                    tracing::info!(request_id = %request_id, "LLM successfully processed");
                                    let mut final_summary = entry.classification.summary;
                                    
                                    // Apply Auto-Linking
                                    let mut final_enriched = text.clone();
                                    if !candidates.is_empty() {
                                        final_summary = crate::linking::auto_link(&final_summary, &candidates);
                                        final_enriched = crate::linking::auto_link(&final_enriched, &candidates);
                                    }

                                    let ev = brain_common::SourcingEvent::LlmProcessed { 
                                        summary: final_summary, 
                                        tags: entry.classification.tags,
                                        enriched_text: Some(final_enriched),
                                    };
                                    let rec = brain_common::SourcingEventRecord {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        aggregate_id: record.aggregate_id.clone(),
                                        event: ev,
                                        created_at: chrono::Utc::now(),
                                    };
                                    let _ = store.append_audit_event(&rec).await;
                                }
                                Ok(Err(e)) => {
                                    tracing::error!(request_id = %request_id, error = %e, "Pipeline process error");
                                    let ev = brain_common::SourcingEvent::FallbackTriggered { reason: format!("Pipeline failed: {}", e) };
                                    let rec = brain_common::SourcingEventRecord { id: uuid::Uuid::new_v4().to_string(), aggregate_id: record.aggregate_id.clone(), event: ev, created_at: chrono::Utc::now() };
                                    let _ = store.append_audit_event(&rec).await;
                                }
                                Err(e) => {
                                    tracing::error!(request_id = %request_id, error = %e, "Pipeline process timeout");
                                    let ev = brain_common::SourcingEvent::FallbackTriggered { reason: "Pipeline timeout (60s)".to_string() };
                                    let rec = brain_common::SourcingEventRecord { id: uuid::Uuid::new_v4().to_string(), aggregate_id: record.aggregate_id.clone(), event: ev, created_at: chrono::Utc::now() };
                                    let _ = store.append_audit_event(&rec).await;
                                }
                            }
                        }
                    }
                    Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
                    Err(e) => { tracing::error!("Worker error: {:?}", e); tokio::time::sleep(std::time::Duration::from_secs(1)).await; }
                }
            } else {
                break;
            }
        }
    }

    async fn worker_embeddings(&self) {
        loop {
            if let Some(ref store) = self.raw_event_store {
                match store.next_unprocessed_event("EmbeddingProcessRequested").await {
                    Ok(Some(record)) => {
                        if let brain_common::SourcingEvent::EmbeddingProcessRequested { text } = record.event {
                            if let Some(ref embeddings) = self.embeddings {
                                if let Ok(vector) = embeddings.embed(&text).await {
                                    if let Some(ref vs) = self.vector_store {
                                        let _ = vs.upsert(&record.aggregate_id, vector).await;
                                        let _ = vs.save().await;
                                    }
                                    let ev = brain_common::SourcingEvent::EmbeddingGenerated { vector_id: record.aggregate_id.clone() };
                                    let rec = brain_common::SourcingEventRecord { id: uuid::Uuid::new_v4().to_string(), aggregate_id: record.aggregate_id.clone(), event: ev, created_at: chrono::Utc::now() };
                                    let _ = store.append_audit_event(&rec).await;
                                }
                            }
                        }
                    }
                    Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
                    Err(e) => { tracing::error!("Worker error: {:?}", e); tokio::time::sleep(std::time::Duration::from_secs(1)).await; }
                }
            } else {
                break;
            }
        }
    }

    async fn worker_storage(&self) {
        loop {
            if let Some(ref store) = self.raw_event_store {
                // Here we can fetch FallbackTriggered or LlmProcessed
                // For simplicity, let's poll LlmProcessed and FallbackTriggered. We can do it by querying specific event types.
                let mut processed_id = None;
                if let Ok(Some(rec)) = store.next_unprocessed_event("LlmProcessed").await { processed_id = Some(rec.aggregate_id); }
                else if let Ok(Some(rec)) = store.next_unprocessed_event("FallbackTriggered").await { processed_id = Some(rec.aggregate_id); }
                
                if let Some(agg_id) = processed_id {
                    if let Ok(Some(entry)) = self.rebuild(&agg_id).await {
                        // Store it
                        if let Ok(path) = self.vault.write_entry(&entry).await {
                            tracing::info!("Async EventStored: {}", path);
                            let ev = brain_common::SourcingEvent::EntryStored { path };
                            let rec = brain_common::SourcingEventRecord { id: uuid::Uuid::new_v4().to_string(), aggregate_id: agg_id.clone(), event: ev, created_at: chrono::Utc::now() };
                            let _ = store.append_audit_event(&rec).await;
                        }
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            } else {
                break;
            }
        }
    }

    async fn worker_projection(&self) {
        loop {
            if let Some(ref store) = self.raw_event_store {
                match store.next_unprojected_event_any().await {
                    Ok(Some(record)) => {
                        let mut entry = store.load_projection(&record.aggregate_id).await
                            .unwrap_or_default()
                            .unwrap_or_else(|| {
                                let mut e = brain_common::ProjectionEntry::default();
                                e.id = record.aggregate_id.clone();
                                e
                            });

                        match record.event {
                            brain_common::SourcingEvent::MessageIngested { text, .. } => {
                                entry.raw = text;
                            }
                            brain_common::SourcingEvent::LlmProcessed { summary, tags, enriched_text: _ } => {
                                entry.summary = summary;
                                entry.tags = tags.clone();
                                
                                // Build Graph Links
                                for tag in tags {
                                    if let Ok(related) = store.find_by_tag(&tag).await {
                                        for other_id in related {
                                            if other_id != entry.id {
                                                let _ = store.create_link(&entry.id, &other_id).await;
                                                let _ = store.create_link(&other_id, &entry.id).await;
                                            }
                                        }
                                    }
                                }
                            }
                            brain_common::SourcingEvent::FallbackTriggered { .. } => {
                                entry.is_fallback = true;
                            }
                            _ => {}
                        }

                        let _ = store.save_projection(&entry).await;
                        let _ = store.mark_event_projected(&record.id).await;
                    }
                    Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
                    Err(e) => { tracing::error!("Projection worker error: {:?}", e); tokio::time::sleep(std::time::Duration::from_secs(1)).await; }
                }
            } else {
                break;
            }
        }
    }

    /// Быстрое чтение записи из Read Model (O(1))
    pub async fn get_entry(&self, id: &str) -> Result<Option<brain_common::ProjectionEntry>> {
        if let Some(ref store) = self.raw_event_store {
            store.load_projection(id).await
        } else {
            Ok(None)
        }
    }

    /// Rebuild BrainEntry from audit events
    pub async fn rebuild(&self, aggregate_id: &str) -> Result<Option<BrainEntry>> {
        if let Some(ref store) = self.raw_event_store {
            let events = store.load_audit_events(aggregate_id).await?;
            if events.is_empty() { return Ok(None); }
            
            let mut entry = BrainEntry::fallback("", brain_common::EntrySource::Telegram { user_id: 0, message_id: 0, processing_msg_id: None });
            entry.id = brain_common::EntryId::from_string(aggregate_id);
            let mut is_fallback = false;

            for record in events {
                match record.event {
                    brain_common::SourcingEvent::MessageIngested { text, source } => {
                        entry.raw_text = text;
                        entry.source = source;
                    }
                    brain_common::SourcingEvent::LlmProcessed { summary, tags, enriched_text } => {
                        entry.classification.summary = summary;
                        entry.classification.tags = tags;
                        entry.classification.enriched_text = enriched_text;
                    }
                    brain_common::SourcingEvent::FallbackTriggered { .. } => {
                        is_fallback = true;
                    }
                    brain_common::SourcingEvent::LlmProcessRequested { .. } | 
                    brain_common::SourcingEvent::EmbeddingProcessRequested { .. } | 
                    brain_common::SourcingEvent::EmbeddingGenerated { .. } |
                    brain_common::SourcingEvent::EntryStored { .. } => {}
                }
            }

            if is_fallback {
                entry.classification.area = brain_common::Area::Custom("Fallback".to_string());
            }

            Ok(Some(entry))
        } else {
            Ok(None)
        }
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

    pub async fn delete_entry(&self, file_path: &str) -> Result<()> {
        self.vault.delete_entry(file_path).await
    }

    pub async fn append_to_entry(&self, file_path: &str, text: &str) -> Result<()> {
        self.vault.append_to_entry(file_path, text).await
    }

    pub async fn find_path_by_id(&self, entry_id: &str) -> Result<String> {
        let results = self.vault.search_by_text(entry_id).await?;
        if let Some(res) = results.first() {
            Ok(res.file_path.clone())
        } else {
            Err(BrainError::Vault("Entry not found".into()))
        }
    }

    pub async fn read_entry(&self, file_path: &str) -> Result<String> {
        self.vault.read_entry(file_path).await
    }

    pub async fn save_direct(&self, title: &str, text: &str, area: &str, tags: Vec<String>) -> Result<String> {
        let entry = BrainEntry {
            id: brain_common::EntryId::new(),
            raw_text: text.to_string(),
            source: brain_common::EntrySource::Telegram { user_id: 0, message_id: 0, processing_msg_id: None },
            created_at: chrono::Utc::now(),
            classification: brain_common::Classification {
                entry_type: brain_common::EntryType::Knowledge,
                area: brain_common::Area::Custom(area.to_string()),
                para_category: brain_common::ParaCategory::Resources,
                suggested_title: title.to_string(),
                suggested_links: vec![],
                tags,
                entities: vec![],
                summary: "System generated".to_string(),
                enriched_text: Some(text.to_string()),
                confidence: 1.0,
            },
        };
        self.vault.write_entry(&entry).await
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
