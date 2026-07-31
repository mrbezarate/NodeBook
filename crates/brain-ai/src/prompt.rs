//! Шаблоны промптов для маленьких моделей (Phi-3, Qwen2).
pub struct PromptBuilder;

impl PromptBuilder {
    /// Промпт для классификации текста.
    pub fn classification_prompt(text: &str, categories: &[&str]) -> String {
        format!(
            "Classify the following text into ONE of these categories: {}\n\nText: \"{}\"\n\nRespond with ONLY a JSON object: {{\"category\": \"...\", \"confidence\": 0.0-1.0}}\nDo not include any other text.",
            categories.join(", "), text
        )
    }

    /// Промпт для извлечения сущностей.
    pub fn entity_extraction_prompt(text: &str) -> String {
        format!(
            "Extract named entities from this text. Return JSON array of objects with \"name\" and \"type\" fields.\nTypes: Technology, Person, Place, Concept, Tool, Language, Framework\n\nText: \"{}\"\n\nRespond with ONLY a JSON array. No other text.",
            text
        )
    }

    /// Промпт для генерации заголовка.
    pub fn title_prompt(text: &str, entry_type: &str) -> String {
        format!(
            "Create a short, descriptive title (3-7 words) for this {} entry:\n\"{}\"\n\nRespond with ONLY the title. No quotes, no other text.",
            entry_type, text
        )
    }

    /// Промпт для генерации краткого содержания.
    pub fn summary_prompt(text: &str) -> String {
        format!(
            "Summarize in 1-2 sentences:\n\"{}\"\n\nRespond with ONLY the summary.",
            text
        )
    }
}
