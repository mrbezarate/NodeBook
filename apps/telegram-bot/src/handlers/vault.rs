//! Обработка переключения и управления базами данных / хранилищами (Vaults).
use teloxide::prelude::*;
use std::sync::Arc;
use tokio::sync::RwLock;
use brain_core::engine::BrainEngine;
use brain_vault::VaultRegistry;
use crate::state::{StateManager, UserState};
use crate::keyboard::inline::vault_menu_keyboard;

pub const REGISTRY_PATH: &str = "./vaults/registry.json";

/// Отправить меню управления базами знаний (Vaults).
pub fn format_vault_menu(registry: &VaultRegistry) -> String {
    let active = registry.get_active_vault();
    let active_name = active.map(|v| v.name.as_str()).unwrap_or("base_1");
    let active_path = active.map(|v| v.path.as_str()).unwrap_or("./vaults/base_1");

    let mut list_str = String::new();
    for v in &registry.vaults {
        let is_active = v.id == registry.active_vault_id;
        if is_active {
            list_str.push_str(&format!("• <b>{}</b> (активна) — <code>{}</code>\n", v.name, v.path));
        } else {
            list_str.push_str(&format!("• <b>{}</b> — <code>{}</code>\n", v.name, v.path));
        }
    }

    format!(
        "🗄️ <b>Управление хранилищами (базами знаний)</b>\n\n\
        📍 <b>Активная база:</b> <code>{}</code>\n\
        📂 <b>Путь на диске:</b> <code>{}</code>\n\n\
        📋 <b>Все доступные базы ({}):</b>\n{}\n\
        <i>Нажмите кнопку ниже, чтобы переключить активную базу, создать новую или переименовать текущую.</i>",
        active_name,
        active_path,
        registry.vaults.len(),
        list_str
    )
}

pub async fn send_vault_menu(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    vault_registry: &Arc<RwLock<VaultRegistry>>,
) -> anyhow::Result<()> {
    let registry = vault_registry.read().await;
    let text = format_vault_menu(&registry);
    let keyboard = vault_menu_keyboard(&registry);

    bot.send_message(chat_id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn handle_vault_callback(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    message_id: teloxide::types::MessageId,
    user_id: u64,
    action: &str,
    _engine: &Arc<BrainEngine>,
    state_manager: &Arc<StateManager>,
    vault_registry: &Arc<RwLock<VaultRegistry>>,
) -> anyhow::Result<()> {
    match action {
        "create" => {
            state_manager.set(user_id, UserState::WaitingForNewVaultName).await;
            bot.send_message(
                chat_id,
                "➕ <b>Создание нового хранилища</b>\n\n\
                Введите название для новой базы знаний (например, <i>Мои Проекты</i> или <i>base_2</i>):"
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        }
        "rename" => {
            state_manager.set(user_id, UserState::WaitingForRenameVault).await;
            let registry = vault_registry.read().await;
            let current_name = registry.get_active_vault().map(|v| v.name.as_str()).unwrap_or("base_1");
            bot.send_message(
                chat_id,
                format!(
                    "✏️ <b>Переименование хранилища</b>\n\n\
                    Текущее название: <b>{}</b>\n\n\
                    Введите новое название прямо в чат:",
                    current_name
                )
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;
        }
        _ if action.starts_with("switch:") => {
            let target_id = action.trim_start_matches("switch:");
            let mut registry = vault_registry.write().await;
            if registry.switch_active_vault(REGISTRY_PATH, target_id) {
                let new_path = registry.get_active_path();
                let active_name = registry.get_active_vault().map(|v| v.name.clone()).unwrap_or_default();
                
                // Initialize engine vault directory structure
                let path_buf = std::path::PathBuf::from(&new_path);
                let _ = tokio::fs::create_dir_all(&path_buf).await;

                let text = format_vault_menu(&registry);
                let keyboard = vault_menu_keyboard(&registry);

                let _ = bot.edit_message_text(chat_id, message_id, text)
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .reply_markup(keyboard)
                    .await;

                bot.send_message(
                    chat_id,
                    format!("🔄 <b>База переключена!</b>\nТеперь активное хранилище: <b>{}</b> (<code>{}</code>)", active_name, new_path)
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub async fn handle_vault_text_input(
    bot: &Bot,
    msg: &teloxide::types::Message,
    user_id: u64,
    text: &str,
    user_state: UserState,
    state_manager: &Arc<StateManager>,
    vault_registry: &Arc<RwLock<VaultRegistry>>,
) -> anyhow::Result<()> {
    let chat_id = msg.chat.id;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        bot.send_message(chat_id, "⚠️ Название базы не может быть пустым. Попробуйте еще раз:").await?;
        return Ok(());
    }

    match user_state {
        UserState::WaitingForNewVaultName => {
            state_manager.reset(user_id).await;
            let mut registry = vault_registry.write().await;
            let created_info = registry.create_vault(REGISTRY_PATH, trimmed);
            
            bot.send_message(
                chat_id,
                format!(
                    "✅ <b>Новое хранилище создано и активировано!</b>\n\n\
                    🏷 <b>Название:</b> {}\n\
                    🆔 <b>ID:</b> <code>{}</code>\n\
                    📂 <b>Путь:</b> <code>{}</code>",
                    created_info.name,
                    created_info.id,
                    created_info.path
                )
            )
            .parse_mode(teloxide::types::ParseMode::Html)
            .await?;

            send_vault_menu(bot, chat_id, vault_registry).await?;
        }
        UserState::WaitingForRenameVault => {
            state_manager.reset(user_id).await;
            let mut registry = vault_registry.write().await;
            if registry.rename_active_vault(REGISTRY_PATH, trimmed) {
                bot.send_message(
                    chat_id,
                    format!("✅ <b>Хранилище переименовано в «{}»!</b>", trimmed)
                )
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
            } else {
                bot.send_message(chat_id, "❌ Не удалось переименовать хранилище.").await?;
            }

            send_vault_menu(bot, chat_id, vault_registry).await?;
        }
        _ => {}
    }

    Ok(())
}
