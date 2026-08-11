use brain_common::{Entity, EntityType, SemanticLink};
use brain_core::db::SqliteKnowledgeStore;
use brain_core::traits::KnowledgeStore;
use std::fs;

#[tokio::test]
async fn test_knowledge_store_new_entity() {
    let _ = fs::remove_file("test_db1.sqlite");
    let store = SqliteKnowledgeStore::new("test_db1.sqlite").unwrap();
    
    let mut entity = Entity::new("Space Cowboy RPG", EntityType::Project);
    entity.id = "project_space_cowboy".to_string();
    entity.summary = "Вдохновлена Cowboy Bebop. Геймплей похож на Kenshi.".to_string();
    entity.tags = vec!["gamedev".to_string(), "rpg".to_string()];
    entity.links.push(SemanticLink {
        target: "Cowboy Bebop".to_string(),
        relation: "InspiredBy".to_string(),
    });
    
    store.save_entity(&entity).await.unwrap();
    
    let loaded = store.get_entity("project_space_cowboy").await.unwrap().unwrap();
    assert_eq!(loaded.name, "Space Cowboy RPG");
    assert_eq!(loaded.tags.len(), 2);
    assert_eq!(loaded.links.len(), 1);
    
    let _ = fs::remove_file("test_db1.sqlite");
}

#[tokio::test]
async fn test_knowledge_store_update_entity() {
    let _ = fs::remove_file("test_db2.sqlite");
    let store = SqliteKnowledgeStore::new("test_db2.sqlite").unwrap();
    
    // 1. Создаем начальный Entity
    let mut entity = Entity::new("Space Cowboy RPG", EntityType::Project);
    entity.id = "project_space_cowboy".to_string();
    entity.summary = "Начальная идея".to_string();
    store.save_entity(&entity).await.unwrap();
    
    // 2. Симулируем второй инжест (новая мысль добавляет мультиплеер)
    let mut loaded = store.get_entity("project_space_cowboy").await.unwrap().unwrap();
    loaded.summary = format!("{}\n\nНовое обновление:\nДобавить мультиплеер", loaded.summary);
    loaded.tags.push("multiplayer".to_string());
    
    store.save_entity(&loaded).await.unwrap();
    
    // 3. Проверяем, что обновление успешно применилось (без дублей)
    let final_entity = store.get_entity("project_space_cowboy").await.unwrap().unwrap();
    assert_eq!(final_entity.tags.len(), 1);
    assert_eq!(final_entity.tags[0], "multiplayer");
    assert!(final_entity.summary.contains("Добавить мультиплеер"));
    
    // 4. Проверяем, что в базе по-прежнему один Entity
    let all = store.list_entities(None).await.unwrap();
    assert_eq!(all.len(), 1);
    
    let _ = fs::remove_file("test_db2.sqlite");
}
