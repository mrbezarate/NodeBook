//! Токенизатор текста (русский/английский).

/// Результат токенизации.
#[derive(Debug, Clone)]
pub struct TokenizedText {
    pub words: Vec<String>,
    pub language: Language,
    pub urls: Vec<String>,
    pub hashtags: Vec<String>,
    pub mentions: Vec<String>,
}

/// Определённый язык текста.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    Russian,
    English,
    Mixed,
    Unknown,
}

/// Токенизатор текста.
pub struct Tokenizer;

impl Tokenizer {
    /// Токенизировать текст: разбить на слова, определить язык, извлечь URL/хештеги.
    pub fn tokenize(text: &str) -> TokenizedText {
        let words: Vec<String> = text
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| c.is_ascii_punctuation()).to_string())
            .filter(|w| !w.is_empty())
            .collect();

        let language = Self::detect_language(text);

        let urls: Vec<String> = words.iter()
            .filter(|w| w.starts_with("http://") || w.starts_with("https://"))
            .cloned()
            .collect();

        let hashtags: Vec<String> = words.iter()
            .filter(|w| w.starts_with('#') && w.len() > 1)
            .cloned()
            .collect();

        let mentions: Vec<String> = words.iter()
            .filter(|w| w.starts_with('@') && w.len() > 1)
            .cloned()
            .collect();

        TokenizedText { words, language, urls, hashtags, mentions }
    }

    /// Определить язык по наличию кириллических символов.
    pub fn detect_language(text: &str) -> Language {
        let total_alpha: usize = text.chars().filter(|c| c.is_alphabetic()).count();
        if total_alpha == 0 {
            return Language::Unknown;
        }
        let cyrillic: usize = text.chars().filter(|c| matches!(*c, '\u{0400}'..='\u{04FF}')).count();
        let ratio = cyrillic as f32 / total_alpha as f32;
        if ratio > 0.7 { Language::Russian }
        else if ratio < 0.3 { Language::English }
        else { Language::Mixed }
    }
}
