//! In-memory vector store с JSON persistence.
use brain_common::EntryId;
use serde::{Deserialize, Serialize};
use crate::similarity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEmbedding {
    pub id: EntryId,
    pub vector: Vec<f32>,
    pub file_path: String,
    pub title: String,
    pub text_preview: String,
}

/// In-memory хранилище векторов.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorStore {
    entries: Vec<StoredEmbedding>,
}

impl VectorStore {
    pub fn new() -> Self { Self { entries: Vec::new() } }

    pub fn add(&mut self, entry: StoredEmbedding) { self.entries.push(entry); }

    pub fn search(&self, query: &[f32], limit: usize, threshold: f32) -> Vec<(EntryId, f32, String)> {
        let mut results: Vec<_> = self.entries.iter()
            .map(|e| { let score = similarity::cosine_similarity(query, &e.vector); (e.id.clone(), score, e.title.clone()) })
            .filter(|(_, score, _)| *score >= threshold)
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    pub fn len(&self) -> usize { self.entries.len() }
    pub fn is_empty(&self) -> bool { self.entries.is_empty() }

    /// Сохранить в JSON файл.
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Загрузить из JSON файла.
    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}
