use crate::traits::Plugin;
use crate::types::{PluginCommand, PluginManifest, PluginMessage, PluginResponse, PluginStatus};
use async_trait::async_trait;
use brain_common::{BrainError, Result};
use reqwest::Client;
use std::time::Duration;
use tracing::warn;

pub struct HttpPluginBridge {
    manifest: PluginManifest,
    client: Client,
    endpoint_url: String,
}

impl HttpPluginBridge {
    pub fn new(manifest: PluginManifest, endpoint_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            manifest,
            client,
            endpoint_url,
        }
    }
}

#[async_trait]
impl Plugin for HttpPluginBridge {
    fn manifest(&self) -> PluginManifest {
        self.manifest.clone()
    }

    async fn init(&self) -> Result<()> {
        let url = format!("{}/health", self.endpoint_url);
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => {
                warn!("Plugin {} health check failed: status {}", self.manifest.id, resp.status());
                Ok(())
            }
            Err(e) => {
                warn!("Plugin {} endpoint unreachable: {}", self.manifest.id, e);
                Ok(())
            }
        }
    }

    async fn handle_command(&self, cmd: &PluginCommand) -> Result<PluginResponse> {
        let url = format!("{}/api/command", self.endpoint_url);
        let resp = self
            .client
            .post(&url)
            .json(cmd)
            .send()
            .await
            .map_err(|e| BrainError::System(format!("External plugin request failed: {}", e)))?;

        if resp.status().is_success() {
            let res: PluginResponse = resp.json().await.map_err(|e| {
                BrainError::Serialization(format!("Invalid response from external plugin: {}", e))
            })?;
            Ok(res)
        } else {
            Ok(PluginResponse::Ignored)
        }
    }

    async fn handle_message(&self, msg: &PluginMessage) -> Result<PluginResponse> {
        let url = format!("{}/api/message", self.endpoint_url);
        let resp = self
            .client
            .post(&url)
            .json(msg)
            .send()
            .await
            .map_err(|e| BrainError::System(format!("External plugin request failed: {}", e)))?;

        if resp.status().is_success() {
            let res: PluginResponse = resp.json().await.map_err(|e| {
                BrainError::Serialization(format!("Invalid response from external plugin: {}", e))
            })?;
            Ok(res)
        } else {
            Ok(PluginResponse::Ignored)
        }
    }

    async fn status(&self) -> PluginStatus {
        let url = format!("{}/health", self.endpoint_url);
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => PluginStatus::Active,
            _ => PluginStatus::Offline,
        }
    }
}
