//! Все ключевые трейты системы Brain.
//!
//! Каждый трейт определяет один шаг обработки. Реализации подключаются
//! через dependency injection в Pipeline и BrainEngine.

use async_trait::async_trait;
use brain_common::{
    Area, BrainEntry, Classification, Entity, EntryType, ParaCategory, Result, SearchResult,
};

// ── Классификация (алгоритм + AI fallback) ──────────────────

/// Классифицирует текст в тип записи.
#[async_trait]
pub trait TypeClassifier: Send + Sync {
    async fn classify_type(&self, text: &str) -> Result<(EntryType, f32)>;
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
    async fn generate_tags(&self, text: &str, classification: &Classification) -> Result<Vec<String>>;
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
    async fn generate_title(&self, text: &str, entry_type: &EntryType) -> Result<String>;
}

/// Предлагает связи с существующими заметками.
#[async_trait]
pub trait LinkSuggester: Send + Sync {
    async fn suggest_links(&self, text: &str, limit: usize) -> Result<Vec<String>>;
}

// ── AI-провайдер ────────────────────────────────────────────

/// Абстракция AI-провайдера (Ollama, OpenAI, Noop).
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Завершить текст (generation).
    async fn complete(&self, prompt: &str) -> Result<String>;
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
