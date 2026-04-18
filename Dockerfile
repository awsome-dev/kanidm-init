# --- Stage 1: kanidm-init の静的ビルド ---
FROM --platform=$BUILDPLATFORM rust:1.95-slim AS builder
ARG TARGETPLATFORM
WORKDIR /usr/src/init
COPY . .

# クロスコンパイルに必要なパッケージを網羅
RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    libssl-dev \
    gcc-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    gcc-x86-64-linux-gnu \
    libc6-dev-amd64-cross \
    perl \
    make
    
# 変数の代入をサブシェル実行結果のキャプチャに変更し、後続コマンドへ確実に渡す
RUN export TARGET=$(case "$TARGETPLATFORM" in "linux/amd64") echo "x86_64-unknown-linux-musl" ;; "linux/arm64") echo "aarch64-unknown-linux-musl" ;; esac) && \
    export CC=$(case "$TARGETPLATFORM" in "linux/amd64") echo "x86_64-linux-gnu-gcc" ;; "linux/arm64") echo "aarch64-linux-gnu-gcc" ;; esac) && \
    export AR=$(case "$TARGETPLATFORM" in "linux/amd64") echo "x86_64-linux-gnu-ar" ;; "linux/arm64") echo "aarch64-linux-gnu-ar" ;; esac) && \
    rustup target add $TARGET && \
    # ターゲット固有の CC を明示的に指定してビルド
    # openssl-sys (vendored) はこれを見てターゲット用の C コンパイルを開始する
    CC=$CC AR=$AR cargo build --release --target $TARGET && \
    cp target/$TARGET/release/kanidm-init /usr/src/init/kanidm-init-bin

# --- Stage 2: 公式イメージへの組み込み ---
#FROM kanidm/kanidm:latest
FROM docker.io/kanidm/server:latest

# バイナリと起動スクリプトをコピー
COPY --from=builder /usr/src/init/kanidm-init-bin /sbin/kanidm-init
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

# 既存の CMD を上書きし、entrypoint.sh を経由させる
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
