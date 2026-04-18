use kanidm_client::{KanidmClient, KanidmClientBuilder};
use std::path::Path;
use crate::error::{AppError, AppResult};

/// AppResult に対して Conflict (HTTP 409) かどうかを判定する拡張トレイト
pub trait ConflictCheck {
    fn is_conflict(&self) -> bool;
}

impl<T> ConflictCheck for AppResult<T> {
    fn is_conflict(&self) -> bool {
        match self {
            Err(e) => e.is_conflict(),
            _ => false,
        }
    }
}

/// 取得済みのリカバリコードを使用して認証済みクライアントを生成する
pub async fn create_client_with_recovery_code(
    uri: &str,
    ca_path: &str,
    admin_id: &str,
    recovery_code: &str,
) -> AppResult<KanidmClient> {
    let mut builder = KanidmClientBuilder::new();
    builder = builder.address(uri.to_string());

    // CA証明書の読み込み
    if Path::new(ca_path).exists() {
        builder = builder
            .add_root_certificate_filepath(ca_path)
            .map_err(|e| AppError::from(e).context("Failed to load CA certificate"))?;
    } else {
        #[cfg(debug_assertions)]
        {
            // デバッグ時は証明書エラーを無視するオプションを許容
            builder = builder.danger_accept_invalid_certs(true);
        }
        #[cfg(not(debug_assertions))]
        {
            return Err(AppError::Other(format!(
                "Production error: CA certificate not found at {}",
                ca_path
            )));
        }
    }

    // クライアントのビルド
    let client = builder
        .build()
        .map_err(|e| AppError::from(e).context("Failed to build Kanidm client"))?;

    // リカバリコードをパスワードとして使用して認証
    client
        .auth_simple_password(admin_id, recovery_code)
        .await
        .map_err(|e| AppError::from(e).context("Authentication failed with recovery code"))?;

    Ok(client)
}
