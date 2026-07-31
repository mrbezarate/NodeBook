# 🧠 Brain: Agentic Knowledge Operating System

![Rust](https://img.shields.io/badge/rust-v1.75+-orange.svg)
![Architecture](https://img.shields.io/badge/architecture-Event%20Driven-blue.svg)
![Status](https://img.shields.io/badge/status-Pre--Production-success.svg)

**Brain** is an autonomous, agentic operating system designed to process, structure, and synthesize personal knowledge. It acts as an intelligent bridge between chaotic raw thoughts (via Telegram) and highly structured, interconnected knowledge bases (Obsidian Vault).

Unlike simple LLM wrappers, **Brain** employs a deeply decoupled, event-driven architecture designed for high resilience, offline-first processing, and automated knowledge graph consolidation.

---

## 🏗 Architecture Highlights

The system is built entirely in **Rust** (Workspace with isolated crates) and follows advanced architectural patterns:

### 1. Event-Driven Knowledge Pipeline
Instead of immediate blocking processing, incoming data follows a resilient pipeline:
* **Ingestion:** Raw text is captured and durably stored as a `RawEvent` in SQLite.
* **Job Queue:** Background workers (`Consolidator`) pick up asynchronous tasks to prevent data loss during network/LLM failures.
* **Extraction:** Local/Remote LLMs extract entities, facts, and semantics.
* **Identity Resolution:** Algorithms fuzzy-match and resolve entities against the existing knowledge graph.
* **Projection Engine:** Event streams are aggregated into static snapshots (Markdown files) in the Obsidian Vault.

### 2. The Agentic Pipeline
The core reasoning engine doesn't just format text; it analyzes it. 
* **Entity Validation:** Detects whether a mentioned entity is a "Concept", "Person", "Project", or "Idea".
* **Semantic Linking:** Suggests and creates bidirectional links (`[[Node]]`) between related thoughts.
* **Idempotency & Rebuilds:** Any entity can be fully reconstructed from its historical raw events via the `/rebuild` command.

### 3. Output Contract v2
The core logic (`brain-core`) is completely decoupled from the delivery mechanism (`brain-telegram-bot`). 
Using the `OutputSink` trait and `ResourceLifecycle` (Temporary, Persistent, Cached), Brain cleanly hands over formatting and cleanup to adapters, making it ready for Discord, CLI, or Web UI integration out of the box.

### 4. Zero-Flake E2E Testing (Fixtures)
The `fixtures/` architecture captures real, raw LLM outputs (including broken JSON and hallucinations) to run deterministic E2E integration tests in milliseconds—without querying the live LLM.

---

## 📦 Project Structure

Brain is split into highly cohesive Rust crates:

```text
crates/
├── brain-common      # Domain types, Event structures, Output Contract
├── brain-core        # Consolidator, Pipeline, Projection Engine, Identity Resolver
├── brain-ai          # LLM Providers (Ollama, OpenAI) abstraction
├── brain-vault       # Obsidian Markdown storage, Frontmatter & PARA routing
├── brain-analytics   # System metrics, extraction fail rates
├── brain-memory      # Short-term / Working memory abstraction
└── brain-events      # Internal Event Bus
apps/
└── telegram-bot      # Telegram Gateway & CLI OutputSink adapter
```

---

## 🚀 Getting Started

### Prerequisites
* Rust 1.75+
* Docker & Docker Compose
* Ollama (Running locally for Agentic Extraction)

### Running Locally

1. **Clone the repository:**
   ```bash
   git clone https://github.com/mrbezarate/Telegramm-Obsidian.git
   cd Telegramm-Obsidian
   ```

2. **Set up Environment Variables:**
   Copy `.env.example` to `.env` and fill in your Telegram Bot Token and Obsidian Vault path.

3. **Run via Cargo:**
   ```bash
   cargo run --bin brain-telegram-bot
   ```

4. **Run via Docker:**
   ```bash
   docker-compose up -d --build
   ```

---

## 🛠 Commands (Telegram UI)

* `[Text]` — Ingest raw knowledge, ideas, or links.
* `/debug <event_id>` — Retrieve a deep architectural trace (Extraction -> Projection -> Output) for any event.
* `/rebuild <event_id>` — Force the projection engine to recalculate a node from scratch.
* `/metrics` — View system health, JSON parsing success rates, and extraction statistics.
* `/diary` — Enter interactive end-of-day reflection mode.

---

## 🎯 Current Status: *Reality Validation (Phase 0.2)*
The pre-production foundation (v0.1) is complete. The system is currently in a "Core Freeze" state, focusing entirely on **Reality Validation**—ingesting real-world unstructured data to evaluate the agentic pipeline's practical UX and entity resolution accuracy before migrating to complex distributed systems.
