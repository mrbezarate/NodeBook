FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/app

# Зависимости для сборки (openssl, pkg-config)
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Копируем workspace целиком
COPY . .

# Собираем release binary
RUN cargo build --release --bin brain-telegram-bot

# ── Финальный образ ──────────────────────────────────────────
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

# Копируем бинарник
COPY --from=builder /usr/src/app/target/release/brain-telegram-bot /usr/local/bin/brain-telegram-bot

# Создаём директории
RUN mkdir -p /app/base/001\ Daily /app/logs

CMD ["brain-telegram-bot"]
