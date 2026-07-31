//! # brain-core
//!
//! Ядро системы Brain — определения трейтов и обработочный пайплайн.
//! Все компоненты подключаются через трейты для максимальной модульности.

pub mod engine;
pub mod pipeline;
pub mod agentic_pipeline;
pub mod traits;
pub mod consolidator;
pub mod extractor;
pub mod identity;
pub mod projection;
pub mod db;

pub use engine::{BrainEngine, BrainEngineBuilder, Stats};
pub use pipeline::{Pipeline, PipelineBuilder};
pub use agentic_pipeline::AgenticPipeline;
pub use traits::*;
