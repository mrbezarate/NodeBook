//! Обработка текстовых сообщений — основная точка входа.
use teloxide::prelude::*;
use std::sync::Arc;
use brain_core::engine::BrainEngine;
use brain_common::EntrySource;
use crate::state::{StateManager, UserState};

/// Обработать входящее текстовое сообщение.
pub async fn handle_message(
    bot: Bot,
    msg: teloxide::types::Message,
    engine: Arc<BrainEngine>,
    state_manager: Arc<StateManager>,
) -> anyhow::Result<()> {
    let text = match msg.text() {
        Some(t) => t,
        None => return Ok(()),
    };
    
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let chat_id = msg.chat.id;

    // Аутентификация: проверяем allowed_users (пустой список = доступ для всех)
    let allowed = &engine.config.telegram.allowed_users;
    if !allowed.is_empty() && !allowed.contains(&user_id) {
        bot.send_message(chat_id, "🔒 Доступ запрещён.").await?;
        return Ok(());
    }
    
    // Check if it's a command
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
        _ => {
            // Default: ingest as a brain entry
            let processing_msg = bot.send_message(chat_id, "⏳ Обрабатываю...").await?;
            let source = EntrySource::Telegram { user_id, message_id: msg.id.0 };
            match engine.ingest(text, source).await {
                Ok(entry) => {
                    let response = format!(
                        "✅ <b>Сохранено</b>\n\n\
                        <b>📌 {}</b>\n\
                        Тип: {}\n\
                        Область: {}\n\
                        PARA: {:?}",
                        entry.classification.suggested_title,
                        entry.classification.entry_type,
                        entry.classification.area,
                        entry.classification.para_category
                    );
                    bot.edit_message_text(chat_id, processing_msg.id, response)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    bot.edit_message_text(chat_id, processing_msg.id, format!("❌ Ошибка: {}", e)).await?;
                }
            }
        }
    }
    Ok(())
}
