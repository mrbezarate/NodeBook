//! Inline keyboards для Telegram.
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// Клавиатура шкалы 1-10 (для дневника).
pub fn scale_keyboard(prefix: &str) -> InlineKeyboardMarkup {
    let row1: Vec<InlineKeyboardButton> = (1..=5).map(|i| InlineKeyboardButton::callback(format!("{i}"), format!("{prefix}:{i}"))).collect();
    let row2: Vec<InlineKeyboardButton> = (6..=10).map(|i| InlineKeyboardButton::callback(format!("{i}"), format!("{prefix}:{i}"))).collect();
    InlineKeyboardMarkup::new(vec![row1, row2])
}

/// Да/Нет клавиатура.
pub fn yes_no_keyboard(prefix: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("✅ Да", format!("{prefix}:yes")),
        InlineKeyboardButton::callback("❌ Нет", format!("{prefix}:no")),
    ]])
}

/// Действия с сохранённой записью.
pub fn entry_actions_keyboard(file_path: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("📁 Путь к файлу", format!("path:{file_path}")),
    ]])
}
