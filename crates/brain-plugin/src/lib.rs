pub mod bridge;
pub mod registry;
pub mod traits;
pub mod types;

pub use bridge::HttpPluginBridge;
pub use registry::PluginRegistry;
pub use traits::{Plugin, SharedPlugin};
pub use types::{
    PluginCapability, PluginCommand, PluginManifest, PluginMessage, PluginResponse, PluginStatus,
};
