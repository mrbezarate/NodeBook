//! Уникальный идентификатор записи (EntryId).

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Уникальный идентификатор записи в системе Brain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntryId(String);

impl EntryId {
    /// Создать новый уникальный идентификатор (UUID v4).
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Создать из существующей строки.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Получить строковое представление.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EntryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
