FROM rust:slim-bookworm AS builder

# Install build dependencies for Rust and C bindings (SQLite, FontConfig, OpenSSL)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libfontconfig1-dev \
    libfreetype6-dev \
    libsqlite3-dev \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /app

# Copy everything
COPY . .

# Build the application for release
RUN cargo build --release -p brain-telegram-bot

# Final Stage: Minimal Debian image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libfontconfig1 \
    libfreetype6 \
    libsqlite3-0 \
    libssl3 \
    ca-certificates \
    wget \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create necessary directories for volumes
RUN mkdir -p /app/data /app/logs

# Copy the build artifact and config from the builder stage
COPY --from=builder /app/target/release/brain-telegram-bot /usr/local/bin/
COPY --from=builder /app/config.toml /app/config.toml

# Environment Variables
ENV RUST_LOG=info
ENV VAULT_PATH=/app/data
ENV LOG_FILE=/app/logs/events.jsonl

# Expose the health check port
EXPOSE 8080

ENTRYPOINT ["brain-telegram-bot"]
