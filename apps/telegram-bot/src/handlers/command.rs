//! Обработка команд бота.
use teloxide::prelude::*;
use std::sync::Arc;
use brain_core::engine::BrainEngine;
use crate::state::{StateManager, UserState};

pub async fn handle_command(
    bot: &Bot,
    msg: &teloxide::types::Message,
    text: &str,
    engine: &Arc<BrainEngine>,
    state_manager: &Arc<StateManager>,
    vault_registry: &Arc<tokio::sync::RwLock<brain_vault::VaultRegistry>>,
    plugin_registry: &Arc<brain_plugin::PluginRegistry>,
    tunnel_manager: &Arc<crate::tunnel::TunnelManager>,
) -> anyhow::Result<()> {
    let mut parts = text.split_whitespace();
    let raw_cmd = parts.next().unwrap_or("");
    let cmd = raw_cmd.split('@').next().unwrap_or(raw_cmd);
    let args: String = parts.collect::<Vec<&str>>().join(" ");
    let chat_id = msg.chat.id;
    let user_id = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
    let web_app_url = tunnel_manager.get_url().await;
    
    match cmd {
        "/start" => {
            state_manager.reset(user_id).await;
            let reply_kb = crate::keyboard::reply::main_menu_keyboard_with_app(web_app_url.as_deref());
            let send_call = bot.send_message(chat_id, cmd_start())
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(reply_kb);
            send_call.await?;

            if let Some(ref url) = web_app_url {
                bot.send_message(chat_id, "🚀 <b>Запуск Telegram Mini App:</b>")
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(crate::keyboard::inline::webapp_keyboard(url))
                    .await?;
            }
        }
        "/help" => {
            let reply_kb = crate::keyboard::reply::main_menu_keyboard_with_app(web_app_url.as_deref());
            bot.send_message(chat_id, cmd_help())
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(reply_kb)
                .await?;
        }
        "/cancel" => {
            state_manager.reset(user_id).await;
            let reply_kb = crate::keyboard::reply::main_menu_keyboard_with_app(web_app_url.as_deref());
            bot.send_message(chat_id, "✅ <b>Действие отменено.</b> Возврат в главное меню.")
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(reply_kb)
                .await?;
        }
        "/base" | "/vault" => {
            crate::handlers::vault::send_vault_menu(bot, chat_id, vault_registry).await?;
        }
        "/app" | "/webapp" => {
            if let Some(ref url) = web_app_url {
                bot.send_message(
                    chat_id,
                    "📱 <b>NodeBook Web App & Mini App</b>\n\n\
                    Нажмите кнопку ниже, чтобы запустить приложение прямо внутри Telegram:\n\
                    • 🎵 <b>Spotify Player:</b> музыка с обложками\n\
                    • 📹 <b>Медиа Галерея:</b> просмотр и загрузка видео\n\
                    • 📖 <b>База Знаний:</b> чистое чтение и свойства",
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(crate::keyboard::inline::webapp_keyboard(url))
                .await?;
            } else {
                bot.send_message(
                    chat_id,
                    "⏳ <b>Инициализация защищенного HTTPS туннеля...</b>\n\
                    Пожалуйста, повторите команду <code>/app</code> через пару секунд.",
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            }
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
        "/analytics" => {
            crate::handlers::analytics::send_analytics_menu(bot, chat_id).await?;
        }
        "/viz" => {
            crate::handlers::visual::handle_visual_command(bot.clone(), msg.clone(), engine.clone()).await?;
        }
        "/today" => {
            let today = chrono::Local::now().date_naive();
            let filename = format!("{}.md", today);
            let vault_path = std::path::Path::new(&engine.config.vault.path)
                .join(&engine.config.vault.daily_folder)
                .join(&filename);
            match tokio::fs::read_to_string(&vault_path).await {
                Ok(content) => {
                    let char_count = content.chars().count();
                    let escaped = if char_count > 3500 {
                        let prefix: String = content.chars().take(3500).collect();
                        format!("{}\n\n<i>...обрезано</i>", teloxide::utils::html::escape(&prefix))
                    } else {
                        teloxide::utils::html::escape(&content)
                    };
                    bot.send_message(chat_id, format!("📅 <b>Запись за сегодня ({}):</b>\n\n{}", today, escaped))
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
                        💡 <i>Для детального анализа записей, настроения и графа связей нажмите кнопку <b>«📊 Аналитика»</b>.</i>",
                        life_stats,
                        stats.total_entries,
                        engine.config.vault.path
                    );
                    bot.send_message(chat_id, response)
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("❌ Ошибка получения статистики: {}", e)).await?;
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
                    bot.send_message(chat_id, format!("❌ Ошибка получения метрик: {}", e)).await?;
                }
            }
        }
        "/debug" => {
            if let Some(event_id) = args.split_whitespace().next() {
                match engine.get_debug_trace(event_id).await {
                    Ok(trace) => {
                        let char_count = trace.chars().count();
                        let truncated = if char_count > 3500 {
                            let prefix: String = trace.chars().take(3500).collect();
                            format!("{}\n\n...обрезано", teloxide::utils::html::escape(&prefix))
                        } else {
                            teloxide::utils::html::escape(&trace)
                        };
                        bot.send_message(chat_id, format!("🛠 <b>Debug Trace</b>\n\n<pre>{}</pre>", truncated))
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("❌ Ошибка: {}", e)).await?;
                    }
                }
            } else {
                bot.send_message(chat_id, "⚠️ Укажите event_id: <code>/debug &lt;event_id&gt;</code>")
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
        }
        "/rebuild" => {
            if let Some(event_id) = args.split_whitespace().next() {
                match engine.rebuild_from_event(event_id).await {
                    Ok(_) => {
                        bot.send_message(chat_id, format!("Событие {} отправлено на повторную обработку.", event_id))
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("Ошибка при ребилде: {}", e)).await?;
                    }
                }
            } else {
                bot.send_message(chat_id, "Укажите event_id: <code>/rebuild &lt;event_id&gt;</code>")
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
        }
        "/delete" | "/del" | "/rm" => {
            if let Some(target) = args.split_whitespace().next() {
                match engine.delete_record(target).await {
                    Ok(_) => {
                        bot.send_message(chat_id, format!("Запись '{}' успешно удалена из всех баз данных и хранилищ.", target))
                            .await?;
                    }
                    Err(e) => {
                        bot.send_message(chat_id, format!("Ошибка при удалении записи: {}", e)).await?;
                    }
                }
            } else {
                bot.send_message(chat_id, "Укажите ID или путь: <code>/delete &lt;id_or_path&gt;</code>")
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
        }
        _ => {
            let clean_cmd = cmd.trim_start_matches('/');
            let cmd_args = args.split_whitespace().map(|s| s.to_string()).collect();
            let plugin_cmd = brain_plugin::PluginCommand {
                command: clean_cmd.to_string(),
                args: cmd_args,
                user_id,
                chat_id: chat_id.0,
            };

            if let Ok(Some(resp)) = plugin_registry.dispatch_command(&plugin_cmd).await {
                crate::handlers::plugin_helper::send_plugin_response(bot, chat_id, resp).await?;
            } else {
                bot.send_message(chat_id, format!("❓ <b>Неизвестная команда:</b> <code>{}</code>\n\nИспользуйте /help для просмотра списка команд.", cmd))
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
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
    "🧠 <b>Привет! Я NodeBook — ваша персональная операционная система знаний и медиа.</b>\n\n\
    Я объединяю хранилище Obsidian (PARA), вечерний дневник, трекинг аналитики, мультимедиа-загрузчик FSocial и Web Mini App.\n\n\
    <b>Возможности:</b>\n\
    • 💬 <i>Отправьте любую мысль или заметку</i> — сохраню в Obsidian.\n\
    • 🔗 <i>Отправьте ссылку на видео или трек</i> (YouTube, Spotify, TikTok, Reels, SoundCloud, VK) — скачаю сразу в лучшем качестве!\n\
    • 📹 <code>/dl &lt;ссылка&gt;</code> — скачать видео в высоком разрешении.\n\
    • 🎵 <code>/mp3 &lt;ссылка&gt;</code> — скачать аудио с обложкой в 320 kbps.\n\
    • 📱 <code>/app</code> — открыть Web Mini App (музыкальный плеер Spotify, видео-хаб, чистый просмотр заметок).\n\
    • 📖 <code>/diary</code> — вечерний опросник и рефлексия.\n\
    • 🔍 <code>/search</code> — быстрый поиск по базе знаний.\n\n\
    Используйте меню команд или кнопки внизу чата!".to_string()
}

fn cmd_help() -> String {
    "📋 <b>Полный справочник возможностей системы:</b>\n\n\
    <b>База знаний и дневник:</b>\n\
    /start — Главное меню и приветствие\n\
    /app — Открыть Web Mini App (Spotify плеер, видео, база)\n\
    /base — Управление хранилищами Obsidian\n\
    /diary — Запустить вечерний обзор дня\n\
    /search — Поиск по вашей базе знаний\n\
    /today — Посмотреть сегодняшнюю дневниковую заметку\n\
    /stats — Статистика заметок и времени жизни\n\
    /analytics — Меню детальной аналитики и графиков\n\
    /viz — Генерация наглядных чартов и Life Wheel\n\
    /cancel — Отменить текущее действие\n\n\
    <b>Медиа-загрузчик (FSocial Engine):</b>\n\
    /grammar <code>&lt;фраза&gt;</code> — Анализ грамматики через Gemini AI\n\
    /tutor <code>&lt;сообщение&gt;</code> — Диалог на английском с AI-репетитором\n\n\
    <b>Разработчикам и диагностика:</b>\n\
    /metrics — Метрики пайплайна и резолвера\n\
    /debug <code>&lt;event_id&gt;</code> — Трейс обработки события\n\
    /rebuild <code>&lt;event_id&gt;</code> — Переобработать событие".to_string()
}
