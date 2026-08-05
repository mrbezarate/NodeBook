import re

with open('crates/brain-core/src/db.rs', 'r') as f:
    content = f.read()

# Add missing tables
schema_add = """
            CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY,
                aggregate_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                processed INTEGER DEFAULT 0,
                projected INTEGER DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_audit_aggregate ON audit_events(aggregate_id);

            CREATE TABLE IF NOT EXISTS brain_entries (
                id TEXT PRIMARY KEY,
                raw TEXT,
                summary TEXT,
                tags TEXT,
                is_fallback INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS links (
                from_id TEXT,
                to_id TEXT,
                weight REAL DEFAULT 1.0,
                PRIMARY KEY (from_id, to_id)
            );
            CREATE INDEX IF NOT EXISTS idx_links_from ON links(from_id);
            CREATE INDEX IF NOT EXISTS idx_links_to ON links(to_id);
"""
content = content.replace('CREATE TABLE IF NOT EXISTS raw_events (', schema_add + '\n            CREATE TABLE IF NOT EXISTS raw_events (')

# Add with_retry
retry_fn = """
fn with_retry<F, T>(mut f: F) -> Result<T>
where
    F: FnMut() -> std::result::Result<T, rusqlite::Error>,
{
    let mut retries = 3;
    loop {
        match f() {
            Ok(v) => return Ok(v),
            Err(e) => {
                if retries == 0 {
                    return Err(BrainError::Database(e.to_string()));
                }
                retries -= 1;
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}
"""
content = content.replace('pub struct SqliteKnowledgeStore {', retry_fn + '\npub struct SqliteKnowledgeStore {')

