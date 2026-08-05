//! Обработчик вечернего дневника — FSM, inline-кнопки, сохранение в vault.
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::*;
use tokio::sync::RwLock;
use chrono::{NaiveDate, Datelike};

use brain_core::engine::BrainEngine;
use brain_diary::evening_review::{EveningReview, ReviewState};
use brain_diary::day_info::DayInfo;
use brain_common::DiaryMetrics;
use crate::state::{StateManager, UserState};
use crate::keyboard::inline::{scale_keyboard, yes_no_keyboard};

// ── Отслеживание сообщений для удаления ─────────────────────

static TRACKED_MESSAGES: std::sync::LazyLock<RwLock<HashMap<u64, Vec<i32>>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

async fn track_message(user_id: u64, msg_id: i32) {
    TRACKED_MESSAGES.write().await.entry(user_id).or_default().push(msg_id);
}

async fn delete_tracked_messages(bot: &Bot, chat_id: ChatId, user_id: u64) {
    if let Some(msg_ids) = TRACKED_MESSAGES.write().await.remove(&user_id) {
        for msg_id in msg_ids {
            let _ = bot.delete_message(chat_id, MessageId(msg_id)).await;
        }
    }
}

// ── Форматирование ──────────────────────────────────────────

fn russian_month(m: u32) -> &'static str {
    match m {
        1 => "января", 2 => "февраля", 3 => "марта", 4 => "апреля",
        5 => "мая", 6 => "июня", 7 => "июля", 8 => "августа",
        9 => "сентября", 10 => "октября", 11 => "ноября", 12 => "декабря",
        _ => "",
    }
}

fn russian_weekday(date: chrono::NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "Понедельник",
        chrono::Weekday::Tue => "Вторник",
        chrono::Weekday::Wed => "Среда",
        chrono::Weekday::Thu => "Четверг",
        chrono::Weekday::Fri => "Пятница",
        chrono::Weekday::Sat => "Суббота",
        chrono::Weekday::Sun => "Воскресенье",
    }
}

// ── Погода ───────────────────────────────────────────────────

/// Получить погоду через wttr.in (бесплатный API, не нужен ключ).
/// Возвращает (погода_сегодня, погода_завтра).
async fn fetch_weather(city: &str) -> (String, String) {
    let url = format!(
        "https://wttr.in/{}?format=j1&lang=ru",
        urlencoding(city)
    );
    let result = async {
        let resp = reqwest::get(&url).await?;
        let json: serde_json::Value = resp.json().await?;

        // Текущая погода
        let current = &json["current_condition"][0];
        let temp = current["temp_C"].as_str().unwrap_or("?");
        let feels = current["FeelsLikeC"].as_str().unwrap_or("?");
        let desc_ru = current["lang_ru"].as_array()
            .and_then(|a| a.first())
            .and_then(|v| v["value"].as_str())
            .unwrap_or(current["weatherDesc"].as_array()
                .and_then(|a| a.first())
                .and_then(|v| v["value"].as_str())
                .unwrap_or("?"));
        let humidity = current["humidity"].as_str().unwrap_or("?");
        let wind = current["windspeedKmph"].as_str().unwrap_or("?");

        let today_str = format!(
            "{}°C (ощущ. {}°C), {}, 💧{}%, 💨{} км/ч",
            temp, feels, desc_ru, humidity, wind
        );

        // Завтрашняя погода
        let tomorrow = json["weather"].as_array()
            .and_then(|w| w.get(1));
        let tomorrow_str = if let Some(tw) = tomorrow {
            let max = tw["maxtempC"].as_str().unwrap_or("?");
            let min = tw["mintempC"].as_str().unwrap_or("?");
            let desc = tw["hourly"].as_array()
                .and_then(|h| h.get(4)) // ~12:00
                .and_then(|h| h["lang_ru"].as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v["value"].as_str()))
                .unwrap_or("?");
            format!("{}..{}°C, {}", min, max, desc)
        } else {
            "нет данных".to_string()
        };

        Ok::<_, anyhow::Error>((today_str, tomorrow_str))
    }.await;

    match result {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Weather fetch failed: {}", e);
            ("не удалось получить".to_string(), "не удалось получить".to_string())
        }
    }
}

/// URL-encode для названия города.
fn urlencoding(s: &str) -> String {
    s.replace(' ', "+")
        .replace('/', "%2F")
}

