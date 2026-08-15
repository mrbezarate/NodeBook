//! Доменные типы системы Brain.
//!
//! Все основные структуры данных: типы записей, области знаний,
//! PARA-категории, сущности, классификация, метрики дневника.

use crate::id::EntryId;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// ── Тип записи ──────────────────────────────────────────────

/// Тип записи — что это за заметка.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntryType {
    Idea,
    Project,
    Task,
    Goal,
    Knowledge,
    Thought,
    Diary,
    Person,
    Book,
    Article,
    Link,
    Quote,
    Habit,
    Problem,
    Solution,
    Finance,
}

impl fmt::Display for EntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Idea => "Идея",
            Self::Project => "Проект",
            Self::Task => "Задача",
            Self::Goal => "Цель",
            Self::Knowledge => "Знание",
            Self::Thought => "Мысль",
            Self::Diary => "Дневник",
            Self::Person => "Человек",
            Self::Book => "Книга",
            Self::Article => "Статья",
            Self::Link => "Ссылка",
            Self::Quote => "Цитата",
            Self::Habit => "Привычка",
            Self::Problem => "Проблема",
            Self::Solution => "Решение",
            Self::Finance => "Финансы",
        };
        write!(f, "{s}")
    }
}

impl EntryType {
    /// Все варианты типов для итерации.
    pub fn all() -> &'static [EntryType] {
        &[
            Self::Idea, Self::Project, Self::Task, Self::Goal,
            Self::Knowledge, Self::Thought, Self::Diary, Self::Person,
            Self::Book, Self::Article, Self::Link, Self::Quote,
            Self::Habit, Self::Problem, Self::Solution, Self::Finance,
        ]
    }
}

// ── Область знаний ──────────────────────────────────────────

/// Область знаний / домен.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Area {
    Programming,
    Health,
    Education,
    Finance,
    Career,
    Psychology,
    GameDev,
    Life,
    Relationships,
    Science,
    Art,
    Music,
    Custom(String),
}

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom(s) => write!(f, "{s}"),
            other => write!(f, "{other:?}"),
        }
    }
}

// ── PARA ────────────────────────────────────────────────────

/// Категория по системе PARA.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParaCategory {
    Projects,
    Areas,
    Resources,
    Archive,
    Inbox,
}

impl fmt::Display for ParaCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

// ── Сущности ────────────────────────────────────────────────

/// Тип извлечённой сущности.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Technology,
    Person,
    Place,
    Concept,
    Tool,
    Language,
    Framework,
    Project,
    Custom(String),
}

/// Извлечённая или хранящаяся сущность (базовый объект Knowledge Store).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub raw_event_id: String,
    pub entity_id: String,
    pub fact: String,
    pub confidence: f32,
    pub schema_version: u32,
    pub extractor_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,                 // Canonical ID (e.g. "project_space_cowboy")
    pub name: String,               // Display/Canonical Name
    pub aliases: Vec<String>,       // Known aliases
    pub entity_type: EntityType,
    pub area: Option<Area>,
    pub summary: String,
    pub tags: Vec<String>,
    pub links: Vec<SemanticLink>,
    pub sources: Vec<EntrySource>,  // Sources of Truth (where this info came from)
    pub observations: Vec<Observation>, // Атомарные факты
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchMethod {
    Exact,
    Alias,
    Fuzzy,
    Semantic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionResult {
    pub entity: Entity,
    pub confidence: f32,
    pub matched_by: MatchMethod,
}

impl Entity {
    pub fn new(name: &str, entity_type: EntityType) -> Self {
        let id = name.to_lowercase().replace(" ", "_");
        Self {
            id: format!("{:?}_{}", entity_type, id).to_lowercase(),
            name: name.to_string(),
            aliases: vec![],
            entity_type,
            area: None,
            summary: String::new(),
            tags: vec![],
            links: vec![],
            sources: vec![],
            observations: vec![],
        }
    }
}

// ── Семантические связи ─────────────────────────────────────

/// Семантическая связь между узлами (Semantic Graph Edge)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticLink {
    pub target: String,
    pub relation: String,
}

// ── Классификация ───────────────────────────────────────────

/// Результат классификации сообщения пользователя.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub entry_type: EntryType,
    pub area: Area,
    pub para_category: ParaCategory,
    pub entities: Vec<Entity>,
    pub tags: Vec<String>,
    pub confidence: f32,
    pub suggested_title: String,
    pub suggested_links: Vec<SemanticLink>,
    pub summary: String,
    pub enriched_text: Option<String>,
}

// ── Источник записи ─────────────────────────────────────────

/// Откуда пришла запись.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntrySource {
    Telegram { user_id: u64, message_id: i32, processing_msg_id: Option<i32> },
    Cli,
    Web,
    Import,
}

