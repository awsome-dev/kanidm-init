use thiserror::Error;
use kanidm_client::ClientError;
use http::StatusCode;

#[derive(Error, Debug)]
pub enum AppError {
    /// ネットワークやOSレベルのIOエラー
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOMLのパースエラー
    #[error("Config parse error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Kanidmクライアント固有のエラーをラップ
    /// ClientErrorがDisplayを実装していないため、{:?} (Debug) で出力
    #[error("Kanidm client error: {0:?}")]
    Kanidm(ClientError),

    /// 文脈（コンテキスト）付きのエラー
    #[error("{message}: {source}")]
    WithContext {
        message: String,
        #[source]
        source: Box<AppError>,
    },

    /// その他のエラー
    #[error("Unknown error: {0}")]
    Other(String),
}

impl AppError {
    /// エラーが HTTP 409 Conflict かどうかを型安全に判定する
    pub fn is_conflict(&self) -> bool {
        match self {
            // anyhowの文字列検索ではなく、型とステータスコードで直接比較
            AppError::Kanidm(ClientError::Http(StatusCode::CONFLICT, _, _)) => true,
            // コンテキストに包まれている場合は中身を再帰的に確認
            AppError::WithContext { source, .. } => source.is_conflict(),
            _ => false,
        }
    }

    /// anyhow の .context() 同様の機能を提供
    pub fn context<S: Into<String>>(self, message: S) -> Self {
        AppError::WithContext {
            message: message.into(),
            source: Box::new(self),
        }
    }
}

/// 独自Result型の定義
pub type AppResult<T> = std::result::Result<T, AppError>;

/// ClientError から AppError への自動変換を実装
impl From<ClientError> for AppError {
    fn from(err: ClientError) -> Self {
        AppError::Kanidm(err)
    }
}
