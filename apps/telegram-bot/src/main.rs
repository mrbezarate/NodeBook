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

/// Настройка планировщика
async fn setup_scheduler(
    bot: teloxide::Bot,
    config: brain_config::BrainConfig,
    engine: Arc<brain_core::engine::BrainEngine>,
    state_manager: Arc<StateManager>,
) -> anyhow::Result<tokio_cron_scheduler::JobScheduler> {
    use tokio_cron_scheduler::{JobScheduler, Job};
    
    let sched = JobScheduler::new().await?;

    let review_time = config.diary.evening_review_time.clone();
    let parts: Vec<&str> = review_time.split(':').collect();
    let hour = parts.get(0).unwrap_or(&"21");
    let minute = parts.get(1).unwrap_or(&"00");
    
    // Формат cron: sec min hour day month day_of_week
    let cron_expr = format!("0 {} {} * * *", minute, hour);
    
    tracing::info!("📅 Setup scheduler: evening diary cron {}", cron_expr);
    
    sched.add(Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
        let bot = bot.clone();
        let config = config.clone();
        let engine = engine.clone();
        let state_manager = state_manager.clone();
        
        Box::pin(async move {
            tracing::info!("🌙 Triggering evening diary review");
            for &user_id in &config.telegram.allowed_users {
                let chat_id = teloxide::types::ChatId(user_id as i64);
                let mut success = false;
                let backoff_secs = [5, 30, 120];
                for (i, &delay) in backoff_secs.iter().enumerate() {
                    let attempt = i + 1;
                    match crate::handlers::diary::start_diary(
                        &bot, chat_id, user_id, &engine, &state_manager
                    ).await {
                        Ok(_) => {
                            tracing::info!("Diary review sent to user {}", user_id);
                            success = true;
                            break;
                        }
                        Err(e) => {
                            tracing::warn!("Attempt {} failed to start diary for user {}: {}", attempt, user_id, e);
                            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                        }
                    }
                }
                if !success {
                    tracing::error!("CRITICAL: Failed to start diary for user {} after 3 attempts", user_id);
                }
            }
        })
    })?).await?;
    
    Ok(sched)
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
    let db_path = std::path::PathBuf::from(&config.vault.path).join("brain.db");
    let knowledge_store = Arc::new(brain_core::db::SqliteKnowledgeStore::new(db_path.to_str().unwrap())?);

    let graph_path = std::path::PathBuf::from(&config.vault.path).join(".brain_graph.json");
    let graph = match brain_graph::KnowledgeGraph::load(graph_path.to_str().unwrap()).await {
        Ok(g) => Arc::new(g),
        Err(_) => Arc::new(brain_graph::KnowledgeGraph::new()),
    };

    // Bot must be created early so OutputSink can use it
    let bot = teloxide::Bot::new(&config.telegram.bot_token);

    // Agentic Pipeline
    let pipeline = Arc::new(brain_core::agentic_pipeline::AgenticPipeline::new(
        heavy_ai_provider.clone(),
        entity_extractor,
        vector_store.clone(),
        Some(context_manager.clone()),
    ));
    
    // Identity Resolver
    let identity_resolver = Arc::new(brain_core::identity::CascadedIdentityResolver::new(
        knowledge_store.clone(),
        heavy_ai_provider.clone(),
        vector_store.clone(),
        embeddings.clone(),
    ));

    // OutputSink: отправляет результат обратно в Telegram пользователю
    // (telegram_sink removed)

    // (Consolidator removed as we now use background pipeline directly)

    let analytics_engine = Arc::new(brain_analytics::engine::LifeAnalyticsEngine::new(
        heavy_ai_provider.clone(),
        event_log_path,
    ));
    
    let engine = BrainEngineBuilder::new()
        .config(config.clone())
        .pipeline(pipeline)
        .vault(vault)
        .vector_store(vector_store)
        .graph(graph)
        .embeddings(embeddings)
        .event_bus(event_bus)
        .context_manager(context_manager)
        .entity_validator(entity_validator)
        .identity_resolver(identity_resolver)
        .with_knowledge_store(vault_store)
        .with_raw_event_store(knowledge_store.clone())
        .build()?;

    let engine = Arc::new(engine);
    engine.start_workers();
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

    // Start health-check server
    let health_store = knowledge_store.clone();
    tokio::spawn(async move {
        use axum::{routing::get, Router, http::StatusCode};
        let app = Router::new().route("/health", get(move || {
            let store = health_store.clone();
            async move {
                // Try to hit the DB
                match store.get_metrics_report().await {
                    Ok(_) => (StatusCode::OK, "OK"),
                    Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "DB ERROR"),
                }
            }
        }));
        
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
        tracing::info!("Health check server listening on port 8080");
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("Health server error: {}", e);
        }
    });

    // Start scheduler
    {
        let bot_clone = bot.clone();
        let config_clone = config.clone();
        let engine_clone = engine.clone();
        let sm_clone = state_manager.clone();
        if let Ok(sched) = setup_scheduler(bot_clone, config_clone, engine_clone, sm_clone).await {
            tokio::spawn(async move {
                if let Err(e) = sched.start().await {
                    tracing::error!("Scheduler error: {}", e);
                }
            });
        }
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
    

    // Start notification worker
    {
        use brain_core::traits::RawEventStore;
        let bot_clone = bot.clone();
        let engine_clone = engine.clone();
        let store_clone = knowledge_store.clone();
        tokio::spawn(async move {
            loop {
                match store_clone.next_unprocessed_event("EntryStored").await {
                    Ok(Some(record)) => {
                        if let Ok(Some(entry)) = engine_clone.rebuild(&record.aggregate_id).await {
                            if let brain_common::EntrySource::Telegram { user_id, processing_msg_id, .. } = entry.source {
                                let chat_id = teloxide::types::ChatId(user_id as i64);
                                use teloxide::requests::Requester;
                                
                                let tags_str = entry.classification.tags.iter().map(|t| format!("#{}", t)).collect::<Vec<_>>().join(" ");
                                let mut reply = format!("✅ <b>Сохранено:</b> {}\n\n{}\n\n📂 {}\n🏷 {}", 
                                    entry.classification.suggested_title,
                                    entry.classification.summary,
                                    entry.classification.area,
                                    tags_str
                                );

                                if let brain_common::SourcingEvent::EntryStored { path } = record.event {
                                    reply.push_str(&format!("\n📝 <code>{}</code>", path));
                                }

                                if let Some(msg_id) = processing_msg_id {
                                    let _ = bot_clone.edit_message_text(chat_id, teloxide::types::MessageId(msg_id as i32), reply)
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await;
                                } else {
                                    let _ = bot_clone.send_message(chat_id, reply)
                                        .parse_mode(teloxide::types::ParseMode::Html)
                                        .await;
                                }
                            }
                        }
                        let _ = store_clone.mark_event_processed(&record.id).await;
                    }
                    Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
                    Err(_) => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
                }
            }
        });
    }

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
