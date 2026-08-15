use anyhow::Result;
use brain_common::SearchResult;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Full-text search indexer using Tantivy.
pub struct BrainIndexer {
    index: Index,
    reader: IndexReader,
    writer: Arc<RwLock<IndexWriter>>,
    
    // Schema fields
    pub id_field: Field,
    pub title_field: Field,
    pub body_field: Field,
    pub tags_field: Field,
}

impl BrainIndexer {
    /// Creates a new indexer. If path is None, uses an in-memory index.
    pub fn new(path: Option<&Path>) -> Result<Self> {
        let mut schema_builder = Schema::builder();
        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let body_field = schema_builder.add_text_field("body", TEXT | STORED);
        let tags_field = schema_builder.add_text_field("tags", TEXT | STORED);
        let schema = schema_builder.build();

        let index = if let Some(p) = path {
            std::fs::create_dir_all(p)?;
            let _ = std::fs::remove_file(p.join(".tantivy-writer.lock"));
            let _ = std::fs::remove_file(p.join(".tantivy-meta.lock"));
            Index::open_or_create(tantivy::directory::MmapDirectory::open(p)?, schema.clone())?
        } else {
            Index::create_in_ram(schema.clone())
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        // Use 50MB of heap for index writer, with automatic stale lock recovery
        let writer = match index.writer(50_000_000) {
            Ok(w) => w,
            Err(e) => {
                if let Some(p) = path {
                    let _ = std::fs::remove_file(p.join(".tantivy-writer.lock"));
                    let _ = std::fs::remove_file(p.join(".tantivy-meta.lock"));
                    index.writer(50_000_000)?
                } else {
                    return Err(e.into());
                }
            }
        };

        Ok(Self {
            index,
            reader,
            writer: Arc::new(RwLock::new(writer)),
            id_field,
            title_field,
            body_field,
            tags_field,
        })
    }

    /// Add or update a document in the index.
    pub async fn add_document(&self, id: &str, title: &str, body: &str, tags: &[String]) -> Result<()> {
        let tags_str = tags.join(" ");
        let mut writer = self.writer.write().await;
        
        // Remove existing document with the same ID before inserting to avoid duplicates
        writer.delete_term(Term::from_field_text(self.id_field, id));
        
        writer.add_document(doc!(
            self.id_field => id,
            self.title_field => title,
            self.body_field => body,
            self.tags_field => tags_str
        ))?;
        writer.commit()?;
        Ok(())
    }

    /// Search the index across title, body, and tags.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![
            self.title_field,
            self.body_field,
            self.tags_field,
        ]);
        
        let query = query_parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;
        
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let id = retrieved_doc.get_first(self.id_field).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let title = retrieved_doc.get_first(self.title_field).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let body = retrieved_doc.get_first(self.body_field).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            
            // Just return a short snippet from the body
            let snippet = body.chars().take(200).collect::<String>();
            
            results.push(SearchResult {
                entry_id: brain_common::EntryId::from_string(&id),
                file_path: id.clone(),
                title,
                snippet,
                score,
            });
        }
        
        Ok(results)
    }
}
