//! # brain-parser
//!
//! Алгоритмический парсер текста. Никакого AI — чистые regex и правила.
//! Токенизация, нормализация, извлечение паттернов.

pub mod extractor;
pub mod normalizer;
pub mod patterns;
pub mod tokenizer;

pub use extractor::Extractor;
pub use normalizer::Normalizer;
pub use patterns::{PatternMatcher, PatternMatch};
pub use tokenizer::Tokenizer;
