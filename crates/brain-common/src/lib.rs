//! # brain-common
//!
//! Общие типы, ошибки и идентификаторы для всей системы Brain.
//! Этот crate не содержит логики — только определения данных.

pub mod error;
pub mod id;
pub mod types;
pub mod output;

pub use error::{BrainError, Result};
pub use id::EntryId;
pub use types::*;
pub use output::*;
