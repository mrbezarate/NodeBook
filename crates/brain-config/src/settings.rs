//! Полная иерархия конфигурации Brain.

use brain_common::{BrainError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Главная конфигурация системы Brain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainConfig {
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub vault: VaultConfig,
    #[serde(default)]
    pub ai: AiConfig,
    #[serde(default)]
    pub embeddings: EmbeddingsConfig,
    #[serde(default)]
    pub classifier: ClassifierConfig,
    #[serde(default)]
    pub diary: DiaryConfig,
    #[serde(default)]
    pub analytics: AnalyticsConfig,
    #[serde(default)]
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for BrainConfig {
    fn default() -> Self {
        Self {
            telegram: TelegramConfig::default(),
            vault: VaultConfig::default(),
            ai: AiConfig::default(),
            embeddings: EmbeddingsConfig::default(),
            classifier: ClassifierConfig::default(),
            diary: DiaryConfig::default(),
            analytics: AnalyticsConfig::default(),
            scheduler: SchedulerConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl BrainConfig {
    /// Загрузить конфигурацию из файла.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| BrainError::Config(e.to_string()))
    }

    /// Загрузить конфигурацию из стандартных путей, либо вернуть default.
    /// Порядок поиска: ./config.toml → ~/.config/brain/config.toml → default
    pub fn load_or_default() -> Self {
        // 1. Текущая директория
        if let Ok(cfg) = Self::load("config.toml") {
            tracing::info!("Loaded config from ./config.toml");
            return cfg;
        }
        // 2. XDG config
        if let Some(config_dir) = dirs::config_dir() {
            let path = config_dir.join("brain").join("config.toml");
            if let Ok(cfg) = Self::load(&path) {
                tracing::info!("Loaded config from {}", path.display());
                return cfg;
            }
        }
        // 3. Default
        tracing::warn!("No config file found, using defaults");
        Self::default()
    }
}

// ── Telegram ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    #[serde(default)]
    pub bot_token: String,
    #[serde(default)]
    pub allowed_users: Vec<u64>,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self { bot_token: String::new(), allowed_users: vec![] }
    }
}

// ── Vault ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    #[serde(default = "default_vault_path")]
    pub path: String,
    #[serde(default = "default_daily_folder")]
    pub daily_folder: String,
    #[serde(default = "default_templates_folder")]
    pub templates_folder: String,
    #[serde(default)]
    pub para: ParaConfig,
}

fn default_vault_path() -> String { "~/Obsidian/Brain".into() }
fn default_daily_folder() -> String { "Daily".into() }
fn default_templates_folder() -> String { "Templates".into() }

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            path: default_vault_path(),
            daily_folder: default_daily_folder(),
            templates_folder: default_templates_folder(),
            para: ParaConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParaConfig {
    #[serde(default = "default_projects")]
    pub projects: String,
    #[serde(default = "default_areas")]
    pub areas: String,
    #[serde(default = "default_resources")]
    pub resources: String,
    #[serde(default = "default_archive")]
    pub archive: String,
    #[serde(default = "default_inbox")]
    pub inbox: String,
}

fn default_projects() -> String { "Projects".into() }
fn default_areas() -> String { "Areas".into() }
fn default_resources() -> String { "Resources".into() }
fn default_archive() -> String { "Archive".into() }
fn default_inbox() -> String { "Inbox".into() }

impl Default for ParaConfig {
    fn default() -> Self {
        Self {
            projects: default_projects(),
            areas: default_areas(),
            resources: default_resources(),
            archive: default_archive(),
            inbox: default_inbox(),
        }
    }
}

// ── AI ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_ai_provider")]
    pub provider: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub openai: OpenAiConfig,
}

fn default_ai_provider() -> String { "ollama".into() }
fn default_temperature() -> f32 { 0.3 }
fn default_max_tokens() -> u32 { 512 }

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: default_ai_provider(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            ollama: OllamaConfig::default(),
            openai: OpenAiConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_url")]
    pub base_url: String,
    #[serde(default = "default_ollama_model")]
    pub model: String,
    #[serde(default = "default_heavy_model")]
    pub heavy_model: String,
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
}

