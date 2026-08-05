//! Глубокая аналитика и статистика по дням, неделям, месяцам и графу Obsidian.
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use std::sync::Arc;
use brain_core::engine::BrainEngine;
use chrono::{Datelike, Local, NaiveDate};

pub fn analytics_main_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📈 Статистика", "analytics:menu:stats"),
            InlineKeyboardButton::callback("🧠 AI Инсайты", "analytics:menu:ai"),
        ],
        vec![
            InlineKeyboardButton::callback("🎨 Визуализация", "analytics:menu:viz"),
        ],
    ])
}

pub fn analytics_stats_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("Неделя", "analytics:week"),
            InlineKeyboardButton::callback("Месяц", "analytics:month"),
        ],
        vec![
            InlineKeyboardButton::callback("Год", "analytics:year"),
            InlineKeyboardButton::callback("Всё время", "analytics:all"),
        ],
        vec![
            InlineKeyboardButton::callback("⬅️ Назад", "analytics:menu:main"),
        ],
    ])
}

pub fn analytics_ai_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("Неделя", "analytics:ai_insights:7"),
            InlineKeyboardButton::callback("Месяц", "analytics:ai_insights:30"),
        ],
        vec![
            InlineKeyboardButton::callback("Год", "analytics:ai_insights:365"),
            InlineKeyboardButton::callback("Вся база", "analytics:ai_insights:9999"),
        ],
        vec![
            InlineKeyboardButton::callback("⬅️ Назад", "analytics:menu:main"),
        ],
    ])
}

pub fn analytics_viz_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("📅 Календарь", "visual:calendar"),
            InlineKeyboardButton::callback("🕸 Баланс (Радар)", "visual:radar"),
        ],
        vec![
            InlineKeyboardButton::callback("🌌 Граф Obsidian", "analytics:graph"),
        ],
        vec![
            InlineKeyboardButton::callback("⬅️ Назад", "analytics:menu:main"),
        ],
    ])
}

