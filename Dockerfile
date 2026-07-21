# syntax=docker/dockerfile:1
FROM rust:1.88-bookworm AS builder
WORKDIR /build

COPY Cargo.toml ./
COPY src ./src
COPY web ./web
COPY game-packs ./game-packs

RUN cargo build --release --locked || cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 10001 game \
    && mkdir -p /app/game-packs/drainage /data \
    && chown -R game:game /app /data

WORKDIR /app
COPY --from=builder /build/target/release/civic-sim-server /app/civic-sim-server
COPY --chown=game:game game-packs/drainage/game.json /app/game-packs/drainage/game.json

USER game
ENV PORT=8080 \
    RUST_LOG=civic_sim_server=info,tower_http=info \
    GAME_PACK_PATH=/app/game-packs/drainage/game.json \
    SESSION_STORE_PATH=/data/sessions.json

VOLUME ["/data"]
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=5 \
  CMD curl --fail --silent http://127.0.0.1:8080/api/v1/health > /dev/null || exit 1

ENTRYPOINT ["/app/civic-sim-server"]
