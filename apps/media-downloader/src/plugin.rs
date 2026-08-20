use crate::downloader::MediaDownloader;
use async_trait::async_trait;
use brain_common::Result;
use brain_plugin::{
    Plugin, PluginCapability, PluginCommand, PluginManifest, PluginMessage, PluginResponse,
    PluginStatus,
};
use fsocial_common::models::Quality;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MediaDownloaderPlugin {
    downloader: Arc<MediaDownloader>,
    url_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl MediaDownloaderPlugin {
    pub fn new(download_dir: impl Into<PathBuf>) -> Self {
        Self {
            downloader: Arc::new(MediaDownloader::new(download_dir)),
            url_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[allow(dead_code)]
    async fn store_url(&self, url: &str) -> String {
        let short_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let mut cache = self.url_cache.write().await;
        if cache.len() > 1000 {
            cache.clear();
        }
        cache.insert(short_id.clone(), url.to_string());
        short_id
    }

    async fn get_url(&self, short_id: &str) -> Option<String> {
        let cache = self.url_cache.read().await;
        cache.get(short_id).cloned()
    }
}

#[async_trait]
impl Plugin for MediaDownloaderPlugin {
    fn manifest(&self) -> PluginManifest {
        PluginManifest {
            id: "media-downloader".to_string(),
            name: "FSocial Media Downloader Engine".to_string(),
            version: "0.2.0".to_string(),
            description: "Downloads video and audio from YouTube, TikTok, Instagram, Spotify, SoundCloud, Pinterest".to_string(),
            capabilities: vec![PluginCapability::MediaDownload],
            author: "mrbezarate / FSocial Media Team".to_string(),
            is_external: false,
            endpoint_url: None,
        }
    }

    async fn handle_command(&self, cmd: &PluginCommand) -> Result<PluginResponse> {
        let clean_cmd = cmd.command.split('@').next().unwrap_or(&cmd.command);
        match clean_cmd {
            "dl" | "download" | "video" => {
                if cmd.args.is_empty() {
                    return Ok(PluginResponse::Text(
                        "📹 <b>FSocial Video Downloader</b>\n\n\
                        <b>Использование:</b> <code>/dl &lt;URL&gt;</code>\n\
                        Автоматически скачивает видео в наилучшем доступном качестве (YouTube, TikTok, Reels, VK, Pinterest)."
                            .to_string(),
                    ));
                }

                let url = &cmd.args[0];
                let (url_clean, platform, media_type) = MediaDownloader::detect_url(url).unwrap_or((url.clone(), fsocial_common::models::Platform::Unknown, fsocial_common::models::MediaType::Video));
                
                if platform == fsocial_common::models::Platform::Spotify 
                    || platform == fsocial_common::models::Platform::SoundCloud 
                    || media_type == fsocial_common::models::MediaType::Audio 
                {
                    return Ok(PluginResponse::Text(
                        "⚠️ <b>Это аудио-ресурс</b>\n\n\
                        Команда <code>/dl</code> предназначена только для видео и медиафайлов.\n\
                        Для скачивания музыки и плейлистов в MP3 используйте: <code>/mp3 <URL></code>"
                            .to_string(),
                    ));
                }

                match self.downloader.download_auto(&url_clean, false).await {
                    Ok((path, item)) => {
                        let caption = format!("📹 <b>{}</b>\n⚡ Скачано в максимальном качестве", crate::ui::html_escape(&item.title));
                        Ok(PluginResponse::Media {
                            title: item.title,
                            file_path: path.to_string_lossy().to_string(),
                            caption: Some(caption),
                        })
                    }
                    Err(e) => Ok(PluginResponse::Error(format!("Ошибка скачивания видео: {}", e))),
                }
            }
            "mp3" | "audio" | "music" => {
                if cmd.args.is_empty() {
                    return Ok(PluginResponse::Text(
                        "🎵 <b>FSocial Audio Downloader</b>\n\n\
                        <b>Использование:</b> <code>/mp3 &lt;URL&gt;</code>\n\
                        Автоматически скачивает аудио в MP3 (320 kbps) с обложкой альбома (Spotify, SoundCloud, YouTube, TikTok, Reels, VK)."
                            .to_string(),
                    ));
                }

                let url = &cmd.args[0];
                let (url_clean, _platform, _media_type) = MediaDownloader::detect_url(url).unwrap_or((url.clone(), fsocial_common::models::Platform::Unknown, fsocial_common::models::MediaType::Audio));

                match self.downloader.download_auto(&url_clean, true).await {
                    Ok((path, item)) => {
                        let author = item.uploader.as_deref().unwrap_or("Неизвестен");
                        let caption = format!("🎵 <b>{}</b>\n👤 <i>{}</i>\n💾 Сохранено в медиатеку Web Player", crate::ui::html_escape(&item.title), crate::ui::html_escape(author));
                        Ok(PluginResponse::Media {
                            title: item.title,
                            file_path: path.to_string_lossy().to_string(),
                            caption: Some(caption),
                        })
                    }
                    Err(e) => Ok(PluginResponse::Error(format!("Ошибка скачивания аудио: {}", e))),
                }
            }
            _ => Ok(PluginResponse::Ignored),
        }
    }

    async fn handle_message(&self, msg: &PluginMessage) -> Result<PluginResponse> {
        let urls = MediaDownloader::detect_all_urls(&msg.text);
        if urls.is_empty() {
            return Ok(PluginResponse::Ignored);
        }

        let mut all_responses = Vec::new();
        let mut errors = Vec::new();

        for (url, platform, media_type) in urls {
            let is_audio = platform == fsocial_common::models::Platform::Spotify
                || platform == fsocial_common::models::Platform::SoundCloud
                || media_type == fsocial_common::models::MediaType::Audio;

            let is_playlist = media_type == fsocial_common::models::MediaType::Playlist
                || url.contains("/playlist/")
                || url.contains("/album/")
                || url.contains("list=");

            // 1. Check for Playlists
            if is_playlist {
                match self.downloader.download_playlist(&url, is_audio).await {
                    Ok(items) => {
                        let total = items.len();
                        for (idx, (path, item)) in items.into_iter().enumerate() {
                            let caption = if is_audio {
                                let author = item.uploader.as_deref().unwrap_or("Неизвестен");
                                format!("🎵 <b>[{}/{}] {}</b>\n👤 <i>{}</i>\n💾 Сохранено в медиатеку Web Player", idx + 1, total, crate::ui::html_escape(&item.title), crate::ui::html_escape(author))
                            } else {
                                format!("📹 <b>[{}/{}] {}</b>\n⚡ Скачано в максимальном качестве", idx + 1, total, crate::ui::html_escape(&item.title))
                            };
                            all_responses.push(PluginResponse::Media {
                                title: item.title,
                                file_path: path.to_string_lossy().to_string(),
                                caption: Some(caption),
                            });
                        }
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!("Ошибка авто-загрузки плейлиста {}: {}", url, e);
                    }
                }
            }

            // 2. Check for TikTok / Instagram photo slideshows
            if platform == fsocial_common::models::Platform::TikTok || platform == fsocial_common::models::Platform::Instagram {
                if let Ok(paths) = self.downloader.download_images(&url).await {
                    if paths.len() >= 2 {
                        let paths_str = paths.into_iter().map(|p| p.to_string_lossy().to_string()).collect();
                        let caption = format!("📸 <b>Фото-галерея</b>\n⚡ Скачано в максимальном качестве");
                        all_responses.push(PluginResponse::MediaGroup {
                            title: "Slideshow".to_string(),
                            file_paths: paths_str,
                            caption: Some(caption),
                        });
                        continue;
                    }
                }
            }

            // 3. Regular single item download
            match self.downloader.download_auto(&url, is_audio).await {
                Ok((path, item)) => {
                    let caption = if is_audio {
                        let author = item.uploader.as_deref().unwrap_or("Неизвестен");
                        format!("🎵 <b>{}</b>\n👤 <i>{}</i>\n💾 Сохранено в медиатеку Web Player", crate::ui::html_escape(&item.title), crate::ui::html_escape(author))
                    } else {
                        format!("📹 <b>{}</b>\n⚡ Скачано в максимальном качестве", crate::ui::html_escape(&item.title))
                    };
                    all_responses.push(PluginResponse::Media {
                        title: item.title,
                        file_path: path.to_string_lossy().to_string(),
                        caption: Some(caption),
                    });
                }
                Err(e) => {
                    tracing::warn!("Ошибка авто-загрузки {}: {}", url, e);
                    errors.push(format!("❌ {}: {}", url, e));
                }
            }
        }

        if all_responses.is_empty() && !errors.is_empty() {
            return Ok(PluginResponse::Error(errors.join("\n")));
        }
        
        if all_responses.len() == 1 {
            Ok(all_responses.into_iter().next().unwrap())
        } else if all_responses.is_empty() {
            Ok(PluginResponse::Ignored)
        } else {
            Ok(PluginResponse::Batch(all_responses))
        }
    }

    async fn handle_callback(&self, callback_data: &str, _user_id: u64) -> Result<PluginResponse> {
        if let Some(rest) = callback_data.strip_prefix("mldl:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() == 2 {
                let q_id = parts[0];
                let key_or_url = parts[1];

                let url = match self.get_url(key_or_url).await {
                    Some(u) => u,
                    None => key_or_url.to_string(),
                };

                let quality = Quality::from_callback(q_id).unwrap_or(Quality::Best);

                match self.downloader.download(&url, quality.clone()).await {
                    Ok(path) => {
                        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("media");
                        Ok(PluginResponse::Media {
                            title: filename.to_string(),
                            file_path: path.to_string_lossy().to_string(),
                            caption: None,
                        })
                    }
                    Err(e) => Ok(PluginResponse::Error(format!(
                        "Download failed: {}\n\n(Tip: Ensure `yt-dlp` is installed on server)",
                        e
                    ))),
                }
            } else {
                Ok(PluginResponse::Ignored)
            }
        } else {
            Ok(PluginResponse::Ignored)
        }
    }

    async fn status(&self) -> PluginStatus {
        PluginStatus::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dl_usage_no_args() {
        let plugin = MediaDownloaderPlugin::new("/tmp/test_downloads");
        let cmd = PluginCommand {
            command: "dl".to_string(),
            args: vec![],
            user_id: 12345,
            chat_id: 12345,
        };
        let res = plugin.handle_command(&cmd).await.unwrap();
        match res {
            PluginResponse::Text(t) => assert!(t.contains("FSocial Video Downloader")),
            _ => panic!("Expected Text response"),
        }
    }

    #[tokio::test]
    async fn test_mp3_usage_no_args() {
        let plugin = MediaDownloaderPlugin::new("/tmp/test_downloads");
        let cmd = PluginCommand {
            command: "mp3".to_string(),
            args: vec![],
            user_id: 12345,
            chat_id: 12345,
        };
        let res = plugin.handle_command(&cmd).await.unwrap();
        match res {
            PluginResponse::Text(t) => assert!(t.contains("FSocial Audio Downloader")),
            _ => panic!("Expected Text response"),
        }
    }

    #[tokio::test]
    #[ignore = "requires external network and live yt-dlp credentials"]
    async fn test_spotify_url_handling() {
        let plugin = MediaDownloaderPlugin::new("/tmp/test_downloads");
        let spotify_url = "https://open.spotify.com/playlist/37i9dQZF1E8LWhdHrhPehc";
        let cmd = PluginCommand {
            command: "mp3".to_string(),
            args: vec![spotify_url.to_string()],
            user_id: 12345,
            chat_id: 12345,
        };
        let res = plugin.handle_command(&cmd).await.unwrap();
        match res {
            PluginResponse::Media { file_path, .. } => {
                assert!(std::path::Path::new(&file_path).exists() || file_path.contains(".mp3"));
            }
            PluginResponse::Text(txt) => {
                assert!(txt.contains("Скачивание") || txt.contains("Spotify") || txt.contains("плейлист"));
            }
            _ => panic!("Expected Media or Text response, got {:?}", res),
        }
    }
}
