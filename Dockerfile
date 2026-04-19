# --- Stage 1: kanidm_init の静的ビルド ---
FROM --platform=$BUILDPLATFORM rust:1.95-slim AS builder
ARG TARGETPLATFORM
WORKDIR /usr/src/init
COPY . .

# Zig と cargo-zigbuild のインストール
# Zig が C コンパイラ (zig cc) として振る舞い、適切な musl ターゲットをリンクします
RUN apt-get update && apt-get install -y \
    curl \
    python3 \
    python3-pip \
    pkg-config \
    libssl-dev \
    && pip3 install cargo-zigbuild --break-system-packages

# Zig を利用したクロスビルド
RUN export TARGET=$(case "$TARGETPLATFORM" in "linux/amd64") echo "x86_64-unknown-linux-musl" ;; "linux/arm64") echo "aarch64-unknown-linux-musl" ;; esac) && \
    rustup target add $TARGET && \
    # cargo build の代わりに cargo zigbuild を使用
    # これにより、リンカーエラー (__isoc23_sscanf 等) を回避できます
    cargo zigbuild --release --target $TARGET && \
    echo "--- Target directory structure ---" && \
    find target -name kanidm_init -ls && \
    echo "--- Attempting to copy binary ---" && \
    find target -name kanidm_init -type f -executable | grep "release" | xargs -I {} cp -v {} /usr/src/init/kanidm_init-bin && \
    chmod +x /usr/src/init/kanidm_init-bin

# entrypoint.sh に権限を付与（Stage 2 で chmod が使えないため）
# COPY entrypoint.sh /usr/src/init/entrypoint.sh
# RUN chmod +x /usr/src/init/entrypoint.sh

# --- Stage 2: 公式イメージへの組み込み ---
FROM docker.io/kanidm/server:latest

# バイナリをコピー
COPY --from=builder /usr/src/init/kanidm_init-bin /sbin/kanidm_init
# 権限付与済みのファイルを builder からコピー
# COPY --from=builder /usr/src/init/entrypoint.sh /usr/local/bin/entrypoint.sh

# 既存の CMD を上書きし、entrypoint.sh を経由させる
# ENTRYPOINT ["/sbin/kanidm_init"]