fn format_day_info_message(info: &DayInfo, weather_today: &str, weather_tomorrow: &str) -> String {
    let date = info.date;
    format!(
        "🌙 Добрый вечер.\n\n\
        ━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n\
        📅 Сегодня: {} {} {}, {}\n\
        🔢 Твой {}-й день жизни\n\
        ⏳ Осталось примерно {} дней\n\
        📊 Прожито: {:.1}% жизни\n\n\
        🌤 Сейчас: {}\n\
        🌅 Завтра: {}\n\n\
        ━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n\
        Готов к вечернему обзору?",
        date.day(), russian_month(date.month()), date.year(), russian_weekday(date),
        info.days_lived, info.days_remaining, info.life_percentage,
        weather_today, weather_tomorrow
    )
}

/// Эмодзи для текущего вопроса.
fn metric_question_text(state: &ReviewState) -> &'static str {
    match state {
        ReviewState::AskingDayRating   => "📊 Как оценишь сегодняшний день?",
        ReviewState::AskingMood        => "😊 Настроение?",
        ReviewState::AskingEnergy      => "⚡ Энергия?",
        ReviewState::AskingStress      => "😰 Стресс? (1 = сильный, 10 = нет стресса)",
        ReviewState::AskingMotivation  => "🔥 Мотивация?",
        ReviewState::AskingProductivity => "🎯 Продуктивность?",
        ReviewState::AskingSleep       => "😴 Сколько часов сна? (напиши число)",
        ReviewState::AskingExercise    => "🏃 Была тренировка?",
        ReviewState::AskingGoodEvents  => "🌟 Напиши хорошие события за сегодня.\nКаждое с новой строки.",
        ReviewState::AskingBadEvents   => "😔 Напиши плохие события / что расстроило.\n(Напиши \"нет\" если день был идеальный)",
        ReviewState::AskingFreeThoughts => "💭 Свободные мысли, размышления, идеи.\n(Напиши \"skip\" чтобы пропустить)",
        ReviewState::Complete          => "✨ Обзор завершён!",
        _ => "",
    }
}

/// Нужна ли inline-клавиатура для текущего состояния?
fn needs_scale_keyboard(state: &ReviewState) -> bool {
    matches!(state,
        ReviewState::AskingDayRating | ReviewState::AskingMood |
        ReviewState::AskingEnergy | ReviewState::AskingStress |
        ReviewState::AskingMotivation | ReviewState::AskingProductivity
    )
}

fn metrics_average(m: &DiaryMetrics) -> f32 {
    let vals: Vec<f32> = [
        m.day_rating.map(|v| v as f32),
        m.mood.map(|v| v as f32),
        m.energy.map(|v| v as f32),
        m.productivity.map(|v| v as f32),
        m.stress.map(|v| v as f32),
        m.motivation.map(|v| v as f32),
    ].iter().filter_map(|v| *v).collect();
    if vals.is_empty() { return 0.0; }
    vals.iter().sum::<f32>() / vals.len() as f32
}

fn format_diary_summary(metrics: &DiaryMetrics) -> String {
    let date = metrics.date;
    let avg = metrics_average(metrics);

    let mut s = format!(
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n\
        📋 ИТОГИ ДНЯ — {} {} {}\n\n\
        📊 Метрики:\n",
        date.day(), russian_month(date.month()), date.year()
    );

    // Основные метрики в две колонки
    let m = metrics;
    let lines = [
        ("📊 День",           m.day_rating,    "😊 Настроение",     m.mood),
        ("⚡ Энергия",        m.energy,        "🎯 Продуктивность", m.productivity),
        ("😰 Стресс",        m.stress,        "🔥 Мотивация",      m.motivation),
    ];
    for (l1, v1, l2, v2) in &lines {
        s.push_str(&format!("   {}: {}  │  {}: {}\n",
            l1, v1.map_or("-".to_string(), |v| format!("{}/10", v)),
            l2, v2.map_or("-".to_string(), |v| format!("{}/10", v)),
        ));
    }
    if let Some(sleep) = m.sleep_hours {
        s.push_str(&format!("   😴 Сон: {} ч", sleep));
    }
    if let Some(exercise) = m.exercise {
        s.push_str(&format!("  │  🏃 Тренировка: {}\n", if exercise { "Да" } else { "Нет" }));
    } else {
        s.push('\n');
    }

    s.push_str(&format!("\n   📈 Среднее: {:.1}/10\n", avg));

    if let Some(ref good) = m.good_events {
        s.push_str("\n🌟 Хорошее:\n");
        for line in good.lines() {
            let line = line.trim();
            if !line.is_empty() {
                s.push_str(&format!("   • {}\n", line));
            }
        }
    }
    if let Some(ref bad) = m.bad_events {
        if bad.to_lowercase() != "нет" {
            s.push_str("\n😔 Плохое:\n");
            for line in bad.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    s.push_str(&format!("   • {}\n", line));
                }
            }
        }
    }
    if let Some(ref thoughts) = m.free_thoughts {
        if thoughts.to_lowercase() != "skip" {
            s.push_str(&format!("\n💭 Мысли:\n{}\n", thoughts));
        }
    }

    s.push_str("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n");
    s.push_str(&format!("✅ Запись сохранена в Obsidian\n📁 Daily/{}.md\n\n", date));
    s.push_str("Спокойной ночи! 🌙");
    s
}

