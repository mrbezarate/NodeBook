//! Reply-клавиатура (Главное меню).
use teloxide::types::{KeyboardButton, KeyboardMarkup};

/// Главное меню бота.
pub fn main_menu_keyboard() -> KeyboardMarkup {
    KeyboardMarkup::new(vec![
        vec![
            KeyboardButton::new("📖 Дневник дня"),
            KeyboardButton::new("📊 Аналитика и Граф"),
        ],
        vec![
            KeyboardButton::new("🔍 Поиск"),
            KeyboardButton::new("📅 Запись за сегодня"),
        ],
        vec![
            KeyboardButton::new("⌛ Статистика жизни"),
            KeyboardButton::new("ℹ️ Справка"),
        ],
    ])
    .resize_keyboard()
}
