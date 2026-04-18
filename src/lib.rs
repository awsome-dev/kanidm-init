use crate::error::{AppError, AppResult};
use serde::Serialize;
use kanidmd_core::admin::{AdminTaskRequest, AdminTaskResponse, ClientCodec};
use kanidmd_core::config::{Configuration, ServerConfigUntagged};
use tokio::net::UnixStream;
use tokio_util::codec::Framed;
use futures::{SinkExt, StreamExt};
use std::path::PathBuf;

pub mod conf;
pub mod client;
pub mod person;
pub mod oauth2;
pub mod error;

// 共通レスポンス
#[derive(Serialize, Debug)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ApiResponse<T> {
    Success(T),
    Error { message: String, code: Option<u16> },
}

pub async fn execute_recovery(
    config_path: &str,
    target_account: &str,
) -> AppResult<String> {
    // 1. 設定ファイルの読み込み試行
    let path = PathBuf::from(config_path);
    let maybe_sconfig = if path.exists() {
        Some(
            ServerConfigUntagged::new(path)
                .map_err(|e| AppError::Other(e.to_string()).context(format!("Failed to parse server config at {}", config_path)))?
        )
    } else {
        None
    };

    // 2. Configuration のビルド
    let config = Configuration::build()
        .add_opt_toml_config(maybe_sconfig)
        .finish()
        .ok_or_else(|| AppError::Other("Configuration build failed. Check the error messages above for missing settings (like TLS or Domain).".to_string()))?;

    // 3. 管理用ソケットへの接続
    let socket_path = &config.adminbindpath;

    let stream = UnixStream::connect(socket_path).await
        .map_err(|e| AppError::from(e).context(format!("Failed to connect to admin socket at {}", socket_path)))?;

    let mut reqs = Framed::new(stream, ClientCodec);

    // 4. リカバリリクエストの構成と送信
    let req = AdminTaskRequest::RecoverAccount {
        name: target_account.to_string(),
    };

    reqs.send(req).await
        .map_err(|e| AppError::Other(format!("Failed to send recovery request: {:?}", e)))?;
    reqs.flush().await
        .map_err(|e| AppError::Other(format!("Failed to flush admin socket: {:?}", e)))?;

    // 5. レスポンス処理
    match reqs.next().await {
        Some(Ok(AdminTaskResponse::RecoverAccount { password })) => {
            Ok(password)
        }
        Some(Ok(AdminTaskResponse::Error)) => {
            Err(AppError::Other("The server encountered an error processing the recovery. Check server logs.".to_string()))
        }
        Some(Err(e)) => {
            Err(AppError::Other(format!("Codec/Protocol error: {:?}", e)))
        }
        None => {
            Err(AppError::Other("The admin socket was closed unexpectedly.".to_string()))
        }
        _ => Err(AppError::Other("Received an unexpected response type from the server.".to_string())),
    }
}