// ── Сохранение в Vault ──────────────────────────────────────

/// Генерация полного Obsidian-совместимого Markdown с YAML frontmatter.
fn generate_obsidian_markdown(metrics: &DiaryMetrics) -> String {
    let m = metrics;
    let date = m.date;
    let avg = metrics_average(m);
    let yesterday = date.pred_opt().unwrap_or(date);
    let tomorrow = date.succ_opt().unwrap_or(date);

    // YAML frontmatter
    let mut md = format!("\
---
date: {date}
day_rating: {dr}
mood: {mood}
energy: {energy}
productivity: {prod}
stress: {stress}
motivation: {motiv}
sleep_hours: {sleep}
exercise: {exercise}
average: {avg:.1}
type: daily
tags: [дневник, daily, итоги]
created_by: brain-bot
---

",
        date = date,
        dr = m.day_rating.unwrap_or(0),
        mood = m.mood.unwrap_or(0),
        energy = m.energy.unwrap_or(0),
        prod = m.productivity.unwrap_or(0),
        stress = m.stress.unwrap_or(0),
        motiv = m.motivation.unwrap_or(0),
        sleep = m.sleep_hours.unwrap_or(0.0),
        exercise = m.exercise.unwrap_or(false),
        avg = avg,
    );

    // Заголовок и навигационная цепочка для Графа Obsidian
    md.push_str(&format!("# 📅 {} {} {} — {}\n\n",
        date.day(), russian_month(date.month()), date.year(), russian_weekday(date)
    ));
    md.push_str(&format!("[[{}|◄ Вчера]] | **{}** | [[{}|Завтра ►]]\n\n", yesterday, date, tomorrow));

    // Таблица метрик
    md.push_str("## 📊 Метрики\n\n");
    md.push_str("| Метрика | Оценка |\n|---------|--------|\n");
    if let Some(v) = m.day_rating    { md.push_str(&format!("| ⭐ Оценка дня | {}/10 |\n", v)); }
    if let Some(v) = m.mood          { md.push_str(&format!("| 😊 Настроение | {}/10 |\n", v)); }
    if let Some(v) = m.energy        { md.push_str(&format!("| ⚡ Энергия | {}/10 |\n", v)); }
    if let Some(v) = m.productivity  { md.push_str(&format!("| 🎯 Продуктивность | {}/10 |\n", v)); }
    if let Some(v) = m.stress        { md.push_str(&format!("| 😰 Стресс | {}/10 |\n", v)); }
    if let Some(v) = m.motivation    { md.push_str(&format!("| 🔥 Мотивация | {}/10 |\n", v)); }
    if let Some(v) = m.sleep_hours   { md.push_str(&format!("| 😴 Сон | {} ч |\n", v)); }
    if let Some(v) = m.exercise      { md.push_str(&format!("| 🏃 Тренировка | {} |\n", if v { "Да" } else { "Нет" })); }
    md.push_str(&format!("\n**Среднее: {:.1}/10**\n\n", avg));
    md.push_str("---\n\n");

    // Хорошее
    md.push_str("## 🌟 Хорошее\n\n");
    if let Some(ref good) = m.good_events {
        for line in good.lines() {
            let line = line.trim();
            if !line.is_empty() {
                md.push_str(&format!("- {}\n", line));
            }
        }
    } else {
        md.push_str("- _нет записей_\n");
    }
    md.push_str("\n---\n\n");

    // Плохое
    md.push_str("## 😔 Плохое\n\n");
    if let Some(ref bad) = m.bad_events {
        if bad.to_lowercase() != "нет" {
            for line in bad.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    md.push_str(&format!("- {}\n", line));
                }
            }
        } else {
            md.push_str("- _ничего плохого!_ 🎉\n");
        }
    } else {
        md.push_str("- _нет записей_\n");
    }
    md.push_str("\n---\n\n");

    // Мысли
    md.push_str("## 💭 Мысли\n\n");
    if let Some(ref thoughts) = m.free_thoughts {
        if thoughts.to_lowercase() != "skip" {
            md.push_str(thoughts);
            md.push('\n');
        } else {
            md.push_str("_пропущено_\n");
        }
    } else {
        md.push_str("_нет записей_\n");
    }
    md.push_str("\n---\n\n");

    // Теги для Графа Obsidian
    md.push_str("## 🏷 Граф и Теги\n");
    md.push_str("#дневник #daily #итоги #brain-os\n");

    md
}

