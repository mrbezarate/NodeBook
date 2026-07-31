//! # brain-embeddings — Векторные embeddings и семантический поиск.
pub mod search;
pub mod similarity;
pub mod store;

pub use search::SemanticSearch;
pub use store::VectorStore;
