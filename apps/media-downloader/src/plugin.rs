use crate::downloader::MediaDownloader;
use async_trait::async_trait;
use brain_common::Result;
use brain_plugin::{
    Plugin, PluginCapability, PluginCommand, PluginManifest, PluginMessage, PluginResponse,
    PluginStatus,
};
use fsocial_common::models::Quality;
use std::path::PathBuf;
use std::sync::Arc;

pub struct MediaDownloaderPlugin {
    downloader: Arc<MediaDownloader>,
}

impl MediaDownloaderPlugin {
    pub fn new(download_dir: impl Into<PathBuf>) -> Self {
        Self {
            downloader: Arc::new(MediaDownloader::new(download_dir)),
        }
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
        match cmd.command.as_str() {
            "dl" | "download" | "video" | "mp3" => {
                if cmd.args.is_empty() {
                    return Ok(PluginResponse::Text(
                        "📥 *FSocial Media Downloader*\n\nUsage:\n`/dl <URL>` — Download video (quality selector)\n`/mp3 <URL>` — Download MP3 audio\n\nSupported: YouTube, TikTok, Instagram, Spotify, SoundCloud, Pinterest".to_string(),
                    ));
                }

                let url = &cmd.args[0];
                let is_mp3 = cmd.command == "mp3";

                let meta = self.downloader.fetch_metadata(url).await.unwrap_or_else(|_| {
                    crate::downloader::MediaMetadata {
                        url: url.to_string(),
                        title: "Media File".to_string(),
                        platform: fsocial_common::models::Platform::Unknown,
                        media_type: fsocial_common::models::MediaType::Video,
                        duration_secs: None,
                        uploader: None,
                    }
                });

                let mut options = Vec::new();
                if is_mp3 {
                    for q in Quality::audio_options() {
                        options.push((q.display_name().to_string(), format!("mldl:{}:{}", q.callback_id(), url)));
                    }
                } else {
                    options.push((Quality::Best.display_name().to_string(), format!("mldl:{}:{}", Quality::Best.callback_id(), url)));
                    options.push((Quality::Video1080p.display_name().to_string(), format!("mldl:{}:{}", Quality::Video1080p.callback_id(), url)));
                    options.push((Quality::Video720p.display_name().to_string(), format!("mldl:{}:{}", Quality::Video720p.callback_id(), url)));
                    options.push((Quality::AudioBest.display_name().to_string(), format!("mldl:{}:{}", Quality::AudioBest.callback_id(), url)));
                }

                let text = format!(
                    "🎥 *FSocial Engine Media Detected*\n\n📌 *Title:* {}\n🌐 *Platform:* {}\n\nSelect quality to start download:",
                    meta.title, meta.platform
                );

                Ok(PluginResponse::Keyboard { text, options })
            }
            _ => Ok(PluginResponse::Ignored),
        }
    }

    async fn handle_message(&self, msg: &PluginMessage) -> Result<PluginResponse> {
        if let Some((url, platform, _media_type)) = MediaDownloader::detect_url(&msg.text) {
            let text = format!(
                "🔗 *{} Media Link Detected*\n\nSelect quality to download:",
                platform
            );
            let options = vec![
                ("⭐ Лучшее качество".to_string(), format!("mldl:q_best:{}", url)),
                ("📹 1080p Full HD".to_string(), format!("mldl:q_v1080:{}", url)),
                ("📹 720p HD".to_string(), format!("mldl:q_v720:{}", url)),
                ("🎵 MP3 Лучшее аудио".to_string(), format!("mldl:q_abest:{}", url)),
            ];
            return Ok(PluginResponse::Keyboard { text, options });
        }

        Ok(PluginResponse::Ignored)
    }

    async fn handle_callback(&self, callback_data: &str, _user_id: u64) -> Result<PluginResponse> {
        if let Some(rest) = callback_data.strip_prefix("mldl:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() == 2 {
                let q_id = parts[0];
                let url = parts[1];

                let quality = Quality::from_callback(q_id).unwrap_or(Quality::Best);

                match self.downloader.download(url, quality.clone()).await {
                    Ok(path) => {
                        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("media");
                        Ok(PluginResponse::Media {
                            title: filename.to_string(),
                            file_path: path.to_string_lossy().to_string(),
                            caption: Some(format!("Downloaded via FSocial Engine ({})", quality.display_name())),
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
