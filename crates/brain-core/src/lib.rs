//! # brain-core
//!
//! Ядро системы Brain — определения трейтов и обработочный пайплайн.
//! Все компоненты подключаются через трейты для максимальной модульности.

pub mod engine;
pub mod pipeline;
pub mod traits;

pub use engine::{BrainEngine, BrainEngineBuilder, Stats};
pub use pipeline::{Pipeline, PipelineBuilder};
pub use traits::*;
