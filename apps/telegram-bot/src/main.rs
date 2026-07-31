//! 🧠 Brain Telegram Bot — основной интерфейс системы.

use tracing_subscriber::EnvFilter;

mod handlers;
mod keyboard;
mod output_sink;
mod state;

use brain_common::{Classification, EntryType, Result};
use brain_core::traits::*;
use brain_core::pipeline::PipelineBuilder;
use brain_core::engine::BrainEngineBuilder;
use async_trait::async_trait;
use std::sync::Arc;
use crate::state::StateManager;

// Old generators removed: AiTagGenerator, AiTitleGenerator, AiLinkSuggester are now inside AgenticPipeline

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
    let (ai_provider, embeddings, heavy_ai_provider): (Arc<dyn AiProvider>, Arc<dyn EmbeddingProvider>, Arc<dyn AiProvider>) = if config.ai.provider == "openai" {
        let provider = Arc::new(brain_ai::openai::OpenAiProvider::new(
            config.ai.openai.base_url.clone(),
            config.ai.openai.api_key.clone(),
            config.ai.openai.model.clone(),
            config.ai.openai.embedding_model.clone(),
        ));
        let heavy = Arc::new(brain_ai::openai::OpenAiProvider::new(
            config.ai.openai.base_url.clone(),
            config.ai.openai.api_key.clone(),
            config.ai.openai.heavy_model.clone(),
            config.ai.openai.embedding_model.clone(),
        ));
        (provider.clone(), provider.clone(), heavy.clone())
    } else {
        let ollama = Arc::new(brain_ai::ollama::OllamaProvider::new(
            config.ai.ollama.base_url.clone(),
            config.ai.ollama.model.clone(),
            config.ai.ollama.embedding_model.clone(),
        ));
        let heavy = Arc::new(brain_ai::ollama::OllamaProvider::new(
            config.ai.ollama.base_url.clone(),
            config.ai.ollama.heavy_model.clone(),
            config.ai.ollama.embedding_model.clone(),
        ));
        (ollama.clone(), ollama.clone(), heavy.clone())
    };

    // Initialize base components
    let entity_extractor = Arc::new(brain_classifier::HybridEntityExtractor::new());
    let vault = Arc::new(brain_vault::ObsidianVault::new(config.vault.path.clone(), config.vault.para.clone()));
    
    let vector_path = std::path::PathBuf::from(&config.vault.path).join(".brain_vectors.json");
    let vector_store = Arc::new(brain_retrieval::VectorStore::load(vector_path).await?);
    
    let event_log_path = std::path::PathBuf::from(&config.vault.path).join(".brain_events.jsonl");
    let event_logger = brain_events::JsonlEventLogger::new(event_log_path.clone()).await?;
    let event_bus: Arc<dyn brain_events::EventBus> = Arc::new(event_logger);
    
    let context_manager: Arc<dyn brain_core::traits::ContextManager> = Arc::new(brain_memory::BrainMemory::new(
        embeddings.clone(),
        vector_store.clone(),
        vault.clone(),
    ));

    let semantic_validator = Arc::new(brain_reasoner::SemanticEntityValidator::new(
        embeddings.clone(),
        vector_store.clone(),
        0.90, // Similarity threshold for merging entities
    ));

    let entity_validator: Arc<dyn brain_core::traits::EntityValidator> = semantic_validator.clone();
    let identity_resolver: Arc<dyn brain_core::traits::IdentityResolver> = semantic_validator.clone();
    
    let vault_store = Arc::new(brain_vault::EntityVault::new(config.vault.path.clone()));
    let knowledge_store = Arc::new(brain_core::db::SqliteKnowledgeStore::new("brain.db")?);

    // Bot must be created early so OutputSink can use it
    let bot = teloxide::Bot::new(&config.telegram.bot_token);

    // Agentic Pipeline
    let pipeline = Arc::new(brain_core::agentic_pipeline::AgenticPipeline::new(
        heavy_ai_provider.clone(),
        entity_extractor,
        vector_store.clone(),
        Some(context_manager.clone()),
    ));
    
    // Consolidator
    let identity_resolver = Arc::new(brain_core::identity::CascadedIdentityResolver::new(
        knowledge_store.clone(),
        heavy_ai_provider.clone(),
    ));

    // OutputSink: отправляет результат обратно в Telegram пользователю
    let owner_chat_id = config.telegram.allowed_users
        .first()
        .copied()
        .unwrap_or(0);
    let telegram_sink = output_sink::TelegramOutputSink::new(
        bot.clone(),
        teloxide::types::ChatId(owner_chat_id as i64),
    );

    let consolidator = Arc::new(brain_core::consolidator::Consolidator::new(
        heavy_ai_provider.clone(),
        knowledge_store.clone(),
        identity_resolver.clone(),
        Arc::new(brain_core::projection::SimpleProjectionEngine::new(knowledge_store.clone())),
        knowledge_store.clone(),
        Arc::new(brain_core::projection::ObsidianRenderer { base_path: config.vault.path.clone().into() }),
        Some(telegram_sink),
    ));

    let analytics_engine = Arc::new(brain_analytics::engine::LifeAnalyticsEngine::new(
        heavy_ai_provider.clone(),
        event_log_path,
    ));
    
    let engine = BrainEngineBuilder::new()
        .config(config.clone())
        .pipeline(pipeline)
        .vault(vault)
        .vector_store(vector_store)
        .event_bus(event_bus)
        .context_manager(context_manager)
        .entity_validator(entity_validator)
        .identity_resolver(identity_resolver)
        .with_knowledge_store(vault_store)
        .with_raw_event_store(knowledge_store)
        .build()?;

    let engine = Arc::new(engine);
    let state_manager = Arc::new(StateManager::new());

    tracing::info!("🧠 Brain bot initialized");
    
    // Register native commands menu in Telegram UI
    let commands = vec![
        teloxide::types::BotCommand::new("start", "🚀 Главное меню"),
        teloxide::types::BotCommand::new("diary", "📖 Вечерний обзор дня"),
        teloxide::types::BotCommand::new("search", "🔍 Поиск по базе знаний"),
        teloxide::types::BotCommand::new("today", "📅 Заметка за сегодня"),
        teloxide::types::BotCommand::new("stats", "📊 Статистика знаний"),
        teloxide::types::BotCommand::new("help", "ℹ️ Справка и возможности"),
    ];
    if let Err(e) = bot.set_my_commands(commands).await {
        tracing::warn!("Failed to set bot commands menu: {}", e);
    }

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

    // Spawn Consolidator background worker
    {
        let cons = consolidator.clone();
        tokio::spawn(async move {
            tracing::info!("Starting Consolidator background worker");
            loop {
                if let Err(e) = cons.run_pending_job().await {
                    tracing::error!("Consolidator worker error: {}", e);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    }
    
    // Build handler tree with message + callback branches
    use teloxide::prelude::*;
    
    let engine_msg = engine.clone();
    let sm_msg = state_manager.clone();
    let ae_msg = analytics_engine.clone();
    let message_handler = Update::filter_message().endpoint(
        move |bot: teloxide::Bot, msg: teloxide::types::Message| {
            let engine = engine_msg.clone();
            let sm = sm_msg.clone();
            let ae = ae_msg.clone();
            async move {
                if let Err(e) = handlers::message::handle_message(bot, msg, engine, sm, ae).await {
                    tracing::error!("Message handler error: {}", e);
                }
                Ok::<(), std::convert::Infallible>(())
            }
        }
    );
    
    let engine_cb = engine.clone();
    let sm_cb = state_manager.clone();
    let ae_cb = analytics_engine.clone();
    let callback_handler = Update::filter_callback_query().endpoint(
        move |bot: teloxide::Bot, query: teloxide::types::CallbackQuery| {
            let engine = engine_cb.clone();
            let sm = sm_cb.clone();
            let ae = ae_cb.clone();
            async move {
                if let Err(e) = handlers::callback::handle_callback(bot, query, engine, sm, ae).await {
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
