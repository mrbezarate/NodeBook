use async_trait::async_trait;
use brain_common::{BrainError, Entity, EntityType, Result, SemanticLink, EntrySource, RawEvent, Job, Observation};
use crate::traits::{KnowledgeStore, RawEventStore};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

pub struct SqliteKnowledgeStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteKnowledgeStore {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path).map_err(|e| BrainError::Database(e.to_string()))?;
        
        // Включаем внешние ключи (Foreign Keys)
        conn.execute("PRAGMA foreign_keys = ON;", []).map_err(|e| BrainError::Database(e.to_string()))?;
        
        // 1. Создаем нормализованную схему
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS raw_events (
                id TEXT PRIMARY KEY,
                source_type TEXT NOT NULL,
                source_id TEXT NOT NULL,
                external_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                payload TEXT NOT NULL,
                text TEXT NOT NULL,
                language TEXT,
                embedding_id TEXT,
                status TEXT NOT NULL,
                processor_version TEXT,
                retry_count INTEGER DEFAULT 0,
                error TEXT,
                processed_at DATETIME
            );

            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                raw_event_id TEXT,
                job_type TEXT NOT NULL,
                status TEXT NOT NULL,
                attempt INTEGER DEFAULT 0,
                scheduled_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                started_at DATETIME,
                finished_at DATETIME,
                error TEXT,
                FOREIGN KEY(raw_event_id) REFERENCES raw_events(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                area TEXT,
                summary TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS observations (
                id TEXT PRIMARY KEY,
                raw_event_id TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                fact TEXT NOT NULL,
                confidence REAL NOT NULL,
                schema_version INTEGER NOT NULL DEFAULT 1,
                extractor_version TEXT NOT NULL DEFAULT 'v1',
                llm_model TEXT,
                prompt_version TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(raw_event_id) REFERENCES raw_events(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS aliases (
                entity_id TEXT NOT NULL,
                alias TEXT NOT NULL,
                FOREIGN KEY(entity_id) REFERENCES entities(id) ON DELETE CASCADE,
                UNIQUE(entity_id, alias)
            );

            CREATE TABLE IF NOT EXISTS tags (
                entity_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                FOREIGN KEY(entity_id) REFERENCES entities(id) ON DELETE CASCADE,
                UNIQUE(entity_id, tag)
            );

            CREATE TABLE IF NOT EXISTS relations (
                from_entity TEXT NOT NULL,
                relation TEXT NOT NULL,
                to_entity TEXT NOT NULL,
                FOREIGN KEY(from_entity) REFERENCES entities(id) ON DELETE CASCADE,
                UNIQUE(from_entity, relation, to_entity)
            );

            CREATE TABLE IF NOT EXISTS sources (
                entity_id TEXT NOT NULL,
                source_json TEXT NOT NULL,
                FOREIGN KEY(entity_id) REFERENCES entities(id) ON DELETE CASCADE,
                UNIQUE(entity_id, source_json)
            );

            CREATE TABLE IF NOT EXISTS system_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                metric_name TEXT NOT NULL,
                metric_value REAL NOT NULL,
                event_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            "
        ).map_err(|e| BrainError::Database(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl KnowledgeStore for SqliteKnowledgeStore {
    async fn get_entity(&self, id: &str) -> Result<Option<Entity>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn.prepare("SELECT name, entity_type, area, summary FROM entities WHERE id = ?1").unwrap();
        let entity_base = stmt.query_row(params![id], |row| {
            let name: String = row.get(0)?;
            let entity_type_str: String = row.get(1)?;
            let area_str: Option<String> = row.get(2)?;
            let summary: String = row.get(3)?;
            
            let entity_type = match entity_type_str.as_str() {
                "Project" => EntityType::Project,
                "Person" => EntityType::Person,
                "Technology" => EntityType::Technology,
                _ => EntityType::Concept,
            };
            
            let mut entity = Entity::new(&name, entity_type);
            entity.id = id.to_string();
            entity.summary = summary;
            
            if let Some(area_s) = area_str {
                entity.area = serde_json::from_str(&format!("\"{}\"", area_s)).ok();
            }
            Ok(entity)
        }).optional().map_err(|e| BrainError::Database(e.to_string()))?;
        
        let mut entity = match entity_base {
            Some(e) => e,
            None => return Ok(None),
        };

        // Загружаем теги
        let mut tags_stmt = conn.prepare("SELECT tag FROM tags WHERE entity_id = ?1").unwrap();
        let tags_iter = tags_stmt.query_map(params![id], |row| row.get(0)).unwrap();
        entity.tags = tags_iter.filter_map(std::result::Result::ok).collect();

        // Загружаем алиасы
        let mut aliases_stmt = conn.prepare("SELECT alias FROM aliases WHERE entity_id = ?1").unwrap();
        let aliases_iter = aliases_stmt.query_map(params![id], |row| row.get(0)).unwrap();
        entity.aliases = aliases_iter.filter_map(std::result::Result::ok).collect();

        // Загружаем связи (Relations)
        let mut rels_stmt = conn.prepare("SELECT relation, to_entity FROM relations WHERE from_entity = ?1").unwrap();
        let rels_iter = rels_stmt.query_map(params![id], |row| {
            std::result::Result::Ok(SemanticLink {
                relation: row.get(0)?,
                target: row.get(1)?,
            })
        }).unwrap();
        entity.links = rels_iter.filter_map(std::result::Result::ok).collect();

        // Загружаем sources
        let mut src_stmt = conn.prepare("SELECT source_json FROM sources WHERE entity_id = ?1").unwrap();
        let src_iter = src_stmt.query_map(params![id], |row| {
            let json_str: String = row.get(0)?;
            std::result::Result::Ok(serde_json::from_str::<EntrySource>(&json_str).ok())
        }).unwrap();
        entity.sources = src_iter.filter_map(std::result::Result::ok).flatten().collect();
        
        Ok(Some(entity))
    }

    async fn save_entity(&self, entity: &Entity) -> Result<()> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| BrainError::Database(e.to_string()))?;
        
        let area_str = entity.area.as_ref().map(|a| a.to_string());
        let entity_type_str = format!("{:?}", entity.entity_type);
        
        // 1. Сохраняем базовую сущность
        tx.execute(
            "INSERT INTO entities (id, name, entity_type, area, summary)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                entity_type = excluded.entity_type,
                area = excluded.area,
                summary = excluded.summary",
            params![entity.id, entity.name, entity_type_str, area_str, entity.summary],
        ).map_err(|e| BrainError::Database(e.to_string()))?;
        
        // 2. Алиасы (сохраняем через INSERT OR IGNORE для уникальности)
        for alias in &entity.aliases {
            tx.execute(
                "INSERT OR IGNORE INTO aliases (entity_id, alias) VALUES (?1, ?2)",
                params![entity.id, alias],
            ).map_err(|e| BrainError::Database(e.to_string()))?;
        }

        // 3. Теги
        for tag in &entity.tags {
            tx.execute(
                "INSERT OR IGNORE INTO tags (entity_id, tag) VALUES (?1, ?2)",
                params![entity.id, tag],
            ).map_err(|e| BrainError::Database(e.to_string()))?;
        }

        // 4. Связи (Relations)
        for link in &entity.links {
            tx.execute(
                "INSERT OR IGNORE INTO relations (from_entity, relation, to_entity) VALUES (?1, ?2, ?3)",
                params![entity.id, link.relation, link.target],
            ).map_err(|e| BrainError::Database(e.to_string()))?;
        }

        // 5. Источники (Sources)
        for source in &entity.sources {
            let src_json = serde_json::to_string(source).unwrap_or_default();
            if !src_json.is_empty() {
                tx.execute(
                    "INSERT OR IGNORE INTO sources (entity_id, source_json) VALUES (?1, ?2)",
                    params![entity.id, src_json],
                ).map_err(|e| BrainError::Database(e.to_string()))?;
            }
        }

        tx.commit().map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(())
    }

    async fn list_entities(&self, filter_type: Option<EntityType>) -> Result<Vec<Entity>> {
        let ids = {
            let conn = self.conn.lock().unwrap();
            let mut result_ids = Vec::new();
            
            let mut query = "SELECT id FROM entities".to_string();
            if let Some(t) = &filter_type {
                let t_str = format!("{:?}", t);
                query.push_str(" WHERE entity_type = ?1");
                let mut stmt = conn.prepare(&query).unwrap();
                let rows = stmt.query_map(params![t_str], |r| r.get::<_, String>(0)).map_err(|e| BrainError::Database(e.to_string()))?;
                for row in rows {
                    if let Ok(id) = row {
                        result_ids.push(id);
                    }
                }
            } else {
                let mut stmt = conn.prepare(&query).unwrap();
                let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| BrainError::Database(e.to_string()))?;
                for row in rows {
                    if let Ok(id) = row {
                        result_ids.push(id);
                    }
                }
            }
            result_ids
        };
        
        let mut entities = Vec::new();
        for id in ids {
            if let Ok(Some(e)) = self.get_entity(&id).await {
                entities.push(e);
            }
        }
        
        Ok(entities)
    }
}