// ── Event Sourcing ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEvent {
    pub id: String,
    pub source_type: String, // e.g. "telegram"
    pub source_id: String,   // user_id
    pub external_id: Option<String>, // message_id
    pub payload: String,     // original JSON
    pub text: String,        // extracted text
    pub status: String,      // "pending", "processing", "completed", "failed"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub raw_event_id: String,
    pub job_type: String, // "consolidate"
    pub status: String,   // "pending", "running", "completed", "failed"
}

// ── Запись Brain ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourcingEvent {
    MessageIngested { text: String, source: EntrySource },
    LlmProcessRequested { text: String, source: EntrySource },
    EmbeddingProcessRequested { text: String },
    
    LlmProcessed { 
        title: Option<String>, 
        summary: String, 
        tags: Vec<String>, 
        enriched_text: Option<String>,
        #[serde(default)]
        area: Option<String>,
        #[serde(default)]
        para: Option<String>,
    },
    EmbeddingGenerated { vector_id: String },
    EntryStored { path: String },
    FallbackTriggered { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionEntry {
    pub id: String,
    pub title: String,
    pub raw: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub is_fallback: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Default for ProjectionEntry {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: "".to_string(),
            raw: "".to_string(),
            summary: "".to_string(),
            tags: vec![],
            is_fallback: false,
            created_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcingEventRecord {
    pub id: String,
    pub aggregate_id: String,
    pub event: SourcingEvent,
    pub created_at: DateTime<Utc>,
}

/// Обработанная запись, готовая к сохранению.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainEntry {
    pub id: EntryId,
    pub raw_text: String,
    pub classification: Classification,
    pub created_at: DateTime<Utc>,
    pub source: EntrySource,
}

impl BrainEntry {
    pub fn fallback(text: &str, source: EntrySource) -> Self {
        Self {
            id: EntryId::new(),
            raw_text: text.to_string(),
            classification: Classification {
                entry_type: EntryType::Idea,
                area: Area::Custom("Fallback".to_string()),
                para_category: ParaCategory::Inbox,
                entities: vec![],
                tags: vec![],
                confidence: 0.1,
                suggested_title: {
                    let preview: String = text.chars().take(20).collect();
                    format!("Unprocessed: {}", preview)
                },
                suggested_links: vec![],
                summary: "This entry was created as a fallback due to a pipeline failure.".to_string(),
                enriched_text: None,
            },
            created_at: Utc::now(),
            source,
        }
    }
}

// ── Embedding ───────────────────────────────────────────────

/// Векторное представление текста.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub id: EntryId,
    pub vector: Vec<f32>,
    pub text_preview: String,
}

// ── Результат поиска ────────────────────────────────────────

/// Результат поиска по базе знаний.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub entry_id: EntryId,
    pub file_path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

// ── Метрики дневника ────────────────────────────────────────

/// Данные вечернего обзора / дневника.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryMetrics {
    pub date: NaiveDate,
    pub day_rating: Option<u8>,
    pub mood: Option<u8>,
    pub energy: Option<u8>,
    pub stress: Option<u8>,
    pub motivation: Option<u8>,
    pub productivity: Option<u8>,
    pub sleep_hours: Option<f32>,
    pub exercise: Option<bool>,
    pub good_events: Option<String>,
    pub bad_events: Option<String>,
    pub free_thoughts: Option<String>,
}

impl DiaryMetrics {
    /// Создать пустые метрики для указанной даты.
    pub fn new(date: NaiveDate) -> Self {
        Self {
            date,
            day_rating: None,
            mood: None,
            energy: None,
            stress: None,
            motivation: None,
            productivity: None,
            sleep_hours: None,
            exercise: None,
            good_events: None,
            bad_events: None,
            free_thoughts: None,
        }
    }
}

/// Отчёт о системных метриках
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsReport {
    pub processed_events: i64,
    pub avg_latency_ms: f64,
    pub json_success_rate: f64,
    pub empty_responses_percent: f64,
    pub avg_entities_extracted: f64,
    pub avg_confidence: f64,
    
    pub identity_exact: i64,
    pub identity_alias: i64,
    pub identity_fuzzy: i64,
    pub identity_semantic: i64,
    pub identity_nomatch: i64,
    
    pub total_entities: i64,
    pub total_observations: i64,
    pub avg_obs_per_entity: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_unicode_safety() {
        let cyrillic_text = "Привет, это длинная тестовая строка на русском языке для проверки UTF-8 границы.";
        let entry = BrainEntry::fallback(cyrillic_text, EntrySource::Cli);
        assert!(entry.classification.suggested_title.starts_with("Unprocessed: Привет, это длинная"));

        let emoji_text = "🎉🚀🔥💡✨🌟⭐📝🔍📊🎯⚡😊😴🏃";
        let entry2 = BrainEntry::fallback(emoji_text, EntrySource::Cli);
        assert_eq!(entry2.classification.suggested_title, format!("Unprocessed: {}", emoji_text));
    }
}
