//! 🧠 Brain Telegram Bot — основной интерфейс системы.

use tracing_subscriber::EnvFilter;

mod handlers;
mod keyboard;
mod state;

use brain_common::{Classification, EntryType, Result};
use brain_core::traits::*;
use brain_core::pipeline::PipelineBuilder;
use brain_core::engine::BrainEngineBuilder;
use async_trait::async_trait;
use std::sync::Arc;
use crate::state::StateManager;

struct AiTagGenerator { ai: Arc<dyn AiProvider> }
#[async_trait]
impl TagGenerator for AiTagGenerator {
    async fn generate_tags(&self, text: &str, classification: &Classification) -> Result<Vec<String>> {
        let prompt = format!("Extract 3-5 relevant hashtags from this text. Output ONLY words with #. Text: {}", text);
        if let Ok(response) = self.ai.complete(&prompt).await {
            let tags: Vec<String> = response.split_whitespace().filter(|s| s.starts_with('#')).map(|s| s.replace('#', "")).collect();
            if !tags.is_empty() { return Ok(tags); }
        }
        Ok(vec![format!("{:?}", classification.entry_type).to_lowercase()])
    }
}

struct AiTitleGenerator { ai: Arc<dyn AiProvider> }
#[async_trait]
impl TitleGenerator for AiTitleGenerator {
    async fn generate_title(&self, text: &str, entry_type: &EntryType) -> Result<String> {
        let prompt = format!("Create a short title (max 5 words) for this text. Output ONLY the title. Text: {}", text);
        if let Ok(response) = self.ai.complete(&prompt).await {
            let title = response.trim().replace('"', "");
            if !title.is_empty() { return Ok(title); }
        }
        Ok(format!("{:?} - {}", entry_type, text.split_whitespace().take(5).collect::<Vec<_>>().join(" ")))
    }
}

struct AiLinkSuggester { ai: Arc<dyn AiProvider> }
#[async_trait]
impl LinkSuggester for AiLinkSuggester {
    async fn suggest_links(&self, text: &str, _limit: usize) -> Result<Vec<String>> {
        let prompt = format!("Extract 1-3 key concepts from this text. Output them as [[Concept]]. Text: {}", text);
        if let Ok(response) = self.ai.complete(&prompt).await {
            let links: Vec<String> = response.split_whitespace().filter(|s| s.starts_with("[[") && s.ends_with("]]")).map(|s| s.to_string()).collect();
            return Ok(links);
        }
        Ok(vec![])
    }
}

