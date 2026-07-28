# syntax=docker/dockerfile:1
FROM node:22-bookworm-slim AS web-builder
WORKDIR /web

COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/tsconfig.json web/vite.config.ts ./
COPY web/city-src ./city-src
COPY web/city-data ./city-data
COPY web/tests ./tests
RUN npm run typecheck && npm test && npm run build

FROM rust:1.88-bookworm AS builder
WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY web ./web
COPY --from=web-builder /web/dist ./web/dist
COPY game-packs ./game-packs

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 10001 game \
    && mkdir -p /app/game-packs /data \
    && chown -R game:game /app /data

WORKDIR /app
COPY --from=builder /build/target/release/civic-sim-server /app/civic-sim-server
COPY --chown=game:game game-packs /app/game-packs

USER game
ENV PORT=8080 \
    RUST_LOG=civic_sim_server=info,tower_http=info \
    GAME_PACKS_PATH=/app/game-packs \
    DEFAULT_GAME_PACK_ID=civic-drainage-v1 \
    GAME_PACK_PATH=/app/game-packs/drainage/game.json \
    ABSTRACTION_CATALOG_PATH=/app/game-packs/abstractions.json \
    SESSION_STORE_PATH=/data/sessions.json \
    GENERATED_PACK_STORE_PATH=/data/generated-packs.json \
    CAMPAIGN_STORE_PATH=/data/campaigns.json

VOLUME ["/data"]
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=5 \
  CMD curl --fail --silent http://127.0.0.1:8080/api/v1/health > /dev/null || exit 1

ENTRYPOINT ["/app/civic-sim-server"]
