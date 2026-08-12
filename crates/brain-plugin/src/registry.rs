use crate::traits::SharedPlugin;
use crate::types::{PluginCapability, PluginCommand, PluginManifest, PluginMessage, PluginResponse, PluginStatus};
use brain_common::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, SharedPlugin>>>,
    enabled: Arc<RwLock<HashMap<String, bool>>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            enabled: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, plugin: SharedPlugin) -> Result<()> {
        let manifest = plugin.manifest();
        let id = manifest.id.clone();

        plugin.init().await?;
        info!("Registered plugin: {} v{}", manifest.name, manifest.version);

        self.plugins.write().await.insert(id.clone(), plugin);
        self.enabled.write().await.insert(id, true);
        Ok(())
    }

    pub async fn set_enabled(&self, id: &str, is_enabled: bool) {
        if self.plugins.read().await.contains_key(id) {
            self.enabled.write().await.insert(id.to_string(), is_enabled);
            info!("Plugin {} enabled status set to {}", id, is_enabled);
        }
    }

    pub async fn is_enabled(&self, id: &str) -> bool {
        *self.enabled.read().await.get(id).unwrap_or(&false)
    }

    pub async fn list_manifests(&self) -> Vec<PluginManifest> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.manifest()).collect()
    }

    pub async fn get_status(&self, id: &str) -> Option<PluginStatus> {
        let plugins = self.plugins.read().await;
        if let Some(plugin) = plugins.get(id) {
            if !self.is_enabled(id).await {
                return Some(PluginStatus::Disabled);
            }
            Some(plugin.status().await)
        } else {
            None
        }
    }

    pub async fn dispatch_command(&self, cmd: &PluginCommand) -> Result<Option<PluginResponse>> {
        let plugins = self.plugins.read().await;
        for (id, plugin) in plugins.iter() {
            if !self.is_enabled(id).await {
                continue;
            }
            match plugin.handle_command(cmd).await {
                Ok(PluginResponse::Ignored) => continue,
                Ok(resp) => return Ok(Some(resp)),
                Err(e) => {
                    warn!("Error dispatching command to plugin {}: {}", id, e);
                }
            }
        }
        Ok(None)
    }

    pub async fn dispatch_message(&self, msg: &PluginMessage) -> Result<Option<PluginResponse>> {
        let plugins = self.plugins.read().await;
        for (id, plugin) in plugins.iter() {
            if !self.is_enabled(id).await {
                continue;
            }
            match plugin.handle_message(msg).await {
                Ok(PluginResponse::Ignored) => continue,
                Ok(resp) => return Ok(Some(resp)),
                Err(e) => {
                    warn!("Error dispatching message to plugin {}: {}", id, e);
                }
            }
        }
        Ok(None)
    }

    pub async fn dispatch_callback(&self, callback_data: &str, user_id: u64) -> Result<Option<PluginResponse>> {
        let plugins = self.plugins.read().await;
        for (id, plugin) in plugins.iter() {
            if !self.is_enabled(id).await {
                continue;
            }
            match plugin.handle_callback(callback_data, user_id).await {
                Ok(PluginResponse::Ignored) => continue,
                Ok(resp) => return Ok(Some(resp)),
                Err(e) => {
                    warn!("Error dispatching callback to plugin {}: {}", id, e);
                }
            }
        }
        Ok(None)
    }

    pub async fn find_by_capability(&self, cap: PluginCapability) -> Vec<SharedPlugin> {
        let plugins = self.plugins.read().await;
        let mut matches = Vec::new();
        for (id, plugin) in plugins.iter() {
            if self.is_enabled(id).await && plugin.manifest().capabilities.contains(&cap) {
                matches.push(plugin.clone());
            }
        }
        matches
    }
}