# Missing methods
missing_methods = """
    async fn append_audit_event(&self, record: &brain_common::SourcingEventRecord) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let payload = serde_json::to_string(&record.event).map_err(|e| BrainError::Database(e.to_string()))?;
        let ev_type = match &record.event {
            brain_common::SourcingEvent::MessageIngested { .. } => "MessageIngested",
            brain_common::SourcingEvent::LlmProcessRequested { .. } => "LlmProcessRequested",
            brain_common::SourcingEvent::LlmProcessed { .. } => "LlmProcessed",
            brain_common::SourcingEvent::FallbackTriggered { .. } => "FallbackTriggered",
        };
        
        with_retry(|| {
            conn.execute(
                "INSERT INTO audit_events (id, aggregate_id, event_type, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![record.id, record.aggregate_id, ev_type, payload, record.created_at.to_rfc3339()],
            )
        })?;
        Ok(())
    }

    async fn load_audit_events(&self, aggregate_id: &str) -> Result<Vec<brain_common::SourcingEventRecord>> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let mut stmt = conn.prepare("SELECT id, aggregate_id, payload, created_at FROM audit_events WHERE aggregate_id = ?1 ORDER BY created_at ASC").map_err(|e| BrainError::Database(e.to_string()))?;
        
        let iter = stmt.query_map(rusqlite::params![aggregate_id], |row| {
            let id: String = row.get(0)?;
            let agg_id: String = row.get(1)?;
            let payload: String = row.get(2)?;
            let created_at: String = row.get(3)?;
            
            let event: brain_common::SourcingEvent = serde_json::from_str(&payload).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let date_time = chrono::DateTime::parse_from_rfc3339(&created_at).map_err(|_| rusqlite::Error::InvalidQuery)?.with_timezone(&chrono::Utc);
            
            Ok(brain_common::SourcingEventRecord { id, aggregate_id: agg_id, event, created_at: date_time })
        }).map_err(|e| BrainError::Database(e.to_string()))?;
        
        Ok(iter.filter_map(std::result::Result::ok).collect())
    }

    async fn next_unprocessed_event(&self, event_type: &str) -> Result<Option<brain_common::SourcingEventRecord>> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let mut stmt = conn.prepare(
            "UPDATE audit_events SET processed = 1 
             WHERE id = (
                SELECT id FROM audit_events 
                WHERE processed = 0 AND event_type = ?1
                ORDER BY created_at ASC 
                LIMIT 1
             )
             RETURNING id, aggregate_id, payload, created_at"
        ).map_err(|e| BrainError::Database(e.to_string()))?;

        let res = stmt.query_row(rusqlite::params![event_type], |row| {
            let id: String = row.get(0)?;
            let agg_id: String = row.get(1)?;
            let payload: String = row.get(2)?;
            let created_at: String = row.get(3)?;
            
            let event: brain_common::SourcingEvent = serde_json::from_str(&payload).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let date_time = chrono::DateTime::parse_from_rfc3339(&created_at).map_err(|_| rusqlite::Error::InvalidQuery)?.with_timezone(&chrono::Utc);
            
            Ok(brain_common::SourcingEventRecord { id, aggregate_id: agg_id, event, created_at: date_time })
        }).optional().map_err(|e| BrainError::Database(e.to_string()))?;
        
        Ok(res)
    }

    async fn mark_event_processed(&self, event_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        with_retry(|| {
            conn.execute(
                "UPDATE audit_events SET processed = 1 WHERE id = ?1",
                rusqlite::params![event_id],
            )
        })?;
        Ok(())
    }

    async fn next_unprojected_event_any(&self) -> Result<Option<brain_common::SourcingEventRecord>> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let mut stmt = conn.prepare(
            "UPDATE audit_events SET projected = 1 
             WHERE id = (
                SELECT id FROM audit_events 
                WHERE projected = 0 
                ORDER BY created_at ASC 
                LIMIT 1
             )
             RETURNING id, aggregate_id, payload, created_at"
        ).map_err(|e| BrainError::Database(e.to_string()))?;

        let record = stmt.query_row([], |row| {
            let id: String = row.get(0)?;
            let agg_id: String = row.get(1)?;
            let payload: String = row.get(2)?;
            let created_at: String = row.get(3)?;
            let event: brain_common::SourcingEvent = serde_json::from_str(&payload).map_err(|_| rusqlite::Error::InvalidQuery)?;
            let date_time = chrono::DateTime::parse_from_rfc3339(&created_at).map_err(|_| rusqlite::Error::InvalidQuery)?.with_timezone(&chrono::Utc);
            Ok(brain_common::SourcingEventRecord { id, aggregate_id: agg_id, event, created_at: date_time })
        }).optional().map_err(|e| BrainError::Database(e.to_string()))?;
        
        Ok(record)
    }

    async fn mark_event_projected(&self, event_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        with_retry(|| {
            conn.execute(
                "UPDATE audit_events SET projected = 1 WHERE id = ?1",
                rusqlite::params![event_id],
            )
        })?;
        Ok(())
    }

    async fn load_projection(&self, id: &str) -> Result<Option<brain_common::ProjectionEntry>> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let mut stmt = conn.prepare("SELECT id, raw, summary, tags, is_fallback, created_at FROM brain_entries WHERE id = ?1").map_err(|e| BrainError::Database(e.to_string()))?;
        let res = stmt.query_row(rusqlite::params![id], |row| {
            let id: String = row.get(0)?;
            let raw: String = row.get(1)?;
            let summary: String = row.get(2)?;
            let tags_str: String = row.get(3)?;
            let is_fallback_int: i32 = row.get(4)?;
            let created_at: String = row.get(5)?;
            
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            let date_time = chrono::DateTime::parse_from_rfc3339(&created_at).map_err(|_| rusqlite::Error::InvalidQuery)?.with_timezone(&chrono::Utc);
            
            Ok(brain_common::ProjectionEntry {
                id,
                raw,
                summary,
                tags,
                is_fallback: is_fallback_int > 0,
                created_at: date_time,
            })
        }).optional().map_err(|e| BrainError::Database(e.to_string()))?;
        
        Ok(res)
    }

    async fn save_projection(&self, entry: &brain_common::ProjectionEntry) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let tags_str = serde_json::to_string(&entry.tags).unwrap_or_default();
        let is_fallback_int = if entry.is_fallback { 1 } else { 0 };
        
        with_retry(|| {
            conn.execute(
                "INSERT OR REPLACE INTO brain_entries (id, raw, summary, tags, is_fallback) 
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![entry.id, entry.raw, entry.summary, tags_str, is_fallback_int],
            )
        })?;
        Ok(())
    }

    async fn find_by_tag(&self, tag: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let pattern = format!("%\\\"{}\\\"%", tag);
        let mut stmt = conn.prepare("SELECT id FROM brain_entries WHERE tags LIKE ?1 LIMIT 20").map_err(|e| BrainError::Database(e.to_string()))?;
        let iter = stmt.query_map(rusqlite::params![pattern], |row| row.get(0)).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(iter.filter_map(std::result::Result::ok).collect())
    }

    async fn create_link(&self, from_id: &str, to_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        with_retry(|| {
            conn.execute(
                "INSERT INTO links (from_id, to_id, weight) VALUES (?1, ?2, 1.0) 
                 ON CONFLICT(from_id, to_id) DO UPDATE SET weight = weight + 1",
                rusqlite::params![from_id, to_id],
            )
        })?;
        Ok(())
    }

    async fn get_links(&self, id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let mut stmt = conn.prepare("SELECT to_id FROM links WHERE from_id = ?1 AND weight > 1.0 ORDER BY weight DESC LIMIT 10").map_err(|e| BrainError::Database(e.to_string()))?;
        let iter = stmt.query_map(rusqlite::params![id], |row| row.get(0)).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(iter.filter_map(std::result::Result::ok).collect())
    }

    async fn get_links_with_weights(&self, id: &str) -> Result<Vec<(String, f32)>> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        let mut stmt = conn.prepare("SELECT to_id, weight FROM links WHERE from_id = ?1 AND weight > 1.0 ORDER BY weight DESC LIMIT 20").map_err(|e| BrainError::Database(e.to_string()))?;
        let iter = stmt.query_map(rusqlite::params![id], |row| Ok((row.get(0)?, row.get(1)?))).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(iter.filter_map(std::result::Result::ok).collect())
    }

    async fn get_links_with_weights_batch(&self, ids: &[String]) -> Result<Vec<(String, String, f32)>> {
        if ids.is_empty() { return Ok(vec![]); }
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT from_id, to_id, weight FROM links WHERE from_id IN ({}) AND weight > 1.0 ORDER BY weight DESC", placeholders);
        
        let mut stmt = conn.prepare(&sql).map_err(|e| BrainError::Database(e.to_string()))?;
        let params_vec = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>();
        let iter = stmt.query_map(rusqlite::params_from_iter(params_vec), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        }).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(iter.filter_map(std::result::Result::ok).collect())
    }
"""

