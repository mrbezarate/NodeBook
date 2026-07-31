//! Все ключевые трейты системы Brain.
//!
//! Каждый трейт определяет один шаг обработки. Реализации подключаются
//! через dependency injection в Pipeline и BrainEngine.

use async_trait::async_trait;
use brain_common::{
    Area, BrainEntry, Classification, Entity, EntityType, EntryType, Observation, ParaCategory, ResolutionResult, Result, SearchResult, RawEvent, Job
};

/// The assembled context for a given input text.
#[derive(Debug, Clone, Default)]
pub struct ActiveContext {
    pub semantic_results: Vec<(String, f32)>,
    pub keyword_results: Vec<SearchResult>,
}

impl ActiveContext {
    pub fn to_prompt_string(&self) -> String {
        let mut out = String::new();
        if !self.semantic_results.is_empty() || !self.keyword_results.is_empty() {
            out.push_str("\n--- ACTIVE CONTEXT (RELEVANT PAST KNOWLEDGE) ---\n");
            for (id, score) in &self.semantic_results {
                out.push_str(&format!("- Related Concept (Score: {:.2}): {}\n", score, id));
            }
            for res in &self.keyword_results {
                out.push_str(&format!("- Keyword Match: {} (Relevance: {:.2})\n", res.title, res.score));
            }
            out.push_str("--------------------------------------------------\n");
        }
        out
    }
}

/// Gathers context from various storages.
#[async_trait]
pub trait ContextManager: Send + Sync {
    async fn gather_context(&self, text: &str) -> Result<ActiveContext>;
}

// ── Классификация (алгоритм + AI fallback) ──────────────────

/// Классифицирует текст в тип записи.
#[async_trait]
pub trait TypeClassifier: Send + Sync {
    async fn classify_type(&self, text: &str, context: &str) -> Result<(EntryType, f32)>;
}

/// Определяет область знаний из текста.
#[async_trait]
pub trait AreaDetector: Send + Sync {
    async fn detect_area(&self, text: &str, entry_type: &EntryType) -> Result<(Area, f32)>;
}

/// Извлекает именованные сущности из текста.
#[async_trait]
pub trait EntityExtractor: Send + Sync {
    async fn extract_entities(&self, text: &str) -> Result<Vec<Entity>>;
}

/// Генерирует теги из текста и классификации.
#[async_trait]
pub trait TagGenerator: Send + Sync {
    async fn generate_tags(&self, text: &str, classification: &Classification, context: &str) -> Result<Vec<String>>;
}

// ── Маршрутизация (чистый алгоритм) ─────────────────────────

/// Маршрутизирует записи в категории PARA.
#[async_trait]
pub trait ParaRouter: Send + Sync {
    async fn route(&self, entry_type: &EntryType, area: &Area, text: &str) -> Result<ParaCategory>;
}

/// Генерирует заголовок для заметки.
#[async_trait]
pub trait TitleGenerator: Send + Sync {
    async fn generate_title(&self, text: &str, entry_type: &EntryType, context: &str) -> Result<String>;
}

/// Предлагает связи с существующими заметками.
#[async_trait]
pub trait LinkSuggester: Send + Sync {
    async fn suggest_links(&self, text: &str, limit: usize, context: &str) -> Result<Vec<brain_common::SemanticLink>>;
}

// ── Reasoner / Validator ─────────────────────────────────────

#[async_trait]
pub trait EntityValidator: Send + Sync {
    async fn validate_entity(&self, entity: Entity) -> Result<Entity>;
}

/// Сервис Хранилища Знаний (Knowledge Store).
/// Оперирует сущностями (Entity), а не заметками.
#[async_trait]
pub trait KnowledgeStore: Send + Sync {
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>>;
    async fn save_entity(&self, entity: &Entity) -> Result<()>;
    async fn list_entities(&self, filter_type: Option<EntityType>) -> Result<Vec<Entity>>;
}

