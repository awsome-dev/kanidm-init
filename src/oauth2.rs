use kanidm_client::KanidmClient;
use crate::error::{AppError, AppResult};
use url::Url;

/// OAuth2 Resource Server (App) を作成する
pub async fn create(
    client: &KanidmClient,
    name: &str,
    display_name: &str,
    origin_url: &str,
) -> AppResult<()> {
    client
        .idm_oauth2_rs_basic_create(name, display_name, origin_url)
        .await
        .map_err(|e| AppError::from(e).context("Failed to create oauth2 app"))?;
    Ok(())
}

/// Redirect URL を追加する
pub async fn add_redirect_url(
    client: &KanidmClient,
    name: &str,
    redirect_url: &str,
) -> AppResult<()> {
    let r_url = Url::parse(redirect_url)
        .map_err(|e| AppError::Other(format!("Invalid redirect URL format: {}", e)))?;
        
    client
        .idm_oauth2_client_add_origin(&name.to_string(), &r_url)
        .await
        .map_err(|e| AppError::from(e).context("Failed to add redirect URL"))?;
    Ok(())
}

/// Scope Map を更新する (固定で idm_admins グループに紐付け)
pub async fn add_scopes(
    client: &KanidmClient,
    name: &str,
    scopes: &[String],
) -> AppResult<()> {
    let scope_refs: Vec<&str> = scopes.iter().map(|s| s.as_str()).collect();
    client
        .idm_oauth2_rs_update_scope_map(name, "idm_admins", scope_refs)
        .await
        .map_err(|e| AppError::from(e).context("Failed to update scope map"))?;
    Ok(())
}

/// OAuth2 Resource Server (App) を削除する
pub async fn delete(client: &KanidmClient, name: &str) -> AppResult<()> {
    client
        .idm_oauth2_rs_delete(name)
        .await
        .map_err(|e| AppError::from(e).context("Failed to delete oauth2 app"))?;
    Ok(())
}