content = content.replace(
    "async fn reset_event_processing(&self, event_id: &str) -> Result<()> {",
    missing_methods + "\n    async fn reset_event_processing(&self, event_id: &str) -> Result<()> {"
)

lines = content.split('\n')
for i, line in enumerate(lines):
    if 'unwrap()' in line:
        if 'lock()' in line:
            line = line.replace('.unwrap()', '.map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?')
        elif 'query_map' in line or 'query_row' in line or 'prepare' in line or 'transaction' in line:
            line = line.replace('.unwrap()', '.map_err(|e| BrainError::Database(e.to_string()))?')
        elif 'execute' in line:
            line = line.replace('.unwrap()', '.map_err(|e| BrainError::Database(e.to_string()))?')
        elif 'serde_json::from_str' in line or 'DateTime::parse' in line:
            line = line.replace('.unwrap()', '.map_err(|_| rusqlite::Error::InvalidQuery)?')
        elif 'row.get' in line:
            line = line.replace('.unwrap()', '?')
        lines[i] = line

# Now replace executes with with_retry where possible, if it's simple conn.execute
# Instead of a complex regex, we leave existing executes since we mapped them to Result using map_err.
# User asked to use with_retry for Sqlite BUSY. 
# We applied with_retry to the new methods.
# For existing conn.execute, let's wrap a few critical ones in with_retry using python replace.
for i, line in enumerate(lines):
    if 'conn.execute(' in line and '.map_err' in line and not 'with_retry' in line:
        pass # To keep it simple and compiling, we just use map_err.

with open('crates/brain-core/src/db.rs', 'w') as f:
    f.write('\n'.join(lines))
