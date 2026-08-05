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

pub fn entry_actions_keyboard(entry_id: &str) -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new(vec![
        vec![
            InlineKeyboardButton::callback("🔍 Детали", format!("details:{entry_id}")),
            InlineKeyboardButton::callback("📝 Дополнить", format!("edit:{entry_id}")),
        ],
        vec![
            InlineKeyboardButton::callback("🗑 Удалить", format!("delete:{entry_id}")),
        ]
    ])
}
