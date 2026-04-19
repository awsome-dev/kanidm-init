use kanidm_client::{KanidmClient};
use crate::{conf::{KanidmConfig, BootstrapConfig}};
use crate::{person, oauth2};
use crate::error::{AppError, AppResult};
use crate::util::save_setup_readme;

/// セットアップのメインロジック
pub async fn execute_bootstrap_flow(
    client: KanidmClient,
    k_conf: KanidmConfig,
    b_conf: BootstrapConfig,
) -> AppResult<()> {
    println!("Starting bootstrap flow...");
    match person::count_admins(&client).await {
        Ok(0) => {
            // PersonとOAuth2の作成を並行開始
            let (p_res, o_res) = tokio::join!(
                person::create(&client, &b_conf.person, &b_conf.display_person_name),
                oauth2::create(&client, &b_conf.app_name, &b_conf.display_app_name, &k_conf.origin)
            );
            match p_res {
                Err(e) if !e.is_conflict() => return Err(e.context("Failed to ensure person exists")),
                _ => {
                    // グループ追加
                    match person::add_to_group(&client, &b_conf.person, "idm_admins").await {
                        Err(e) => return Err(e.context("Failed to add to group")),
                        Ok(_) => {
                            // グループ追加に成功したらトークンを発行
                            match person::generate_reset_token(&client, &b_conf.person).await {
                                Err(e) => return Err(e.context("Failed to generate reset token")),
                                Ok(token) => {
                                    // READMEを保存
                                    match save_setup_readme(&k_conf, &b_conf, &token) {
                                        Err(e) => return Err(e),
                                        Ok(_) => println!("Group membership updated and setup README saved."),
                                    }
                                }
                            }
                        }
                    }
                    match o_res {
                        Err(e) if !e.is_conflict() => return Err(e.context("Failed to create OAuth2 app")),
                        _ => {
                            // Redirect URLとScopesの追加を並行実行
                            let (url_res, scope_res) = tokio::join!(
                                oauth2::add_redirect_url(&client, &b_conf.app_name, &b_conf.callback_url),
                                oauth2::add_scopes(&client, &b_conf.app_name, &b_conf.scopes)
                            );
                            match url_res {
                                Err(e) => return Err(e.context("Failed to set redirect URL")),
                                Ok(_) => match scope_res {
                                    Err(e) => return Err(e.context("Failed to sync scopes")),
                                    Ok(_) => {
                                        println!("Bootstrap successful.");
                                        return Ok(());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(count) => {
            println!("Initial setup is already complete ({} admins found).", count);
            Ok(())
        },
        Err(e) => Err(AppError::Other(format!("Failed to check admin count: {}", e))),
    }
}
