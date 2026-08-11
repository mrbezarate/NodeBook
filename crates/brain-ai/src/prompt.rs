//! Шаблоны промптов для AI-провайдеров (Gemini, OpenAI, Ollama).
pub struct PromptBuilder;

impl PromptBuilder {
    /// Промпт для классификации текста.
    pub fn classification_prompt(text: &str, categories: &[&str]) -> String {
        format!(
            "Classify the following text into EXACTLY ONE of these categories: {}\n\nText: \"{}\"\n\nRespond with ONLY a JSON object: {{\"category\": \"...\", \"confidence\": 0.0-1.0}}\nDo not include markdown wrappers or extra text.",
            categories.join(", "), text
        )
    }

    /// Промпт для извлечения сущностей.
    pub fn entity_extraction_prompt(text: &str) -> String {
        format!(
            "Extract named entities and core concepts from this text. Match the language of the original text.\nTypes: Technology, Person, Place, Concept, Tool, Project, Framework\n\nText: \"{}\"\n\nRespond with ONLY a JSON array of objects: [{{\"name\": \"...\", \"type\": \"...\"}}]. No other text.",
            text
        )
    }

    /// Промпт для генерации заголовка.
    pub fn title_prompt(text: &str, entry_type: &str) -> String {
        format!(
            "Create a short, natural, descriptive title (2-5 words) for this {} entry in the SAME LANGUAGE as the text:\n\"{}\"\n\nRespond with ONLY the plain title text. No quotes, no markdown, no prefixes.",
            entry_type, text
        )
    }

    /// Промпт для генерации краткого содержания.
    pub fn summary_prompt(text: &str) -> String {
        format!(
            "Summarize the following message in 1-2 clear, direct sentences in the SAME LANGUAGE as the text. Focus on facts and action items without meta-talk:\n\"{}\"\n\nRespond with ONLY the summary text.",
            text
        )
    }
}
