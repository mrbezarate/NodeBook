//! Определение области знаний — ключевые слова → AI fallback.
use async_trait::async_trait;
use brain_common::{Area, EntryType, Result};
use brain_core::{AiProvider, AreaDetector};
use std::sync::Arc;

pub struct HybridAreaDetector { ai: Option<Arc<dyn AiProvider>> }

impl HybridAreaDetector {
    pub fn new(ai: Option<Arc<dyn AiProvider>>) -> Self { Self { ai } }

    fn detect_by_keywords(text: &str) -> Option<(Area, f32)> {
        let lower = text.to_lowercase();
        let map: &[(&[&str], Area, f32)] = &[
            (&["rust", "python", "код", "программ", "git", "api", "docker", "linux", "баг", "debug", "компилятор"], Area::Programming, 0.85),
            (&["здоровье", "сон", "тренировка", "спорт", "болезн", "врач", "диета", "фитнес"], Area::Health, 0.85),
            (&["учёба", "курс", "лекция", "универ", "экзамен", "обучение", "книга"], Area::Education, 0.80),
            (&["деньги", "зарплата", "бюджет", "инвест", "крипт", "банк", "кредит", "рубл", "$"], Area::Finance, 0.85),
            (&["работа", "карьер", "собеседован", "резюме", "проект", "дедлайн", "босс"], Area::Career, 0.80),
            (&["психолог", "тревог", "стресс", "медитац", "эмоци", "терапи"], Area::Psychology, 0.85),
            (&["игр", "unity", "unreal", "геймд", "game", "level"], Area::GameDev, 0.85),
            (&["отношен", "друг", "семья", "парень", "девушка", "жена", "муж"], Area::Relationships, 0.80),
        ];
        for (keywords, area, conf) in map {
            if keywords.iter().any(|kw| lower.contains(kw)) {
                return Some((area.clone(), *conf));
            }
        }
        None
    }
}

#[async_trait]
impl AreaDetector for HybridAreaDetector {
    async fn detect_area(&self, text: &str, _entry_type: &EntryType) -> Result<(Area, f32)> {
        if let Some(result) = Self::detect_by_keywords(text) { return Ok(result); }
        // AI fallback would go here
        Ok((Area::Life, 0.3))
    }
}
