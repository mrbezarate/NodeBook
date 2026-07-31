//! Извлечение данных из текста (даты, суммы, URL, время). Чистый алгоритм.

use regex::Regex;
use std::sync::LazyLock;

/// Извлечённые данные из текста.
#[derive(Debug, Clone, Default)]
pub struct Extracted {
    pub dates: Vec<String>,
    pub amounts: Vec<String>,
    pub urls: Vec<String>,
    pub mentions: Vec<String>,
    pub hashtags: Vec<String>,
    pub times: Vec<String>,
}

static RE_URL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://[^\s]+").unwrap()
});

static RE_AMOUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\d+[\s.,]?\d*)\s*(руб|₽|рублей|р\.|\$|usd|евро|€)").unwrap()
});

static RE_MENTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@(\w+)").unwrap()
});

static RE_HASHTAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"#(\w+)").unwrap()
});

static RE_TIME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:в\s+)?(\d{1,2}[:.]\d{2})|(?:через\s+(\d+)\s+(?:час|мин))").unwrap()
});

static RE_DATE_RU: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(завтра|послезавтра|вчера|сегодня|в понедельник|во вторник|в среду|в четверг|в пятницу|в субботу|в воскресенье|\d{1,2}\s+(?:января|февраля|марта|апреля|мая|июня|июля|августа|сентября|октября|ноября|декабря))").unwrap()
});

/// Экстрактор данных из текста.
pub struct Extractor;

impl Extractor {
    /// Извлечь все структурированные данные из текста.
    pub fn extract_all(text: &str) -> Extracted {
        Extracted {
            dates: Self::extract_dates(text),
            amounts: Self::extract_amounts(text),
            urls: Self::extract_urls(text),
            mentions: Self::extract_mentions(text),
            hashtags: Self::extract_hashtags(text),
            times: Self::extract_times(text),
        }
    }

    pub fn extract_dates(text: &str) -> Vec<String> {
        RE_DATE_RU.find_iter(text).map(|m| m.as_str().to_string()).collect()
    }

    pub fn extract_amounts(text: &str) -> Vec<String> {
        RE_AMOUNT.find_iter(text).map(|m| m.as_str().to_string()).collect()
    }

    pub fn extract_urls(text: &str) -> Vec<String> {
        RE_URL.find_iter(text).map(|m| m.as_str().to_string()).collect()
    }

    pub fn extract_mentions(text: &str) -> Vec<String> {
        RE_MENTION.captures_iter(text)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .collect()
    }

    pub fn extract_hashtags(text: &str) -> Vec<String> {
        RE_HASHTAG.captures_iter(text)
            .map(|c| c.get(1).unwrap().as_str().to_string())
            .collect()
    }

    pub fn extract_times(text: &str) -> Vec<String> {
        RE_TIME.find_iter(text).map(|m| m.as_str().to_string()).collect()
    }
}
