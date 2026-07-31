//! Задачи.
use chrono::NaiveDate;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus { Todo, InProgress, Done, Cancelled }
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority { Low, Medium, High, Critical }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String, pub title: String, pub status: TaskStatus,
    pub priority: Priority, pub due_date: Option<NaiveDate>,
    pub tags: Vec<String>, pub created_at: chrono::DateTime<chrono::Utc>,
}