/// Вывести меню аналитики
pub async fn send_analytics_menu(bot: &Bot, chat_id: ChatId) -> anyhow::Result<()> {
    let text = "📊 <b>Центр Аналитики и Визуализации</b>\n\n\
    Выберите раздел:";
    
    bot.send_message(chat_id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(analytics_main_keyboard())
        .await?;
    Ok(())
}

/// Обработчик нажатий кнопок в меню аналитики
pub async fn handle_analytics_callback(
    bot: &Bot,
    chat_id: ChatId,
    message_id: teloxide::types::MessageId,
    action: &str,
    engine: &Arc<BrainEngine>,
    analytics_engine: &Arc<brain_analytics::engine::LifeAnalyticsEngine>,
) -> anyhow::Result<()> {
    match action {
        "menu:main" => {
            let text = "📊 <b>Центр Аналитики и Визуализации</b>\n\nВыберите раздел:";
            let res = bot.edit_message_text(chat_id, message_id, text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_main_keyboard())
                .await;
            if res.is_err() {
                let _ = bot.delete_message(chat_id, message_id).await;
                bot.send_message(chat_id, text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(analytics_main_keyboard())
                    .await?;
            }
        }
        "menu:stats" => {
            bot.edit_message_text(chat_id, message_id, "📈 <b>Статистика:</b>\nВыберите период:")
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_stats_keyboard())
                .await?;
        }
        "menu:ai" => {
            bot.edit_message_text(chat_id, message_id, "🧠 <b>AI Инсайты:</b>\nВыберите период для анализа:")
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_ai_keyboard())
                .await?;
        }
        "menu:viz" => {
            bot.edit_message_text(chat_id, message_id, "🎨 <b>Визуализация:</b>\nВыберите тип:")
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_viz_keyboard())
                .await?;
        }
        "week" => {
            let report = analyze_period(engine, 7).await;
            bot.edit_message_text(chat_id, message_id, format!("📅 <b>Аналитика за 7 дней:</b>\n\n{}", report))
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_stats_keyboard())
                .await?;
        }
        "month" => {
            let report = analyze_period(engine, 30).await;
            bot.edit_message_text(chat_id, message_id, format!("🗓 <b>Аналитика за 30 дней:</b>\n\n{}", report))
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_stats_keyboard())
                .await?;
        }
        "year" => {
            let report = analyze_period(engine, 365).await;
            bot.edit_message_text(chat_id, message_id, format!("🎆 <b>Аналитика за этот год (365 дней):</b>\n\n{}", report))
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_stats_keyboard())
                .await?;
        }
        "all" => {
            let report = analyze_period(engine, 9999).await;
            bot.edit_message_text(chat_id, message_id, format!("♾ <b>Аналитика за всё время:</b>\n\n{}", report))
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_stats_keyboard())
                .await?;
        }
        "graph" => {
            let report = analyze_obsidian_graph(engine).await;
            let text = format!("🕸 <b>Структура Графа Знаний Obsidian:</b>\n\n{}", report);
            let res = bot.edit_message_text(chat_id, message_id, &text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(analytics_viz_keyboard())
                .await;
            if res.is_err() {
                let _ = bot.delete_message(chat_id, message_id).await;
                bot.send_message(chat_id, text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(analytics_viz_keyboard())
                    .await?;
            }
        }
        _ if action.starts_with("ai_insights:") => {
            let days_str = action.trim_start_matches("ai_insights:");
            let period_days: usize = days_str.parse().unwrap_or(7);
            
            let today = Local::now().format("%Y-%m-%d").to_string();
            let title = if period_days == 9999 {
                format!("AI Insight (All Time) {}", today)
            } else {
                format!("AI Insight ({} Days) {}", period_days, today)
            };

            // 1. Поиск уже сгенерированного инсайта за сегодня
            if let Ok(results) = engine.search(&title).await {
                for res in results {
                    if res.title == title {
                        // Нашли! Читаем из базы
                        if let Ok(content) = engine.read_entry(&res.file_path).await {
                            let header_marker = format!("# {}\n\n", title);
                            let body = if let Some(idx) = content.find(&header_marker) {
                                content[idx + header_marker.len()..].trim().to_string()
                            } else if let Some(idx) = content.find("\n---\n") {
                                content[idx + 5..].trim().to_string()
                            } else {
                                content
                            };
                            
                            bot.edit_message_text(chat_id, message_id, format!("🧠 <b>Сохранённые AI Инсайты за {}:</b>\n\n{}", today, body))
                                .parse_mode(teloxide::types::ParseMode::Html)
                                .reply_markup(analytics_ai_keyboard())
                                .await?;
                            return Ok(());
                        }
                    }
                }
            }

            // 2. Если не нашли — генерируем новый
            let msg_text = if period_days == 9999 {
                "⏳ Генерирую глубокие AI-инсайты (по всей базе)... Это займет время."
            } else {
                "⏳ Генерирую глубокие AI-инсайты (Life Analytics)... Это может занять около минуты."
            };
            
            bot.edit_message_text(chat_id, message_id, msg_text).await?;
            
            let metrics = fetch_diary_metrics(engine, period_days as i64).await;
            match analytics_engine.generate_life_insights(&metrics, period_days).await {
                Ok(insight_text) => {
                    // Сохраняем в Vault
                    let tags = vec!["insight".to_string(), "analytics".to_string()];
                    let _ = engine.save_direct(&title, &insight_text, "Life", tags).await;

                    bot.edit_message_text(chat_id, message_id, format!("🧠 <b>AI Инсайты о вашей жизни:</b>\n\n{}", insight_text))
                        .parse_mode(teloxide::types::ParseMode::Html)
                        .reply_markup(analytics_ai_keyboard())
                        .await?;
                }
                Err(e) => {
                    bot.edit_message_text(chat_id, message_id, format!("❌ Ошибка AI: {}", e)).await?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

struct DayData {
    day_rating: f32,
    mood: f32,
    energy: f32,
    productivity: f32,
    stress: f32,
    motivation: f32,
    sleep: f32,
    exercise: bool,
}

/// Считать файлы из base/Daily и проанализировать за N дней
async fn analyze_period(engine: &Arc<BrainEngine>, days: i64) -> String {
    let daily_dir = std::path::Path::new(&engine.config.vault.path).join("Daily");
    let today = Local::now().date_naive();
    let cutoff = today - chrono::Duration::days(days);

    let mut entries: Vec<DayData> = Vec::new();
    let mut exercise_count = 0;

    if let Ok(mut dir) = tokio::fs::read_dir(&daily_dir).await {
        while let Ok(Some(entry)) = dir.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if let Ok(date) = NaiveDate::parse_from_str(filename, "%Y-%m-%d") {
                    if date >= cutoff && date <= today {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            let data = parse_day_frontmatter(&content);
                            if data.exercise { exercise_count += 1; }
                            entries.push(data);
                        }
                    }
                }
            }
        }
    }

    if entries.is_empty() {
        return format!("📭 За этот период ({}) пока нет сохранённых записей дневника.", days);
    }

    let n = entries.len() as f32;
    let avg_rating: f32 = entries.iter().map(|e| e.day_rating).sum::<f32>() / n;
    let avg_mood: f32 = entries.iter().map(|e| e.mood).sum::<f32>() / n;
    let avg_energy: f32 = entries.iter().map(|e| e.energy).sum::<f32>() / n;
    let avg_prod: f32 = entries.iter().map(|e| e.productivity).sum::<f32>() / n;
    let avg_stress: f32 = entries.iter().map(|e| e.stress).sum::<f32>() / n;
    let avg_sleep: f32 = entries.iter().map(|e| e.sleep).sum::<f32>() / n;

    format!(
        "📊 Записей проанализировано: <b>{}</b>\n\n\
        ⭐ Средняя оценка дня: <b>{:.1}/10</b>\n\
        😊 Настроение: <b>{:.1}/10</b>\n\
        ⚡ Энергия: <b>{:.1}/10</b>\n\
        🎯 Продуктивность: <b>{:.1}/10</b>\n\
        😰 Уровень стресса: <b>{:.1}/10</b>\n\
        😴 Средний сон: <b>{:.1} ч</b>\n\
        🏃 Тренировок выполнено: <b>{}</b> из {}",
        entries.len(),
        avg_rating,
        avg_mood,
        avg_energy,
        avg_prod,
        avg_stress,
        avg_sleep,
        exercise_count,
        entries.len()
    )
}

fn parse_day_frontmatter(content: &str) -> DayData {
    let mut data = DayData {
        day_rating: 0.0, mood: 0.0, energy: 0.0,
        productivity: 0.0, stress: 0.0, motivation: 0.0,
        sleep: 0.0, exercise: false,
    };

    for line in content.lines() {
        if let Some((key, val)) = line.split_once(':') {
            let key = key.trim();
            let val = val.trim();
            match key {
                "day_rating" => data.day_rating = val.parse().unwrap_or(0.0),
                "mood" => data.mood = val.parse().unwrap_or(0.0),
                "energy" => data.energy = val.parse().unwrap_or(0.0),
                "productivity" => data.productivity = val.parse().unwrap_or(0.0),
                "stress" => data.stress = val.parse().unwrap_or(0.0),
                "motivation" => data.motivation = val.parse().unwrap_or(0.0),
                "sleep_hours" => data.sleep = val.parse().unwrap_or(0.0),
                "exercise" => data.exercise = val == "true",
                _ => {}
            }
        }
    }
    data
}

async fn fetch_diary_metrics(engine: &Arc<BrainEngine>, days: i64) -> Vec<brain_common::DiaryMetrics> {
    let daily_dir = std::path::Path::new(&engine.config.vault.path).join("Daily");
    let today = Local::now().date_naive();
    let cutoff = today - chrono::Duration::days(days);
    let mut metrics = Vec::new();

    if let Ok(mut dir) = tokio::fs::read_dir(&daily_dir).await {
        while let Ok(Some(entry)) = dir.next_entry().await {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if let Ok(date) = NaiveDate::parse_from_str(filename, "%Y-%m-%d") {
                    if date >= cutoff && date <= today {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            let data = parse_day_frontmatter(&content);
                            let mut m = brain_common::DiaryMetrics::new(date);
                            m.mood = Some(data.mood as u8);
                            m.stress = Some(data.stress as u8);
                            m.productivity = Some(data.productivity as u8);
                            m.sleep_hours = Some(data.sleep);
                            metrics.push(m);
                        }
                    }
                }
            }
        }
    }
    metrics.sort_by_key(|m| m.date);
    metrics
}

