# --- Stage 1: kanidm_init の静的ビルド ---
FROM --platform=$BUILDPLATFORM rust:1.95-slim AS builder

ARG TARGETPLATFORM
WORKDIR /usr/src/init

# =========================
# ① システム依存（最小＆安定）
# =========================
RUN apt-get update && apt-get install -y \
    curl \
    python3 \
    python3-pip \
    pkg-config \
    libssl-dev \
    gcc \
 && rm -rf /var/lib/apt/lists/*

# =========================
# ② Rustキャッシュは sccache ではなく BuildKit に寄せる
# =========================
ENV CARGO_INCREMENTAL=0

# =========================
# ③ Zig + cargo-zigbuild
# =========================
RUN pip3 install --no-cache-dir cargo-zigbuild --break-system-packages

# =========================
# ④ target解決
# =========================
RUN case "$TARGETPLATFORM" in \
      "linux/amd64") echo "x86_64-unknown-linux-musl" > /tmp/target ;; \
      "linux/arm64") echo "aarch64-unknown-linux-musl" > /tmp/target ;; \
    esac

RUN rustup target add $(cat /tmp/target)

# =========================
# ⑤ 依存キャッシュ層（最重要）
# =========================
COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch

# =========================
# ⑥ ソースコピー（最後）
# =========================
COPY . .

# =========================
# ⑦ ビルド
# =========================
RUN cargo zigbuild --release --target $(cat /tmp/target) \
 && cp target/$(cat /tmp/target)/release/kanidm_init /usr/local/bin/kanidm_init

# --- Stage 2: 公式イメージ ---
FROM docker.io/kanidm/server:latest

COPY --from=builder --chmod=0755 /usr/local/bin/kanidm_init /sbin/kanidm_init
