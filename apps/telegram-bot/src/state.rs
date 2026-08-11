//! Управление состоянием пользователя.
use brain_diary::EveningReview;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum UserState {
    Idle,
    DiaryReview(EveningReview),
    WaitingForSearch,
    WaitingForReminder,
    WaitingForNewVaultName,
    WaitingForRenameVault,
    Editing { entry_id: String, field: String },
}

pub struct StateManager {
    states: Arc<RwLock<HashMap<u64, UserState>>>,
}

impl StateManager {
    pub fn new() -> Self { Self { states: Arc::new(RwLock::new(HashMap::new())) } }
    pub async fn get(&self, user_id: u64) -> UserState {
        self.states.read().await.get(&user_id).cloned().unwrap_or(UserState::Idle)
    }
    pub async fn set(&self, user_id: u64, state: UserState) {
        self.states.write().await.insert(user_id, state);
    }
    pub async fn reset(&self, user_id: u64) {
        self.states.write().await.remove(&user_id);
    }
}
