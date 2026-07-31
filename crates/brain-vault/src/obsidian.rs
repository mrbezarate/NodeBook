//! ObsidianVault — реализация VaultStorage.
use async_trait::async_trait;
use brain_common::{BrainEntry, BrainError, Result, SearchResult, EntryId};
use brain_config::ParaConfig;
use brain_core::VaultStorage;
use crate::markdown::MarkdownBuilder;
use crate::para::VaultParaRouter;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct ObsidianVault { root: PathBuf, para_router: VaultParaRouter }

impl ObsidianVault {
    pub fn new(root: impl Into<PathBuf>, para_config: ParaConfig) -> Self {
        let root = root.into();
        let para_router = VaultParaRouter::new(para_config, root.clone());
        Self { root, para_router }
    }
}

#[async_trait]
impl VaultStorage for ObsidianVault {
    async fn write_entry(&self, entry: &BrainEntry) -> Result<String> {
        let path = self.para_router.build_path(
            &entry.classification.para_category,
            &entry.classification.area,
            &entry.classification.suggested_title,
        );
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let md = MarkdownBuilder::build(entry);
        tokio::fs::write(&path, md).await?;
        tracing::info!("Written: {}", path.display());
        Ok(path.to_string_lossy().to_string())
    }

    async fn read_entry(&self, path: &str) -> Result<String> {
        tokio::fs::read_to_string(path).await.map_err(|e| BrainError::Vault(e.to_string()))
    }

    async fn list_entries(&self, folder: &str) -> Result<Vec<String>> {
        let search_path = if folder.is_empty() { self.root.clone() } else { self.root.join(folder) };
        let entries: Vec<String> = WalkDir::new(search_path)
            .into_iter().filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .map(|e| e.path().to_string_lossy().to_string())
            .collect();
        Ok(entries)
    }

    async fn search_by_tag(&self, tag: &str) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let entries = self.list_entries("").await?;
        for path in entries {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if content.contains(&format!("- {}", tag)) || content.contains(&format!("#{}", tag)) {
                    results.push(SearchResult {
                        entry_id: EntryId::from_string(&path),
                        file_path: path.clone(),
                        title: Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
                        snippet: content.chars().take(200).collect(),
                        score: 1.0,
                    });
                }
            }
        }
        Ok(results)
    }

    async fn search_by_text(&self, query: &str) -> Result<Vec<SearchResult>> {
        let mut results = Vec::new();
        let lower_query = query.to_lowercase();
        let entries = self.list_entries("").await?;
        for path in entries {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if content.to_lowercase().contains(&lower_query) {
                    results.push(SearchResult {
                        entry_id: EntryId::from_string(&path),
                        file_path: path.clone(),
                        title: Path::new(&path).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
                        snippet: content.chars().take(200).collect(),
                        score: 0.8,
                    });
                }
            }
        }
        Ok(results)
    }

    async fn entry_exists(&self, title: &str) -> Result<bool> {
        let entries = self.list_entries("").await?;
        Ok(entries.iter().any(|p| p.contains(title)))
    }
}
