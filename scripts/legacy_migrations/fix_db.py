with open('crates/brain-core/src/db.rs', 'r') as f:
    lines = f.readlines()

missing_methods = """
    async fn reset_event_processing(&self, event_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| BrainError::Database(format!("Lock error: {}", e)))?;
        conn.execute("DELETE FROM observations WHERE raw_event_id = ?1", rusqlite::params![event_id]).map_err(|e| BrainError::Database(e.to_string()))?;
        Ok(())
    }

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
"""

# Find the last closing brace in the file
last_brace_idx = len(lines) - 1
while last_brace_idx >= 0 and lines[last_brace_idx].strip() != '}':
    last_brace_idx -= 1

if last_brace_idx >= 0:
    lines.insert(last_brace_idx, missing_methods + "\n")

with open('crates/brain-core/src/db.rs', 'w') as f:
    f.writelines(lines)
