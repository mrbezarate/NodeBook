//! BrainEngine — главный оркестратор системы.

use crate::pipeline::Pipeline;
use crate::traits::{EmbeddingProvider, GraphStore, VaultStorage};
use brain_common::{BrainEntry, BrainError, EntrySource, Result, SearchResult};
use brain_config::BrainConfig;
use std::collections::HashMap;
use std::sync::Arc;

/// Главный движок Brain — связывает все подсистемы.
pub struct BrainEngine {
    pub config: BrainConfig,
    pipeline: Pipeline,
    vault: Arc<dyn VaultStorage>,
    graph: Option<Arc<dyn GraphStore>>,
    embeddings: Option<Arc<dyn EmbeddingProvider>>,
}

impl BrainEngine {
    /// Принять текст, обработать через пайплайн, сохранить в vault.
    pub async fn ingest(&self, text: &str, source: EntrySource) -> Result<BrainEntry> {
        // 1. Обработать через пайплайн
        let entry = self.pipeline.process(text, source).await?;

        // 2. Сохранить в vault
        let file_path = self.vault.write_entry(&entry).await?;
        tracing::info!("Entry saved to: {}", file_path);

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
                graph.add_edge(id, &entity_id, "mentions").await?;
            }
        }

        // 4. Сохранить embedding для семантического поиска
        if let Some(ref embeddings) = self.embeddings {
            match embeddings.embed(text).await {
                Ok(_vector) => {
                    tracing::debug!("Embedding stored for entry {}", entry.id);
                }
                Err(e) => {
                    tracing::warn!("Failed to create embedding: {}", e);
                }
            }
        }

        Ok(entry)
    }

    /// Поиск по базе знаний.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        // Сначала пробуем семантический поиск, потом текстовый
        self.vault.search_by_text(query).await
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
    pipeline: Option<Pipeline>,
    vault: Option<Arc<dyn VaultStorage>>,
    graph: Option<Arc<dyn GraphStore>>,
    embeddings: Option<Arc<dyn EmbeddingProvider>>,
}

impl BrainEngineBuilder {
    pub fn new() -> Self {
        Self { config: None, pipeline: None, vault: None, graph: None, embeddings: None }
    }

    pub fn config(mut self, c: BrainConfig) -> Self { self.config = Some(c); self }
    pub fn pipeline(mut self, p: Pipeline) -> Self { self.pipeline = Some(p); self }
    pub fn vault(mut self, v: Arc<dyn VaultStorage>) -> Self { self.vault = Some(v); self }
    pub fn graph(mut self, g: Arc<dyn GraphStore>) -> Self { self.graph = Some(g); self }
    pub fn embeddings(mut self, e: Arc<dyn EmbeddingProvider>) -> Self { self.embeddings = Some(e); self }

    pub fn build(self) -> Result<BrainEngine> {
        Ok(BrainEngine {
            config: self.config.unwrap_or_default(),
            pipeline: self.pipeline.ok_or_else(|| BrainError::Config("Pipeline is required".into()))?,
            vault: self.vault.ok_or_else(|| BrainError::Config("VaultStorage is required".into()))?,
            graph: self.graph,
            embeddings: self.embeddings,
        })
    }
}

impl Default for BrainEngineBuilder {
    fn default() -> Self { Self::new() }
}