/// Сервис разрешения сущностей (Identity Resolver).
#[async_trait]
pub trait RawEventStore: Send + Sync {
    async fn save_raw_event(&self, event: &RawEvent) -> Result<()>;
    async fn create_job(&self, job: &Job) -> Result<()>;
    async fn get_next_pending_job(&self, job_type: &str) -> Result<Option<Job>>;
    async fn update_job_status(&self, job_id: &str, status: &str) -> Result<()>;
    async fn get_raw_event(&self, event_id: &str) -> Result<Option<RawEvent>>;
    async fn save_observation(&self, observation: &Observation) -> Result<()>;
    async fn get_observations(&self, entity_id: &str) -> Result<Vec<Observation>>;
    async fn get_debug_trace(&self, event_id: &str) -> Result<String>;
    async fn record_metric(&self, name: &str, value: f64, event_id: Option<&str>) -> Result<()>;
    async fn get_metrics_report(&self) -> Result<brain_common::SystemMetricsReport>;
    async fn reset_event_processing(&self, event_id: &str) -> Result<()>;
}

/// Позволяет найти существующую сущность по имени/алиасу.
#[async_trait]
pub trait IdentityResolver: Send + Sync {
    /// Попытаться свести имя к существующей сущности с возвратом уверенности
    async fn resolve(&self, query: &str) -> Result<Option<ResolutionResult>>;
    async fn register_alias(&self, canonical_id: &str, alias: &str) -> Result<()>;
}

/// Вычисляет Entity Snapshot из набора Observations.
#[async_trait]
pub trait ProjectionEngine: Send + Sync {
    async fn project(&self, entity_id: &str) -> Result<Entity>;
}

/// Отвечает исключительно за отображение Entity Snapshot (например, в Markdown для Obsidian).
#[async_trait]
pub trait Renderer: Send + Sync {
    async fn render(&self, entity: &Entity) -> Result<()>;
}

// ── AI-провайдер ────────────────────────────────────────────

/// Абстракция AI-провайдера (Ollama, OpenAI, Noop).
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Завершить текст (generation).
    async fn complete(&self, prompt: &str) -> Result<String>;
    /// Завершить текст с требованием вернуть JSON.
    async fn complete_json(&self, prompt: &str) -> Result<String>;
    /// Классифицировать текст по категориям.
    async fn classify(&self, text: &str, categories: &[&str]) -> Result<(String, f32)>;
}

/// Абстракция для создания embeddings.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Получить embedding для одного текста.
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    /// Получить embeddings для пакета текстов.
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}

/// Хранилище векторов для семантического поиска.
#[async_trait]
pub trait VectorStorage: Send + Sync {
    async fn upsert(&self, entry_id: &str, vector: Vec<f32>) -> Result<()>;
    async fn search(&self, query_vector: &[f32], limit: usize) -> Result<Vec<(String, f32)>>;
    async fn save(&self) -> Result<()>;
}

/// Абстракция доставки исходящих сообщений платформы
#[async_trait]
pub trait OutputSink: Send + Sync {
    async fn send(&self, output: brain_common::output::Output) -> Result<()>;
}

// ── Хранилище ───────────────────────────────────────────────

/// Абстракция хранилища Obsidian Vault.
#[async_trait]
pub trait VaultStorage: Send + Sync {
    /// Записать обработанную запись в vault.
    async fn write_entry(&self, entry: &BrainEntry) -> Result<String>;
    /// Прочитать содержимое файла.
    async fn read_entry(&self, path: &str) -> Result<String>;
    /// Список файлов в папке.
    async fn list_entries(&self, folder: &str) -> Result<Vec<String>>;
    /// Поиск по тегу.
    async fn search_by_tag(&self, tag: &str) -> Result<Vec<SearchResult>>;
    /// Полнотекстовый поиск.
    async fn search_by_text(&self, query: &str) -> Result<Vec<SearchResult>>;
    /// Проверить существование файла.
    async fn entry_exists(&self, title: &str) -> Result<bool>;
}

/// Операции с графом знаний.
#[async_trait]
pub trait GraphStore: Send + Sync {
    /// Добавить узел в граф.
    async fn add_node(&self, id: &str, label: &str, node_type: &str) -> Result<()>;
    /// Добавить связь между узлами.
    async fn add_edge(&self, from: &str, to: &str, relation: &str) -> Result<()>;
    /// Получить соседние узлы.
    async fn get_neighbors(&self, id: &str) -> Result<Vec<String>>;
    /// Найти путь между узлами.
    async fn find_path(&self, from: &str, to: &str) -> Result<Vec<String>>;
}
