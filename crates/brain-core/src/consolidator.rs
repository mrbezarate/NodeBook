use std::sync::Arc;
use brain_common::{BrainError, Entity, Observation, RawEvent, Result, MatchMethod};
use crate::traits::{AiProvider, IdentityResolver, KnowledgeStore, ProjectionEngine, RawEventStore, Renderer};

use crate::extractor::Extractor;

pub struct Consolidator {
    extractor: Extractor,
    raw_event_store: Arc<dyn RawEventStore>,
    identity_resolver: Arc<dyn IdentityResolver>,
    projection_engine: Arc<dyn ProjectionEngine>,
    knowledge_store: Arc<dyn KnowledgeStore>,
    renderer: Arc<dyn Renderer>,
    output_sink: Option<Arc<dyn crate::traits::OutputSink>>,
}

impl Consolidator {
    pub fn new(
        ai_provider: Arc<dyn AiProvider>,
        raw_event_store: Arc<dyn RawEventStore>,
        identity_resolver: Arc<dyn IdentityResolver>,
        projection_engine: Arc<dyn ProjectionEngine>,
        knowledge_store: Arc<dyn KnowledgeStore>,
        renderer: Arc<dyn Renderer>,
        output_sink: Option<Arc<dyn crate::traits::OutputSink>>,
    ) -> Self {
        Self {
            extractor: Extractor::new(ai_provider),
            raw_event_store,
            identity_resolver,
            projection_engine,
            knowledge_store,
            renderer,
            output_sink,
        }
    }

    pub async fn run_pending_job(&self) -> Result<()> {
        let job_opt = self.raw_event_store.get_next_pending_job("consolidate").await?;
        if let Some(job) = job_opt {
            self.raw_event_store.update_job_status(&job.id, "running").await?;
            
            match self.process_job(&job.raw_event_id).await {
                Ok(_) => {
                    self.raw_event_store.update_job_status(&job.id, "completed").await?;
                }
                Err(e) => {
                    tracing::error!("Job failed: {}", e);
                    self.raw_event_store.update_job_status(&job.id, "failed").await?;
                }
            }
        }
        Ok(())
    }

    async fn process_job(&self, event_id: &str) -> Result<()> {
        let event = self.raw_event_store.get_raw_event(event_id).await?
            .ok_or_else(|| BrainError::Database("Event not found".into()))?;

        // 1. LLM Extractor
        let start_time = std::time::Instant::now();
        let structured_obs = match self.extractor.extract(&event.text).await {
            Ok(obs) => {
                let latency = start_time.elapsed().as_millis() as f64;
                let _ = self.raw_event_store.record_metric("latency_ms", latency, Some(&event_id)).await;
                let _ = self.raw_event_store.record_metric("json_parse_error", 0.0, Some(&event_id)).await;
                
                let is_empty = if obs.summary.is_empty() && obs.entities.is_empty() { 1.0 } else { 0.0 };
                let _ = self.raw_event_store.record_metric("extractor_empty_response", is_empty, Some(&event_id)).await;
                let _ = self.raw_event_store.record_metric("extractor_entities_count", obs.entities.len() as f64, Some(&event_id)).await;
                let _ = self.raw_event_store.record_metric("extractor_confidence", obs.confidence as f64, Some(&event_id)).await;
                
                tracing::info!(
                    latency_ms = latency,
                    confidence = obs.confidence,
                    entities_found = obs.entities.len(),
                    "Extractor completed successfully"
                );
                obs
            },
            Err(e) => {
                let latency = start_time.elapsed().as_millis() as f64;
                let _ = self.raw_event_store.record_metric("latency_ms", latency, Some(&event_id)).await;
                let _ = self.raw_event_store.record_metric("json_parse_error", 1.0, Some(&event_id)).await;
                
                tracing::error!(
                    latency_ms = latency,
                    error = %e,
                    "Extractor failed (JSON parse error or LLM failure)"
                );
                return Err(e);
            }
        };
        
        let fact_text = serde_json::to_string(&structured_obs).unwrap_or_else(|_| event.text.clone()); 

        // 2. Identity Resolver
        // Передаём каноническое имя первой извлечённой сущности, а не длинное summary
        let query_text = structured_obs.entities.first().unwrap_or(&structured_obs.summary);
        let resolution = self.identity_resolver.resolve(query_text).await?;
        
        let mut match_type = "identity_nomatch";
        let entity_id = match &resolution {
            Some(res) if res.confidence >= 0.85 => {
                match_type = match res.matched_by {
                    MatchMethod::Exact => "identity_exact",
                    MatchMethod::Alias => "identity_alias",
                    MatchMethod::Fuzzy => "identity_fuzzy",
                    MatchMethod::Semantic => "identity_semantic",
                };
                res.entity.id.clone()
            },
            _ => format!("entity_for_{}", event.id),
        };
        let _ = self.raw_event_store.record_metric(match_type, 1.0, Some(&event_id)).await;

        // 3. Сохраняем Observation в БД 
        let observation = Observation {
            id: format!("obs_{}", event.id),
            raw_event_id: event.id.clone(),
            entity_id: entity_id.clone(),
            fact: fact_text,
            confidence: structured_obs.confidence,
            schema_version: 1,
            extractor_version: "v1".to_string(),
        };
        self.raw_event_store.save_observation(&observation).await?;

        // 4. Projection Engine: пересчитываем Entity
        let entity = self.projection_engine.project(&entity_id).await?;

        // 5. Сохраняем Entity (опционально, так как это кэш/snapshot)
        self.knowledge_store.save_entity(&entity).await?;

        // 6. Renderer: обновляем Markdown
        self.renderer.render(&entity).await?;

        // 7. Output Delivery — одно финальное сообщение
        if let Some(sink) = &self.output_sink {
            let output = brain_common::output::Output::persistent_resource(
                format!("{}.md", entity.name)
            );
            let _ = sink.send(output).await;
        }

        Ok(())
    }
}
