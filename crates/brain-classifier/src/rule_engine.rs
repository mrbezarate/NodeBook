//! Rule Engine — ОСНОВНОЙ классификатор. Чистый алгоритм.
use brain_common::EntryType;
use brain_parser::{Normalizer, PatternMatcher};

/// Результат правил.
pub struct RuleResult {
    pub entry_type: EntryType,
    pub confidence: f32,
}

/// Классификатор на основе правил (regex + ключевые слова).
pub struct RuleEngine;

impl RuleEngine {
    /// Классифицировать текст по правилам. Возвращает None если уверенность слишком низкая.
    pub fn classify(text: &str) -> Option<RuleResult> {
        let normalized = Normalizer::normalize(text);
        let best = PatternMatcher::best_match(&normalized)?;
        let entry_type = match best.0 {
            "task" => EntryType::Task,
            "idea" => EntryType::Idea,
            "goal" => EntryType::Goal,
            "diary" => EntryType::Diary,
            "finance" => EntryType::Finance,
            "knowledge" => EntryType::Knowledge,
            "habit" => EntryType::Habit,
            "person" => EntryType::Person,
            "book" => EntryType::Book,
            _ => return None,
        };
        Some(RuleResult { entry_type, confidence: best.1 })
    }
}
