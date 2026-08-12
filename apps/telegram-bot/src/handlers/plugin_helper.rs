use brain_plugin::PluginResponse;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile};

pub async fn send_plugin_response(
    bot: &Bot,
    chat_id: ChatId,
    resp: PluginResponse,
) -> anyhow::Result<()> {
    match resp {
        PluginResponse::Text(text) => {
            if bot
                .send_message(chat_id, &text)
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await
                .is_err()
            {
                bot.send_message(chat_id, text).await?;
            }
        }
        PluginResponse::Media {
            title: _,
            file_path,
            caption,
        } => {
            let path = std::path::Path::new(&file_path);
            if path.exists() {
                let input_file = InputFile::file(path);
                let is_audio = file_path.ends_with(".mp3") || file_path.ends_with(".ogg");

                if is_audio {
                    let mut req = bot.send_audio(chat_id, input_file);
                    if let Some(cap) = caption {
                        req = req.caption(cap);
                    }
                    req.await?;
                } else {
                    let mut req = bot.send_video(chat_id, input_file);
                    if let Some(cap) = caption {
                        req = req.caption(cap);
                    }
                    req.await?;
                }
            } else {
                bot.send_message(chat_id, format!("❌ Media file not found: {}", file_path))
                    .await?;
            }
        }
        PluginResponse::Keyboard { text, options } => {
            let mut buttons = Vec::new();
            for (label, cb_data) in options {
                buttons.push(vec![InlineKeyboardButton::callback(label, cb_data)]);
            }
            let keyboard = InlineKeyboardMarkup::new(buttons);
            bot.send_message(chat_id, text)
                .reply_markup(keyboard)
                .await?;
        }
        PluginResponse::Error(err) => {
            bot.send_message(chat_id, format!("❌ Plugin Error: {}", err))
                .await?;
        }
        PluginResponse::Handled | PluginResponse::Ignored => {}
    }
    Ok(())
}
