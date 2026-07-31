//! # brain-vault — Obsidian Vault (PARA, Markdown, Frontmatter, Wikilinks).
pub mod entity_store;
pub mod frontmatter;
pub mod markdown;
pub mod obsidian;
pub mod para;
pub mod template;
pub mod wikilink;

pub use obsidian::ObsidianVault;
pub use para::VaultParaRouter;
pub use entity_store::EntityVault;