async fn save_diary_to_vault(metrics: &DiaryMetrics, vault_path: &str, daily_folder: &str) -> anyhow::Result<()> {
    let daily_dir = std::path::Path::new(vault_path).join(daily_folder);
    tokio::fs::create_dir_all(&daily_dir).await?;

    let file_path = daily_dir.join(format!("{}.md", metrics.date));
    let md = generate_obsidian_markdown(metrics);

    tokio::fs::write(&file_path, &md).await?;
    tracing::info!("📁 Diary saved to {}", file_path.display());
    Ok(())
}

// ── Завершение дневника ─────────────────────────────────────

async fn finish_diary(
    bot: &Bot,
    chat_id: ChatId,
    user_id: u64,
    metrics: &DiaryMetrics,
    vault_path: &str,
    daily_folder: &str,
    state_manager: &StateManager,
) -> anyhow::Result<()> {
    // Сохранить в Vault
    if let Err(e) = save_diary_to_vault(metrics, vault_path, daily_folder).await {
        tracing::error!("Failed to save diary: {}", e);
        bot.send_message(chat_id, format!("⚠️ Ошибка сохранения: {}", e)).await?;
    }

    // Удалить все промежуточные сообщения
    delete_tracked_messages(bot, chat_id, user_id).await;

    // Отправить итог
    let summary = format_diary_summary(metrics);
    bot.send_message(chat_id, summary).await?;

    // Сбросить состояние
    state_manager.reset(user_id).await;

    Ok(())
}

// ── Публичные обработчики ───────────────────────────────────

/// Запустить вечерний дневник.
pub async fn start_diary(
    bot: &Bot,
    chat_id: ChatId,
    user_id: u64,
    engine: &Arc<BrainEngine>,
    state_manager: &Arc<StateManager>,
) -> anyhow::Result<()> {
    let review = EveningReview::new();

    let birth_date = NaiveDate::parse_from_str(&engine.config.diary.birth_date, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap());

    let day_info = DayInfo::calculate(birth_date, engine.config.diary.life_expectancy_years);

    // Получаем погоду
    let city = if engine.config.diary.city.is_empty() {
        engine.config.diary.timezone.split('/').last().unwrap_or("Moscow")
    } else {
        &engine.config.diary.city
    };
    let (weather_today, weather_tomorrow) = fetch_weather(city).await;

    let text = format_day_info_message(&day_info, &weather_today, &weather_tomorrow);

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("Начать ✨", "diary:start"),
    ]]);

    let sent = bot.send_message(chat_id, text)
        .reply_markup(keyboard)
        .await?;

    track_message(user_id, sent.id.0).await;
    state_manager.set(user_id, UserState::DiaryReview(review)).await;

    tracing::info!("🌙 Diary started for user {}", user_id);
    Ok(())
}

