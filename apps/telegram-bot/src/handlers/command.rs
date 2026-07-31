//! Обработка команд бота.
use teloxide::prelude::*;
use std::sync::Arc;
use brain_core::engine::BrainEngine;
use crate::state::{StateManager, UserState};
use crate::keyboard::reply::main_menu_keyboard;

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
            state_manager.reset(user_id).await;
            bot.send_message(chat_id, cmd_start())
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(main_menu_keyboard())
                .await?;
        }
        "/help" => {
            bot.send_message(chat_id, cmd_help())
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(main_menu_keyboard())
                .await?;
        }
        "/diary" => {
            crate::handlers::diary::start_diary(bot, chat_id, user_id, engine, state_manager).await?;
        }
        "/search" => {
            if args.is_empty() {
                state_manager.set(user_id, UserState::WaitingForSearch).await;
                bot.send_message(
                    chat_id,
                    "🔍 <b>Поиск по вашей базе знаний</b>\n\n\
                    Введите фразу или ключевые слова для поиска (например, <i>проект</i>, <i>здоровье</i> или <i>цели</i>):"
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            } else {
                execute_search(bot, chat_id, &args, engine).await?;
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
                    bot.send_message(chat_id, format!("📅 <b>Запись за сегодня ({}):</b>\n\n{}", today, truncated))
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
                Err(_) => {
                    bot.send_message(chat_id, format!("📭 Записи за сегодня ({}) пока нет.\n\nПросто отправьте сообщение, и оно сохранится!", today)).await?;
                }
            }
        }
        "/stats" => {
            let life_stats = crate::handlers::analytics::format_life_stats(
                &engine.config.diary.birth_date,
                engine.config.diary.life_expectancy_years,
            );
            match engine.get_stats().await {
                Ok(stats) => {
                    let response = format!(
                        "{}\n\n\
                        📝 <b>Всего заметок в вашей базе Obsidian:</b> {}\n\
                        📁 <b>Путь к хранилищу:</b> <code>{}</code>\n\n\
                        💡 <i>Для детального анализа записей, настроения и графа связей нажмите кнопку <b>«📊 Аналитика и Граф»</b>.</i>",
                        life_stats,
                        stats.total_entries,
                        engine.config.vault.path
                    );
                    bot.send_message(chat_id, response).parse_mode(teloxide::types::ParseMode::Html).await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ Ошибка получения статистики: {}", e)).await?;
                }
            }
        }
        "/debug" => {
            if args.is_empty() {
                bot.send_message(chat_id, "⚠️ Укажите event_id: /debug <id>").await?;
            } else {
                match engine.get_debug_trace(&args).await {
                    Ok(trace) => {
                        let truncated = if trace.len() > 4000 {
                            format!("{}\n\n...обрезано", &trace[..4000])
                        } else {
                            trace
                        };
                        bot.send_message(chat_id, format!("🛠 <b>Debug Trace</b>\n\n<pre>{}</pre>", truncated))
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ Ошибка: {}", e)).await?;
                    }
                }
            }
        }
        "/rebuild" => {
            if args.is_empty() {
                bot.send_message(chat_id, "⚠️ Укажите event_id: /rebuild <id>").await?;
            } else {
                match engine.rebuild_from_event(&args).await {
                    Ok(_) => {
                        bot.send_message(chat_id, format!("✅ Событие {} отправлено на повторную обработку.", args))
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ Ошибка при ребилде: {}", e)).await?;
                    }
                }
            }
        }
        "/metrics" => {
            match engine.get_metrics_report().await {
                Ok(report) => {
                    let formatted = format!(
                        "📊 <b>System Metrics Report</b>\n\
                        \n<b>Пайплайн</b>\n\
                        • Обработано: <b>{}</b>\n\
                        • Latency: <b>{:.0} ms</b>\n\
                        \n<b>Extractor</b>\n\
                        • JSON parse success: <b>{:.1}%</b>\n\
                        • Пустых ответов: <b>{:.1}%</b>\n\
                        • В среднем сущностей: <b>{:.1}</b>\n\
                        • Confidence (avg): <b>{:.2}</b>\n\
                        \n<b>Identity Resolver</b>\n\
                        • Exact: <b>{}</b>\n\
                        • Alias: <b>{}</b>\n\
                        • Fuzzy: <b>{}</b>\n\
                        • Semantic: <b>{}</b>\n\
                        • No Match: <b>{}</b>\n\
                        \n<b>Projection</b>\n\
                        • Entities: <b>{}</b>\n\
                        • Observations: <b>{}</b>\n\
                        • Obs per Entity (avg): <b>{:.1}</b>",
                        report.processed_events, report.avg_latency_ms, 
                        report.json_success_rate, report.empty_responses_percent, report.avg_entities_extracted, report.avg_confidence,
                        report.identity_exact, report.identity_alias, report.identity_fuzzy, report.identity_semantic, report.identity_nomatch,
                        report.total_entities, report.total_observations, report.avg_obs_per_entity
                    );
                    bot.send_message(chat_id, formatted)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ Ошибка: {}", e)).await?;
                }
            }
        }
        _ => {
            bot.send_message(chat_id, "❓ Команда не распознана. Воспользуйтесь меню или введите /help.").await?;
        }
    }
    Ok(())
}

