use crate::error::{AppError, AppResult};
use kanidm_client::KanidmClient;

/// Person (ユーザー) を作成する
pub async fn create(
    client: &KanidmClient,
    person_id: &str,
    displayname: &str,
) -> AppResult<()> {
    client
        .idm_person_account_create(person_id, displayname)
        .await
        .map_err(|e| AppError::from(e).context("Failed to create person"))?;
    Ok(())
}

/// ユーザーのリセットトークンを発行する
/// 成功するとトークン文字列を返す
pub async fn generate_reset_token(
    client: &KanidmClient,
    person_id: &str,
) -> AppResult<String> {
    let intent_token = client
        .idm_person_account_credential_update_intent(person_id, Some(3600))
        .await
        .map_err(|e| AppError::from(e).context("Failed to generate reset token intent"))?;
    
    Ok(intent_token.token)
}

/// 指定したグループにPersonを追加する (idm_admins等)
pub async fn add_to_group(
    client: &KanidmClient, 
    person_id: &str, 
    group_id: &str
) -> AppResult<()> {
    let members = [person_id];
    client
        .idm_group_add_members(group_id, &members)
        .await
        .map_err(|e| AppError::from(e).context("Failed to add person to group"))?;
    Ok(())
}

/// Personが特定のグループに所属しているか確認する
pub async fn is_member_of(
    client: &KanidmClient, 
    person_id: &str, 
    group_id: &str
) -> AppResult<bool> {
    let person_opt = client
        .idm_person_account_get(person_id)
        .await
        .map_err(|e| AppError::from(e).context("Failed to get person entry"))?;

    if let Some(entry) = person_opt {
        Ok(entry
            .attrs
            .get("memberof")
            .map(|members: &Vec<String>| members.iter().any(|m| m == group_id))
            .unwrap_or(false))
    } else {
        Ok(false)
    }
}

/// ユーザー(person)を削除する
pub async fn delete(client: &KanidmClient, person_id: &str) -> AppResult<()> {
    client
        .idm_person_account_delete(person_id)
        .await
        .map_err(|e| AppError::from(e).context("Failed to delete person"))?;
    Ok(())
}

/// 管理者の人数をカウントする
pub async fn count_admins(client: &KanidmClient) -> AppResult<usize> {
    let group_id = "idm_admins";
    
    // 1. グループエントリを取得
    let group_opt = client
        .idm_group_get(group_id)
        .await
        .map_err(|e| AppError::from(e).context("Failed to get group entry"))?;

    // 2. member 属性の数を数える
    if let Some(entry) = group_opt {
        Ok(entry
            .attrs
            .get("member")
            .map(|members| members.len())
            .unwrap_or(0))
    } else {
        // グループが存在しない（初期状態）
        Ok(0)
    }
}