fn default_ollama_url() -> String { "http://localhost:11434".into() }
fn default_ollama_model() -> String { "gemma-light".into() }
fn default_heavy_model() -> String { "gemma-heavy".into() }
fn default_embedding_model() -> String { "nomic-embed-text".into() }

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: default_ollama_url(),
            model: default_ollama_model(),
            heavy_model: default_heavy_model(),
            embedding_model: default_embedding_model(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenAiConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_openai_model")]
    pub model: String,
    #[serde(default = "default_openai_url")]
    pub base_url: String,
    #[serde(default)]
    pub heavy_model: String,
    #[serde(default)]
    pub embedding_model: String,
}

fn default_openai_model() -> String { "gpt-4o-mini".into() }
fn default_openai_url() -> String { "https://api.openai.com/v1".into() }

// ── Embeddings ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_dimension")]
    pub dimension: usize,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
    #[serde(default = "default_storage")]
    pub storage: String,
}

fn default_true() -> bool { true }
fn default_dimension() -> usize { 768 }
fn default_similarity_threshold() -> f32 { 0.75 }
fn default_storage() -> String { "memory".into() }

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dimension: default_dimension(),
            similarity_threshold: default_similarity_threshold(),
            storage: default_storage(),
        }
    }
}

// ── Classifier ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifierConfig {
    #[serde(default = "default_strategy")]
    pub strategy: String,
    #[serde(default = "default_confidence")]
    pub confidence_threshold: f32,
    #[serde(default = "default_fallback_type")]
    pub fallback_type: String,
    #[serde(default = "default_fallback_area")]
    pub fallback_area: String,
}

fn default_strategy() -> String { "hybrid".into() }
fn default_confidence() -> f32 { 0.7 }
fn default_fallback_type() -> String { "thought".into() }
fn default_fallback_area() -> String { "Life".into() }

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            strategy: default_strategy(),
            confidence_threshold: default_confidence(),
            fallback_type: default_fallback_type(),
            fallback_area: default_fallback_area(),
        }
    }
}

// ── Diary ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryConfig {
    #[serde(default = "default_review_time")]
    pub evening_review_time: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_city")]
    pub city: String,
    #[serde(default = "default_life_expectancy")]
    pub life_expectancy_years: u32,
    #[serde(default = "default_birth_date")]
    pub birth_date: String,
}

fn default_review_time() -> String { "21:00".into() }
fn default_timezone() -> String { "Asia/Yekaterinburg".into() }
fn default_city() -> String { "Yekaterinburg".into() }
fn default_life_expectancy() -> u32 { 80 }
fn default_birth_date() -> String { "2000-01-01".into() }

impl Default for DiaryConfig {
    fn default() -> Self {
        Self {
            evening_review_time: default_review_time(),
            timezone: default_timezone(),
            city: default_city(),
            life_expectancy_years: default_life_expectancy(),
            birth_date: default_birth_date(),
        }
    }
}

// ── Analytics ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    #[serde(default = "default_min_data_points")]
    pub min_data_points: usize,
    #[serde(default = "default_trend_window")]
    pub trend_window_days: usize,
    #[serde(default = "default_true")]
    pub insight_generation: bool,
}

fn default_min_data_points() -> usize { 7 }
fn default_trend_window() -> usize { 14 }

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            min_data_points: default_min_data_points(),
            trend_window_days: default_trend_window(),
            insight_generation: true,
        }
    }
}

// ── Scheduler ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    #[serde(default = "default_check_interval")]
    pub check_interval_seconds: u64,
    #[serde(default = "default_max_reminders")]
    pub max_reminders_per_day: u32,
}

fn default_check_interval() -> u64 { 60 }
fn default_max_reminders() -> u32 { 20 }

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            check_interval_seconds: default_check_interval(),
            max_reminders_per_day: default_max_reminders(),
        }
    }
}

// ── Logging ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub file: String,
}

fn default_log_level() -> String { "info".into() }

impl Default for LoggingConfig {
    fn default() -> Self {
        Self { level: default_log_level(), file: String::new() }
    }
}
