use std::sync::Arc;
use tempfile::tempdir;
use async_trait::async_trait;
use uuid::Uuid;

use brain_common::{BrainError, EntrySource, RawEvent, Job, Result, ResolutionResult, Entity, EntityType};
use brain_core::db::SqliteKnowledgeStore;
use brain_core::traits::{AiProvider, IdentityResolver, RawEventStore};
use brain_core::projection::{SimpleProjectionEngine, ObsidianRenderer};
use brain_core::consolidator::Consolidator;

// --- Mocks ---

struct MockAiProvider;

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn complete(&self, _prompt: &str) -> Result<String> {
        Ok("mock response".into())
    }
    
    async fn complete_json(&self, _prompt: &str) -> Result<String> {
        Ok(r#"{
            "summary": "Project supports multiplayer",
            "entities": ["Space Cowboy", "Multiplayer"],
            "tags": ["idea", "feature"],
            "confidence": 0.95
        }"#.into())
    }
    
    async fn classify(&self, _text: &str, _categories: &[&str]) -> Result<(String, f32)> {
        Ok(("mock_category".into(), 1.0))
    }
}

struct MockIdentityResolver;

#[async_trait]
impl IdentityResolver for MockIdentityResolver {
    async fn resolve(&self, _query: &str) -> Result<Option<ResolutionResult>> {
        // Всегда возвращаем новую сущность с низким confidence или ничего, чтобы создать новую
        Ok(None)
    }

    async fn register_alias(&self, _canonical_id: &str, _alias: &str) -> Result<()> {
        Ok(())
    }
}

// --- Test ---

#[tokio::test]
async fn test_full_vertical_slice() {
    tracing_subscriber::fmt().try_init().ok();
    
    // 1. Создаем временную SQLite
    let db_dir = tempdir().expect("failed to create temp dir for db");
    let db_path = db_dir.path().join("brain.db");
    let store = Arc::new(SqliteKnowledgeStore::new(db_path.to_str().unwrap()).expect("failed to create db"));

    // 2. Создаем временный Obsidian Vault
    let vault_dir = tempdir().expect("failed to create temp dir for vault");
    let renderer = Arc::new(ObsidianRenderer {
        base_path: vault_dir.path().to_path_buf(),
    });

    let ai = Arc::new(MockAiProvider);
    let resolver = Arc::new(MockIdentityResolver);
    let projection = Arc::new(SimpleProjectionEngine::new(store.clone()));

    let consolidator = Consolidator::new(
        ai,
        store.clone(),
        resolver,
        projection,
        store.clone(),
        renderer,
    );

    // 3. Создаем RawEvent и Job
    let event_id = Uuid::new_v4().to_string();
    let event = RawEvent {
        id: event_id.clone(),
        source_type: "telegram".into(),
        source_id: "user_1".into(),
        external_id: Some("msg_1".into()),
        payload: "{}".into(),
        text: "Add multiplayer feature to Space Cowboy".into(),
        status: "pending".into(),
    };
    store.save_raw_event(&event).await.expect("Failed to save raw event");

    let job_id = Uuid::new_v4().to_string();
    let job = Job {
        id: job_id.clone(),
        raw_event_id: event_id.clone(),
        job_type: "consolidate".into(),
        status: "pending".into(),
    };
    store.create_job(&job).await.expect("Failed to create job");

    // 4. Запускаем Consolidator
    consolidator.run_pending_job().await.expect("Consolidator failed");

    // 5. Проверяем

    // ✓ Job == completed
    // We cannot easily get job by ID unless we implement it, but we can just check if any pending jobs exist.
    let next_job = store.get_next_pending_job("consolidate").await.unwrap();
    assert!(next_job.is_none(), "Job should be completed and not pending");

    // Check if the file was created in Obsidian vault
    let mut found_md = false;
    for entry in std::fs::read_dir(vault_dir.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
            found_md = true;
            let content = std::fs::read_to_string(entry.path()).unwrap();
            
            // ✓ Markdown появился
            assert!(content.contains("type: Concept")); // From SimpleProjectionEngine
            assert!(content.contains("area: Some(Life)")); 
            assert!(content.contains("Project supports multiplayer")); // Summary
        }
    }
    assert!(found_md, "Markdown file was not created in vault");

    // To verify SQLite rows directly we'd need to inspect the db.
    let conn = rusqlite::Connection::open(db_path).unwrap();
    let count: i64 = conn.query_row("SELECT count(*) FROM observations WHERE raw_event_id = ?1", 
        rusqlite::params![event_id], |row| row.get(0)).unwrap();
    
    // ✓ Observation появилась
    assert_eq!(count, 1, "Expected exactly 1 observation for this event");

    let entity_count: i64 = conn.query_row("SELECT count(*) FROM entities", [], |row| row.get(0)).unwrap();
    
    // ✓ Entity появилась
    assert_eq!(entity_count, 1, "Expected exactly 1 entity saved to store");

    println!("Full vertical slice test passed successfully!");
}

