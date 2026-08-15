use brain_media_downloader::MediaDownloader;
use teloxide::prelude::*;
use teloxide::types::{ChatId, InputFile, ParseMode};
use tracing::{info, warn};

pub async fn handle_playlist_stream(
    bot: &Bot,
    chat_id: ChatId,
    text: &str,
    downloader: &MediaDownloader,
) {
    let urls = MediaDownloader::detect_all_urls(text);
    if urls.is_empty() {
        return;
    }

    for (url, platform, _media_type) in urls {
        info!("Handling playlist stream for: {}", url);

        // 1. If Spotify playlist or album
        if platform == fsocial_common::models::Platform::Spotify || url.contains("spotify.com") {
            if let Some(entity) = MediaDownloader::extract_spotify_entity(&url).await {
                let total = entity.tracks.len();
                if total > 0 {
                    let _ = bot.send_message(
                        chat_id,
                        format!(
                            "⏳ <b>Плейлист: {}</b>\nОбнаружено <b>{}</b> треков. Начинаю последовательную загрузку...",
                            html_escape(&entity.title),
                            total
                        ),
                    )
                    .parse_mode(ParseMode::Html)
                    .await;

                    let mut downloaded_count = 0;
                    for (idx, track) in entity.tracks.iter().enumerate() {
                        let cover_target = track.cover_url.as_deref().or(entity.cover_url.as_deref());
                        match downloader.download_spotify_track_direct(&track.title, &track.artist, cover_target, &track.url).await {
                            Ok((path, item)) => {
                                downloaded_count += 1;
                                let mut req = bot.send_audio(chat_id, InputFile::file(&path));
                                let caption = format!(
                                    "🎵 <b>[{}/{}] {}</b>\n👤 <i>{}</i>\n💾 Сохранено в медиатеку",
                                    idx + 1,
                                    total,
                                    html_escape(&item.title),
                                    html_escape(item.uploader.as_deref().unwrap_or("Неизвестен"))
                                );
                                req = req.caption(caption).parse_mode(ParseMode::Html);
                                if let Some(ref cover) = item.cover_file {
                                    let cover_path = downloader.download_dir().join(cover);
                                    if cover_path.exists() {
                                        req = req.thumbnail(InputFile::file(cover_path));
                                    }
                                }
                                if let Some(ref artist) = item.uploader {
                                    req = req.performer(artist);
                                }
                                req = req.title(&item.title);
                                if let Err(e) = req.await {
                                    warn!("Failed to send audio to Telegram: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!("Failed to download Spotify track {} - {}: {}", track.artist, track.title, e);
                            }
                        }
                    }

                    let _ = bot.send_message(
                        chat_id,
                        format!(
                            "✅ <b>Плейлист «{}» успешно загружен!</b>\nСкачано <b>{} из {}</b> треков. Все треки доступны в Web Player.",
                            html_escape(&entity.title),
                            downloaded_count,
                            total
                        ),
                    )
                    .parse_mode(ParseMode::Html)
                    .await;
                    continue;
                }
            }
        }

        // 2. Generic YouTube / SoundCloud / other playlists
        match downloader.download_playlist(&url, true).await {
            Ok(items) => {
                let total = items.len();
                let _ = bot.send_message(
                    chat_id,
                    format!("⏳ Найдено <b>{}</b> треков в плейлисте. Отправляю...", total),
                )
                .parse_mode(ParseMode::Html)
                .await;

                for (idx, (path, item)) in items.into_iter().enumerate() {
                    let mut req = bot.send_audio(chat_id, InputFile::file(&path));
                    let caption = format!(
                        "🎵 <b>[{}/{}] {}</b>\n👤 <i>{}</i>\n💾 Сохранено в медиатеку",
                        idx + 1,
                        total,
                        html_escape(&item.title),
                        html_escape(item.uploader.as_deref().unwrap_or("Неизвестен"))
                    );
                    req = req.caption(caption).parse_mode(ParseMode::Html);
                    if let Some(ref cover) = item.cover_file {
                        let cover_path = downloader.download_dir().join(cover);
                        if cover_path.exists() {
                            req = req.thumbnail(InputFile::file(cover_path));
                        }
                    }
                    if let Some(ref artist) = item.uploader {
                        req = req.performer(artist);
                    }
                    req = req.title(&item.title);
                    let _ = req.await;
                }

                let _ = bot.send_message(
                    chat_id,
                    format!("✅ <b>Плейлист полностью загружен!</b> ({} треков)", total),
                )
                .parse_mode(ParseMode::Html)
                .await;
            }
            Err(e) => {
                let _ = bot.send_message(
                    chat_id,
                    format!("❌ Ошибка загрузки плейлиста: {}", e),
                )
                .await;
            }
        }
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
