# syntax=docker/dockerfile:1.6

FROM --platform=$BUILDPLATFORM rust:1.95-slim AS builder

ARG TARGETPLATFORM
WORKDIR /usr/src/init

# -------------------------
# minimal deps（ここが速さの本体）
# -------------------------
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    gcc \
 && rm -rf /var/lib/apt/lists/*

# -------------------------
# Zig build only
# -------------------------
RUN pip3 install --no-cache-dir cargo-zigbuild --break-system-packages

# -------------------------
# target
# -------------------------
RUN case "$TARGETPLATFORM" in \
    linux/amd64) echo "x86_64-unknown-linux-musl" ;; \
    linux/arm64) echo "aarch64-unknown-linux-musl" ;; \
    *) exit 1 ;; \
    esac > /tmp/target

RUN rustup target add $(cat /tmp/target)

# -------------------------
# dependency cache（ここが唯一の高速化ポイント）
# -------------------------
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo fetch

# -------------------------
# source
# -------------------------
COPY . .

# -------------------------
# build（最速設定）
# -------------------------
RUN set -eux; \
    TARGET=$(cat /tmp/target); \
    CARGO_PROFILE_RELEASE_LTO=true \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
    cargo zigbuild --release --target $TARGET; \
    install -m 0755 target/$TARGET/release/kanidm_init /out

# -------------------------
# runtime
# -------------------------
FROM docker.io/kanidm/server:latest

COPY --from=builder /out /sbin/kanidm_init
