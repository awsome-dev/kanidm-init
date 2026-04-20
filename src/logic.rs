use kanidm_client::{KanidmClient};
use futures::future::try_join_all;
use std::future::Future;
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
    // 管理者がWebAuthnを保持しているか（セットアップ完了済みか）を確認
    match person::admin_has_webauthn(&client).await {
        // WebAuthn保持者がいない場合（0人、1人、または2人いて未登録）
        Ok(false) => {
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
        // すでにWebAuthn保持者が存在する場合
        Ok(true) => {
            println!("Initial setup is already complete.");
            Ok(())
        }
        // 判定に失敗した場合
        Err(e) => Err(AppError::Other(format!("Failed to verify administrator status: {}", e))),
    }
}

/// 2つのコレクション(Option)を評価し、いずれか1つでも空でない実体があるか判定する
pub fn any_has_elements(
    set_a: Option<&Vec<String>>,
    set_b: Option<&Vec<String>>,
) -> bool {
    match (set_a, set_b) {
        (Some(v), _) if !v.is_empty() => true,
        (_, Some(v)) if !v.is_empty() => true,
        _ => false,
    }
}

/// 管理者リストから特定のIDを除外する（ターゲットの抽出）
pub fn filter_new_admins(members: &[String], default_id: &str) -> Vec<String> {
    members.iter().filter(|&m| m != default_id).cloned().collect()
}

/// 全員が未登録であることを判定する純粋な論理
pub fn is_all_pending_logic(results: Vec<bool>) -> bool {
    !results.into_iter().any(|has_reg| has_reg)
}

/// [Unit Test用] 通信を伴わず、クロージャで結果をシミュレートできるロジック
/// F が Future を返さない(同期)か、モック用のクロージャを受け取る設計
pub async fn is_new_admin_webauthn_pending_logic<F, Fut>(
    members: Vec<String>,
    default_admin: &str,
    check_webauthn: F,
) -> AppResult<bool>
where
    F: Fn(&str) -> Fut,
    Fut: Future<Output = AppResult<bool>>,
{
    match members.len() {
        2 => {
            let targets = filter_new_admins(&members, default_admin);
            match targets.is_empty() {
                true => Ok(false),
                false => {
                    let checks = targets.iter().map(|id| check_webauthn(id));
                    match try_join_all(checks).await {
                        Err(e) => Err(e),
                        Ok(results) => Ok(is_all_pending_logic(results)),
                    }
                }
            }
        }
        _ => Ok(false),
    }
}
