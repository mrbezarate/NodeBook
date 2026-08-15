//! Automatic Cloudflare HTTPS Tunnel manager for Telegram Mini App.

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct TunnelManager {
    pub current_url: Arc<RwLock<Option<String>>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            current_url: Arc::new(RwLock::new(std::env::var("WEB_APP_URL").ok())),
        }
    }

    pub async fn get_url(&self) -> Option<String> {
        self.current_url.read().await.clone()
    }

    pub async fn start(&self, bot: teloxide::Bot) {
        if let Ok(env_url) = std::env::var("WEB_APP_URL") {
            if !env_url.is_empty() {
                info!("🌐 Using configured WEB_APP_URL: {}", env_url);
                *self.current_url.write().await = Some(env_url.clone());
                let _ = Self::update_telegram_menu_button(&bot, &env_url).await;
                return;
            }
        }

        let current_url = self.current_url.clone();
        tokio::spawn(async move {
            loop {
                info!("🚇 Starting Cloudflare HTTPS Tunnel for Telegram Mini App...");
                let mut cmd = Command::new("./bin/cloudflared");
                cmd.args(["tunnel", "--url", "http://127.0.0.1:8080", "--no-autoupdate"])
                    .stderr(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null());

                match cmd.spawn() {
                    Ok(mut child) => {
                        if let Some(stderr) = child.stderr.take() {
                            let reader = BufReader::new(stderr);
                            let mut lines = reader.lines();

                            while let Ok(Some(line)) = lines.next_line().await {
                                if line.contains("trycloudflare.com") {
                                    if let Some(start) = line.find("https://") {
                                        let sub = &line[start..];
                                        let end = sub
                                            .find(|c: char| c.is_whitespace() || c == '|' || c == '+')
                                            .unwrap_or(sub.len());
                                        let url = sub[..end].trim().to_string();
                                        if url.ends_with(".trycloudflare.com") {
                                            info!("✅ Telegram Mini App HTTPS Tunnel online: {}", url);
                                            *current_url.write().await = Some(url.clone());
                                            let _ = Self::update_telegram_menu_button(&bot, &url).await;
                                        }
                                    }
                                }
                            }
                        }

                        let _ = child.wait().await;
                        warn!("Cloudflare tunnel exited, restarting in 3s...");
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    }
                    Err(e) => {
                        error!("Failed to spawn cloudflared: {}. Ensure ./bin/cloudflared exists.", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                    }
                }
            }
        });
    }

    pub async fn update_telegram_menu_button(bot: &teloxide::Bot, url: &str) -> anyhow::Result<()> {
        if let Ok(parsed_url) = reqwest::Url::parse(url) {
            use teloxide::prelude::*;
            use teloxide::types::{ChatId, MenuButton, WebAppInfo};

            let menu_button = MenuButton::WebApp {
                text: "📱 Open App".to_string(),
                web_app: WebAppInfo { url: parsed_url },
            };

            let _ = bot.set_chat_menu_button().menu_button(menu_button.clone()).await;
            let _ = bot.set_chat_menu_button().chat_id(ChatId(5887915765)).menu_button(menu_button).await;
            info!("🎉 Telegram Mini App menu button successfully updated with {}", url);
        }
        Ok(())
    }
}