#[async_trait]
impl RawEventStore for SqliteKnowledgeStore {
    async fn save_raw_event(&self, event: &RawEvent) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO raw_events (id, source_type, source_id, external_id, payload, text, status) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.id,
                event.source_type,
                event.source_id,
                event.external_id,
                event.payload,
                event.text,
                event.status
            ],
        ).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(())
    }

    async fn create_job(&self, job: &Job) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO jobs (id, raw_event_id, job_type, status) VALUES (?1, ?2, ?3, ?4)",
            params![job.id, job.raw_event_id, job.job_type, job.status],
        ).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_next_pending_job(&self, job_type: &str) -> Result<Option<Job>> {
        let conn = self.conn.lock().unwrap();
        // Атомарный захват Job с использованием UPDATE ... RETURNING
        // SQLite 3.35+ поддерживает RETURNING. Это полностью устраняет гонку воркеров.
        let mut stmt = conn.prepare(
            "UPDATE jobs SET status = 'running' 
             WHERE id = (
                SELECT id FROM jobs 
                WHERE job_type = ?1 AND status = 'pending' 
                ORDER BY scheduled_at ASC 
                LIMIT 1
             )
             RETURNING id, raw_event_id, job_type, status"
        ).unwrap();

        let job = stmt.query_row(params![job_type], |row| {
            Ok(Job {
                id: row.get(0)?,
                raw_event_id: row.get(1)?,
                job_type: row.get(2)?,
                status: row.get(3)?,
            })
        }).optional().map_err(|e| BrainError::Database(e.to_string()))?;
        
        Ok(job)
    }

    async fn update_job_status(&self, job_id: &str, status: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE jobs SET status = ?1 WHERE id = ?2",
            params![status, job_id],
        ).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_raw_event(&self, event_id: &str) -> Result<Option<RawEvent>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, source_type, source_id, external_id, payload, text, status FROM raw_events WHERE id = ?1").unwrap();
        let event = stmt.query_row(params![event_id], |row| {
            Ok(RawEvent {
                id: row.get(0)?,
                source_type: row.get(1)?,
                source_id: row.get(2)?,
                external_id: row.get(3)?,
                payload: row.get(4)?,
                text: row.get(5)?,
                status: row.get(6)?,
            })
        }).optional().map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(event)
    }

    async fn save_observation(&self, observation: &Observation) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO observations (id, raw_event_id, entity_id, fact, confidence, schema_version, extractor_version) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                observation.id,
                observation.raw_event_id,
                observation.entity_id,
                observation.fact,
                observation.confidence,
                observation.schema_version,
                observation.extractor_version,
            ],
        ).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_observations(&self, entity_id: &str) -> Result<Vec<Observation>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, raw_event_id, entity_id, fact, confidence, schema_version, extractor_version FROM observations WHERE entity_id = ?1").unwrap();
        let obs_iter = stmt.query_map(params![entity_id], |row| {
            Ok(Observation {
                id: row.get(0)?,
                raw_event_id: row.get(1)?,
                entity_id: row.get(2)?,
                fact: row.get(3)?,
                confidence: row.get(4)?,
                schema_version: row.get(5)?,
                extractor_version: row.get(6)?,
            })
        }).map_err(|e| BrainError::Database(e.to_string()))?;
        
        let mut observations = Vec::new();
        for obs in obs_iter {
            if let Ok(o) = obs {
                observations.push(o);
            }
        }
        Ok(observations)
    }

    async fn get_debug_trace(&self, event_id: &str) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let mut trace = String::new();

        // 1. Raw Event
        let event: Option<(String, String)> = conn.query_row(
            "SELECT text, status FROM raw_events WHERE id = ?1",
            params![event_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().unwrap_or(None);

        if let Some((text, status)) = event {
            trace.push_str("RawEvent:\n");
            trace.push_str(&format!("\"{}\"\n", text));
            trace.push_str(&format!("(status: {})\n\n", status));
        } else {
            return Ok("Raw event not found.".into());
        }

        // 2. Job Status & Error
        let job: Option<(String, String, Option<String>)> = conn.query_row(
            "SELECT job_type, status, error FROM jobs WHERE raw_event_id = ?1",
            params![event_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional().unwrap_or(None);

        let mut job_status = String::new();
        let mut job_error = None;
        if let Some((jtype, status, error)) = job {
            job_status = status.clone();
            job_error = error;
            if status == "failed" {
                trace.push_str("Job Status:\n");
                trace.push_str(&format!("{} (failed)\n", jtype));
                if let Some(err_text) = &job_error {
                    trace.push_str(&format!("Errors:\n{}\n\n", err_text));
                } else {
                    trace.push_str("Errors:\n<No error details stored>\n\n");
                }
            }
        }

        // 3. Extractor / Observations
        let mut obs_stmt = conn.prepare("SELECT entity_id, fact, confidence, extractor_version, schema_version FROM observations WHERE raw_event_id = ?1").unwrap();
        let obs_iter = obs_stmt.query_map(params![event_id], |row| {
            Ok((
                row.get::<_, String>(0)?, 
                row.get::<_, String>(1)?, 
                row.get::<_, f64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        }).unwrap();

        let mut has_obs = false;
        for obs in obs_iter {
            if let Ok((eid, fact, conf, ext_ver, _schema)) = obs {
                has_obs = true;
                
                // Parse fact as structured observation if possible
                let parsed: std::result::Result<crate::extractor::StructuredObservation, _> = serde_json::from_str(&fact);
                
                trace.push_str("Extractor:\n");
                trace.push_str(&format!("→ Extractor version: {}\n", ext_ver));
                
                if let Ok(structured) = parsed {
                    trace.push_str(&format!("→ Extracted entities: {:?}\n", structured.entities));
                    trace.push_str(&format!("→ fact = {}\n\n", structured.summary));
                } else {
                    trace.push_str("→ Extracted entities: [Not Structured]\n");
                    trace.push_str(&format!("→ fact = {}\n\n", fact));
                }

                trace.push_str("Identity:\n");
                
                // 4. Entity
                let ent: Option<(String, String)> = conn.query_row(
                    "SELECT name, summary FROM entities WHERE id = ?1",
                    params![eid],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                ).optional().unwrap_or(None);
                
                if let Some((name, _summary)) = ent {
                    trace.push_str(&format!("→ matched {} id={}\n", name, eid));
                } else {
                    trace.push_str(&format!("→ matched unknown id={}\n", eid));
                }
                trace.push_str(&format!("→ confidence={}\n\n", conf));
                
                trace.push_str("Projection:\n");
                trace.push_str(&format!("→ updated {}.md\n\n", eid));
            }
        }
        
        if !has_obs && job_status != "failed" {
            trace.push_str("No observations generated yet.\n");
        }

        Ok(trace)
    }

    async fn record_metric(&self, name: &str, value: f64, event_id: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO system_metrics (metric_name, metric_value, event_id) VALUES (?1, ?2, ?3)",
            params![name, value, event_id],
        ).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(())
    }

    async fn get_metrics_report(&self) -> Result<brain_common::SystemMetricsReport> {
        let conn = self.conn.lock().unwrap();
        
        let processed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM raw_events WHERE status = 'completed'",
            [],
            |row| row.get(0)
        ).unwrap_or(0);

        let avg_latency: f64 = conn.query_row(
            "SELECT AVG(metric_value) FROM system_metrics WHERE metric_name = 'latency_ms'",
            [],
            |row| row.get(0)
        ).unwrap_or(0.0);

        let json_errors: i64 = conn.query_row(
            "SELECT SUM(metric_value) FROM system_metrics WHERE metric_name = 'json_parse_error'",
            [],
            |row| row.get(0)
        ).unwrap_or(0);

        let total_entities: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entities",
            [],
            |row| row.get(0)
        ).unwrap_or(0);

        let avg_confidence: f64 = conn.query_row(
            "SELECT AVG(confidence) FROM observations",
            [],
            |row| row.get(0)
        ).unwrap_or(0.0);

        let empty_responses: f64 = conn.query_row(
            "SELECT AVG(metric_value) FROM system_metrics WHERE metric_name = 'extractor_empty_response'",
            [],
            |row| row.get(0)
        ).unwrap_or(0.0);

        let avg_entities: f64 = conn.query_row(
            "SELECT AVG(metric_value) FROM system_metrics WHERE metric_name = 'extractor_entities_count'",
            [],
            |row| row.get(0)
        ).unwrap_or(0.0);

        let identity_exact: i64 = conn.query_row(
            "SELECT SUM(metric_value) FROM system_metrics WHERE metric_name = 'identity_exact'",
            [],
            |row| row.get(0)
        ).unwrap_or(0);
        let identity_alias: i64 = conn.query_row(
            "SELECT SUM(metric_value) FROM system_metrics WHERE metric_name = 'identity_alias'",
            [],
            |row| row.get(0)
        ).unwrap_or(0);
        let identity_fuzzy: i64 = conn.query_row(
            "SELECT SUM(metric_value) FROM system_metrics WHERE metric_name = 'identity_fuzzy'",
            [],
            |row| row.get(0)
        ).unwrap_or(0);
        let identity_semantic: i64 = conn.query_row(
            "SELECT SUM(metric_value) FROM system_metrics WHERE metric_name = 'identity_semantic'",
            [],
            |row| row.get(0)
        ).unwrap_or(0);
        let identity_nomatch: i64 = conn.query_row(
            "SELECT SUM(metric_value) FROM system_metrics WHERE metric_name = 'identity_nomatch'",
            [],
            |row| row.get(0)
        ).unwrap_or(0);

        let total_observations: i64 = conn.query_row(
            "SELECT COUNT(*) FROM observations",
            [],
            |row| row.get(0)
        ).unwrap_or(0);
        
        let avg_obs_per_entity = if total_entities > 0 {
            total_observations as f64 / total_entities as f64
        } else {
            0.0
        };

        let json_success_rate = if processed > 0 {
            100.0 - ((json_errors as f64 / processed as f64) * 100.0)
        } else {
            100.0
        };

        Ok(brain_common::SystemMetricsReport {
            processed_events: processed,
            avg_latency_ms: avg_latency,
            json_success_rate,
            empty_responses_percent: empty_responses * 100.0,
            avg_entities_extracted: avg_entities,
            avg_confidence,
            identity_exact,
            identity_alias,
            identity_fuzzy,
            identity_semantic,
            identity_nomatch,
            total_entities,
            total_observations,
            avg_obs_per_entity,
        })
    }

    async fn reset_event_processing(&self, event_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        // 1. Убираем observations (cascade)
        conn.execute("DELETE FROM observations WHERE raw_event_id = ?1", params![event_id])
            .map_err(|e| BrainError::Database(e.to_string()))?;
        
        // 2. Сбрасываем метрики
        conn.execute("DELETE FROM system_metrics WHERE event_id = ?1", params![event_id])
            .map_err(|e| BrainError::Database(e.to_string()))?;

        // 3. Сбрасываем job status
        conn.execute("UPDATE jobs SET status = 'pending' WHERE raw_event_id = ?1", params![event_id])
            .map_err(|e| BrainError::Database(e.to_string()))?;

        // 4. Сбрасываем event status
        conn.execute("UPDATE raw_events SET status = 'pending' WHERE id = ?1", params![event_id])
            .map_err(|e| BrainError::Database(e.to_string()))?;

        Ok(())
    }
}
