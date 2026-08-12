use crate::types::{PluginCommand, PluginManifest, PluginMessage, PluginResponse, PluginStatus};
use async_trait::async_trait;
use brain_common::Result;
use std::sync::Arc;

#[async_trait]
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> PluginManifest;
    
    async fn init(&self) -> Result<()> {
        Ok(())
    }

    async fn handle_command(&self, cmd: &PluginCommand) -> Result<PluginResponse> {
        let _ = cmd;
        Ok(PluginResponse::Ignored)
    }

    async fn handle_message(&self, msg: &PluginMessage) -> Result<PluginResponse> {
        let _ = msg;
        Ok(PluginResponse::Ignored)
    }

    async fn handle_callback(&self, callback_data: &str, user_id: u64) -> Result<PluginResponse> {
        let _ = (callback_data, user_id);
        Ok(PluginResponse::Ignored)
    }

    async fn status(&self) -> PluginStatus {
        PluginStatus::Active
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

pub type SharedPlugin = Arc<dyn Plugin>;
