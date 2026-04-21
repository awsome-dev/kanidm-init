# --- Stage 1: kanidm_init の静的ビルド ---
FROM --platform=$BUILDPLATFORM rust:1.95-slim AS builder
ARG TARGETPLATFORM
WORKDIR /usr/src/init

# =========================
# ① システム依存（キャッシュレイヤー）
# =========================
RUN apt-get update && apt-get install -y \
    curl \
    python3 \
    python3-pip \
    pkg-config \
    libssl-dev \
    gcc \
 && rm -rf /var/lib/apt/lists/*

# Zig + cargo-zigbuild
RUN pip3 install cargo-zigbuild --break-system-packages

# =========================
# ② sccache導入（Rustコンパイルキャッシュ）
# =========================
RUN curl -L https://github.com/mozilla/sccache/releases/latest/download/sccache-x86_64-unknown-linux-musl.tar.gz \
    | tar -xz \
 && mv sccache-*/sccache /usr/local/bin/ \
 && chmod +x /usr/local/bin/sccache

ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/root/.cache/sccache

# =========================
# ③ 依存定義（キャッシュ最重要）
# =========================
COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main() {}" > src/main.rs

# 依存取得（ここがキャッシュポイント）
RUN cargo fetch

# =========================
# ④ Zigターゲット準備
# =========================
RUN export TARGET=$(case "$TARGETPLATFORM" in \
    "linux/amd64") echo "x86_64-unknown-linux-musl" ;; \
    "linux/arm64") echo "aarch64-unknown-linux-musl" ;; esac) && \
    rustup target add $TARGET

# =========================
# ⑤ ソース（最後にコピー）
# =========================
COPY . .

# =========================
# ⑥ ビルド
# =========================
RUN export TARGET=$(case "$TARGETPLATFORM" in \
    "linux/amd64") echo "x86_64-unknown-linux-musl" ;; \
    "linux/arm64") echo "aarch64-unknown-linux-musl" ;; esac) && \
    cargo zigbuild --release --target $TARGET && \
    find target -name kanidm_init -type f -executable | grep "release" | \
    xargs -I {} cp -v {} /usr/src/init/kanidm_init-bin && \
    chmod +x /usr/src/init/kanidm_init-bin


# --- Stage 2: 公式イメージ ---
FROM docker.io/kanidm/server:latest

COPY --from=builder --chmod=0755 /usr/src/init/kanidm_init-bin /sbin/kanidm_init
