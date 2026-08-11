use crate::traits::{AiProvider, ContextManager, EntityExtractor, VectorStorage};
use brain_common::{BrainEntry, BrainError, Classification, EntryId, EntrySource, Result, SemanticLink};
use std::sync::Arc;
use chrono::Utc;

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
    enriched_text: Option<String>,
}

pub struct AgenticPipeline {
    ai_provider: Arc<dyn AiProvider>,
    entity_extractor: Arc<dyn EntityExtractor>,
    #[allow(dead_code)]
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
1. Title: Create a highly specific noun or proper noun for the title (e.g. "Екдрасиль" instead of "Idea for a game"). If none exists, synthesize a 1-3 word noun phrase.
2. Type & Area: Choose appropriate type (Project, Idea, Knowledge, Task, etc.) and Area (GameDev, Career, etc.).
3. PARA: Is it a Project (active), Area (ongoing responsibility), Resource (reference material), or Archive?
4. Summary: Generate a concise summary of the knowledge (2-3 sentences max).
5. Tags: Extract a comprehensive list of tags (lowercase, no spaces, e.g., "gamedev", "rpg", "procedural_generation").
6. Semantic Edges: Describe the relations this note has with other concepts. (target, relation type like "InspiredBy", "SimilarTo", "HasFeature").
7. Enriched Text: Take the exact original user message, but replace any mentioned entities, concepts, or project names with Obsidian wikilinks (e.g. wrap them in [[...]] like `проект [[Екдрасиль]]`). Do not change the tone, grammar, or words otherwise.

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
  ],
  "enriched_text": "..."
}}
"#,
            context_str, clean_text, entities
        );

        let mut retries = 3;
        let mut parsed: Option<AgenticOutput> = None;
        let mut last_err = String::new();

        while retries > 0 {
            match self.ai_provider.complete_json(&prompt).await {
                Ok(json_str) => {
                    match serde_json::from_str::<AgenticOutput>(&json_str) {
                        Ok(result) => {
                            parsed = Some(result);
                            break;
                        }
                        Err(e) => {
                            last_err = format!("JSON parse error: {}", e);
                            tracing::warn!("LLM output parsing failed, retrying. Error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    last_err = format!("LLM completion error: {:?}", e);
                    tracing::warn!("LLM completion failed, retrying. Error: {:?}", e);
                }
            }
            retries -= 1;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        let parsed = parsed.ok_or_else(|| BrainError::Parser(format!("Failed after 3 retries. Last error: {}", last_err)))?;

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
            enriched_text: parsed.enriched_text,
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
