#!/bin/sh
set -e

# ==============================================================================
# Kanidm Self-Contained Bootstrap Entrypoint
# ==============================================================================
# このスクリプトは、Kanidm サーバーの起動前に初期化ツール (kanidm_init) を実行し、
# 管理者アカウントの確立と初期設定を自動的に完結させます。
# ==============================================================================

echo "--- Bootstrapping Phase Start ---"

# 1. 初期設定プロセスの実行
# kanidm_init は内部で以下の「冪等性の判定」と「フェーズ 1~3」を順次実行する：
#   - [判定] idm_admins グループの人数を確認。1人以上なら即時終了。
#   - [フェーズ1] 特権パスワード (idm_admin) の抽出と一時有効化。
#   - [フェーズ2] 初期設定 TOML に基づくリソース作成 (ユーザー/OAuth2等)。
#   - [フェーズ3] 特権パスワードの再ランダム化による無効化。
#
# 引数解説:
#   --config-path: kanidmd 本体の server.toml パス
#   --setup-toml:  初期ユーザーや OAuth2 設定を記述した設定ファイルパス

if [ -f "/data/setup.toml" ]; then
    echo "Executing kanidm_init with /data/setup.toml..."
    /sbin/kanidm_init \
        --config-path /data/server.toml \
        --setup-toml /data/setup.toml
    echo "kanidm_init execution completed."
else
    echo "Warning: /data/setup.toml not found. Skipping initialization phase."
fi

echo "--- Bootstrapping Phase End ---"

# 2. 本来の Kanidm サーバー起動
# exec を使用することで、kanidmd が PID 1 として動作し、
# シグナルハンドリング（コンテナ停止時のクリーンアップ等）を正しく行えるようにする。

echo "Starting kanidmd server..."
exec /sbin/kanidmd server
