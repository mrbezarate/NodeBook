use anyhow::Result;
use brain_common::EntryId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Vector representation with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub entry_id: String,
    pub vector: Vec<f32>,
}

use tokio::sync::RwLock;

/// A lightweight, in-memory vector store using cosine similarity.
#[derive(Debug, Default)]
pub struct VectorStore {
    entries: RwLock<HashMap<String, VectorEntry>>,
    save_path: std::path::PathBuf,
}

impl VectorStore {
    pub fn new(save_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            save_path: save_path.into(),
        }
    }

    /// Load the vector store from a JSON file.
    pub async fn load(path: impl Into<std::path::PathBuf>) -> Result<Self> {
        let path = path.into();
        if path.exists() {
            let data = tokio::fs::read_to_string(&path).await?;
            let entries: HashMap<String, VectorEntry> = serde_json::from_str(&data)?;
            Ok(Self { entries: RwLock::new(entries), save_path: path })
        } else {
            Ok(Self::new(path))
        }
    }

    pub async fn save_to_disk(&self) -> Result<()> {
        if let Some(parent) = self.save_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let entries = self.entries.read().await;
        let data = serde_json::to_string_pretty(&*entries)?;
        tokio::fs::write(&self.save_path, data).await?;
        Ok(())
    }

    /// Upsert a vector into the store.
    pub async fn upsert(&self, entry_id: &str, vector: Vec<f32>) {
        let mut entries = self.entries.write().await;
        entries.insert(
            entry_id.to_string(),
            VectorEntry {
                entry_id: entry_id.to_string(),
                vector,
            },
        );
    }

    /// Search for the top-k most similar vectors using cosine similarity.
    pub async fn search(&self, query_vector: &[f32], limit: usize) -> Vec<(String, f32)> {
        let entries = self.entries.read().await;
        let mut results: Vec<(String, f32)> = entries
            .values()
            .map(|entry| {
                let score = cosine_similarity(query_vector, &entry.vector);
                (entry.entry_id.clone(), score)
            })
            // Filter out entries with no similarity (score > 0.0 or some threshold)
            .filter(|(_, score)| *score > 0.5) // basic threshold
            .collect();

        // Sort descending by score
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        results.into_iter().take(limit).collect()
    }
}

use async_trait::async_trait;
use brain_core::traits::VectorStorage;

#[async_trait]
impl VectorStorage for VectorStore {
    async fn upsert(&self, entry_id: &str, vector: Vec<f32>) -> brain_common::Result<()> {
        self.upsert(entry_id, vector).await;
        Ok(())
    }

    async fn search(&self, query_vector: &[f32], limit: usize) -> brain_common::Result<Vec<(String, f32)>> {
        Ok(self.search(query_vector, limit).await)
    }

    async fn get(&self, entry_id: &str) -> brain_common::Result<Option<Vec<f32>>> {
        let entries = self.entries.read().await;
        Ok(entries.get(entry_id).map(|e| e.vector.clone()))
    }

    async fn save(&self) -> brain_common::Result<()> {
        self.save_to_disk().await.map_err(|e| brain_common::BrainError::Vault(e.to_string()))
    }
}

/// Computes the cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for (v1, v2) in a.iter().zip(b.iter()) {
        dot_product += v1 * v2;
        norm_a += v1 * v1;
        norm_b += v2 * v2;
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot_product / (norm_a.sqrt() * norm_b.sqrt())
}
