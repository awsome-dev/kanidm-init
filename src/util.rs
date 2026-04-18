use crate::{conf::{KanidmConfig, BootstrapConfig}};
use crate::error::{AppError, AppResult};
use std::path::Path;
use std::fs;

/// セットアップ完了後のReadMeを作成・保存する
pub fn save_setup_readme(
    k_conf: &KanidmConfig,
    b_conf: &BootstrapConfig,
    token: &str,
) -> AppResult<()> {
    let readme_content = format!(
r#"# Kanidm Setup (Passkey / TPM / SE)

Kanidm のデプロイが完了しました。以下の初期認証情報を使用してログインし、パスワードレス設定を行ってください。

## 初期ログイン手順
1. ブラウザで管理画面（ {}/ui/login ）にアクセス
2. 以下の認証情報を入力してログイン
    - **Username**: `{}`
    - **Password**: `{}`
3. ログイン後、ただちに正規のパスワードへ変更してください。

## パスワードレス（WebAuthn）の設定
1. 設定メニューから "Passkey / WebAuthn" を登録。
2. 以降は、生体認証やセキュリティキー（指紋認証、顔認証、YubiKey等）のみでログインが可能になります。"#,
        k_conf.origin.trim_end_matches('/'),
        b_conf.person,
        token
    );

    let readme_path = Path::new(&b_conf.readme_dir).join("ReadMe.md");

    // ディレクトリ作成
    if let Some(parent) = readme_path.parent() {
        match fs::create_dir_all(parent) {
            Err(e) => return Err(AppError::from(e).context(format!("Failed to create directory: {:?}", parent))),
            Ok(_) => (),
        }
    }

    // ファイル書き込み
    match fs::write(&readme_path, readme_content) {
        Err(e) => Err(AppError::from(e).context(format!("Failed to write ReadMe to {:?}", readme_path))),
        Ok(_) => {
            println!("Success! Instructions and token saved to {:?}", readme_path);
            Ok(())
        }
    }
}