/// Обработать callback от inline-кнопки дневника.
pub async fn handle_diary_callback(
    bot: &Bot,
    chat_id: ChatId,
    message_id: MessageId,
    user_id: u64,
    prefix: &str,
    value: &str,
    engine: &Arc<BrainEngine>,
    state_manager: &Arc<StateManager>,
) -> anyhow::Result<()> {
    let state = state_manager.get(user_id).await;
    let mut review = match state {
        UserState::DiaryReview(r) => r,
        _ => return Ok(()),
    };

    match prefix {
        "diary" if value == "start" => {
            // Перейти к первому вопросу
            review.state = ReviewState::AskingDayRating;
            let q = metric_question_text(&review.state);
            let kb = scale_keyboard("metric");
            bot.edit_message_text(chat_id, message_id, q)
                .reply_markup(kb)
                .await?;
        }

        "metric" => {
            // Обработать ответ на шкалу 1-10
            review.process_answer(value);

            let q = metric_question_text(&review.state);

            if needs_scale_keyboard(&review.state) {
                let kb = scale_keyboard("metric");
                bot.edit_message_text(chat_id, message_id, q)
                    .reply_markup(kb)
                    .await?;
            } else if matches!(review.state, ReviewState::AskingSleep) {
                // Сон — нужен текстовый ввод, удалим кнопки
                let _ = bot.delete_message(chat_id, message_id).await;
                let sent = bot.send_message(chat_id, q).await?;
                track_message(user_id, sent.id.0).await;
            } else if matches!(review.state, ReviewState::AskingExercise) {
                let kb = yes_no_keyboard("exercise");
                bot.edit_message_text(chat_id, message_id, q)
                    .reply_markup(kb)
                    .await?;
            } else if matches!(review.state, ReviewState::AskingGoodEvents | ReviewState::AskingBadEvents | ReviewState::AskingFreeThoughts) {
                // Текстовый ввод
                let _ = bot.delete_message(chat_id, message_id).await;
                let sent = bot.send_message(chat_id, q).await?;
                track_message(user_id, sent.id.0).await;
            } else if review.is_complete() {
                let _ = bot.delete_message(chat_id, message_id).await;
                finish_diary(bot, chat_id, user_id, &review.metrics, &engine.config.vault.path, &engine.config.vault.daily_folder, state_manager).await?;
                return Ok(());
            }
        }

        "exercise" => {
            // yes / no
            review.process_answer(if value == "yes" { "да" } else { "нет" });

            let q = metric_question_text(&review.state);

            if matches!(review.state, ReviewState::AskingGoodEvents | ReviewState::AskingBadEvents | ReviewState::AskingFreeThoughts) {
                let _ = bot.delete_message(chat_id, message_id).await;
                let sent = bot.send_message(chat_id, q).await?;
                track_message(user_id, sent.id.0).await;
            } else if review.is_complete() {
                let _ = bot.delete_message(chat_id, message_id).await;
                finish_diary(bot, chat_id, user_id, &review.metrics, &engine.config.vault.path, &engine.config.vault.daily_folder, state_manager).await?;
                return Ok(());
            }
        }

        _ => {}
    }

    state_manager.set(user_id, UserState::DiaryReview(review)).await;
    Ok(())
}

/// Обработать текстовое сообщение в режиме дневника.
pub async fn handle_diary_text(
    bot: &Bot,
    msg: &Message,
    user_id: u64,
    engine: &Arc<BrainEngine>,
    state_manager: &Arc<StateManager>,
) -> anyhow::Result<()> {
    let text = msg.text().unwrap_or("");
    let chat_id = msg.chat.id;

    // Трекаем сообщение пользователя для удаления
    track_message(user_id, msg.id.0).await;

    let state = state_manager.get(user_id).await;
    let mut review = match state {
        UserState::DiaryReview(r) => r,
        _ => return Ok(()),
    };

    review.process_answer(text);

    if review.is_complete() {
        finish_diary(bot, chat_id, user_id, &review.metrics, &engine.config.vault.path, &engine.config.vault.daily_folder, state_manager).await?;
    } else {
        let q = metric_question_text(&review.state);

        if needs_scale_keyboard(&review.state) {
            let kb = scale_keyboard("metric");
            let sent = bot.send_message(chat_id, q)
                .reply_markup(kb)
                .await?;
            track_message(user_id, sent.id.0).await;
        } else if matches!(review.state, ReviewState::AskingExercise) {
            let kb = yes_no_keyboard("exercise");
            let sent = bot.send_message(chat_id, q)
                .reply_markup(kb)
                .await?;
            track_message(user_id, sent.id.0).await;
        } else {
            // Текстовый ввод
            let sent = bot.send_message(chat_id, q).await?;
            track_message(user_id, sent.id.0).await;
        }

        state_manager.set(user_id, UserState::DiaryReview(review)).await;
    }

    Ok(())
}
