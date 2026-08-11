use std::fs;
use std::sync::Arc;
use serde::Deserialize;
use tempfile::tempdir;

use brain_common::{Entity, EntityType};
use brain_core::db::SqliteKnowledgeStore;
use brain_core::traits::{IdentityResolver, KnowledgeStore, AiProvider};
use brain_core::identity::CascadedIdentityResolver;
use async_trait::async_trait;

#[derive(Deserialize)]
struct GoldenCase {
    entity: String,
    inputs: Vec<String>,
    expected: String,
}

struct MockAiProvider;

#[async_trait]
impl AiProvider for MockAiProvider {
    async fn complete(&self, prompt: &str) -> brain_common::Result<String> {
        // Simple mock for semantic match: if the query contains words from candidate, return candidate
        // In real world, LLM handles this.
        let query = prompt.split('\'').nth(1).unwrap_or(prompt).to_lowercase();
        if query.contains("космическая") || query.contains("космический") || query.contains("кораблями") || query.contains("bebop") {
            return Ok("Space Cowboy RPG".into());
        }
        if query.contains("бот") || query.contains("дневник") || query.contains("daybook") || query.contains("телеге") {
            return Ok("Daybook".into());
        }
        if query.contains("язык") || query.contains("rust") || query.contains("раст") {
            return Ok("Rust".into());
        }
        if query.contains("докер") || query.contains("docker") || query.contains("контейнер") {
            return Ok("Docker".into());
        }
        if query.contains("ml") || query.contains("машинное") || query.contains("нейросети") {
            return Ok("Machine Learning".into());
        }
        if query.contains("тренировка") || query.contains("качалка") || query.contains("зал") || query.contains("gym") {
            return Ok("Gym Workout".into());
        }
        if query.contains("обсидиан") || query.contains("obsidian") || query.contains("база") {
            return Ok("Obsidian".into());
        }
        Ok("NONE".into())
    }
    
    async fn complete_json(&self, _prompt: &str) -> brain_common::Result<String> {
        Ok("{}".into())
    }
    
    async fn classify(&self, _text: &str, _categories: &[&str]) -> brain_common::Result<(String, f32)> {
        Ok(("".into(), 1.0))
    }
}

#[tokio::test]
async fn test_golden_dataset() {
    let db_dir = tempdir().expect("failed to create temp dir for db");
    let db_path = db_dir.path().join("brain.db");
    let store = Arc::new(SqliteKnowledgeStore::new(db_path.to_str().unwrap()).unwrap());
    
    let ai = Arc::new(MockAiProvider);
    let resolver = CascadedIdentityResolver::new(store.clone(), ai, None, None);

    // 1. Load cases from directory
    let mut cases: Vec<GoldenCase> = Vec::new();
    let Ok(entries) = fs::read_dir("tests/data/identity_cases") else {
        println!("Skipping golden dataset test: tests/data/identity_cases directory not found.");
        return;
    };
    for entry in entries {
        let entry = entry.unwrap();
        if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
            let json_data = fs::read_to_string(entry.path()).expect("Failed to read dataset file");
            let file_cases: Vec<GoldenCase> = serde_json::from_str(&json_data).expect("Failed to parse dataset file");
            cases.extend(file_cases);
        }
    }

    // 2. Pre-populate KnowledgeStore with canonical entities
    for case in &cases {
        let mut entity = Entity::new(&case.entity, EntityType::Concept);
        entity.id = case.expected.clone();
        
        // Let's add some aliases for realism
        if case.expected == "project_space_cowboy_rpg" {
            entity.aliases = vec!["SC RPG".into()];
        } else if case.expected == "concept_rust" {
            entity.aliases = vec!["Rustlang".into()];
        }

        store.save_entity(&entity).await.unwrap();
    }

    // 3. Run Benchmark
    let mut passed = 0;
    let mut total = 0;
    let mut failures = Vec::new();

    for case in cases {
        for input in case.inputs {
            total += 1;
            let resolution = resolver.resolve(&input).await.unwrap();
            match resolution {
                Some(res) if res.entity.id == case.expected => {
                    passed += 1;
                }
                Some(res) => {
                    failures.push(format!("FAIL: '{}' -> resolved to '{}' instead of '{}'", input, res.entity.id, case.expected));
                }
                None => {
                    failures.push(format!("FAIL: '{}' -> failed to resolve to '{}'", input, case.expected));
                }
            }
        }
    }

    println!("Passed {}/{} cases", passed, total);
    for fail in &failures {
        println!("{}", fail);
    }

    assert_eq!(passed, total, "Not all cases passed the benchmark");
}
