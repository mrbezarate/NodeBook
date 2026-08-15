use fsocial_common::models::{InfoResponse, Quality, QualityOption};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFilterMode {
    All,
    VideoOnly,
    AudioOnly,
}

pub struct UiBuilder;

impl UiBuilder {
    pub fn build_info_message(info: &InfoResponse, mode: MediaFilterMode) -> String {
        let qualities = Self::filter_qualities(&info.available_qualities, mode);

        if info.is_playlist {
            let safe_title = html_escape(&info.title);
            let mut dur_str = String::new();
            if let Some(dur) = info.duration_secs {
                let hours = dur / 3600;
                let minutes = (dur % 3600) / 60;
                let seconds = dur % 60;
                let d_str = if hours > 0 {
                    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
                } else {
                    format!("{:02}:{:02}", minutes, seconds)
                };
                dur_str = format!("\n⏱ <i>{}</i>", d_str);
            }
            let type_str = match mode {
                MediaFilterMode::AudioOnly => "Аудио-плейлист",
                MediaFilterMode::VideoOnly => "Видео-плейлист",
                MediaFilterMode::All => "Плейлист",
            };
            return format!(
                "📁 <b>{} ({})</b>\n—\n<i>Элементов: {}</i>{}\n\nФорматы для загрузки ↓",
                safe_title,
                type_str,
                info.playlist_count.unwrap_or(0),
                dur_str
            );
        }

        let mut lines = Vec::new();
        let icon = match mode {
            MediaFilterMode::AudioOnly => "🎧",
            MediaFilterMode::VideoOnly => "📹",
            MediaFilterMode::All => if info.available_qualities.iter().all(|q| q.quality.is_audio()) { "🎧" } else { "📹" },
        };
        lines.push(format!("{} <b>{}</b>", icon, html_escape(&info.title)));

        if let Some(ref author) = info.uploader {
            lines.push(format!("👤 <i>{}</i>", html_escape(author)));
        }

        if let Some(dur) = info.duration_secs {
            let hours = dur / 3600;
            let minutes = (dur % 3600) / 60;
            let seconds = dur % 60;
            let d_str = if hours > 0 {
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                format!("{:02}:{:02}", minutes, seconds)
            };
            lines.push(format!("⏱ <i>{}</i>", d_str));
        }

        lines.push(String::new());

        for opt in &qualities {
            let size_mb = opt.filesize_bytes.map(|b| b / (1024 * 1024)).unwrap_or(0);
            let q_name = match opt.quality {
                Quality::Video360p => "360p",
                Quality::Video480p => "480p",
                Quality::Video720p => "720p",
                Quality::Video1080p => "1080p",
                Quality::Video1440p => "1440p",
                Quality::Video4K => "4K",
                Quality::Best => "Best",
                Quality::Audio128 => "128k",
                Quality::Audio256 => "256k",
                Quality::AudioBest => "MP3",
            };

            let speed_icon = match size_mb {
                0..=35 => "⚡️",
                36..=120 => "🚀",
                _ => "⚖️",
            };

            let size_str = if size_mb > 0 {
                format!("{:>4}MB", size_mb)
            } else {
                "  ~MB".to_string()
            };

            let time_str = match size_mb {
                0..=35 => "~1-3 сек",
                36..=120 => "~3-10 сек",
                _ => "~15+ сек",
            };

            lines.push(format!("{} {:>5} | {} | ⏳ {}", speed_icon, q_name, size_str, time_str));
        }

        lines.push(String::new());
        let has_large_files = qualities.iter().any(|q| {
            q.filesize_bytes
                .map(|b| b as f64 / (1024.0 * 1024.0) > 50.0)
                .unwrap_or(false)
        });

        if has_large_files {
            lines.push("⚠️ Файлы > 50 МБ ограничены лимитами Telegram.".to_string());
        }

        lines.join("\n")
    }

