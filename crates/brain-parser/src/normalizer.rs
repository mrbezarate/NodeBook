//! Нормализация текста (чистый алгоритм).

/// Нормализатор текста.
pub struct Normalizer;

impl Normalizer {
    /// Полная нормализация: lowercase + пробелы + markdown + telegram.
    pub fn normalize(text: &str) -> String {
        let text = Self::to_lowercase(text);
        let text = Self::collapse_whitespace(&text);
        let text = Self::strip_markdown(&text);
        let text = Self::clean_telegram(&text);
        text.trim().to_string()
    }

    /// Привести к нижнему регистру.
    pub fn to_lowercase(text: &str) -> String {
        text.to_lowercase()
    }

    /// Схлопнуть множественные пробелы в один.
    pub fn collapse_whitespace(text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut prev_space = false;
        for ch in text.chars() {
            if ch.is_whitespace() {
                if !prev_space {
                    result.push(' ');
                    prev_space = true;
                }
            } else {
                result.push(ch);
                prev_space = false;
            }
        }
        result
    }

    /// Убрать markdown-форматирование (* _ ` #).
    pub fn strip_markdown(text: &str) -> String {
        text.replace("**", "")
            .replace("__", "")
            .replace("~~", "")
            .replace('`', "")
            .replace("```", "")
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                if trimmed.starts_with("# ") || trimmed.starts_with("## ") || trimmed.starts_with("### ") {
                    trimmed.trim_start_matches('#').trim().to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Убрать Telegram-специфичное форматирование.
    pub fn clean_telegram(text: &str) -> String {
        // Убираем /команды в начале строки
        if text.starts_with('/') {
            if let Some(rest) = text.split_once(' ') {
                return rest.1.to_string();
            }
        }
        text.to_string()
    }
}
