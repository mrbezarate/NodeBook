//! Обработка команд бота.
use teloxide::prelude::*;
use std::sync::Arc;
use brain_core::engine::BrainEngine;
use crate::state::StateManager;

pub async fn handle_command(
    bot: &Bot,
    msg: &teloxide::types::Message,
    text: &str,
    engine: &Arc<BrainEngine>,
    state_manager: &Arc<StateManager>,
) -> anyhow::Result<()> {
    let mut parts = text.split_whitespace();
    let cmd = parts.next().unwrap_or("");
    let args: String = parts.collect::<Vec<&str>>().join(" ");
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    
    match cmd {
        "/start" => {
            bot.send_message(chat_id, cmd_start()).parse_mode(teloxide::types::ParseMode::Html).await?;
        }
        "/help" => {
            bot.send_message(chat_id, cmd_help()).parse_mode(teloxide::types::ParseMode::Html).await?;
        }
        "/diary" => {
            crate::handlers::diary::start_diary(bot, chat_id, user_id, engine, state_manager).await?;
        }
        "/search" => {
            if args.is_empty() {
                bot.send_message(chat_id, "🔍 Использование: /search <запрос>").await?;
            } else {
                match engine.search(&args).await {
                    Ok(results) => {
                        if results.is_empty() {
                            bot.send_message(chat_id, "🔍 Ничего не найдено.").await?;
                        } else {
                            let mut response = format!("🔍 Найдено {} результатов:\n\n", results.len());
                            for (i, r) in results.iter().take(10).enumerate() {
                                response.push_str(&format!("{}. <b>{}</b>\n   {}\n\n", i + 1, r.title, r.snippet));
                            }
                            bot.send_message(chat_id, response).parse_mode(teloxide::types::ParseMode::Html).await?;
                        }
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ Ошибка поиска: {}", e)).await?;
                    }
                }
            }
        }
        "/today" => {
            let today = chrono::Local::now().date_naive();
            let filename = format!("{}.md", today);
            let vault_path = std::path::Path::new(&engine.config.vault.path)
                .join(&engine.config.vault.daily_folder)
                .join(&filename);
            match tokio::fs::read_to_string(&vault_path).await {
                Ok(content) => {
                    let truncated = if content.len() > 3500 {
                        format!("{}\n\n<i>...обрезано</i>", &content[..3500])
                    } else {
                        content
                    };
                    bot.send_message(chat_id, truncated).parse_mode(teloxide::types::ParseMode::Html).await?;
                }
                Err(_) => {
                    bot.send_message(chat_id, "📭 Записи за сегодня пока нет.").await?;
                }
            }
        }
        "/stats" => {
            match engine.get_stats().await {
                Ok(stats) => {
                    let response = format!(
                        "📊 <b>Статистика Brain</b>\n\n📝 Всего записей: {}",
                        stats.total_entries
                    );
                    bot.send_message(chat_id, response).parse_mode(teloxide::types::ParseMode::Html).await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ Ошибка: {}", e)).await?;
                }
            }
        }
        _ => {
            bot.send_message(chat_id, "❓ Команда не распознана. /help — список команд.").await?;
        }
    }
    Ok(())
}

fn cmd_start() -> String {
    "🧠 <b>Brain — Персональная ОС знаний</b>\n\n\
    Просто пиши мне мысли, идеи, задачи — я всё классифицирую, сохраню и свяжу автоматически.\n\n\
    /diary — Вечерний обзор дня\n\
    /help — Список команд".to_string()
}

fn cmd_help() -> String {
    "📋 <b>Команды:</b>\n\n\
    /start — Приветствие\n\
    /diary — Вечерний обзор\n\
    /search <запрос> — Поиск по знаниям\n\
    /today — Запись за сегодня\n\
    /stats — Статистика\n\
    /help — Эта справка\n\n\
    💡 Или просто напиши мысль — я сохраню.".to_string()
}