/// Сканировать всю базу Obsidian (base/) и подсчитать граф тегов и ссылок [[ ]]
async fn analyze_obsidian_graph(engine: &Arc<BrainEngine>) -> String {
    let vault_dir = std::path::Path::new(&engine.config.vault.path);
    let mut file_count = 0;
    let mut tag_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut wikilink_count = 0;

    let mut stack = vec![vault_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        if let Ok(mut entries) = tokio::fs::read_dir(&dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    file_count += 1;
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        // Подсчёт [[ссылок]]
                        wikilink_count += content.matches("[[").count();

                        // Подсчёт #тегов
                        for word in content.split_whitespace() {
                            if word.starts_with('#') && word.len() > 1 {
                                let tag = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-').to_lowercase();
                                if !tag.is_empty() {
                                    *tag_map.entry(tag).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut sorted_tags: Vec<(&String, &usize)> = tag_map.iter().collect();
    sorted_tags.sort_by(|a, b| b.1.cmp(a.1));

    let top_tags_str = if sorted_tags.is_empty() {
        "<i>теги не найдены</i>".to_string()
    } else {
        sorted_tags.iter().take(8).map(|(t, c)| format!("#{} ({})", t, c)).collect::<Vec<_>>().join(", ")
    };

    format!(
        "📄 Всего Markdown заметок: <b>{}</b>\n\
        🔗 Всего связей между заметками ([[ссылок]]): <b>{}</b>\n\
        🏷 Уникальных тегов: <b>{}</b>\n\n\
        🔥 <b>Топ тегов:</b>\n{}",
        file_count,
        wikilink_count,
        tag_map.len(),
        top_tags_str
    )
}

/// Сухая статистика жизни (Memento Mori)
pub fn format_life_stats(birth_date_str: &str, life_expectancy_years: u32) -> String {
    let birth_date = NaiveDate::parse_from_str(birth_date_str, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2009, 1, 24).unwrap());
    
    let today = Local::now().date_naive();
    let days_lived = (today - birth_date).num_days();
    
    let total_days = (life_expectancy_years as i64) * 365;
    let days_remaining = (total_days - days_lived).max(0);
    let years_remaining = days_remaining as f32 / 365.0;
    
    let age_years = days_lived / 365;
    let age_months = (days_lived % 365) / 30;
    
    let lived_pct = (days_lived as f32 / total_days as f32) * 100.0;

    // Дней до следующего Дня Рождения (24 января)
    let this_year_bday = NaiveDate::from_ymd_opt(today.year(), birth_date.month(), birth_date.day())
        .unwrap_or(today);
    
    let next_bday = if this_year_bday >= today {
        this_year_bday
    } else {
        NaiveDate::from_ymd_opt(today.year() + 1, birth_date.month(), birth_date.day())
            .unwrap_or(today)
    };
    
    let days_to_bday = (next_bday - today).num_days();

    format!(
        "⌛ <b>Сухая статистика жизни (Memento Mori)</b>\n\n\
        🎂 <b>Дата рождения:</b> {}\n\
        👤 <b>Возраст:</b> {} лет, {} мес. (прожито {} дней)\n\
        📊 <b>Прожито жизни:</b> {:.1}% (от {} лет / {} дней)\n\
        ⏳ <b>Осталось до {} лет:</b> {} дней (~{:.1} лет)\n\n\
        🎉 <b>До следующего Дня Рождения:</b> {} дней ({})",
        birth_date.format("%d.%m.%Y"),
        age_years, age_months, days_lived,
        lived_pct, life_expectancy_years, total_days,
        life_expectancy_years, days_remaining, years_remaining,
        days_to_bday, next_bday.format("%d.%m.%Y")
    )
}
