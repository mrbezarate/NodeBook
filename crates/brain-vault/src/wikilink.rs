//! Обработка [[wikilinks]].
use regex::Regex;
use std::sync::LazyLock;

static RE_WIKILINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").unwrap());

pub struct WikilinkProcessor;

impl WikilinkProcessor {
    /// Извлечь все [[wikilinks]] из текста.
    pub fn extract(text: &str) -> Vec<String> {
        RE_WIKILINK.captures_iter(text).map(|c| c[1].to_string()).collect()
    }

    /// Сгенерировать wikilinks из списка сущностей.
    pub fn from_entities(entities: &[String]) -> String {
        entities.iter().map(|e| format!("[[{}]]", e)).collect::<Vec<_>>().join(", ")
    }
}
