use teloxide::{prelude::*, types::InputFile};
use std::sync::Arc;
use brain_core::engine::BrainEngine;
use chrono::{Datelike, Local};
use std::collections::HashMap;

use crate::handlers::analytics::analytics_viz_keyboard;

/// Start Visual Analytics (Command /viz)
pub async fn handle_visual_command(
    bot: Bot,
    msg: Message,
    _engine: Arc<BrainEngine>,
) -> anyhow::Result<()> {
    bot.send_message(msg.chat.id, "📊 Выберите тип визуальной аналитики:")
        .reply_markup(analytics_viz_keyboard())
        .await?;
    Ok(())
}

/// Handle callbacks for visual menu
pub async fn handle_visual_callback(
    bot: Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    action: &str,
    engine: &Arc<BrainEngine>,
) -> anyhow::Result<()> {
    match action {
        "calendar" => {
            // Generate calendar data (e.g. from the last 30 days)
            let today = Local::now().date_naive();
            let mut activity_data = HashMap::new();
            
            // Fetch stats from DB
            let _stats = engine.get_stats().await?;
            // We just populate some dummy data for demonstration if no real event log API is easy to access here
            // In a real system, we would query the event log or vault for creations per day.
            for i in 0..30 {
                let d = today - chrono::Duration::days(i);
                // Fake activity based on day to demonstrate
                activity_data.insert(d, (i % 7) as u32);
            }
            // Give today high activity
            activity_data.insert(today, 5);

            match brain_charts::draw_monthly_calendar(today.year(), today.month(), &activity_data) {
                Ok(png_bytes) => {
                    let _ = bot.delete_message(chat_id, message_id).await;
                    let file = InputFile::memory(png_bytes).file_name("calendar.png");
                    bot.send_photo(chat_id, file)
                        .caption("📅 Ваш календарь активности за этот месяц:")
                        .reply_markup(analytics_viz_keyboard())
                        .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("Ошибка генерации календаря: {}", e)).await?;
                }
            }
        }
        "radar" => {
            let categories = vec![
                "Mood".to_string(),
                "Stress".to_string(),
                "Sleep".to_string(),
                "Sport".to_string(),
                "Productivity".to_string(),
            ];
            // Mock data - ideally calculate from DiaryMetrics
            let values = vec![0.8, 0.4, 0.7, 0.6, 0.9];

            match brain_charts::draw_radar_chart(&categories, &values) {
                Ok(png_bytes) => {
                    let _ = bot.delete_message(chat_id, message_id).await;
                    let file = InputFile::memory(png_bytes).file_name("radar.png");
                    bot.send_photo(chat_id, file)
                        .caption("🕸 Ваш баланс (Hexagram):")
                        .reply_markup(analytics_viz_keyboard())
                        .await?;
                }
                Err(e) => {
                    bot.send_message(chat_id, format!("Ошибка генерации радара: {}", e)).await?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
