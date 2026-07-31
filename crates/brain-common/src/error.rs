//! Единая система ошибок для всего проекта Brain.

use thiserror::Error;

/// Основной тип ошибки Brain.
#[derive(Error, Debug)]
pub enum BrainError {
    #[error("AI provider error: {0}")]
    Ai(String),

    #[error("Classification failed: {0}")]
    Classification(String),

    #[error("Vault error: {0}")]
    Vault(String),

    #[error("Parser error: {0}")]
    Parser(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Telegram error: {0}")]
    Telegram(String),
}

/// Удобный алиас для Result с BrainError.
pub type Result<T> = std::result::Result<T, BrainError>;
