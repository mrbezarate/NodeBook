//! Хранение и управление базами данных / ваултами (Vault Registry).
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultInfo {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultRegistry {
    pub active_vault_id: String,
    pub vaults: Vec<VaultInfo>,
}

impl Default for VaultRegistry {
    fn default() -> Self {
        Self {
            active_vault_id: "base_1".to_string(),
            vaults: vec![VaultInfo {
                id: "base_1".to_string(),
                name: "base_1".to_string(),
                path: "./vaults/base_1".to_string(),
                created_at: Utc::now().to_rfc3339(),
            }],
        }
    }
}

impl VaultRegistry {
    pub fn load_or_create(registry_file: impl AsRef<Path>, default_path: &str) -> Self {
        let registry_path = registry_file.as_ref();
        if registry_path.exists() {
            if let Ok(content) = std::fs::read_to_string(registry_path) {
                if let Ok(registry) = serde_json::from_str::<VaultRegistry>(&content) {
                    return registry;
                }
            }
        }

        let default_vault_path = if default_path.is_empty() || default_path == "~/Obsidian/Brain" {
            "./vaults/base_1".to_string()
        } else {
            default_path.to_string()
        };

        let registry = VaultRegistry {
            active_vault_id: "base_1".to_string(),
            vaults: vec![VaultInfo {
                id: "base_1".to_string(),
                name: "base_1".to_string(),
                path: default_vault_path,
                created_at: Utc::now().to_rfc3339(),
            }],
        };

        let _ = registry.save(registry_path);
        registry
    }

    pub fn save(&self, registry_file: impl AsRef<Path>) -> std::io::Result<()> {
        let path = registry_file.as_ref();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }

    pub fn get_active_vault(&self) -> Option<&VaultInfo> {
        self.vaults.iter().find(|v| v.id == self.active_vault_id)
    }

    pub fn get_active_path(&self) -> String {
        self.get_active_vault()
            .map(|v| v.path.clone())
            .unwrap_or_else(|| "./vaults/base_1".to_string())
    }

    pub fn create_vault(&mut self, registry_file: impl AsRef<Path>, name: &str) -> VaultInfo {
        let count = self.vaults.len() + 1;
        let id = format!("base_{}", count);
        let safe_folder_name = name.chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect::<String>();
        let folder_name = if safe_folder_name.trim().is_empty() {
            format!("base_{}", count)
        } else {
            safe_folder_name
        };

        let path = format!("./vaults/{}", folder_name);
        
        let path_buf = PathBuf::from(&path);
        let _ = std::fs::create_dir_all(path_buf.join("001 Projects"));
        let _ = std::fs::create_dir_all(path_buf.join("002 Areas"));
        let _ = std::fs::create_dir_all(path_buf.join("003 Resources"));
        let _ = std::fs::create_dir_all(path_buf.join("004 Archive"));
        let _ = std::fs::create_dir_all(path_buf.join("000 Inbox"));
        let _ = std::fs::create_dir_all(path_buf.join("Daily"));

        let info = VaultInfo {
            id: id.clone(),
            name: name.to_string(),
            path,
            created_at: Utc::now().to_rfc3339(),
        };

        self.vaults.push(info.clone());
        self.active_vault_id = id;
        let _ = self.save(registry_file);
        info
    }

    pub fn rename_active_vault(&mut self, registry_file: impl AsRef<Path>, new_name: &str) -> bool {
        if let Some(v) = self.vaults.iter_mut().find(|v| v.id == self.active_vault_id) {
            v.name = new_name.to_string();
            let _ = self.save(registry_file);
            true
        } else {
            false
        }
    }

    pub fn switch_active_vault(&mut self, registry_file: impl AsRef<Path>, vault_id: &str) -> bool {
        if self.vaults.iter().any(|v| v.id == vault_id) {
            self.active_vault_id = vault_id.to_string();
            let _ = self.save(registry_file);
            true
        } else {
            false
        }
    }
}
