//! In-memory граф знаний с JSON persistence.
use async_trait::async_trait;
use brain_common::Result;
use brain_core::GraphStore;
use crate::node::{Node, NodeType};
use crate::edge::{Edge, Relation};
use std::collections::HashMap;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Сериализуемая внутренняя структура графа.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GraphData {
    pub nodes: HashMap<String, Node>,
    pub edges: Vec<Edge>,
}

/// Граф знаний с потокобезопасным доступом.
pub struct KnowledgeGraph {
    data: RwLock<GraphData>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self { data: RwLock::new(GraphData::default()) }
    }

    /// Создать граф из загруженных данных.
    fn from_data(data: GraphData) -> Self {
        Self { data: RwLock::new(data) }
    }

    /// Сохранить граф в JSON файл.
    pub async fn save(&self, path: &str) -> std::io::Result<()> {
        let data = self.data.read().await;
        let json = serde_json::to_string_pretty(&*data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        tokio::fs::write(path, json).await
    }

    /// Загрузить граф из JSON файла.
    pub async fn load(path: &str) -> std::io::Result<Self> {
        let s = tokio::fs::read_to_string(path).await?;
        let data: GraphData = serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(Self::from_data(data))
    }

    /// Получить количество узлов.
    pub async fn node_count(&self) -> usize {
        self.data.read().await.nodes.len()
    }

    /// Получить количество рёбер.
    pub async fn edge_count(&self) -> usize {
        self.data.read().await.edges.len()
    }
}

fn parse_node_type(s: &str) -> NodeType {
    match s.to_lowercase().as_str() {
        "person" => NodeType::Person,
        "idea" => NodeType::Idea,
        "project" => NodeType::Project,
        "technology" => NodeType::Technology,
        "book" => NodeType::Book,
        "habit" => NodeType::Habit,
        "problem" => NodeType::Problem,
        "area" => NodeType::Area,
        "tag" => NodeType::Tag,
        _ => NodeType::Entry,
    }
}

fn parse_relation(s: &str) -> Relation {
    match s.to_lowercase().as_str() {
        "partof" | "part_of" => Relation::PartOf,
        "influences" => Relation::Influences,
        "contradicts" => Relation::Contradicts,
        "extends" => Relation::Extends,
        "references" | "mentions" => Relation::References,
        "taggedwith" | "tagged_with" => Relation::TaggedWith,
        "belongstoarea" | "belongs_to_area" => Relation::BelongsToArea,
        _ => Relation::RelatedTo,
    }
}

#[async_trait]
impl GraphStore for KnowledgeGraph {
    async fn add_node(&self, id: &str, label: &str, node_type: &str) -> Result<()> {
        let node = Node {
            id: id.to_string(),
            label: label.to_string(),
            node_type: parse_node_type(node_type),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        };
        self.data.write().await.nodes.insert(id.to_string(), node);
        tracing::debug!("Graph: added node '{}' ({})", label, node_type);
        Ok(())
    }

    async fn add_edge(&self, from: &str, to: &str, relation: &str) -> Result<()> {
        let edge = Edge {
            from: from.to_string(),
            to: to.to_string(),
            relation: parse_relation(relation),
            weight: 1.0,
            created_at: Utc::now(),
        };
        self.data.write().await.edges.push(edge);
        tracing::debug!("Graph: added edge {} -> {} ({})", from, to, relation);
        Ok(())
    }

    async fn get_neighbors(&self, id: &str) -> Result<Vec<String>> {
        let data = self.data.read().await;
        Ok(crate::query::find_neighbors(&data.edges, id))
    }

    async fn find_path(&self, from: &str, to: &str) -> Result<Vec<String>> {
        let data = self.data.read().await;
        Ok(crate::query::bfs_path(&data.edges, from, to).unwrap_or_default())
    }
}
