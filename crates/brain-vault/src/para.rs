//! PARA-маршрутизация. Чистый алгоритм — таблица правил.
use async_trait::async_trait;
use brain_common::{Area, EntryType, ParaCategory, Result};
use brain_config::ParaConfig;
use brain_core::ParaRouter;
use std::path::PathBuf;

pub struct VaultParaRouter { pub config: ParaConfig, pub vault_root: PathBuf }

impl VaultParaRouter {
    pub fn new(config: ParaConfig, vault_root: impl Into<PathBuf>) -> Self {
        Self { config, vault_root: vault_root.into() }
    }

    /// Построить полный путь к файлу: vault_root/para_folder/area/title.md
    pub fn build_path(&self, para: &ParaCategory, area: &Area, title: &str, entry_id: &str) -> PathBuf {
        let para_folder = match para {
            ParaCategory::Projects => &self.config.projects,
            ParaCategory::Areas => &self.config.areas,
            ParaCategory::Resources => &self.config.resources,
            ParaCategory::Archive => &self.config.archive,
            ParaCategory::Inbox => &self.config.inbox,
        };
        let safe_title: String = title.chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M");
        let id_prefix: String = entry_id.chars().take(6).collect();
        self.vault_root.join(para_folder).join(area.to_string()).join(format!("{}_{}_{}.md", timestamp, safe_title.trim(), id_prefix))
    }
}

#[async_trait]
impl ParaRouter for VaultParaRouter {
    async fn route(&self, entry_type: &EntryType, _area: &Area, _text: &str) -> Result<ParaCategory> {
        let category = match entry_type {
            EntryType::Project | EntryType::Task | EntryType::Goal => ParaCategory::Projects,
            EntryType::Habit | EntryType::Diary | EntryType::Finance => ParaCategory::Areas,
            EntryType::Knowledge | EntryType::Book | EntryType::Article
            | EntryType::Link | EntryType::Quote
            | EntryType::Idea | EntryType::Thought | EntryType::Problem
            | EntryType::Solution | EntryType::Person => ParaCategory::Resources,
        };
        Ok(category)
    }
}
