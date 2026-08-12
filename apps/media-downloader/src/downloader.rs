use fsocial_common::models::{MediaType, Platform, Quality};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command as StdCommand;
use std::sync::LazyLock;
use tracing::{info, warn};

static URL_PATTERNS: LazyLock<Vec<(Regex, Platform, MediaType)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"https?://(?:www\.)?(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/shorts/|youtube\.com/live/|youtube\.com/playlist\?list=)[^\s]+").unwrap(), Platform::YouTube, MediaType::Video),
        (Regex::new(r"https?://(?:www\.)?(?:tiktok\.com/@[^/]+/video/|tiktok\.com/t/|vm\.tiktok\.com/|vt\.tiktok\.com/|v\.tiktok\.com/)[^\s]+").unwrap(), Platform::TikTok, MediaType::Video),
        (Regex::new(r"https?://(?:www\.)?instagram\.com/(?:reel|p)/[^\s]+").unwrap(), Platform::Instagram, MediaType::Video),
        (Regex::new(r"https?://(?:open\.)?spotify\.com/track/[^\s]+").unwrap(), Platform::Spotify, MediaType::Audio),
        (Regex::new(r"https?://(?:open\.)?spotify\.com/(?:album|playlist)/[^\s]+").unwrap(), Platform::Spotify, MediaType::Playlist),
        (Regex::new(r"https?://(?:www\.)?soundcloud\.com/[^\s]+").unwrap(), Platform::SoundCloud, MediaType::Audio),
        (Regex::new(r"https?://(?:www\.|pin\.)?(?:pinterest\.com/(?:pin|video)/|pin\.it/)[^\s]+").unwrap(), Platform::Pinterest, MediaType::Video),
    ]
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    pub url: String,
    pub title: String,
    pub platform: Platform,
    pub media_type: MediaType,
    pub duration_secs: Option<u64>,
    pub uploader: Option<String>,
}

pub struct MediaDownloader {
    download_dir: PathBuf,
}

impl MediaDownloader {
    pub fn new(download_dir: impl Into<PathBuf>) -> Self {
        let dir = download_dir.into();
        let _ = std::fs::create_dir_all(&dir);
        Self { download_dir: dir }
    }

    pub fn detect_url(text: &str) -> Option<(String, Platform, MediaType)> {
        for (regex, platform, media_type) in URL_PATTERNS.iter() {
            if let Some(m) = regex.find(text) {
                return Some((m.as_str().to_string(), platform.clone(), media_type.clone()));
            }
        }
        None
    }

    pub async fn fetch_metadata(&self, url: &str) -> anyhow::Result<MediaMetadata> {
        let (url_str, platform, media_type) = Self::detect_url(url).unwrap_or_else(|| {
            (url.to_string(), Platform::Unknown, MediaType::Video)
        });

        // Try yt-dlp metadata extraction
        let output = StdCommand::new("yt-dlp")
            .args(["--dump-json", "--no-warnings", &url_str])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    let title = json
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Untitled Media")
                        .to_string();
                    let uploader = json.get("uploader").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let duration_secs = json.get("duration").and_then(|v| v.as_u64());

                    return Ok(MediaMetadata {
                        url: url_str,
                        title,
                        platform,
                        media_type,
                        duration_secs,
                        uploader,
                    });
                }
            }
        }

        // Fallback metadata
        Ok(MediaMetadata {
            url: url_str,
            title: format!("{} Media ({})", platform, uuid::Uuid::new_v4().to_string()[..8].to_string()),
            platform,
            media_type,
            duration_secs: None,
            uploader: None,
        })
    }

    pub async fn download(&self, url: &str, quality: Quality) -> anyhow::Result<PathBuf> {
        let file_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let ext = if quality.is_audio() { "mp3" } else { "mp4" };
        
        let output_template = self.download_dir.join(format!("{}.%(ext)s", file_id));
        let expected_file = self.download_dir.join(format!("{}.{}", file_id, ext));

        info!("FSocial Downloader: starting download for {} with quality {}...", url, quality.display_name());

        let mut cmd = StdCommand::new("yt-dlp");
        cmd.arg("-o").arg(&output_template);

        if quality.is_audio() {
            cmd.args(["-x", "--audio-format", "mp3"]);
        } else {
            cmd.args(["-f", quality.ytdlp_format()]);
        }

        cmd.arg(url);

        let status = cmd.status();

        match status {
            Ok(s) if s.success() => {
                if expected_file.exists() {
                    Ok(expected_file)
                } else {
                    let entries = std::fs::read_dir(&self.download_dir)?;
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.file_name().and_then(|n| n.to_str()).map_or(false, |name| name.starts_with(&file_id)) {
                            return Ok(path);
                        }
                    }
                    Err(anyhow::anyhow!("Downloaded file not found on disk"))
                }
            }
            Ok(s) => {
                warn!("yt-dlp process failed with exit code: {:?}", s.code());
                Err(anyhow::anyhow!("Download failed with exit code {:?}", s.code()))
            }
            Err(e) => {
                warn!("yt-dlp command failed to execute: {}. (Ensure yt-dlp is installed on system)", e);
                Err(anyhow::anyhow!("yt-dlp executable not found. Please install yt-dlp to enable video downloads."))
            }
        }
    }
}
