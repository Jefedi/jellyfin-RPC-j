# syntax=docker/dockerfile:1.6

# ---------- Build stage ----------
FROM rust:1.82-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/jellyfin-rpc

COPY Cargo.toml Cargo.lock ./
COPY jellyfin-rpc ./jellyfin-rpc
COPY jellyfin-rpc-cli ./jellyfin-rpc-cli

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/jellyfin-rpc/target \
    cargo build --release --bin jellyfin-rpc \
    && cp target/release/jellyfin-rpc /usr/local/bin/jellyfin-rpc \
    && strip /usr/local/bin/jellyfin-rpc

# ---------- Runtime stage ----------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 1000 jellyfin-rpc \
    && useradd  --system --uid 1000 --gid jellyfin-rpc --create-home --home-dir /home/jellyfin-rpc jellyfin-rpc

COPY --from=builder /usr/local/bin/jellyfin-rpc /usr/local/bin/jellyfin-rpc

USER jellyfin-rpc
WORKDIR /home/jellyfin-rpc

VOLUME ["/config"]

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/jellyfin-rpc"]
CMD ["-c", "/config/main.json", "-i", "/config/urls.json"]
