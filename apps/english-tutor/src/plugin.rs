use crate::tutor::EnglishTutorEngine;
use async_trait::async_trait;
use brain_common::Result;
use brain_core::traits::AiProvider;
use brain_plugin::{
    Plugin, PluginCapability, PluginCommand, PluginManifest, PluginMessage, PluginResponse,
    PluginStatus,
};
use std::sync::Arc;

pub struct EnglishTutorPlugin {
    engine: Arc<EnglishTutorEngine>,
}

impl Default for EnglishTutorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl EnglishTutorPlugin {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(EnglishTutorEngine::new()),
        }
    }

    pub fn new_with_ai(ai_provider: Arc<dyn AiProvider>) -> Self {
        Self {
            engine: Arc::new(EnglishTutorEngine::with_ai(ai_provider)),
        }
    }
}

#[async_trait]
impl Plugin for EnglishTutorPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "english-tutor".to_string(),
            name: "English Tutor SRS & AI Assistant".to_string(),
            version: "0.2.0".to_string(),
            description: "Interactive English learning with SRS flashcards, quizzes & Gemini AI conversational tutor".to_string(),
            capabilities: vec![PluginCapability::LanguageLearning],
            author: "NodeBook Ecosystem".to_string(),
            is_external: false,
            endpoint_url: None,
        }
    }

    async fn handle_command(&self, cmd: &PluginCommand) -> Result<PluginResponse> {
        let clean_cmd = cmd.command.split('@').next().unwrap_or(&cmd.command);
        match clean_cmd {
            "english" | "eng" | "vocab" | "quiz" => {
                if clean_cmd == "quiz" || cmd.args.first().map_or(false, |a| a == "quiz") {
                    let (card, options) = self.engine.generate_quiz();

                    let text = format!(
                        "🇬🇧 <b>English Quiz</b>\n\nWord: <b>{}</b> <i>{}</i>\nLevel: <code>{}</code>\n\nChoose the correct translation:",
                        escape_html(&card.word),
                        escape_html(&card.phonetic),
                        escape_html(&card.level)
                    );

                    let mut kb_options = Vec::new();
                    for opt in options {
                        let is_correct = opt == card.translation_ru;
                        let cb_data = if is_correct {
                            format!("eng:ans:correct:{}", card.word)
                        } else {
                            format!("eng:ans:wrong:{}", card.word)
                        };
                        kb_options.push((opt, cb_data));
                    }

                    return Ok(PluginResponse::Keyboard {
                        text,
                        options: kb_options,
                    });
                }

                let card = match self.engine.get_random_card() {
                    Some(c) => c,
                    None => return Ok(PluginResponse::Text("Vocabulary bank is empty.".to_string())),
                };
                let text = format!(
                    "🇬🇧 <b>English Flashcard</b>\n\n📖 <b>Word:</b> {} <i>{}</i>\n📊 <b>Level:</b> <code>{}</code>\n\n🇷🇺 <b>Translation:</b> {}\n\n💬 <b>Example:</b> {}\n<i>{}</i>",
                    escape_html(&card.word),
                    escape_html(&card.phonetic),
                    escape_html(&card.level),
                    escape_html(&card.translation_ru),
                    escape_html(&card.example_en),
                    escape_html(&card.example_ru)
                );

                let options = vec![
                    ("🟢 Next Flashcard".to_string(), "eng:next".to_string()),
                    ("🎯 Take Quiz".to_string(), "eng:quiz".to_string()),
                    ("🤖 AI Practice".to_string(), "eng:tutor".to_string()),
                ];

                Ok(PluginResponse::Keyboard { text, options })
            }
            "grammar" => {
                let input = cmd.args.join(" ");
                if input.is_empty() {
                    return Ok(PluginResponse::Text(
                        "📝 <b>Gemini Grammar Analyzer</b>\n\n<b>Usage:</b> <code>/grammar &lt;your English sentence&gt;</code>\n<b>Example:</b> <code>/grammar I has been working on this project</code>".to_string(),
                    ));
                }

                let feedback = self.engine.analyze_grammar(&input).await;
                Ok(PluginResponse::Text(feedback))
            }
            "tutor" | "practice" => {
                let input = cmd.args.join(" ");
                if input.is_empty() {
                    return Ok(PluginResponse::Text(
                        "🇬🇧 <b>Gemini AI English Tutor</b>\n\n<b>Usage:</b> <code>/tutor &lt;your message in English&gt;</code>\n<b>Example:</b> <code>/tutor Hi! I want to practice speaking about tech and software development.</code>".to_string(),
                    ));
                }

                let response = self.engine.tutor_dialogue(&input).await;
                Ok(PluginResponse::Text(response))
            }
            _ => Ok(PluginResponse::Ignored),
        }
    }

    async fn handle_message(&self, _msg: &PluginMessage) -> Result<PluginResponse> {
        Ok(PluginResponse::Ignored)
    }

    async fn handle_callback(&self, callback_data: &str, _user_id: u64) -> Result<PluginResponse> {
        if let Some(rest) = callback_data.strip_prefix("eng:") {
            if rest == "next" {
                let card = match self.engine.get_random_card() {
                    Some(c) => c,
                    None => return Ok(PluginResponse::Text("Vocabulary bank is empty.".to_string())),
                };
                let text = format!(
                    "🇬🇧 *English Flashcard*\n\n📖 *Word:* {} {}\n📊 *Level:* `{}`\n\n🇷🇺 *Translation:* {}\n\n💬 *Example:* {}\n_{}_",
                    card.word, card.phonetic, card.level, card.translation_ru, card.example_en, card.example_ru
                );
                let options = vec![
                    ("🟢 Next Flashcard".to_string(), "eng:next".to_string()),
                    ("🎯 Take Quiz".to_string(), "eng:quiz".to_string()),
                    ("🤖 AI Practice".to_string(), "eng:tutor".to_string()),
                ];
                return Ok(PluginResponse::Keyboard { text, options });
            } else if rest == "quiz" {
                let (card, options) = self.engine.generate_quiz();

                let text = format!(
                    "🇬🇧 *English Quiz*\n\nWord: *{}* {}\nLevel: `{}`\n\nChoose the correct translation:",
                    card.word, card.phonetic, card.level
                );

                let mut kb_options = Vec::new();
                for opt in options {
                    let is_correct = opt == card.translation_ru;
                    let cb_data = if is_correct {
                        format!("eng:ans:correct:{}", card.word)
                    } else {
                        format!("eng:ans:wrong:{}", card.word)
                    };
                    kb_options.push((opt, cb_data));
                }

                return Ok(PluginResponse::Keyboard {
                    text,
                    options: kb_options,
                });
            } else if rest == "tutor" {
                return Ok(PluginResponse::Text(
                    "🤖 *Gemini AI Conversation Tutor*\n\nSend any sentence starting with `/tutor <message>` to chat in English and receive corrections!".to_string(),
                ));
            } else if let Some(ans) = rest.strip_prefix("ans:") {
                if ans.starts_with("correct:") {
                    let word = &ans["correct:".len()..];
                    let text = format!("✅ *Correct!* You got it right for *{}*!\n\nKeep practicing!", word);
                    let options = vec![
                        ("🟢 Next Flashcard".to_string(), "eng:next".to_string()),
                        ("🎯 Another Quiz".to_string(), "eng:quiz".to_string()),
                    ];
                    return Ok(PluginResponse::Keyboard { text, options });
                } else if ans.starts_with("wrong:") {
                    let word = &ans["wrong:".len()..];
                    let text = format!("❌ *Not quite!* Try again or check the flashcard for *{}*.", word);
                    let options = vec![
                        ("🎯 Try Another Quiz".to_string(), "eng:quiz".to_string()),
                        ("🟢 View Flashcards".to_string(), "eng:next".to_string()),
                    ];
                    return Ok(PluginResponse::Keyboard { text, options });
                }
            }
        }
        Ok(PluginResponse::Ignored)
    }

    async fn status(&self) -> PluginStatus {
        PluginStatus::Active
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}
