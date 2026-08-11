//! # brain-ai — AI-провайдер (Ollama).
//! AI используется ТОЛЬКО для: классификации (fallback), извлечения сущностей, заголовков, summary.
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod prompt;
pub mod provider;
pub mod response;

pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
pub use provider::NoopAiProvider;
pub use prompt::PromptBuilder;
