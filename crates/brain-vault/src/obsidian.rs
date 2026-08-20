//! ObsidianVault — реализация VaultStorage.
use async_trait::async_trait;
use brain_common::{BrainEntry, BrainError, Result, SearchResult};
use brain_config::ParaConfig;
use brain_core::VaultStorage;
use crate::markdown::MarkdownBuilder;
use crate::para::VaultParaRouter;
use std::path::PathBuf;
use walkdir::WalkDir;
use std::sync::Arc;
use brain_indexer::BrainIndexer;

pub struct ObsidianVault { 
    root: PathBuf, 
    para_router: VaultParaRouter,
    indexer: Arc<BrainIndexer>,
}

impl ObsidianVault {
    pub fn new(root: impl Into<PathBuf>, para_config: ParaConfig) -> Self {
        let root = root.into();
        let para_router = VaultParaRouter::new(para_config, root.clone());
        let index_path = root.join(".brain_index");
        let indexer = Arc::new(BrainIndexer::new(Some(&index_path)).expect("Failed to init indexer"));
        Self { root, para_router, indexer }
    }
}

#[async_trait]
impl VaultStorage for ObsidianVault {
    async fn write_entry(&self, entry: &BrainEntry) -> Result<String> {
        let path = self.para_router.build_path(
            &entry.classification.para_category,
            &entry.classification.area,
            &entry.classification.suggested_title,
            entry.id.as_str(),
        );
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let md = MarkdownBuilder::build(entry);
        tokio::fs::write(&path, md).await?;
        
        // Add to full-text index
        let id_str = path.to_string_lossy().to_string();
        if let Err(e) = self.indexer.add_document(&id_str, &entry.classification.suggested_title, &entry.raw_text, &entry.classification.tags).await {
            tracing::warn!("Failed to index document {}: {}", id_str, e);
        }

        tracing::info!("Written: {}", path.display());
        Ok(id_str)
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
        let results = self.indexer.search(tag, 20).map_err(|e| BrainError::Vault(format!("Index search error: {}", e)))?;
        Ok(results)
    }

    async fn search_by_text(&self, query: &str) -> Result<Vec<SearchResult>> {
        let results = self.indexer.search(query, 20).map_err(|e| BrainError::Vault(format!("Index search error: {}", e)))?;
        Ok(results)
    }

    async fn entry_exists(&self, title: &str) -> Result<bool> {
        let entries = self.list_entries("").await?;
        Ok(entries.iter().any(|p| p.contains(title)))
    }

    async fn delete_entry(&self, file_path: &str) -> Result<()> {
        let path = if std::path::Path::new(file_path).is_absolute() {
            std::path::PathBuf::from(file_path)
        } else {
            self.root.join(file_path)
        };
        if path.exists() {
            tokio::fs::remove_file(path).await.map_err(|e| BrainError::Vault(e.to_string()))?;
        }
        Ok(())
    }

    async fn append_to_entry(&self, file_path: &str, text: &str) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .append(true)
            .open(file_path)
            .await
            .map_err(|e| BrainError::Vault(e.to_string()))?;
        let formatted = format!("\n\n---\n*Дополнение:*\n{}", text);
        file.write_all(formatted.as_bytes()).await.map_err(|e| BrainError::Vault(e.to_string()))?;
        Ok(())
    }
}
