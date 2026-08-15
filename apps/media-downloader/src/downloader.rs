use fsocial_common::models::{MediaType, Platform, Quality};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio::process::Command as TokioCommand;
use tracing::{info, warn};

static URL_PATTERNS: LazyLock<Vec<(Regex, Platform, MediaType)>> = LazyLock::new(|| {
    vec![
        (Regex::new(r"https?://(?:www\.)?(?:youtube\.com/watch\?v=|youtu\.be/|youtube\.com/shorts/|youtube\.com/live/|youtube\.com/playlist\?list=)[^\s]+").unwrap(), Platform::YouTube, MediaType::Video),
        (Regex::new(r"https?://(?:www\.)?(?:tiktok\.com/@[^/]+/video/|tiktok\.com/t/|vm\.tiktok\.com/|vt\.tiktok\.com/|v\.tiktok\.com/)[^\s]+").unwrap(), Platform::TikTok, MediaType::Video),
        (Regex::new(r"https?://(?:www\.)?instagram\.com/(?:reel|p|reels)/[^\s]+").unwrap(), Platform::Instagram, MediaType::Video),
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
    pub thumbnail: Option<String>,
    pub is_playlist: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    pub title: String,
    pub uploader: Option<String>,
    pub media_type: String, // "audio" | "video"
    pub file_name: String,
    pub cover_file: Option<String>,
    pub duration_secs: Option<u64>,
    pub source_url: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct SpotifyTrackMeta {
    pub title: String,
    pub artist: String,
    pub url: String,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SpotifyEntity {
    pub title: String,
    pub uploader: Option<String>,
    pub cover_url: Option<String>,
    pub is_playlist: bool,
    pub tracks: Vec<SpotifyTrackMeta>,
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
        // Fallback: any http/https URL
        if let Some(start) = text.find("http://").or_else(|| text.find("https://")) {
            let url_part = text[start..].split_whitespace().next().unwrap_or("");
            if !url_part.is_empty() {
                return Some((url_part.to_string(), Platform::Unknown, MediaType::Video));
            }
        }
        None
    }

    pub fn detect_all_urls(text: &str) -> Vec<(String, Platform, MediaType)> {
        let mut results = Vec::new();
        for (regex, platform, media_type) in URL_PATTERNS.iter() {
            for m in regex.find_iter(text) {
                let url = m.as_str().to_string();
                if !results.iter().any(|(u, _, _): &(String, Platform, MediaType)| u == &url) {
                    results.push((url, platform.clone(), media_type.clone()));
                }
            }
        }
        // If no known platform matched, try generic URL fallback (single only)
        if results.is_empty() {
            if let Some(start) = text.find("http://").or_else(|| text.find("https://")) {
                let url_part = text[start..].split_whitespace().next().unwrap_or("");
                if !url_part.is_empty() {
                    results.push((url_part.to_string(), Platform::Unknown, MediaType::Video));
                }
            }
        }
        results
    }

    pub async fn fetch_info_response(&self, url: &str) -> anyhow::Result<fsocial_common::models::InfoResponse> {
        let (url_str, platform, _media_type) = Self::detect_url(url).unwrap_or_else(|| {
            (url.to_string(), Platform::Unknown, MediaType::Video)
        });

        // 1. Spotify specialized parsing
        if platform == Platform::Spotify || url_str.contains("spotify.com") {
            let client = reqwest::Client::builder()
                .user_agent("facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)")
                .timeout(std::time::Duration::from_secs(8))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());

            let mut title = "Spotify Media".to_string();
            let uploader = Some("Spotify".to_string());
            let mut thumbnail = None;
            let is_playlist = url_str.contains("/playlist/") || url_str.contains("/album/");
            let mut playlist_count = None;

            if let Ok(resp) = client.get(&url_str).send().await {
                if let Ok(html) = resp.text().await {
                    let re_title = Regex::new(r#"<meta property="og:title" content="([^"]+)""#).unwrap();
                    if let Some(c) = re_title.captures(&html) {
                        title = c[1].replace("&amp;", "&").replace("&#39;", "'").replace("&quot;", "\"");
                    }

                    let re_desc = Regex::new(r#"<meta property="og:description" content="([^"]+)""#).unwrap();
                    if let Some(c) = re_desc.captures(&html) {
                        let desc = c[1].to_string();
                        // Parse count e.g. "50 items"
                        let re_count = Regex::new(r"(\d+)\s+items?").unwrap();
                        if let Some(caps) = re_count.captures(&desc) {
                            if let Ok(cnt) = caps[1].parse::<u32>() {
                                playlist_count = Some(cnt);
                            }
                        }
                    }

                    let re_img = Regex::new(r#"<meta property="og:image" content="([^"]+)""#).unwrap();
                    if let Some(c) = re_img.captures(&html) {
                        thumbnail = Some(c[1].to_string());
                    }
                }
            }

            let available_qualities = vec![
                fsocial_common::models::QualityOption {
                    quality: Quality::AudioBest,
                    filesize_bytes: Some(8 * 1024 * 1024),
                    estimated_secs: Some(2),
                    speed_category: "⚡".to_string(),
                    display_label: "🎵 MP3 (320k)".to_string(),
                    full_button_label: "🎵 MP3 (320k)".to_string(),
                },
                fsocial_common::models::QualityOption {
                    quality: Quality::Audio256,
                    filesize_bytes: Some(6 * 1024 * 1024),
                    estimated_secs: Some(1),
                    speed_category: "⚡".to_string(),
                    display_label: "🎵 256k".to_string(),
                    full_button_label: "🎵 256k".to_string(),
                },
                fsocial_common::models::QualityOption {
                    quality: Quality::Audio128,
                    filesize_bytes: Some(3 * 1024 * 1024),
                    estimated_secs: Some(1),
                    speed_category: "⚡".to_string(),
                    display_label: "🎵 128k".to_string(),
                    full_button_label: "🎵 128k".to_string(),
                },
            ];

            return Ok(fsocial_common::models::InfoResponse {
                title,
                uploader,
                thumbnail,
                duration_secs: None,
                available_qualities,
                is_playlist,
                playlist_count,
                playlist_urls: vec![],
                error: None,
            });
        }

        // 2. yt-dlp deep analysis for other platforms
        let output = TokioCommand::new("yt-dlp")
            .args([
                "--dump-json",
                "--no-warnings",
                "--flat-playlist",
                "--extractor-args",
                "youtube:player_client=ios,android,web",
                &url_str,
            ])
            .output()
            .await;

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut title = String::new();
                let mut uploader = None;
                let mut thumbnail = None;
                let mut duration_secs = None;
                let mut is_playlist = false;
                let mut playlist_count = None;
                let mut playlist_urls = Vec::new();
                let mut raw_formats = None;
                let mut available_qualities = Vec::new();

                for line in stdout.lines() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                        if duration_secs.is_none() {
                            duration_secs = json.get("duration").and_then(|v| v.as_f64()).map(|d| d as u64);
                        }
                        if uploader.is_none() {
                            uploader = json.get("uploader")
                                .or_else(|| json.get("channel"))
                                .or_else(|| json.get("artist"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                        }
                        if thumbnail.is_none() {
                            thumbnail = json.get("thumbnail").and_then(|v| v.as_str()).map(|s| s.to_string());
                        }

                        if let Some(t) = json.get("_type").and_then(|v| v.as_str()) {
                            if t == "playlist" || t == "multi_video" {
                                is_playlist = true;
                                if let Some(entries) = json.get("entries").and_then(|v| v.as_array()) {
                                    playlist_count = Some(entries.len() as u32);
                                    for entry in entries {
                                        if let Some(u) = entry.get("url").and_then(|v| v.as_str()) {
                                            playlist_urls.push(u.to_string());
                                        }
                                    }
                                }
                                if title.is_empty() {
                                    title = json.get("title").and_then(|v| v.as_str()).unwrap_or("Playlist").to_string();
                                }
                                continue;
                            }
                        }

                        if title.is_empty() {
                            title = json.get("title").and_then(|v| v.as_str()).unwrap_or("Media").to_string();
                        }

                        if let Some(formats) = json.get("formats").and_then(|v| v.as_array()) {
                            raw_formats = Some(formats.clone());
                            let mut has_audio = false;
                            let mut heights = std::collections::HashSet::new();
                            for f in formats {
                                let acodec = f.get("acodec").and_then(|v| v.as_str()).unwrap_or("none");
                                let vcodec = f.get("vcodec").and_then(|v| v.as_str()).unwrap_or("none");
                                if acodec != "none" {
                                    has_audio = true;
                                }
                                if vcodec != "none" {
                                    if let Some(h) = f.get("height").and_then(|v| v.as_i64()) {
                                        heights.insert(h);
                                    }
                                }
                            }

                            if has_audio {
                                available_qualities.push(Quality::AudioBest);
                            }

                            if !heights.is_empty() {
                                if heights.iter().any(|&h| (240..400).contains(&h)) {
                                    available_qualities.push(Quality::Video360p);
                                }
                                if heights.iter().any(|&h| (400..550).contains(&h)) {
                                    available_qualities.push(Quality::Video480p);
                                }
                                if heights.iter().any(|&h| (700..850).contains(&h)) {
                                    available_qualities.push(Quality::Video720p);
                                }
                                if heights.iter().any(|&h| (1000..1400).contains(&h)) {
                                    available_qualities.push(Quality::Video1080p);
                                }
                                if heights.iter().any(|&h| (1400..2000).contains(&h)) {
                                    available_qualities.push(Quality::Video1440p);
                                }
                                if heights.iter().any(|&h| h >= 2000) {
                                    available_qualities.push(Quality::Video4K);
                                }
                                available_qualities.push(Quality::Best);
                            }
                        }
                    }
                }

                if available_qualities.is_empty() {
                    available_qualities = Quality::audio_options();
                } else {
                    available_qualities.dedup();
                }

                let quality_options: Vec<fsocial_common::models::QualityOption> = available_qualities
                    .into_iter()
                    .map(|q| {
                        let mut sz_bytes: Option<u64> = None;
                        if let Some(ref fmts) = raw_formats {
                            let target_height = match q {
                                Quality::Video360p => Some(360),
                                Quality::Video480p => Some(480),
                                Quality::Video720p => Some(720),
                                Quality::Video1080p => Some(1080),
                                Quality::Video1440p => Some(1440),
                                Quality::Video4K => Some(2160),
                                _ => None,
                            };
                            if let Some(th) = target_height {
                                let mut max_bytes = 0;
                                for f in fmts {
                                    if f.get("height").and_then(|v| v.as_i64()) == Some(th) {
                                        if let Some(b) = f.get("filesize").and_then(|v| v.as_u64()).or_else(|| f.get("filesize_approx").and_then(|v| v.as_u64())) {
                                            max_bytes = max_bytes.max(b);
                                        }
                                    }
                                }
                                if max_bytes > 0 {
                                    sz_bytes = Some(max_bytes);
                                }
                            }
                        }

                        if sz_bytes.is_none() {
                            if let Some(d) = duration_secs {
                                let rate_mb_per_sec = match q {
                                    Quality::Video4K => 4.0,
                                    Quality::Video1440p => 2.5,
                                    Quality::Video1080p => 1.5,
                                    Quality::Video720p => 0.8,
                                    Quality::Video480p => 0.4,
                                    Quality::Video360p => 0.2,
                                    Quality::Best => 2.0,
                                    _ => 0.03,
                                };
                                sz_bytes = Some((d as f64 * rate_mb_per_sec * 1024.0 * 1024.0) as u64);
                            }
                        }

                        let mb = sz_bytes.map(|b| b / (1024 * 1024)).unwrap_or(0);
                        let estimated_secs = sz_bytes.map(|b| (b / (10 * 1024 * 1024)).max(1));
                        let speed_category = match mb {
                            0..=15 => "⚡".to_string(),
                            16..=50 => "🚀".to_string(),
                            51..=150 => "⚖️".to_string(),
                            _ => "🐢".to_string(),
                        };

                        let display_label = if mb > 0 {
                            format!("{} (~{}MB)", q.display_name(), mb)
                        } else {
                            q.display_name().to_string()
                        };

                        fsocial_common::models::QualityOption {
                            quality: q,
                            filesize_bytes: sz_bytes,
                            estimated_secs,
                            speed_category,
                            display_label: display_label.clone(),
                            full_button_label: display_label,
                        }
                    })
                    .collect();

                return Ok(fsocial_common::models::InfoResponse {
                    title: if title.is_empty() { "Media".to_string() } else { title },
                    uploader,
                    thumbnail,
                    duration_secs,
                    available_qualities: quality_options,
                    is_playlist,
                    playlist_count,
                    playlist_urls,
                    error: None,
                });
            }
        }

        // Fallback info response
        Ok(fsocial_common::models::InfoResponse {
            title: format!("{} Media", platform),
            uploader: None,
            thumbnail: None,
            duration_secs: None,
            available_qualities: vec![
                fsocial_common::models::QualityOption {
                    quality: Quality::Best,
                    filesize_bytes: None,
                    estimated_secs: None,
                    speed_category: "🚀".to_string(),
                    display_label: "⭐ Лучшее качество".to_string(),
                    full_button_label: "⭐ Лучшее качество".to_string(),
                },
                fsocial_common::models::QualityOption {
                    quality: Quality::AudioBest,
                    filesize_bytes: None,
                    estimated_secs: None,
                    speed_category: "⚡".to_string(),
                    display_label: "🎵 MP3".to_string(),
                    full_button_label: "🎵 MP3".to_string(),
                },
            ],
            is_playlist: false,
            playlist_count: None,
            playlist_urls: vec![],
            error: None,
        })
    }

    pub fn download_dir(&self) -> &PathBuf {
        &self.download_dir
    }

    pub async fn get_library(&self) -> Vec<MediaItem> {
        let path = self.download_dir.join("media_library.json");
        if let Ok(data) = tokio::fs::read_to_string(&path).await {
            if let Ok(items) = serde_json::from_str::<Vec<MediaItem>>(&data) {
                return items;
            }
        }
        Vec::new()
    }

    pub async fn save_media_item(&self, item: MediaItem) {
        let mut lib = self.get_library().await;
        // Prepend and dedup by id
        lib.retain(|i| i.id != item.id);
        lib.insert(0, item);
        let path = self.download_dir.join("media_library.json");
        if let Ok(json) = serde_json::to_string_pretty(&lib) {
            let _ = tokio::fs::write(&path, json).await;
        }
    }

    pub async fn delete_media_item(&self, id: &str) -> bool {
        let mut lib = self.get_library().await;
        let original_len = lib.len();
        if let Some(item) = lib.iter().find(|i| i.id == id).cloned() {
            let file_path = self.download_dir.join(&item.file_name);
            let _ = tokio::fs::remove_file(file_path).await;
            if let Some(ref cover) = item.cover_file {
                let cover_path = self.download_dir.join(cover);
                let _ = tokio::fs::remove_file(cover_path).await;
            }
        }
        lib.retain(|i| i.id != id);
        if lib.len() != original_len {
            let path = self.download_dir.join("media_library.json");
            if let Ok(json) = serde_json::to_string_pretty(&lib) {
                let _ = tokio::fs::write(&path, json).await;
            }
            true
        } else {
            false
        }
    }

    pub fn ytdlp_cmd() -> TokioCommand {
        if std::path::Path::new("./bin/yt-dlp").exists() {
            TokioCommand::new("./bin/yt-dlp")
        } else if std::path::Path::new("/home/mrbezarate/bin/yt-dlp").exists() {
            TokioCommand::new("/home/mrbezarate/bin/yt-dlp")
        } else {
            TokioCommand::new("yt-dlp")
        }
    }

    pub async fn scrape_spotify_og_image(url: &str) -> Option<String> {
        let client = reqwest::Client::builder()
            .user_agent("facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)")
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();

        if let Ok(resp) = client.get(url).send().await {
            if let Ok(html) = resp.text().await {
                let re_img = Regex::new(r#"<meta property="og:image" content="([^"]+)""#).unwrap();
                if let Some(c) = re_img.captures(&html) {
                    return Some(c[1].to_string());
                }
            }
        }
        None
    }

    pub async fn extract_spotify_entity(url: &str) -> Option<SpotifyEntity> {
        let re = Regex::new(r"open\.spotify\.com/(track|playlist|album)/([a-zA-Z0-9]+)").unwrap();
        let caps = re.captures(url)?;
        let entity_type = &caps[1];
        let entity_id = &caps[2];

        let embed_url = format!("https://open.spotify.com/embed/{}/{}", entity_type, entity_id);
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let resp = client.get(&embed_url).send().await.ok()?;
        let html = resp.text().await.ok()?;

        let re_json = Regex::new(r#"(?s)<script[^>]*id="__NEXT_DATA__"[^>]*>(.*?)</script>"#).unwrap();
        let json_match = re_json.captures(&html)?;
        let json_data: serde_json::Value = serde_json::from_str(&json_match[1]).ok()?;

        let entity = json_data
            .get("props")?
            .get("pageProps")?
            .get("state")?
            .get("data")?
            .get("entity")?;

        let title = entity.get("title")
            .or_else(|| entity.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("Spotify Audio")
            .to_string();

        let mut tracks = Vec::new();
        let mut uploader = None;
        let mut cover_url = None;

        if let Some(cover_sources) = entity.get("coverArt").and_then(|c| c.get("sources")).and_then(|s| s.as_array()) {
            if let Some(first_source) = cover_sources.first() {
                cover_url = first_source.get("url").and_then(|u| u.as_str()).map(|s| s.to_string());
            }
        }

        if cover_url.is_none() {
            cover_url = Self::scrape_spotify_og_image(url).await;
        }

        if entity_type == "track" {
            let mut artist_name = entity.get("subtitle").and_then(|s| s.as_str()).unwrap_or("").to_string();
            if artist_name.is_empty() {
                if let Some(artists) = entity.get("artists").and_then(|a| a.as_array()) {
                    let names: Vec<&str> = artists.iter().filter_map(|a| a.get("name").and_then(|n| n.as_str())).collect();
                    artist_name = names.join(", ");
                }
            }
            if !artist_name.is_empty() {
                uploader = Some(artist_name.clone());
            }
            tracks.push(SpotifyTrackMeta {
                title: title.clone(),
                artist: artist_name,
                url: format!("https://open.spotify.com/track/{}", entity_id),
                cover_url: cover_url.clone(),
            });
        } else {
            // Playlist or Album
            let default_cover = if entity_type == "album" { cover_url.clone() } else { None };
            if let Some(tracklist) = entity.get("trackList").and_then(|tl| tl.as_array()) {
                for item in tracklist {
                    let t_title = item.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string();
                    let t_artist = item.get("subtitle").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    let t_uri = item.get("uri").or_else(|| item.get("id")).and_then(|u| u.as_str()).unwrap_or("");
                    let t_id = t_uri.replace("spotify:track:", "");
                    if !t_title.is_empty() {
                        tracks.push(SpotifyTrackMeta {
                            title: t_title,
                            artist: t_artist,
                            url: format!("https://open.spotify.com/track/{}", t_id),
                            cover_url: default_cover.clone(),
                        });
                    }
                }
            }
        }

        Some(SpotifyEntity {
            title,
            uploader,
            cover_url,
            is_playlist: entity_type != "track",
            tracks,
        })
    }

    pub async fn parse_spotify_track_meta(url: &str) -> (String, Option<String>, Option<String>) {
        if let Some(entity) = Self::extract_spotify_entity(url).await {
            if let Some(first) = entity.tracks.first() {
                let artist = if !first.artist.is_empty() { Some(first.artist.clone()) } else { entity.uploader };
                let cover = first.cover_url.clone().or(entity.cover_url);
                return (first.title.clone(), artist, cover);
            }
        }

        let cover_url = Self::scrape_spotify_og_image(url).await;
        let client = reqwest::Client::builder()
            .user_agent("facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)")
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_default();

        let mut title = "Spotify Track".to_string();
        let mut artist = None;

        if let Ok(resp) = client.get(url).send().await {
            if let Ok(html) = resp.text().await {
                let re_title = Regex::new(r#"<meta property="og:title" content="([^"]+)""#).unwrap();
                let re_desc = Regex::new(r#"<meta property="og:description" content="([^"]+)""#).unwrap();

                if let Some(c) = re_title.captures(&html) {
                    title = c[1].replace("&amp;", "&").replace("&#39;", "'").replace("&quot;", "\"");
                }
                if let Some(c) = re_desc.captures(&html) {
                    let desc = c[1].to_string();
                    let parts: Vec<&str> = desc.split(" · ").collect();
                    if let Some(a) = parts.first() {
                        let clean_artist = a.trim();
                        if !clean_artist.is_empty() && clean_artist != "Song" && clean_artist != "Single" {
                            artist = Some(clean_artist.to_string());
                        }
                    }
                }
            }
        }
        (title, artist, cover_url)
    }

    pub async fn search_spotify_track(artist: &str, title: &str) -> Option<String> {
        let queries = if !artist.is_empty() {
            vec![
                format!("ytmsearch1:\"{}\" \"{}\" audio", artist, title),
                format!("ytsearch1:\"{}\" - \"{}\"", artist, title),
                format!("ytsearch1:{} {} audio", artist, title),
                format!("ytsearch1:{} {}", title, artist),
            ]
        } else {
            vec![
                format!("ytmsearch1:\"{}\" audio", title),
                format!("ytsearch1:{} audio", title),
                format!("ytsearch1:{}", title),
            ]
        };

        for query in queries {
            let output = Self::ytdlp_cmd()
                .args(["--dump-json", "--no-download", "--no-warnings", &query])
                .output()
                .await;

            if let Ok(out) = output {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if let Some(first_line) = stdout.lines().next() {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(first_line) {
                            if let Some(url) = json.get("webpage_url").or_else(|| json.get("url")).and_then(|v| v.as_str()) {
                                return Some(url.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    pub async fn download_spotify_track_direct(
        &self,
        title: &str,
        artist: &str,
        cover_url: Option<&str>,
        source_url: &str,
    ) -> anyhow::Result<(PathBuf, MediaItem)> {
        let file_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let file_name = format!("{}.mp3", file_id);
        let output_template = self.download_dir.join(format!("{}.%(ext)s", file_id)).to_string_lossy().to_string();
        let expected_file = self.download_dir.join(&file_name);

        let cover_name = format!("{}.jpg", file_id);
        let cover_target = self.download_dir.join(&cover_name);
        let mut cover_bytes: Option<Vec<u8>> = None;

        let cover_url_str = if let Some(c) = cover_url {
            Some(c.to_string())
        } else if source_url.contains("spotify.com/track/") {
            Self::scrape_spotify_og_image(source_url).await
        } else {
            None
        };

        if let Some(ref cover) = cover_url_str {
            if let Ok(resp) = reqwest::get(cover).await {
                if let Ok(bytes) = resp.bytes().await {
                    cover_bytes = Some(bytes.to_vec());
                }
            }
        }

        let download_target = if let Some(found_url) = Self::search_spotify_track(artist, title).await {
            info!("Found exact YouTube match for {} - {}: {}", artist, title, found_url);
            found_url
        } else {
            format!("ytsearch1:{} {} audio", artist, title)
        };

        let mut cmd = Self::ytdlp_cmd();
        cmd.arg("-o").arg(&output_template);
        cmd.arg("--no-playlist");
        cmd.arg("--no-warnings");
        cmd.args(["--extractor-args", "youtube:player_client=ios,android,web"]);
        cmd.args([
            "-x",
            "--audio-format", "mp3",
            "--audio-quality", "0",
            "--embed-metadata",
            "--add-metadata",
        ]);
        cmd.arg(&download_target);

        let status = cmd.status().await;
        let downloaded_path = if status.as_ref().map_or(false, |s| s.success()) && expected_file.exists() {
            expected_file
        } else {
            let mut found = None;
            if let Ok(entries) = std::fs::read_dir(&self.download_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with(&file_id) && (name.ends_with(".mp3") || name.ends_with(".m4a") || name.ends_with(".webm") || name.ends_with(".opus")) {
                        found = Some(path);
                        break;
                    }
                }
            }
            found.ok_or_else(|| anyhow::anyhow!("Failed to download Spotify audio for {} - {}", artist, title))?
        };

        // Write Spotify cover after yt-dlp finishes so it is never deleted
        let has_cover = if let Some(ref bytes) = cover_bytes {
            let _ = tokio::fs::write(&cover_target, bytes).await;
            true
        } else {
            cover_target.exists()
        };

        let item = MediaItem {
            id: file_id.clone(),
            title: title.to_string(),
            uploader: if !artist.is_empty() { Some(artist.to_string()) } else { None },
            media_type: "audio".to_string(),
            file_name: downloaded_path.file_name().and_then(|n| n.to_str()).unwrap_or(&file_name).to_string(),
            cover_file: if has_cover || cover_target.exists() { Some(cover_name) } else { None },
            duration_secs: None,
            source_url: source_url.to_string(),
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        };

        self.save_media_item(item.clone()).await;
        Ok((downloaded_path, item))
    }

    pub async fn download_pinterest_image(&self, url: &str) -> anyhow::Result<(PathBuf, MediaItem)> {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        let resp = client.get(url).send().await?;
        let html = resp.text().await?;

        let og_re = regex::Regex::new(r#"<meta(?:[^>]+)og:image(?:[^>]+)>"#).unwrap();
        let content_re = regex::Regex::new(r#"content="([^"]+)""#).unwrap();
        let json_re = regex::Regex::new(r#""image":"(https://i\.pinimg\.com/[^"]+)""#).unwrap();

        let mut img_url_opt = None;
        if let Some(mat) = og_re.find(&html) {
            let tag = mat.as_str();
            if let Some(caps) = content_re.captures(tag) {
                let url_str = caps.get(1).unwrap().as_str();
                img_url_opt = Some(
                    url_str
                        .replace("/736x/", "/originals/")
                        .replace("/474x/", "/originals/")
                        .replace("/236x/", "/originals/"),
                );
            }
        }
        if img_url_opt.is_none() {
            if let Some(caps) = json_re.captures(&html) {
                img_url_opt = Some(caps.get(1).unwrap().as_str().to_string());
            }
        }

        if let Some(img_url) = img_url_opt {
            let file_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
            let file_path = self.download_dir.join(format!("{}.jpg", file_id));
            let img_resp = client.get(&img_url).send().await?;
            let bytes = img_resp.bytes().await?;
            tokio::fs::write(&file_path, &bytes).await?;

            let item = MediaItem {
                id: file_id.clone(),
                title: "Pinterest Image".to_string(),
                uploader: Some("Pinterest".to_string()),
                media_type: "photo".to_string(),
                file_name: format!("{}.jpg", file_id),
                cover_file: Some(format!("{}.jpg", file_id)),
                duration_secs: None,
                source_url: url.to_string(),
                created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            };
            self.save_media_item(item.clone()).await;
            return Ok((file_path, item));
        }

        anyhow::bail!("Failed to extract Pinterest image")
    }

    pub async fn download(&self, url: &str, quality: Quality) -> anyhow::Result<PathBuf> {
        let file_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let is_spotify = url.contains("spotify.com");
        let is_audio = quality.is_audio() || is_spotify || url.contains("soundcloud.com");
        let ext = if is_audio { "mp3" } else { "mp4" };
        
        let output_template = self.download_dir.join(format!("{}.%(ext)s", file_id));
        let expected_file = self.download_dir.join(format!("{}.{}", file_id, ext));
        let cover_target = self.download_dir.join(format!("{}.jpg", file_id));

        info!("FSocial Downloader: starting download for {} ({})", url, quality.display_name());

        let mut title = "Media Track".to_string();
        let mut uploader = None;
        let mut duration_secs = None;
        let mut download_target = url.to_string();

        if is_spotify {
            let (sp_title, sp_artist, sp_cover) = Self::parse_spotify_track_meta(url).await;
            title = sp_title.clone();
            uploader = sp_artist.clone();

            if let Some(ref cover) = sp_cover {
                if let Ok(resp) = reqwest::get(cover).await {
                    if let Ok(bytes) = resp.bytes().await {
                        let _ = tokio::fs::write(&cover_target, &bytes).await;
                    }
                }
            }

            if let Some(found_url) = Self::search_spotify_track(sp_artist.as_deref().unwrap_or(""), &sp_title).await {
                info!("Found exact YouTube match for Spotify track: {}", found_url);
                download_target = found_url;
            } else {
                download_target = format!("ytsearch1:{} {} audio", sp_artist.as_deref().unwrap_or(""), sp_title);
            }
        } else {
            let info = self.fetch_info_response(url).await.ok();
            if let Some(ref inf) = info {
                title = inf.title.clone();
                uploader = inf.uploader.clone();
                duration_secs = inf.duration_secs;
                if let Some(ref thumb_url) = inf.thumbnail {
                    if let Ok(resp) = reqwest::get(thumb_url).await {
                        if let Ok(bytes) = resp.bytes().await {
                            let _ = tokio::fs::write(&cover_target, &bytes).await;
                        }
                    }
                }
            }
        }

        let mut cmd = Self::ytdlp_cmd();
        cmd.arg("-o").arg(&output_template);
        let (_, _, media_type) = Self::detect_url(url).unwrap_or((url.to_string(), Platform::Unknown, MediaType::Video));
        if media_type != MediaType::Playlist && !url.contains("/playlist/") && !url.contains("/album/") {
            cmd.arg("--no-playlist");
        }
        cmd.arg("--no-warnings");
        cmd.args(["--extractor-args", "youtube:player_client=ios,android,web"]);
        cmd.args(["--write-thumbnail", "--convert-thumbnails", "jpg"]);

        if is_audio {
            cmd.args([
                "-x",
                "--audio-format", "mp3",
                "--audio-quality", "0",
                "--embed-thumbnail",
                "--embed-metadata",
                "--add-metadata"
            ]);
        } else {
            cmd.args(["-f", quality.ytdlp_format()]);
        }

        cmd.arg(&download_target);

        let status = cmd.status().await;

        let downloaded_path = match status {
            Ok(s) if s.success() => {
                if expected_file.exists() {
                    expected_file.clone()
                } else {
                    let mut found = None;
                    if let Ok(entries) = std::fs::read_dir(&self.download_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.file_name().and_then(|n| n.to_str()).map_or(false, |name| name.starts_with(&file_id) && !name.ends_with(".jpg") && !name.ends_with(".webp")) {
                                found = Some(path);
                                break;
                            }
                        }
                    }
                    found.ok_or_else(|| anyhow::anyhow!("Downloaded file not found on disk"))?
                }
            }
            Ok(s) => {
                warn!("yt-dlp process failed with exit code: {:?}", s.code());
                return Err(anyhow::anyhow!("Download failed with exit code {:?}", s.code()));
            }
            Err(e) => {
                warn!("yt-dlp command failed to execute: {}. (Ensure yt-dlp is installed on system)", e);
                return Err(anyhow::anyhow!("yt-dlp executable error: {}", e));
            }
        };

        // Determine cover file
        let cover_file = if cover_target.exists() {
            Some(format!("{}.jpg", file_id))
        } else {
            let mut found_thumb = None;
            if let Ok(entries) = std::fs::read_dir(&self.download_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with(&file_id) && (name.ends_with(".jpg") || name.ends_with(".png") || name.ends_with(".webp")) {
                        found_thumb = Some(name.to_string());
                        break;
                    }
                }
            }
            found_thumb
        };

        let media_item = MediaItem {
            id: file_id.clone(),
            title,
            uploader,
            media_type: if is_audio { "audio".to_string() } else { "video".to_string() },
            file_name: downloaded_path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string(),
            cover_file,
            duration_secs,
            source_url: url.to_string(),
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
        };

        self.save_media_item(media_item).await;

        Ok(downloaded_path)
    }

    pub async fn download_auto(&self, url: &str, is_audio: bool) -> anyhow::Result<(PathBuf, MediaItem)> {
        // Check Pinterest photo first
        if url.contains("pinterest.com") || url.contains("pin.it") {
            if let Ok(res) = self.download_pinterest_image(url).await {
                return Ok(res);
            }
        }

        // Check Spotify single track direct
        if url.contains("spotify.com/track/") {
            let (title, artist, cover) = Self::parse_spotify_track_meta(url).await;
            return self.download_spotify_track_direct(&title, artist.as_deref().unwrap_or(""), cover.as_deref(), url).await;
        }

        let quality = if is_audio {
            Quality::AudioBest
        } else {
            Quality::Best
        };

        let path = self.download(url, quality).await?;
        let lib = self.get_library().await;
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let item = lib.into_iter().find(|i| i.file_name == file_name).unwrap_or_else(|| {
            MediaItem {
                id: uuid::Uuid::new_v4().to_string()[..8].to_string(),
                title: "Media".to_string(),
                uploader: None,
                media_type: if is_audio { "audio".to_string() } else { "video".to_string() },
                file_name: file_name.to_string(),
                cover_file: None,
                duration_secs: None,
                source_url: url.to_string(),
                created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
            }
        });

        Ok((path, item))
    }

    pub async fn download_playlist(&self, url: &str, is_audio: bool) -> anyhow::Result<Vec<(PathBuf, MediaItem)>> {
        let mut results = Vec::new();

        // 1. Spotify playlist / album extraction
        if url.contains("spotify.com") {
            if let Some(entity) = Self::extract_spotify_entity(url).await {
                if !entity.tracks.is_empty() {
                    for track in entity.tracks {
                        match self.download_spotify_track_direct(
                            &track.title,
                            &track.artist,
                            track.cover_url.as_deref().or(entity.cover_url.as_deref()),
                            &track.url,
                        ).await {
                            Ok(res) => results.push(res),
                            Err(e) => tracing::warn!("Failed to download Spotify track {} - {}: {}", track.artist, track.title, e),
                        }
                    }
                    if !results.is_empty() {
                        return Ok(results);
                    }
                }
            }
        }

        // 2. YouTube or other playlists via yt-dlp
        let mut track_urls = Vec::new();
        let output = Self::ytdlp_cmd()
            .args(["--dump-json", "--flat-playlist", "--extractor-args", "youtube:player_client=ios,android,web", url])
            .output()
            .await;

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(track_url) = json.get("url").and_then(|v| v.as_str()) {
                            if !track_urls.contains(&track_url.to_string()) {
                                track_urls.push(track_url.to_string());
                            }
                        } else if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
                            let yt_url = format!("https://www.youtube.com/watch?v={}", id);
                            if !track_urls.contains(&yt_url) {
                                track_urls.push(yt_url);
                            }
                        }
                        if track_urls.len() >= 50 {
                            break;
                        }
                    }
                }
            }
        }

        if track_urls.is_empty() {
            results.push(self.download_auto(url, is_audio).await?);
        } else {
            for track_url in track_urls {
                match self.download_auto(&track_url, is_audio).await {
                    Ok(res) => results.push(res),
                    Err(e) => tracing::warn!("Failed to download track {}: {}", track_url, e),
                }
            }
        }

        if results.is_empty() {
            anyhow::bail!("Failed to download playlist tracks");
        }

        Ok(results)
    }

    pub async fn download_images(&self, url: &str) -> anyhow::Result<Vec<PathBuf>> {
        let output = Self::ytdlp_cmd()
            .args([
                "--dump-json",
                "--no-warnings",
                "--extractor-args",
                "youtube:player_client=ios,android,web",
                url,
            ])
            .output()
            .await?;

        let mut paths = Vec::new();
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut image_urls = Vec::new();
            for line in stdout.lines() {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(entries) = json.get("entries").and_then(|v| v.as_array()) {
                        for entry in entries {
                            if let Some(u) = entry.get("url").and_then(|v| v.as_str()) {
                                if !image_urls.contains(&u.to_string()) {
                                    image_urls.push(u.to_string());
                                }
                            }
                        }
                    }
                    if let Some(ext) = json.get("ext").and_then(|v| v.as_str()) {
                        if matches!(ext, "jpg" | "png" | "webp" | "jpeg") {
                            if let Some(u) = json.get("url").and_then(|v| v.as_str()) {
                                if !image_urls.contains(&u.to_string()) {
                                    image_urls.push(u.to_string());
                                }
                            }
                        }
                    }
                    if image_urls.is_empty() {
                        if let Some(thumbs) = json.get("thumbnails").and_then(|v| v.as_array()) {
                            for thumb in thumbs {
                                if let Some(u) = thumb.get("url").and_then(|v| v.as_str()) {
                                    if u.contains("tiktok") || u.contains("instagram") || u.contains("cdn") {
                                        if !image_urls.contains(&u.to_string()) {
                                            image_urls.push(u.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let client = reqwest::Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default();

            for (idx, img_url) in image_urls.iter().enumerate() {
                if let Ok(resp) = client.get(img_url).send().await {
                    if let Ok(bytes) = resp.bytes().await {
                        if bytes.len() > 1024 {
                            let file_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
                            let file_path = self.download_dir.join(format!("{}_{}.jpg", file_id, idx));
                            if tokio::fs::write(&file_path, &bytes).await.is_ok() {
                                paths.push(file_path);
                            }
                        }
                    }
                }
            }
        }

        if paths.is_empty() {
            anyhow::bail!("No images found");
        }

        Ok(paths)
    }
}
