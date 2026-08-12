use crate::srs::{default_vocab_bank, VocabCard};
use brain_core::traits::AiProvider;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::sync::Arc;

pub struct EnglishTutorEngine {
    cards: Vec<VocabCard>,
    ai_provider: Option<Arc<dyn AiProvider>>,
}

impl Default for EnglishTutorEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl EnglishTutorEngine {
    pub fn new() -> Self {
        Self {
            cards: default_vocab_bank(),
            ai_provider: None,
        }
    }

    pub fn with_ai(ai_provider: Arc<dyn AiProvider>) -> Self {
        Self {
            cards: default_vocab_bank(),
            ai_provider: Some(ai_provider),
        }
    }

    pub fn get_random_card(&self) -> Option<VocabCard> {
        let mut rng = thread_rng();
        self.cards.choose(&mut rng).cloned()
    }

    pub fn get_card(&self, id: &str) -> Option<VocabCard> {
        self.cards.iter().find(|c| c.id == id).cloned()
    }

    pub fn generate_quiz(&self) -> (VocabCard, Vec<String>) {
        let card = self.get_random_card().unwrap_or_else(|| self.cards[0].clone());
        let mut options = vec![card.translation_ru.clone()];

        let mut other_cards: Vec<String> = self
            .cards
            .iter()
            .filter(|c| c.id != card.id)
            .map(|c| c.translation_ru.clone())
            .collect();

        other_cards.shuffle(&mut thread_rng());

        for opt in other_cards.into_iter().take(3) {
            options.push(opt);
        }

        options.shuffle(&mut thread_rng());
        (card, options)
    }

    pub async fn analyze_grammar(&self, user_text: &str) -> String {
        let text = user_text.trim();
        if text.is_empty() {
            return "Please type a sentence in English to analyze!".to_string();
        }

        if let Some(ai) = &self.ai_provider {
            let prompt = format!(
                "You are an expert English language tutor. Analyze this sentence written by a student: \"{}\".\n\n\
                Provide a friendly response in Russian containing:\n\
                1. Corrected version (if any errors exist).\n\
                2. Estimated CEFR level (A1, A2, B1, B2, C1, C2).\n\
                3. Detailed grammar & vocabulary explanation.\n\
                4. Two more natural native phrasing alternatives.\n\n\
                Format with clean markdown.",
                text
            );

            match ai.complete(&prompt).await {
                Ok(resp) => return format!("📝 *Gemini AI Grammar Tutor*\n\nSentence: _\"{text}\"_\n\n{}", resp),
                Err(e) => {
                    tracing::warn!("AI grammar analysis failed, using fallback: {}", e);
                }
            }
        }

        Self::evaluate_grammar_feedback_fallback(text)
    }

    pub async fn tutor_dialogue(&self, user_message: &str) -> String {
        let text = user_message.trim();

        if let Some(ai) = &self.ai_provider {
            let prompt = format!(
                "You are a friendly, highly encouraging English conversation tutor. The student says: \"{}\".\n\n\
                Respond as an AI tutor:\n\
                1. Answer their message naturally in English.\n\
                2. Ask 1 open-ended follow-up question to keep the conversation going.\n\
                3. Add a short Russian translation note at the bottom for tricky words.",
                text
            );

            match ai.complete(&prompt).await {
                Ok(resp) => return resp,
                Err(e) => {
                    tracing::warn!("AI dialogue failed: {}", e);
                }
            }
        }

        format!("🇬🇧 *English Practice Mode*\n\nYou said: \"{}\"\n\nGreat practice! Keep writing sentences in English or try `/grammar {}` for a full structural check.", text, text)
    }

    fn evaluate_grammar_feedback_fallback(text: &str) -> String {
        let mut suggestions = Vec::new();

        if !text.chars().next().map_or(false, |c| c.is_uppercase()) {
            suggestions.push("• Capitalize the first letter of your sentence.");
        }

        if !text.ends_with('.') && !text.ends_with('?') && !text.ends_with('!') {
            suggestions.push("• Add a period (.) or question mark (?) at the end.");
        }

        if text.contains(" i ") || text.starts_with("i ") {
            suggestions.push("• Always capitalize the personal pronoun 'I'.");
        }

        let feedback_header = format!("📝 *Grammar & Syntax Analysis*\n\nSentence: _\"{text}\"_\n");

        if suggestions.is_empty() {
            format!("{feedback_header}\n✨ *Great job!* Your sentence structure looks clean and natural.")
        } else {
            format!(
                "{}\n💡 *Suggestions for improvement:*\n{}",
                feedback_header,
                suggestions.join("\n")
            )
        }
    }
}
