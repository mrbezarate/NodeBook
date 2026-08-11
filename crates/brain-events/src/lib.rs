use brain_common::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

/// Типы событий в системе
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    /// Новая запись создана и сохранена
    EntryCreated { entry_id: String, title: String, tags: Vec<String> },
    /// Выполнен поисковой запрос
    SearchPerformed { query: String, results_count: usize },
    /// Добавлены сущности или связи в граф
    EntitiesExtracted { entry_id: String, entities: Vec<String> },
    /// Ошибка в системе
    SystemError { component: String, message: String },
}

/// Базовое событие с метаданными
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
}

impl Event {
    pub fn new(event_type: EventType) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
        }
    }
}

/// Трейт для шины событий
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn emit(&self, event_type: EventType) -> Result<()>;
}

/// Реализация EventBus, сохраняющая события в append-only JSONL файл.
pub struct JsonlEventLogger {
    log_file: PathBuf,
    writer: Mutex<Option<tokio::io::BufWriter<tokio::fs::File>>>,
}

impl JsonlEventLogger {
    pub async fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let log_file = path.into();
        if let Some(parent) = log_file.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .await?;
            
        Ok(Self {
            log_file,
            writer: Mutex::new(Some(tokio::io::BufWriter::new(file))),
        })
    }

    pub fn log_file(&self) -> &std::path::Path {
        &self.log_file
    }
}

#[async_trait]
impl EventBus for JsonlEventLogger {
    async fn emit(&self, event_type: EventType) -> brain_common::Result<()> {
        let event = Event::new(event_type);
        let mut json_str = serde_json::to_string(&event)
            .map_err(|e| brain_common::BrainError::Serialization(e.to_string()))?;
        json_str.push('\n');
        
        let mut writer_guard = self.writer.lock().await;
        if let Some(writer) = writer_guard.as_mut() {
            writer.write_all(json_str.as_bytes()).await?;
            writer.flush().await?;
        }
        Ok(())
    }
}

/// Заглушка, если события не нужно писать
pub struct NoopEventBus;

#[async_trait]
impl EventBus for NoopEventBus {
    async fn emit(&self, _event_type: EventType) -> Result<()> {
        Ok(())
    }
}

/// Чтение лога событий для аналитики
pub struct EventReader {
    log_file: PathBuf,
}

impl EventReader {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { log_file: path.into() }
    }

    /// Прочитать все события из файла лога
    pub async fn read_all_events(&self) -> Result<Vec<Event>> {
        if !self.log_file.exists() {
            return Ok(Vec::new());
        }
        
        let file = File::open(&self.log_file).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut events = Vec::new();
        
        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(event) = serde_json::from_str::<Event>(&line) {
                events.push(event);
            }
        }
        
        Ok(events)
    }
}
