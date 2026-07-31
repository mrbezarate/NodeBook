//! TelegramOutputSink — доставляет результат обработки Consolidator обратно в чат.
use async_trait::async_trait;
use brain_common::output::{Output, OutputPayload};
use brain_common::Result;
use brain_core::traits::OutputSink;
use std::sync::Arc;
use teloxide::prelude::*;

pub struct TelegramOutputSink {
    bot: Bot,
    chat_id: teloxide::types::ChatId,
}

impl TelegramOutputSink {
    pub fn new(bot: Bot, chat_id: teloxide::types::ChatId) -> Arc<Self> {
        Arc::new(Self { bot, chat_id })
    }
}

#[async_trait]
impl OutputSink for TelegramOutputSink {
    async fn send(&self, output: Output) -> Result<()> {
        let msg = match &output.payload {
            OutputPayload::InlineText { text } => {
                format!("✅ <b>Готово! Записал в Obsidian.</b>\n\n{}", text)
            }
            OutputPayload::Resource { resource_id } => {
                let name = std::path::Path::new(resource_id)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(resource_id);
                format!("✅ <b>Готово!</b>\n📁 Сохранено: <code>{}</code>", name)
            }
        };

        self.bot
            .send_message(self.chat_id, msg)
            .parse_mode(teloxide::types::ParseMode::Html)
            .await
            .ok();

        Ok(())
    }
}
