//! Обработка текстовых сообщений — основная точка входа.
use teloxide::prelude::*;
use std::sync::Arc;
use brain_core::engine::BrainEngine;
use brain_common::EntrySource;
use crate::state::{StateManager, UserState};
use crate::keyboard::inline::entry_actions_keyboard;

/// Обработать входящее текстовое сообщение.
pub async fn handle_message(
    bot: Bot,
    msg: teloxide::types::Message,
    engine: Arc<BrainEngine>,
    state_manager: Arc<StateManager>,
    _analytics_engine: Arc<brain_analytics::engine::LifeAnalyticsEngine>,
) -> anyhow::Result<()> {
    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };
    
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let chat_id = msg.chat.id;

    // ЖЁСТКАЯ АУТЕНТИФИКАЦИЯ: Если ID нет в allowed_users — доступ НАГЛУХО ЗАКРЫТ.
    let allowed = &engine.config.telegram.allowed_users;
    if allowed.is_empty() || !allowed.contains(&user_id) {
        tracing::warn!("🚫 Попытка несанкционированного доступа от user_id: {}", user_id);
        bot.send_message(chat_id, "🔒 <b>Доступ запрещён.</b>\nЭто приватная система управления знаниями владельца.")
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        return Ok(());
    }
    
    // Check if text corresponds to main menu buttons
    match text {
        "📖 Дневник дня" => {
            crate::handlers::command::handle_command(&bot, &msg, "/diary", &engine, &state_manager).await?;
            return Ok(());
        }
        "📊 Аналитика и Граф" | "/analytics" => {
            crate::handlers::analytics::send_analytics_menu(&bot, chat_id).await?;
            return Ok(());
        }
        "🔍 Поиск" => {
            crate::handlers::command::handle_command(&bot, &msg, "/search", &engine, &state_manager).await?;
            return Ok(());
        }
        "📅 Запись за сегодня" => {
            crate::handlers::command::handle_command(&bot, &msg, "/today", &engine, &state_manager).await?;
            return Ok(());
        }
        "⌛ Статистика жизни" | "📊 Статистика" => {
            crate::handlers::command::handle_command(&bot, &msg, "/stats", &engine, &state_manager).await?;
            return Ok(());
        }
        "ℹ️ Справка" => {
            crate::handlers::command::handle_command(&bot, &msg, "/help", &engine, &state_manager).await?;
            return Ok(());
        }
        _ => {}
    }

    // Check if it's a command starting with /
    if text.starts_with('/') {
        crate::handlers::command::handle_command(&bot, &msg, text, &engine, &state_manager).await?;
        return Ok(());
    }
    
    // Check user state — route accordingly
    let user_state = state_manager.get(user_id).await;
    match user_state {
        UserState::DiaryReview(_) => {
            // User is in diary review — process text as diary input
            crate::handlers::diary::handle_diary_text(&bot, &msg, user_id, &engine, &state_manager).await?;
        }
        UserState::WaitingForSearch => {
            state_manager.reset(user_id).await;
            crate::handlers::command::execute_search(&bot, chat_id, text, &engine).await?;
        }
        _ => {
            // Default: ingest as a raw event
            let source = EntrySource::Telegram { user_id, message_id: msg.id.0 };
            match engine.ingest_raw_event(text, source).await {
                Ok(event_id) => {
                    let response = format!("✅ <b>Событие принято в очередь.</b>\nID: {}", event_id);
                    bot.send_message(chat_id, response)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ Ошибка: {}", e)).await?;
                }
            }
        }
    }
    Ok(())
}