    pub fn build_quality_options(info: &InfoResponse, short_id: &str, mode: MediaFilterMode) -> Vec<(String, String)> {
        if info.is_playlist {
            let default_q = match mode {
                MediaFilterMode::AudioOnly => Quality::AudioBest,
                MediaFilterMode::VideoOnly => Quality::Video720p,
                MediaFilterMode::All => {
                    if info.available_qualities.iter().all(|q| q.quality.is_audio()) {
                        Quality::AudioBest
                    } else {
                        Quality::Video720p
                    }
                }
            };
            let label = match mode {
                MediaFilterMode::AudioOnly => format!("🎵 Скачать аудио-плейлист ({})", info.playlist_count.unwrap_or(0)),
                MediaFilterMode::VideoOnly => format!("📹 Скачать видео-плейлист ({})", info.playlist_count.unwrap_or(0)),
                MediaFilterMode::All => format!("📥 Скачать плейлист ({})", info.playlist_count.unwrap_or(0)),
            };
            return vec![(
                label,
                format!("mldl:{}:{}", default_q.callback_id(), short_id),
            )];
        }

        let qualities = Self::filter_qualities(&info.available_qualities, mode);

        let mut options = Vec::new();
        for opt in &qualities {
            let label = match opt.quality {
                Quality::Video360p => "360p".to_string(),
                Quality::Video480p => "480p".to_string(),
                Quality::Video720p => "720p".to_string(),
                Quality::Video1080p => "1080p".to_string(),
                Quality::Video1440p => "1440p".to_string(),
                Quality::Video4K => "4K".to_string(),
                Quality::Best => "⭐ Лучшее видео".to_string(),
                Quality::Audio128 => "🎵 MP3 (128k)".to_string(),
                Quality::Audio256 => "🎵 MP3 (256k)".to_string(),
                Quality::AudioBest => "🎵 MP3 (Лучшее)".to_string(),
            };
            options.push((label, format!("mldl:{}:{}", opt.quality.callback_id(), short_id)));
        }

        options
    }

    fn filter_qualities(available: &[QualityOption], mode: MediaFilterMode) -> Vec<QualityOption> {
        match mode {
            MediaFilterMode::VideoOnly => {
                let filtered: Vec<QualityOption> = available.iter().filter(|q| !q.quality.is_audio()).cloned().collect();
                if filtered.is_empty() {
                    vec![
                        QualityOption {
                            quality: Quality::Best,
                            filesize_bytes: None,
                            estimated_secs: None,
                            speed_category: "🚀".to_string(),
                            display_label: "⭐ Лучшее видео".to_string(),
                            full_button_label: "⭐ Лучшее видео".to_string(),
                        },
                        QualityOption {
                            quality: Quality::Video720p,
                            filesize_bytes: None,
                            estimated_secs: None,
                            speed_category: "⚡".to_string(),
                            display_label: "720p".to_string(),
                            full_button_label: "720p HD".to_string(),
                        }
                    ]
                } else {
                    filtered
                }
            }
            MediaFilterMode::AudioOnly => {
                let filtered: Vec<QualityOption> = available.iter().filter(|q| q.quality.is_audio()).cloned().collect();
                if filtered.is_empty() {
                    vec![
                        QualityOption {
                            quality: Quality::AudioBest,
                            filesize_bytes: None,
                            estimated_secs: None,
                            speed_category: "⚡".to_string(),
                            display_label: "🎵 MP3".to_string(),
                            full_button_label: "🎵 MP3 (Лучшее)".to_string(),
                        },
                        QualityOption {
                            quality: Quality::Audio256,
                            filesize_bytes: None,
                            estimated_secs: None,
                            speed_category: "⚡".to_string(),
                            display_label: "🎵 256k".to_string(),
                            full_button_label: "🎵 256 kbps".to_string(),
                        },
                        QualityOption {
                            quality: Quality::Audio128,
                            filesize_bytes: None,
                            estimated_secs: None,
                            speed_category: "⚡".to_string(),
                            display_label: "🎵 128k".to_string(),
                            full_button_label: "🎵 128 kbps".to_string(),
                        },
                    ]
                } else {
                    filtered
                }
            }
            MediaFilterMode::All => available.to_vec(),
        }
    }
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
