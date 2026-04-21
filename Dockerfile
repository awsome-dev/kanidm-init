# syntax=docker/dockerfile:1.6

FROM --platform=$BUILDPLATFORM rust:1.95-slim AS builder

ARG TARGETPLATFORM
WORKDIR /usr/src/init

# =========================
# system deps（必要最小限＋pip確実化）
# =========================
RUN apt-get update && apt-get install -y \
    curl \
    python3 \
    python3-pip \
    pkg-config \
    libssl-dev \
    gcc \
    git \
 && rm -rf /var/lib/apt/lists/*

# =========================
# zig build tool
# =========================
RUN pip3 install --no-cache-dir cargo-zigbuild --break-system-packages

# =========================
# target selection（確実版）
# =========================
RUN case "$TARGETPLATFORM" in \
    linux/amd64) echo "x86_64-unknown-linux-musl" ;; \
    linux/arm64) echo "aarch64-unknown-linux-musl" ;; \
    *) echo "unsupported target" && exit 1 ;; \
    esac > /tmp/target

RUN rustup target add $(cat /tmp/target)

# =========================
# dependency cache layer（安定重視）
# =========================
COPY Cargo.toml Cargo.lock ./

RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo fetch

# =========================
# source
# =========================
COPY . .

# =========================
# build (stable + fast)
# =========================
RUN set -eux; \
    TARGET=$(cat /tmp/target); \
    export CARGO_INCREMENTAL=0; \
    export CARGO_NET_GIT_FETCH_WITH_CLI=true; \
    cargo zigbuild --release --target $TARGET; \
    install -m 0755 target/$TARGET/release/kanidm_init /out

# =========================
# runtime
# =========================
FROM docker.io/kanidm/server:latest

COPY --from=builder /out /sbin/kanidm_init

RUN chmod +x /sbin/kanidm_init
