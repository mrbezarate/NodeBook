use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginCapability {
    MediaDownload,
    LanguageLearning,
    Custom(String),
}

impl fmt::Display for PluginCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MediaDownload => write!(f, "media-download"),
            Self::LanguageLearning => write!(f, "language-learning"),
            Self::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<PluginCapability>,
    pub author: String,
    pub is_external: bool,
    pub endpoint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMessage {
    pub message_id: String,
    pub user_id: u64,
    pub chat_id: i64,
    pub text: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub command: String,
    pub args: Vec<String>,
    pub user_id: u64,
    pub chat_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginResponse {
    Text(String),
    Media {
        title: String,
        file_path: String,
        caption: Option<String>,
    },
    Keyboard {
        text: String,
        options: Vec<(String, String)>, // (label, callback_data)
    },
    Handled,
    Ignored,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginStatus {
    Active,
    Disabled,
    Degraded,
    Offline,
}
