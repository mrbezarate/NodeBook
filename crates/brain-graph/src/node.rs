use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType { Entry, Person, Idea, Project, Technology, Book, Habit, Problem, Area, Tag }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}