/// Background task: schedule evening diary at configured time.
async fn evening_scheduler(
    bot: teloxide::Bot,
    config: brain_config::BrainConfig,
    engine: Arc<brain_core::engine::BrainEngine>,
    state_manager: Arc<StateManager>,
) {
    use chrono::{Local, NaiveTime, Timelike};
    
    let review_time = NaiveTime::parse_from_str(&config.diary.evening_review_time, "%H:%M")
        .unwrap_or_else(|_| NaiveTime::from_hms_opt(21, 0, 0).unwrap());
    
    tracing::info!("📅 Scheduler: evening diary at {}", review_time);
    
    let mut triggered_today = false;
    
    loop {
        let now = Local::now();
        let current_time = now.time();
        let _current_date = now.date_naive();
        
        // Check if it's time and we haven't triggered today
        if current_time.hour() == review_time.hour()
            && current_time.minute() == review_time.minute()
            && !triggered_today
        {
            tracing::info!("🌙 Triggering evening diary review");
            triggered_today = true;
            
            // Send diary to all allowed users
            for &user_id in &config.telegram.allowed_users {
                let chat_id = teloxide::types::ChatId(user_id as i64);
                if let Err(e) = handlers::diary::start_diary(
                    &bot, chat_id, user_id, &engine, &state_manager
                ).await {
                    tracing::error!("Failed to start diary for user {}: {}", user_id, e);
                }
            }
        }
        
        // Reset trigger at midnight
        if current_time.hour() == 0 && current_time.minute() == 0 {
            triggered_today = false;
        }
        
        // Sleep for 30 seconds before next check
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("brain=info".parse()?))
        .init();

    tracing::info!("🧠 Brain — Personal Knowledge OS starting...");
    dotenvy::dotenv().ok();
    
    let mut config = brain_config::BrainConfig::load_or_default();
    if let Ok(token) = std::env::var("BOT_TOKEN") { config.telegram.bot_token = token; }
    
    if config.telegram.bot_token.is_empty() {
        tracing::error!("BOT_TOKEN not set!");
        std::process::exit(1);
    }

    // Initialize AI providers
    let ai_provider = Arc::new(brain_ai::ollama::OllamaProvider::new(
        config.ai.ollama.base_url.clone(),
        config.ai.ollama.model.clone(),
        config.ai.ollama.embedding_model.clone(),
    )) as Arc<dyn brain_core::traits::AiProvider>;

    let heavy_ai_provider = Arc::new(brain_ai::ollama::OllamaProvider::new(
        config.ai.ollama.base_url.clone(),
        config.ai.ollama.heavy_model.clone(),
        config.ai.ollama.embedding_model.clone(),
    )) as Arc<dyn brain_core::traits::AiProvider>;

    // Initialize pipeline components
    let type_classifier = Arc::new(brain_classifier::HybridTypeClassifier::new(Some(ai_provider.clone()), config.classifier.confidence_threshold));
    let area_detector = Arc::new(brain_classifier::HybridAreaDetector::new(Some(ai_provider.clone())));
    let entity_extractor = Arc::new(brain_classifier::HybridEntityExtractor::new());
    let tag_generator = Arc::new(AiTagGenerator { ai: heavy_ai_provider.clone() });
    let para_router = Arc::new(brain_vault::VaultParaRouter::new(config.vault.para.clone(), config.vault.path.clone()));
    let title_generator = Arc::new(AiTitleGenerator { ai: heavy_ai_provider.clone() });
    let link_suggester = Arc::new(AiLinkSuggester { ai: heavy_ai_provider.clone() });

    let pipeline = PipelineBuilder::new()
        .type_classifier(type_classifier)
        .area_detector(area_detector)
        .entity_extractor(entity_extractor)
        .tag_generator(tag_generator)
        .para_router(para_router)
        .title_generator(title_generator)
        .link_suggester(link_suggester)
        .build()?;

    let vault = Arc::new(brain_vault::ObsidianVault::new(config.vault.path.clone(), config.vault.para.clone()));
    
    let engine = BrainEngineBuilder::new()
        .config(config.clone())
        .pipeline(pipeline)
        .vault(vault)
        .build()?;

    let engine = Arc::new(engine);
    let state_manager = Arc::new(StateManager::new());

    tracing::info!("🧠 Brain bot initialized");
    
    let bot = teloxide::Bot::new(&config.telegram.bot_token);
    
    // Spawn evening scheduler
    {
        let bot_clone = bot.clone();
        let config_clone = config.clone();
        let engine_clone = engine.clone();
        let sm_clone = state_manager.clone();
        tokio::spawn(async move {
            evening_scheduler(bot_clone, config_clone, engine_clone, sm_clone).await;
        });
    }
    
    // Build handler tree with message + callback branches
    use teloxide::prelude::*;
    
    let engine_msg = engine.clone();
    let sm_msg = state_manager.clone();
    let message_handler = Update::filter_message().endpoint(
        move |bot: teloxide::Bot, msg: teloxide::types::Message| {
            let engine = engine_msg.clone();
            let sm = sm_msg.clone();
            async move {
                if let Err(e) = handlers::message::handle_message(bot, msg, engine, sm).await {
                    tracing::error!("Message handler error: {}", e);
                }
                Ok::<(), std::convert::Infallible>(())
            }
        }
    );
    
    let engine_cb = engine.clone();
    let sm_cb = state_manager.clone();
    let callback_handler = Update::filter_callback_query().endpoint(
        move |bot: teloxide::Bot, query: teloxide::types::CallbackQuery| {
            let engine = engine_cb.clone();
            let sm = sm_cb.clone();
            async move {
                if let Err(e) = handlers::callback::handle_callback(bot, query, engine, sm).await {
                    tracing::error!("Callback handler error: {}", e);
                }
                Ok::<(), std::convert::Infallible>(())
            }
        }
    );
    
    let handler = dptree::entry()
        .branch(message_handler)
        .branch(callback_handler);
    
    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
