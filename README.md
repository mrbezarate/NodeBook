# 🧠 NodeBook: Personal Knowledge OS & Telegram Mini App

![Rust](https://img.shields.io/badge/rust-v1.75+-orange.svg?style=flat-square&logo=rust)
![Teloxide](https://img.shields.io/badge/telegram-teloxide_0.13-blue.svg?style=flat-square&logo=telegram)
![Axum](https://img.shields.io/badge/web-axum_0.8-black.svg?style=flat-square)
![Tantivy](https://img.shields.io/badge/search-tantivy-green.svg?style=flat-square)
![Gemini AI](https://img.shields.io/badge/AI-Google_Gemini_2.5-8E75B2.svg?style=flat-square&logo=google)
![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)

**NodeBook** is an all-in-one personal operating system and agentic knowledge workspace seamlessly connected to Telegram and a high-performance Telegram Mini App (Web OS). It bridges raw thoughts, media, tasks, daily reflections, and language learning into an interconnected, searchable knowledge base.

---

## ✨ Key Features

### 1. 📚 Personal Knowledge Base & Multi-Vault (Obsidian Compatible)
* **PARA Framework & Markdown Vaults:** Organize knowledge across *Projects*, *Areas*, *Resources*, and *Archives*.
* **Tantivy Full-Text Search:** Instant indexing and fuzzy search across all notes, tags, and frontmatter metadata.
* **Knowledge Graph & WikiLinks:** Automatic resolution of bidirectional `[[WikiLinks]]` and entity relationships.
* **Multi-Vault Switcher:** Create, rename, switch, and manage independent vaults directly from the Web OS or Telegram.

### 2. 🎵 FSocial Media Downloader & Player Engine
* **Universal Multi-Platform Downloader:** Supports YouTube, Spotify (Tracks & Playlists), TikTok, Instagram Reels, Pinterest, and SoundCloud.
* **Smart Audio Candidate Scoring & Variant Precision:**
  * Accurately distinguishes song variations: `(Sped Up)`, `(Slowed + Reverb)`, `(Nightcore)`, `(Remix)`, `(Acoustic)`.
  * Multi-stage search scoring prioritizes YouTube Music official tracks, verified artist channels, and duration checks to prevent downloading wrong covers, 10-hour loops, or low-quality snippets.
* **Multi-Artist Support:** Parses complex artist strings (`"Eminem, Rihanna"`, `"feat."`, `"&"`), generates individual interactive artist badges, and provides aggregated artist discography filtering.
* **Web Audio & Video Player:**
  * ID-based continuous playback engine resilient to playlist sorting and filtering.
  * Native **MediaSession API** integration for lock screen controls, playback sync, and metadata.
  * Custom playlists, favorites, looping, and volume persistence.

### 3. 🌙 Evening Diary & Life Analytics
* **Interactive Reflection FSM:** Guided evening review through Telegram inline keyboards and mini app forms (mood, productivity, energy, gratitude).
* **Automated Weather Integration:** Real-time weather and forecast fetching via `wttr.in`.
* **Smart Cron Scheduler:** Timezone-aware local scheduling (e.g. UTC+5) for automatic daily prompts and analytics reports.

### 4. 🇬🇧 English Tutor SRS & AI Assistant
* **Spaced Repetition System (SRS):** SM-2 flashcard review system for vocabulary retention.
* **Google Gemini AI Integration:** Context-aware grammar explanations, pronunciation tips, translation, and interactive language quizzes.

### 5. 🚇 Cloudflare HTTPS Tunnel & Single-Instance Safety
* **Zero-Config Cloudflare Tunnel:** Built-in supervisor launches a secure HTTPS tunnel and automatically registers the live URL with the Telegram Bot Mini App menu button.
* **Kernel-Level SingleInstanceGuard:** Uses Linux `libc::flock` to guarantee single-instance execution, eliminating duplicate messages and race conditions during long polling.

---

## 🏗 System Architecture

```text
NodeBook (Rust Workspace)
├── apps/
│   ├── telegram-bot        # Telegram Gateway, Axum Web Server & WebApp SPA
│   ├── media-downloader    # FSocial Engine (yt-dlp, Spotify, YouTube, Pinterest)
│   └── english-tutor       # Spaced Repetition System (SRS) & Gemini AI Assistant
│
└── crates/
    ├── brain-core          # Engine, SQLite Event Store, Identity Resolver, Projections
    ├── brain-vault         # Obsidian-compatible Markdown storage & PARA routing
    ├── brain-indexer       # Tantivy full-text search engine & live file watcher
    ├── brain-ai            # LLM abstraction layer (Gemini 2.5 Flash, Ollama)
    ├── brain-diary         # Evening review state machine & day information
    ├── brain-plugin        # Hybrid plugin registration & execution framework
    ├── brain-analytics     # Stats, correlation analysis, and metrics
    ├── brain-scheduler     # Tokio-based cron scheduler
    ├── brain-memory        # Working memory & context manager
    └── brain-common        # Shared domain types, event bus, and contracts
```

---

## 🚀 Quick Start & Installation

### Prerequisites
* **Rust 1.75+** (`rustup default stable`)
* **yt-dlp** and **ffmpeg** (for media downloads and audio extraction)
* **Cloudflared** (optional, automatically managed if installed or placed in `./bin/cloudflared`)

### 1. Clone the Repository
```bash
git clone git@github.com:mrbezarate/NodeBook.git
cd NodeBook
```

### 2. Environment Configuration
Create a `.env` file in the root directory:
```env
BOT_TOKEN=your_telegram_bot_token
GEMINI_API_KEY=your_google_gemini_api_key
ALLOWED_USERS=5887915765
VAULT_PATH=./vault
RUST_LOG=info
```

### 3. Build & Run
```bash
# Debug mode
cargo run --bin brain-telegram-bot

# Optimized Release mode
cargo build --release --bin brain-telegram-bot
./target/release/brain-telegram-bot
```

---

## 🤖 Telegram Bot Commands

| Command | Description |
|---|---|
| `[Text / Link]` | Ingest raw thought, note, or media link (Spotify, YouTube, TikTok, Pinterest) |
| `/start` | Welcome screen and quick launch button for NodeBook Mini App |
| `/diary` | Start interactive evening review & daily reflection |
| `/vaults` | Manage, create, or switch active knowledge vaults |
| `/metrics` | View system health, database metrics, and storage stats |
| `/rebuild` | Recalculate and re-index knowledge projections from raw events |

---

## ⚙️ Running as a Systemd Service

To run NodeBook continuously as a background service:

```ini
# ~/.config/systemd/user/nodebook.service
[Unit]
Description=NodeBook Personal Knowledge OS & Telegram Mini App
After=network.target

[Service]
Type=simple
WorkingDirectory=/home/youruser/NodeBook
ExecStart=/home/youruser/NodeBook/target/release/brain-telegram-bot
Restart=always
RestartSec=5s
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
```

Enable and start the service:
```bash
systemctl --user daemon-reload
systemctl --user enable --now nodebook.service
```

---

## 📄 License
This project is licensed under the [MIT License](LICENSE).
