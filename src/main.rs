use clap::Parser;
use kanidm_init::{execute_recovery, conf::{KanidmConfig, BootstrapConfig}};
use kanidm_init::{person, oauth2};
use kanidm_init::client::create_client_with_recovery_code;
use kanidm_init::error::{AppError, AppResult};
use std::path::Path;
use std::fs;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    config_path: Option<String>,
    #[arg(long, default_value = "config.toml")]
    setup_config: String,
    #[arg(long, default_value = "idm_admin")]
    account: String,
}

/// セットアップ完了後のReadMeを作成・保存する
fn save_setup_readme(
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

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();

    // 1. パス決定（型を String に揃える最も素直な方法）
    let final_config_path = match cli.config_path {
        Some(path) => path,
        None => ["/data/server.toml", "/etc/kanidm/server.toml"]
            .iter()
            .find(|p| Path::new(p).exists())
            .map(|p| p.to_string())
            .unwrap_or_else(|| "/data/server.toml".to_string()),
    };

    // 2. 設定読み込み
    let k_toml = fs::read_to_string(&final_config_path)
        .map_err(|e| AppError::from(e).context(format!("Failed to read server config: {}", final_config_path)))?;
    let k_conf: KanidmConfig = toml::from_str(&k_toml)
        .map_err(|e| AppError::from(e).context("Failed to parse server TOML"))?;

    let b_conf: BootstrapConfig = if Path::new(&cli.setup_config).exists() {
        let b_toml = fs::read_to_string(&cli.setup_config)
            .map_err(|e| AppError::from(e).context("Failed to read bootstrap config"))?;
        toml::from_str(&b_toml).map_err(|e| AppError::from(e).context("Failed to parse bootstrap TOML"))?
    } else {
        BootstrapConfig::default()
    };

    // 3. クライアント準備
    let password = execute_recovery(&final_config_path, &cli.account)
        .await
        .map_err(|e| AppError::Other(e.to_string()))?;

    let client = create_client_with_recovery_code(
        &k_conf.origin, &k_conf.tls_chain, &cli.account, &password,
    ).await?;

    // 4.
    println!("Starting bootstrap flow...");
    match person::count_admins(&client).await {
        Ok(0) => {
            // 4-1 & 4-3. PersonとOAuth2の作成を並行して開始
            // Rust 1.95では、こうした並行処理の結果をタプルで受け取り、
            // そのままmatchで分解する記述がよりスムーズになります。
            let (p_res, o_res) = tokio::join!(
                person::create(&client, &b_conf.person, &b_conf.display_person_name),
                oauth2::create(&client, &b_conf.app_name, &b_conf.display_app_name, &k_conf.origin)
            );
            match p_res {
                Err(e) if !e.is_conflict() => return Err(e.context("Failed to ensure person exists")),
                _ => {
                    // 4-2. person::add_to_group (依存: person::create)
                    let _ = person::add_to_group(&client, &b_conf.person, "idm_admins").await;
                    match o_res {
                        Err(e) if !e.is_conflict() => return Err(e.context("Failed to create OAuth2 app")),
                        _ => {
                            // 4-4 & 4-5. Redirect URLとScopesの追加を並行実行
                            // 依存: oauth2::create の成功
                            let (url_res, scope_res) = tokio::join!(
                                oauth2::add_redirect_url(&client, &b_conf.app_name, &b_conf.callback_url),
                                oauth2::add_scopes(&client, &b_conf.app_name, &b_conf.scopes)
                            );
                            match url_res {
                                Err(e) => return Err(e.context("Failed to set redirect URL")),
                                Ok(_) => match scope_res {
                                    Err(e) => return Err(e.context("Failed to sync scopes")),
                                    Ok(_) => {
                                        // 4-6. 最後にトークンを発行
                                        match person::generate_reset_token(&client, &b_conf.person).await {
                                            Err(e) => return Err(e.context("Failed to generate reset token")),
                                            Ok(token) => {
                                                // --- 外出しした関数を呼び出す ---
                                                match save_setup_readme(&k_conf, &b_conf, &token) {
                                                    Err(e) => return Err(e),
                                                    Ok(_) => println!("Bootstrap successful."),
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(count) => println!("Initial setup is already complete ({} admins found).", count),
        Err(e) => return Err(AppError::Other(format!("Failed to check admin count: {}", e))),
    }

    Ok(())
}
