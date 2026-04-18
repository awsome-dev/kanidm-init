#!/bin/bash
set -e

# 必要に応じてイメージ名を変更してください
IMAGE_NAME="kanidm-custom:latest"

echo "[1/2] Setting up Docker Buildx..."
docker buildx create --use --name kanidm-builder || docker buildx use kanidm-builder

echo "[2/2] Building Multi-arch image (amd64, arm64)..."
# 注意: マルチアーキテクチャの場合、--load (ローカル保存) は使えないため
# レポジトリへの --push か、イメージの確認のみを行う構成になります。
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t "${IMAGE_NAME}" \
  --push \
  -f Dockerfile .

echo "Done! Image pushed: ${IMAGE_NAME}"
