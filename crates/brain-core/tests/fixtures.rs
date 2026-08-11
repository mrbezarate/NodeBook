use std::fs;
use std::sync::Arc;
use serde::Deserialize;
use tempfile::tempdir;

use brain_common::{Result, RawEvent};
use brain_core::db::SqliteKnowledgeStore;
use brain_core::traits::{AiProvider, KnowledgeStore, RawEventStore};
use brain_core::consolidator::Consolidator;
use brain_core::identity::CascadedIdentityResolver;
use brain_core::projection::{SimpleProjectionEngine, ObsidianRenderer};
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Deserialize)]
struct FixtureEvent {
    description: String,
    raw_event_text: String,
    mock_extracted_entities: Vec<String>,
    mock_extracted_summary: String,
    expected_entity_names: Vec<String>,
    mock_error: Option<bool>,
}

struct MockFixtureAiProvider {
    entities: Vec<String>,
    summary: String,
    should_fail: bool,
}

#[async_trait]
impl AiProvider for MockFixtureAiProvider {
    async fn complete(&self, prompt: &str) -> Result<String> {
        let _query = prompt.split('\'').nth(1).unwrap_or(prompt).to_lowercase();
        // Simple mock returning NONE if no strict match
        Ok("NONE".into())
    }
    
    
    async fn complete_json(&self, _prompt: &str) -> Result<String> {
        if self.should_fail {
            return Ok("{ bad json".into());
        }
        let obs = brain_core::extractor::StructuredObservation {
            title: None,
            summary: self.summary.clone(),
            enriched_text: None,
            entities: self.entities.clone(),
            tags: vec![],
            confidence: 0.99,
        };
        Ok(serde_json::to_string(&obs).unwrap())
    }
    
    async fn classify(&self, _text: &str, _categories: &[&str]) -> Result<(String, f32)> {
        Ok(("".into(), 1.0))
    }
}

#[tokio::test]
async fn test_fixtures_pipeline() {
    let db_dir = tempdir().expect("failed to create temp dir for db");
    let vault_dir = tempdir().expect("failed to create temp dir for vault");
    let db_path = db_dir.path().join("brain.db");
    
    let store = Arc::new(SqliteKnowledgeStore::new(db_path.to_str().unwrap()).unwrap());
    
    // Read fixtures
    let mut events = Vec::new();
    let Ok(entries) = fs::read_dir("tests/data/fixtures/telegram") else {
        println!("Skipping fixtures test: tests/data/fixtures/telegram directory not found.");
        return;
    };
    for entry in entries {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
            let json_data = fs::read_to_string(entry.path()).unwrap();
            let file_events: Vec<FixtureEvent> = serde_json::from_str(&json_data).unwrap();
            events.extend(file_events);
        }
    }

    for (i, fixture) in events.into_iter().enumerate() {
        let ai = Arc::new(MockFixtureAiProvider {
            entities: fixture.mock_extracted_entities.clone(),
            summary: fixture.mock_extracted_summary.clone(),
            should_fail: fixture.mock_error.unwrap_or(false),
        });
        let identity_resolver = Arc::new(CascadedIdentityResolver::new(store.clone(), ai.clone(), None, None));
        
        let consolidator = Consolidator::new(
            ai.clone(),
            store.clone(),
            identity_resolver,
            Arc::new(SimpleProjectionEngine::new(store.clone())),
            store.clone(),
            Arc::new(ObsidianRenderer { base_path: vault_dir.path().to_path_buf() }),
            None,
        );
        
        // Insert raw event
        let event_id = Uuid::new_v4().to_string();
        let event = RawEvent {
            id: event_id.clone(),
            source_type: "telegram".into(),
            source_id: "user_1".into(),
            external_id: None,
            payload: "{}".into(),
            text: fixture.raw_event_text.clone(),
            status: "pending".into(),
        };
        store.save_raw_event(&event).await.unwrap();
        let job = brain_common::Job {
            id: Uuid::new_v4().to_string(),
            raw_event_id: event_id.clone(),
            job_type: "consolidate".into(),
            status: "pending".into(),
        };
        store.create_job(&job).await.unwrap();
        
        // Run job
        let _ = consolidator.run_pending_job().await;
        
        let _job_record = store.get_next_pending_job("consolidate").await.unwrap();
        // If it failed, the job should not be pending
        
        // Verify projection
        let entities = store.list_entities(None).await.unwrap();
        let mut found_count = 0;
        for expected_name in &fixture.expected_entity_names {
            for e in &entities {
                if &e.name == expected_name {
                    found_count += 1;
                    println!("Fixture {} passed: Entity '{}' correctly identified/created.", i, expected_name);
                    break;
                }
            }
        }
        assert_eq!(found_count, fixture.expected_entity_names.len(), "Fixture {} failed: expected {:?}, got {:?}", i, fixture.expected_entity_names, entities.iter().map(|e| &e.name).collect::<Vec<_>>());
    }
}
