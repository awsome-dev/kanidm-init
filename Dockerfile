# syntax=docker/dockerfile:1.6

# =========================
# Stage 1: builder
# =========================
FROM --platform=$BUILDPLATFORM rust:1.95-slim AS builder

ARG TARGETPLATFORM
WORKDIR /usr/src/init

# --- system deps（最小・安定優先） ---
RUN apt-get update && apt-get install -y \
    curl \
    python3 \
    python3-pip \
    pkg-config \
    libssl-dev \
    gcc \
 && rm -rf /var/lib/apt/lists/*

# --- zig build tool ---
RUN pip3 install --no-cache-dir cargo-zigbuild --break-system-packages

# --- target決定 ---
RUN case "$TARGETPLATFORM" in \
    linux/amd64) echo "x86_64-unknown-linux-musl" ;; \
    linux/arm64) echo "aarch64-unknown-linux-musl" ;; \
    *) echo "unsupported" && exit 1 ;; \
    esac > /tmp/target

RUN rustup target add $(cat /tmp/target)

# --- source ---
COPY . .

# =========================
# build（シンプル最速安定）
# =========================
RUN set -eux; \
    TARGET=$(cat /tmp/target); \
    cargo zigbuild --release --target $TARGET; \
    install -m 0755 target/$TARGET/release/kanidm_init /out

# =========================
# Stage 2: runtime (distroless)
# =========================
FROM docker.io/kanidm/server:latest

COPY --from=builder --chmod=0755 /out /sbin/kanidm_init
