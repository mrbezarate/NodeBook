//! Обработка callback-запросов (inline keyboards).
use teloxide::prelude::*;
use teloxide::types::CallbackQuery;
use std::sync::Arc;
use brain_core::engine::BrainEngine;
use crate::state::StateManager;

/// Main callback handler — routes to appropriate sub-handler.
pub async fn handle_callback(
    bot: Bot,
    query: CallbackQuery,
    engine: Arc<BrainEngine>,
    state_manager: Arc<StateManager>,
    analytics_engine: Arc<brain_analytics::engine::LifeAnalyticsEngine>,
) -> anyhow::Result<()> {
    let user_id = query.from.id.0;
    let chat_id = query.message.as_ref().map(|m| m.chat().id);
    let message_id = query.message.as_ref().map(|m| m.id());

    // ЖЁСТКАЯ АУТЕНТИФИКАЦИЯ ДЛЯ CALLBACKS
    let allowed = &engine.config.telegram.allowed_users;
    if allowed.is_empty() || !allowed.contains(&user_id) {
        tracing::warn!("🚫 Попытка несанкционированного callback от user_id: {}", user_id);
        bot.answer_callback_query(&query.id).text("🔒 Доступ запрещён.").show_alert(true).await?;
        return Ok(());
    }

    // Answer the callback query first (removes loading spinner)
    bot.answer_callback_query(&query.id).await?;
    
    let data = match query.data.as_deref() {
        Some(d) => d,
        None => return Ok(()),
    };
    
    let (prefix, value) = match parse_callback(data) {
        Some(pv) => pv,
        None => return Ok(()),
    };
    
    match prefix.as_str() {
        "diary" | "metric" | "exercise" => {
            if let (Some(chat_id), Some(message_id)) = (chat_id, message_id) {
                crate::handlers::diary::handle_diary_callback(
                    &bot, chat_id, message_id, user_id, &prefix, &value, &engine, &state_manager
                ).await?;
            }
        }
        "analytics" => {
            if let (Some(chat_id), Some(message_id)) = (chat_id, message_id) {
                crate::handlers::analytics::handle_analytics_callback(
                    &bot, chat_id, message_id, &value, &engine, &analytics_engine
                ).await?;
            }
        }
        "path" => {
            if let Some(chat_id) = chat_id {
                bot.send_message(
                    chat_id,
                    format!("📁 <b>Файл сохранён в вашей хранилище Obsidian:</b>\n<code>{}/.../{}</code>", engine.config.vault.path, value)
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            }
        }
        _ => {
            tracing::warn!("Unknown callback prefix: {}", prefix);
        }
    }
    
    Ok(())
}

pub fn parse_callback(data: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = data.splitn(2, ':').collect();
    if parts.len() == 2 { Some((parts[0].to_string(), parts[1].to_string())) } else { None }
}
