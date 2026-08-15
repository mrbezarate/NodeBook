use brain_plugin::PluginResponse;
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, InputFile};

pub async fn send_plugin_response(
    bot: &Bot,
    chat_id: ChatId,
    resp: PluginResponse,
) -> anyhow::Result<()> {
    let mut stack = vec![resp];

    while let Some(current_resp) = stack.pop() {
        match current_resp {
            PluginResponse::Batch(responses) => {
                stack.extend(responses.into_iter().rev());
            }
            PluginResponse::MediaGroup { title: _, file_paths, caption } => {
                let valid_paths: Vec<String> = file_paths.into_iter().filter(|p| std::path::Path::new(p).exists()).collect();
                if valid_paths.len() == 1 {
                    let path_str = &valid_paths[0];
                    let input_file = InputFile::file(std::path::Path::new(path_str));
                    let mut req = bot.send_photo(chat_id, input_file);
                    if let Some(ref cap) = caption {
                        req = req.caption(cap);
                    }
                    if let Err(e) = req.await {
                        bot.send_message(chat_id, format!("❌ Ошибка отправки фото: {}", e)).await?;
                    }
                    let _ = tokio::fs::remove_file(path_str).await;
                } else if !valid_paths.is_empty() {
                    for (chunk_idx, chunk) in valid_paths.chunks(10).enumerate() {
                        let mut media_group = Vec::new();
                        for (i, path_str) in chunk.iter().enumerate() {
                            let input_file = InputFile::file(std::path::Path::new(path_str));
                            let mut photo = teloxide::types::InputMediaPhoto::new(input_file);
                            if chunk_idx == 0 && i == 0 && caption.is_some() {
                                photo = photo.caption(caption.as_ref().unwrap().clone());
                            }
                            media_group.push(teloxide::types::InputMedia::Photo(photo));
                        }
                        if media_group.len() == 1 {
                            if let teloxide::types::InputMedia::Photo(p) = media_group.remove(0) {
                                let _ = bot.send_photo(chat_id, p.media).await;
                            }
                        } else if !media_group.is_empty() {
                            if let Err(e) = bot.send_media_group(chat_id, media_group).await {
                                bot.send_message(chat_id, format!("❌ Ошибка отправки слайдшоу: {}", e)).await?;
                            }
                        }
                    }
                    for path_str in valid_paths {
                        let _ = tokio::fs::remove_file(path_str).await;
                    }
                }
            }
            PluginResponse::Text(text) => {
                if bot
                    .send_message(chat_id, &text)
                    .parse_mode(teloxide::types::ParseMode::Html)
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
                    if let Ok(meta) = std::fs::metadata(path) {
                        if meta.len() > 50 * 1024 * 1024 {
                            let mb = meta.len() / (1024 * 1024);
                            bot.send_message(
                                chat_id,
                                format!(
                                    "⚠️ <b>Файл скачан на сервер ({} MB)</b>, но превышает лимит загрузки Telegram Bot API (50 MB).\n\n📁 <b>Путь:</b> <code>{}</code>",
                                    mb, file_path
                                ),
                            )
                            .parse_mode(teloxide::types::ParseMode::Html)
                            .await?;
                            continue;
                        }
                    }

                    let input_file = InputFile::file(path);
                    let is_audio = file_path.ends_with(".mp3") || file_path.ends_with(".ogg") || file_path.ends_with(".m4a");

                    let cover_path = path.with_extension("jpg");

                    if is_audio {
                        let mut req = bot.send_audio(chat_id, input_file);
                        if let Some(cap) = caption {
                            req = req.caption(cap).parse_mode(teloxide::types::ParseMode::Html);
                        }
                        if cover_path.exists() {
                            req = req.thumbnail(InputFile::file(&cover_path));
                        }
                        if let Err(e) = req.await {
                            bot.send_message(chat_id, format!("❌ Ошибка отправки аудио: {}", e)).await?;
                        }
                    } else {
                        let mut req = bot.send_video(chat_id, input_file);
                        if let Some(cap) = caption {
                            req = req.caption(cap).parse_mode(teloxide::types::ParseMode::Html);
                        }
                        if cover_path.exists() {
                            req = req.thumbnail(InputFile::file(&cover_path));
                        }
                        if let Err(e) = req.await {
                            bot.send_message(chat_id, format!("❌ Ошибка отправки видео: {}", e)).await?;
                        }
                    }
                } else {
                    bot.send_message(chat_id, format!("❌ Медиа файл не найден: {}", file_path))
                        .await?;
                }
            }
            PluginResponse::Keyboard { text, options } => {
                let mut buttons = Vec::new();
                for (label, cb_data) in options {
                    buttons.push(vec![InlineKeyboardButton::callback(label, cb_data)]);
                }
                let keyboard = InlineKeyboardMarkup::new(buttons);
                if bot
                    .send_message(chat_id, &text)
                    .reply_markup(keyboard.clone())
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await
                    .is_err()
                {
                    bot.send_message(chat_id, text)
                        .reply_markup(keyboard)
                        .await?;
                }
            }
            PluginResponse::Error(err) => {
                bot.send_message(chat_id, format!("❌ {}", err))
                    .parse_mode(teloxide::types::ParseMode::Html)
                    .await?;
            }
            PluginResponse::Handled | PluginResponse::Ignored => {}
        }
    }
    Ok(())
}
