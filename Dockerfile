# syntax=docker/dockerfile:1.6

# --- Stage 1: build ---
FROM --platform=$BUILDPLATFORM rust:1.95-slim AS builder

ARG TARGETPLATFORM
WORKDIR /usr/src/init

# -------------------------
# system deps
# -------------------------
RUN apt-get update && apt-get install -y \
    curl \
    python3 \
    python3-pip \
    pkg-config \
    libssl-dev \
    gcc \
 && rm -rf /var/lib/apt/lists/*

# -------------------------
# rust cache hint (BuildKit優先)
# -------------------------
ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# -------------------------
# zig build helper
# -------------------------
RUN pip3 install --no-cache-dir cargo-zigbuild --break-system-packages

# -------------------------
# target setup
# -------------------------
RUN case "$TARGETPLATFORM" in \
    "linux/amd64") echo "x86_64-unknown-linux-musl" ;; \
    "linux/arm64") echo "aarch64-unknown-linux-musl" ;; \
    *) echo "unknown target" && exit 1 ;; \
    esac > /tmp/target

RUN rustup target add $(cat /tmp/target)

# -------------------------
# dependency cache layer (critical)
# -------------------------
COPY Cargo.toml Cargo.lock ./

RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo fetch

# -------------------------
# source
# -------------------------
COPY . .

# -------------------------
# build
# -------------------------
RUN set -eux; \
    TARGET=$(cat /tmp/target); \
    cargo zigbuild --release --target $TARGET; \
    cp target/$TARGET/release/kanidm_init /usr/local/bin/kanidm_init

# --- Stage 2: runtime ---
FROM docker.io/kanidm/server:latest

COPY --from=builder /usr/local/bin/kanidm_init /sbin/kanidm_init

RUN chmod +x /sbin/kanidm_init
