//! # brain-graph — Граф знаний.
pub mod edge;
pub mod knowledge;
pub mod node;
pub mod query;
pub use knowledge::KnowledgeGraph;
pub use node::{Node, NodeType};
pub use edge::{Edge, Relation};
