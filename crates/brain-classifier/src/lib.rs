//! # brain-classifier — Алгоритмический классификатор. Правила → AI fallback.
pub mod area;
pub mod confidence;
pub mod entity;
pub mod entry_type;
pub mod rule_engine;

pub use entry_type::HybridTypeClassifier;
pub use area::HybridAreaDetector;
pub use entity::HybridEntityExtractor;
pub use rule_engine::RuleEngine;
