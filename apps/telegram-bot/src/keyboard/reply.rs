//! Reply-клавиатура (Главное меню).
use teloxide::types::{KeyboardButton, KeyboardMarkup, WebAppInfo};

/// Главное меню бота.
pub fn main_menu_keyboard() -> KeyboardMarkup {
    main_menu_keyboard_with_app(None)
}

/// Главное меню бота с кнопкой запуска Telegram Web App.
pub fn main_menu_keyboard_with_app(web_app_url: Option<&str>) -> KeyboardMarkup {
    let mut rows = vec![
        vec![
            KeyboardButton::new("📖 Дневник дня"),
            KeyboardButton::new("📊 Аналитика"),
        ],
        vec![
            KeyboardButton::new("🔍 Поиск"),
            KeyboardButton::new("📅 Запись за сегодня"),
        ],
        vec![
            KeyboardButton::new("🗄️ База знаний"),
            KeyboardButton::new("ℹ️ Справка"),
        ],
    ];

    if let Some(url) = web_app_url {
        if let Ok(parsed_url) = reqwest::Url::parse(url) {
            let app_btn = KeyboardButton::new("📱 Запустить Web App")
                .request(teloxide::types::ButtonRequest::WebApp(WebAppInfo { url: parsed_url }));
            rows.insert(0, vec![app_btn]);
        }
    }

    KeyboardMarkup::new(rows).resize_keyboard()
}
