use serde::{Deserialize, Serialize};

/// Жизненный цикл ресурса. Определяет, кто и когда должен удалять ресурс.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceLifecycle {
    /// Временный файл (например, сгенерированный отчет).
    /// Должен быть удален после успешной доставки (OutputSink берет на себя ответственность или сигнализирует об успешной отправке).
    Temporary,
    /// Долгоживущий ресурс, управляемый платформой (например, картинка, сохраненная в базе/S3).
    /// Не удаляется после отправки.
    Persistent,
    /// Кэшированный ресурс, который может быть удален сборщиком мусора платформы.
    Cached,
}

/// Полезная нагрузка ответа системы.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputPayload {
    /// Обычный текстовый ответ (Markdown/HTML).
    InlineText {
        text: String,
    },
    /// Ссылка на ресурс (файл, медиа, бинарные данные).
    /// Разрешение resource_id в конкретный путь/URL происходит на уровне Delivery/StorageResolver.
    Resource {
        resource_id: String,
    },
}

/// Единый контракт исходящего сообщения из ядра (Brain) во внешний мир (Telegram, Web, и т.д.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    pub payload: OutputPayload,
    pub lifecycle: ResourceLifecycle,
}

impl Output {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            payload: OutputPayload::InlineText { text: text.into() },
            lifecycle: ResourceLifecycle::Temporary, // Текст живет в памяти, удалять нечего
        }
    }
    
    pub fn persistent_resource(resource_id: impl Into<String>) -> Self {
        Self {
            payload: OutputPayload::Resource { resource_id: resource_id.into() },
            lifecycle: ResourceLifecycle::Persistent,
        }
    }
    
    pub fn temp_resource(resource_id: impl Into<String>) -> Self {
        Self {
            payload: OutputPayload::Resource { resource_id: resource_id.into() },
            lifecycle: ResourceLifecycle::Temporary,
        }
    }
}
