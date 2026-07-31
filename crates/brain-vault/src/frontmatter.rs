//! YAML frontmatter для Obsidian.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    pub entry_type: String,
    pub area: String,
    pub para: String,
    pub tags: Vec<String>,
    pub created: String,
    pub modified: String,
    pub id: String,
    #[serde(default)]
    pub links: Vec<String>,
    #[serde(default)]
    pub source: String,
}

impl Frontmatter {
    /// Сгенерировать блок frontmatter (--- ... ---).
    pub fn to_yaml_block(&self) -> String {
        format!("---\n{}---\n", serde_yaml::to_string(self).unwrap_or_default())
    }

    /// Распарсить frontmatter из markdown текста.
    pub fn parse_from_markdown(text: &str) -> Option<Self> {
        let text = text.trim();
        if !text.starts_with("---") { return None; }
        let rest = &text[3..];
        let end = rest.find("---")?;
        let yaml = &rest[..end];
        serde_yaml::from_str(yaml).ok()
    }
}
