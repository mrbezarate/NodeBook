//! Гибридный классификатор типа записи: правила → AI fallback.
use async_trait::async_trait;
use brain_common::{EntryType, Result};
use brain_core::{AiProvider, TypeClassifier};
use crate::rule_engine::RuleEngine;
use std::sync::Arc;

pub struct HybridTypeClassifier {
    ai: Option<Arc<dyn AiProvider>>,
    confidence_threshold: f32,
}

impl HybridTypeClassifier {
    pub fn new(ai: Option<Arc<dyn AiProvider>>, confidence_threshold: f32) -> Self {
        Self { ai, confidence_threshold }
    }
}

#[async_trait]
impl TypeClassifier for HybridTypeClassifier {
    async fn classify_type(&self, text: &str, _context: &str) -> Result<(EntryType, f32)> {
        // Шаг 1: Алгоритмические правила
        if let Some(result) = RuleEngine::classify(text) {
            if result.confidence >= self.confidence_threshold {
                tracing::debug!("Rules classified as {:?} ({:.2})", result.entry_type, result.confidence);
                return Ok((result.entry_type, result.confidence));
            }
        }
        // Шаг 2: AI fallback
        if let Some(ref ai) = self.ai {
            let categories: Vec<&str> = EntryType::all().iter().map(|t| match t {
                EntryType::Idea => "idea", EntryType::Task => "task", EntryType::Goal => "goal",
                EntryType::Knowledge => "knowledge", EntryType::Diary => "diary",
                EntryType::Finance => "finance", EntryType::Habit => "habit",
                _ => "thought",
            }).collect();
            if let Ok((cat, conf)) = ai.classify(text, &categories).await {
                let entry_type = match cat.to_lowercase().as_str() {
                    "idea" => EntryType::Idea, "task" => EntryType::Task,
                    "goal" => EntryType::Goal, "knowledge" => EntryType::Knowledge,
                    "diary" => EntryType::Diary, "finance" => EntryType::Finance,
                    "habit" => EntryType::Habit, _ => EntryType::Thought,
                };
                return Ok((entry_type, conf));
            }
        }
        // Fallback: Thought
        Ok((EntryType::Thought, 0.3))
    }
}