#[tokio::test]
async fn test_idempotency() {
    tracing_subscriber::fmt().try_init().ok();
    
    let db_dir = tempdir().expect("failed to create temp dir for db");
    let db_path = db_dir.path().join("brain.db");
    let store = Arc::new(SqliteKnowledgeStore::new(db_path.to_str().unwrap()).unwrap());

    let vault_dir = tempdir().expect("failed to create temp dir for vault");
    let renderer = Arc::new(ObsidianRenderer { base_path: vault_dir.path().to_path_buf() });

    let consolidator = Consolidator::new(
        Arc::new(MockAiProvider),
        store.clone(),
        Arc::new(MockIdentityResolver),
        Arc::new(SimpleProjectionEngine::new(store.clone())),
        store.clone(),
        renderer,
    );

    let event_id = Uuid::new_v4().to_string();
    let event = RawEvent {
        id: event_id.clone(),
        source_type: "telegram".into(),
        source_id: "user_1".into(),
        external_id: None,
        payload: "{}".into(),
        text: "Add multiplayer".into(),
        status: "pending".into(),
    };
    store.save_raw_event(&event).await.unwrap();

    // Первый запуск
    let job_id = Uuid::new_v4().to_string();
    let job = Job {
        id: job_id.clone(),
        raw_event_id: event_id.clone(),
        job_type: "consolidate".into(),
        status: "pending".into(),
    };
    store.create_job(&job).await.unwrap();
    consolidator.run_pending_job().await.unwrap();

    // Запускаем Consolidator ещё раз для этого же RawEvent (эмулируем дублирование Job)
    let job2_id = Uuid::new_v4().to_string();
    let job2 = Job {
        id: job2_id.clone(),
        raw_event_id: event_id.clone(),
        job_type: "consolidate".into(),
        status: "pending".into(),
    };
    store.create_job(&job2).await.unwrap();
    consolidator.run_pending_job().await.unwrap();

    // Проверяем: Observation == 1, Entity == 1
    let conn = rusqlite::Connection::open(db_path).unwrap();
    let obs_count: i64 = conn.query_row("SELECT count(*) FROM observations", [], |row| row.get(0)).unwrap();
    assert_eq!(obs_count, 1, "Idempotency failed: duplicate observations");

    let entity_count: i64 = conn.query_row("SELECT count(*) FROM entities", [], |row| row.get(0)).unwrap();
    assert_eq!(entity_count, 1, "Idempotency failed: duplicate entities");
}

#[tokio::test]
async fn test_crash_recovery() {
    let db_dir = tempdir().expect("failed to create temp dir for db");
    let db_path = db_dir.path().join("brain.db");
    let store = Arc::new(SqliteKnowledgeStore::new(db_path.to_str().unwrap()).unwrap());

    // Мы можем эмулировать сбой: просто вызовем save_observation (как будто Consolidator упал после неё),
    // а потом запустим Consolidator заново и проверим, что он не упадет и корректно перезапишет Observation
    // и продолжит работу.
    
    let event_id = Uuid::new_v4().to_string();
    let event = RawEvent {
        id: event_id.clone(),
        source_type: "telegram".into(),
        source_id: "user_1".into(),
        external_id: None,
        payload: "{}".into(),
        text: "Crash recovery test".into(),
        status: "pending".into(),
    };
    store.save_raw_event(&event).await.unwrap();

    // Эмуляция падения
    let obs = brain_common::Observation {
        id: format!("obs_{}", event_id),
        raw_event_id: event_id.clone(),
        entity_id: format!("entity_for_{}", event_id),
        fact: "Crash recovery test".into(),
        confidence: 0.95,
        schema_version: 1,
        extractor_version: "v1".to_string(),
    };
    store.save_observation(&obs).await.unwrap();

    // Перезапуск 
    let vault_dir = tempdir().expect("failed to create temp dir for vault");
    let consolidator = Consolidator::new(
        Arc::new(MockAiProvider),
        store.clone(),
        Arc::new(MockIdentityResolver),
        Arc::new(SimpleProjectionEngine::new(store.clone())),
        store.clone(),
        Arc::new(ObsidianRenderer { base_path: vault_dir.path().to_path_buf() }),
        None,
    );

    let job = Job {
        id: Uuid::new_v4().to_string(),
        raw_event_id: event_id.clone(),
        job_type: "consolidate".into(),
        status: "pending".into(),
    };
    store.create_job(&job).await.unwrap();
    
    // Consolidator продолжает работу и завершает её
    consolidator.run_pending_job().await.unwrap();

    // Проверяем
    let conn = rusqlite::Connection::open(db_path).unwrap();
    let obs_count: i64 = conn.query_row("SELECT count(*) FROM observations", [], |row| row.get(0)).unwrap();
    assert_eq!(obs_count, 1, "Expected exactly 1 observation");

    let mut found_md = false;
    for entry in std::fs::read_dir(vault_dir.path()).unwrap() {
        if entry.unwrap().path().extension().and_then(|s| s.to_str()) == Some("md") {
            found_md = true;
        }
    }
    assert!(found_md, "Markdown should be rendered after crash recovery");
}
