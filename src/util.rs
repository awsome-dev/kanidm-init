use chrono::{Duration, Local};
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
    // 現在時刻から1時間後の日時を計算（ローカル時間表示）
    let expiry_time = (Local::now() + Duration::seconds(3600))
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    let origin = k_conf.origin.trim_end_matches('/');
    let username = &b_conf.person;
    
    let readme_content = format!(
r#"# Kanidm Setup (Passkey / TPM / SE)

Kanidm のデプロイが完了しました。以下の登録用トークンを使用して、パスワードレス設定を行ってください。

## 初期登録手順
1. ブラウザで以下の登録用URLにアクセスしてください。
    > **URL**: {url}/ui/reset?token={token}

2. 画面が表示されたら、以下のユーザー名を入力し **「Begin」** ボタンを押してください。
    - **Username**: `{username}`

3. Webauthn（Passkey）の登録プロセスが開始されます。登録が完了すると、以降は生体認証やセキュリティキー（指紋認証、顔認証、YubiKey等）のみでログインが可能になります。

## 有効期限
このトークンは以下の日時に失効します。
- **Expiry**: {expiry} （発行から1時間）

## CLIによる登録
CLI環境からは、以下のコマンドを使用して登録することも可能です。
```bash
kanidm person credential use-reset-token {token}
```"#,
        url = origin,
        token = token,
        username = username,
        expiry = expiry_time
    );

    let readme_path = Path::new(&b_conf.readme_dir).join("ReadMe_idm.md");

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
