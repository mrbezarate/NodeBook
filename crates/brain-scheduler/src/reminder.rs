//! Напоминания.
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String, pub text: String, pub due_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>, pub completed: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ReminderStore { reminders: Vec<Reminder> }

impl ReminderStore {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, r: Reminder) { self.reminders.push(r); }
    pub fn get_due(&self) -> Vec<&Reminder> {
        let now = Utc::now();
        self.reminders.iter().filter(|r| !r.completed && r.due_at <= now).collect()
    }
    pub fn complete(&mut self, id: &str) { if let Some(r) = self.reminders.iter_mut().find(|r| r.id == id) { r.completed = true; } }
}
