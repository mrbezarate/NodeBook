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
        bot.send_message(chat_id, "🔒 <b>Доступ запрещён.</b>\nЭто приватная система управления владельца.")
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
        "📊 Аналитика" | "/analytics" => {
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
        "/viz" => {
            crate::handlers::visual::handle_visual_command(bot.clone(), msg.clone(), engine.clone()).await?;
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
        UserState::Editing { entry_id, field: _ } => {
            state_manager.reset(user_id).await;
            match engine.find_path_by_id(&entry_id).await {
                Ok(path) => {
                    if let Err(e) = engine.append_to_entry(&path, text).await {
                        bot.send_message(chat_id, format!("❌ Ошибка при дополнении: {}", e)).await?;
                    } else {
                        bot.send_message(chat_id, "✅ Запись успешно дополнена!").await?;
                    }
                }
                Err(_) => {
                    bot.send_message(chat_id, "❌ Исходная запись не найдена.").await?;
                }
            }
        }
        _ => {
            // Default: ingest as a raw event
            let processing_msg = bot.send_message(chat_id,
                "⏳ <b>Анализирую и классифицирую...</b>\n<i>Мысль сохранена, обрабатываю в фоне →</i> Obsidian"
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;

            let source = EntrySource::Telegram { 
                user_id, 
                message_id: msg.id.0,
                processing_msg_id: Some(processing_msg.id.0)
            };
            
            match engine.ingest(text, source).await {
                Ok((_, id)) => {
                    // Fast CQRS poll: try to get projection, or fallback to simple message
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await; // give projection worker a tiny headstart
                    
                    let mut reply_text = "✅ Принято в обработку.".to_string();
                    if let Ok(Some(proj)) = engine.get_entry(&id).await {
                        let tags = proj.tags.join(", ");
                        reply_text = format!("✅ Записано.\n{}",
                            if !proj.summary.is_empty() { proj.summary.clone() } else { "Ожидает анализ...".to_string() }
                        );
                        if !tags.is_empty() {
                            reply_text.push_str(&format!("\n🏷 {}", tags));
                        }
                    }

                    let _ = bot.delete_message(chat_id, processing_msg.id).await;
                    bot.send_message(chat_id, reply_text)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    let _ = bot.delete_message(chat_id, processing_msg.id).await;
                    bot.send_message(chat_id, format!("❌ Ошибка: {}", e)).await?;
                }
            }
        }
    }
    Ok(())
}
