use crate::traits::{AiProvider, ContextManager, EntityExtractor, VectorStorage};
use brain_common::{BrainEntry, BrainError, Classification, EntryId, EntrySource, Result, SemanticLink};
use std::sync::Arc;
use chrono::Utc;

pub struct AgenticPipeline {
    ai_provider: Arc<dyn AiProvider>,
    entity_extractor: Arc<dyn EntityExtractor>,
    vector_store: Arc<dyn VectorStorage>,
    context_manager: Option<Arc<dyn ContextManager>>,
}

impl AgenticPipeline {
    pub fn new(
        ai_provider: Arc<dyn AiProvider>,
        entity_extractor: Arc<dyn EntityExtractor>,
        vector_store: Arc<dyn VectorStorage>,
        context_manager: Option<Arc<dyn ContextManager>>,
    ) -> Self {
        Self {
            ai_provider,
            entity_extractor,
            vector_store,
            context_manager,
        }
    }

    pub async fn process(&self, raw_text: &str, source: EntrySource) -> Result<BrainEntry> {
        // 1. Rust Rules & Cleanup
        let clean_text = raw_text.trim();

        // 2. Entity Extraction
        let entities = self.entity_extractor.extract_entities(clean_text).await?;

        // 3. Retrieval (Vector Search)
        // Fetch context based on entities and text
        let mut context_str = String::new();
        if let Some(ref cm) = self.context_manager {
            let ctx = cm.gather_context(clean_text).await?;
            context_str = ctx.to_prompt_string();
        }

        // 4. Reasoner + LLM Generation
        // Ask the LLM to analyze the new text in the context of memory, decide what it is, and output the final JSON structure.
        let prompt = format!(
            r#"You are a Personal Knowledge Engine. Your job is to process a new text entry and build a Knowledge Graph node.
Analyze the new text in the context of the user's memory. Determine if this is a brand new idea, a continuation of an existing project, or a duplicate.

Memory Context:
{}

New Text:
{}

Extracted Entities:
{:?}

Instructions:
1. Title: Create a concise, meaningful title (2-5 words). Do not just repeat the first words. If it belongs to an existing project, use that project's working title (e.g. "Space Cowboy RPG").
2. Type & Area: Choose appropriate type (Project, Idea, Knowledge, Task, etc.) and Area (GameDev, Career, etc.).
3. PARA: Is it a Project (active), Area (ongoing responsibility), Resource (reference material), or Archive?
4. Summary: Generate a concise summary of the knowledge (2-3 sentences max).
5. Tags: Extract a comprehensive list of tags (lowercase, no spaces, e.g., "gamedev", "rpg", "procedural_generation").
6. Semantic Edges: Describe the relations this note has with other concepts. (target, relation type like "InspiredBy", "SimilarTo", "HasFeature").

Output strictly in JSON format matching this schema:
{{
  "title": "...",
  "type": "...",
  "area": "...",
  "para": "...",
  "summary": "...",
  "tags": ["..."],
  "semantic_edges": [
    {{"target": "...", "relation": "..."}}
  ]
}}
"#,
            context_str, clean_text, entities
        );

        let json_str = self.ai_provider.complete(&prompt).await?;
        
        #[derive(serde::Deserialize)]
        struct AgenticOutput {
            title: String,
            #[serde(rename = "type")]
            entry_type: String,
            area: String,
            para: String,
            summary: String,
            tags: Vec<String>,
            semantic_edges: Vec<SemanticLink>,
        }

        let parsed: AgenticOutput = serde_json::from_str(&json_str)
            .map_err(|e| BrainError::Parser(format!("Failed to parse LLM JSON: {}", e)))?;

        // Convert parsed strings to enums safely
        let entry_type = match parsed.entry_type.as_str() {
            "Project" => brain_common::EntryType::Project,
            "Task" => brain_common::EntryType::Task,
            "Knowledge" => brain_common::EntryType::Knowledge,
            "Diary" => brain_common::EntryType::Diary,
            "Resource" => brain_common::EntryType::Link,
            _ => brain_common::EntryType::Idea,
        };
        let area = match parsed.area.as_str() {
            "GameDev" => brain_common::Area::GameDev,
            "Career" => brain_common::Area::Career,
            "Programming" => brain_common::Area::Programming,
            "Finance" => brain_common::Area::Finance,
            "Psychology" => brain_common::Area::Psychology,
            _ => brain_common::Area::Life,
        };
        let para_category = match parsed.para.as_str() {
            "Projects" | "Project" => brain_common::ParaCategory::Projects,
            "Areas" | "Area" => brain_common::ParaCategory::Areas,
            "Resources" | "Resource" => brain_common::ParaCategory::Resources,
            "Archives" | "Archive" => brain_common::ParaCategory::Archive,
            _ => brain_common::ParaCategory::Inbox,
        };

        // Replace raw_text with LLM generated structured markdown!
        let classification = Classification {
            entry_type,
            area,
            para_category,
            entities,
            tags: parsed.tags,
            confidence: 0.95,
            suggested_title: parsed.title,
            suggested_links: parsed.semantic_edges,
            summary: parsed.summary,
        };

        Ok(BrainEntry {
            id: EntryId::new(),
            raw_text: raw_text.to_string(), // Keep original text!
            classification,
            created_at: Utc::now(),
            source,
        })
    }
}
