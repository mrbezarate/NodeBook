//! Парсинг ответов AI — обработка JSON и malformed output.
use brain_common::{BrainError, Result};

/// Распарсить ответ AI на запрос классификации.
pub fn parse_classification(response: &str, categories: &[&str]) -> Result<(String, f32)> {
    // Попытка 1: JSON
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(response) {
        if let (Some(cat), Some(conf)) = (v["category"].as_str(), v["confidence"].as_f64()) {
            return Ok((cat.to_string(), conf as f32));
        }
    }
    // Попытка 2: найти JSON в тексте
    if let Some(start) = response.find('{') {
        if let Some(end) = response[start..].find('}') {
            let json_str = &response[start..=start + end];
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let (Some(cat), Some(conf)) = (v["category"].as_str(), v["confidence"].as_f64()) {
                    return Ok((cat.to_string(), conf as f32));
                }
            }
        }
    }
    // Попытка 3: прямое сопоставление с категориями
    let lower = response.to_lowercase();
    for cat in categories {
        if lower.contains(&cat.to_lowercase()) {
            return Ok((cat.to_string(), 0.5));
        }
    }
    Err(BrainError::Ai(format!("Cannot parse AI response: {}", response)))
}