pub async fn execute_search(
    bot: &Bot,
    chat_id: ChatId,
    query: &str,
    engine: &Arc<BrainEngine>,
) -> anyhow::Result<()> {
    match engine.search(query).await {
        Ok(results) => {
            if results.is_empty() {
                bot.send_message(chat_id, format!("🔍 По запросу «<b>{}</b>» ничего не найдено.", query))
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            } else {
                let mut response = format!("🔍 <b>Результаты поиска («{}»):</b>\n\n", query);
                for (i, r) in results.iter().take(10).enumerate() {
                    response.push_str(&format!("{}. 📌 <b>{}</b>\n   <i>{}</i>\n\n", i + 1, r.title, r.snippet));
                }
                bot.send_message(chat_id, response).parse_mode(teloxide::types::ParseMode::Html).await?;
            }
        }
        Err(e) => {
            bot.send_message(chat_id, format!("❌ Ошибка поиска: {}", e)).await?;
        }
    }
    Ok(())
}

fn cmd_start() -> String {
    "🧠 <b>Привет! Я Brain — ваша персональная ОС знаний.</b>\n\n\
    Я автоматически классифицирую ваши мысли, идеи и задачи, раскладываю их по папкам PARA в вашей базе Obsidian и создаю между ними связи.\n\n\
    <b>Как мной пользоваться:</b>\n\
    • 💬 <i>Просто отправьте любое сообщение</i> — я распознаю его тип, тему и сохраню в базу.\n\
    • 📖 <i>Дневник дня</i> — вечерний обзор для трекинга продуктивности и настроения.\n\
    • 🔍 <i>Поиск</i> — быстрый семантический поиск по вашим знаниям.\n\n\
    Воспользуйтесь <b>кнопками меню ниже</b> или кнопкой <b>[/] Меню</b> слева от поля ввода!".to_string()
}

fn cmd_help() -> String {
    "📋 <b>Справка и список команд:</b>\n\n\
    /start — Главное меню и приветствие\n\
    /diary — Запустить вечерний обзор дня\n\
    /search — Поиск по вашей базе знаний\n\
    /today — Посмотреть сегодняшнюю дневниковую запись\n\
    /stats — Посмотреть статистику заметок и личный прогресс\n\
    /metrics — [Dev] Операционные метрики пайплайна\n\
    /debug — [Dev] Отладка Event Pipeline\n\
    /rebuild — [Dev] Переобработать RawEvent\n\
    /help — Показать эту справку\n\n\
    💡 <i>Совет: Вы можете просто отправить любой текст или заметку — алгоритмы и ИИ сделают всю работу за вас!</i>".to_string()
}
