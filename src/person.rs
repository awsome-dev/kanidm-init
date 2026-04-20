use kanidm_client::KanidmClient;
use futures::future::try_join_all;
use crate::error::{AppError, AppResult};
use crate::logic::{any_has_elements, filter_new_admins, is_all_pending_logic};

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

/// 管理者グループのメンバーIDリストを取得する
async fn get_admin_members(client: &KanidmClient) -> AppResult<Vec<String>> {
    match client.idm_group_get("idm_admins").await {
        Err(e) => Err(AppError::from(e).context("Failed to get group entry")),
        Ok(None) => Ok(vec![]),
        Ok(Some(entry)) => {
            match entry.attrs.get("member") {
                Some(members) => Ok(members.clone()),
                None => Ok(vec![]),
            }
        }
    }
}

/// 指定したユーザーのWebAuthn登録状況を確認する
pub async fn has_webauthn_registrations(client: &KanidmClient, person_id: &str) -> AppResult<bool> {
    match client.idm_person_account_get(person_id).await {
        Err(e) => Err(AppError::from(e).context("Failed to get person entry")),
        Ok(None) => Ok(false),
        Ok(Some(entry)) => {
            Ok(any_has_elements(
                entry.attrs.get("passkey"),
                entry.attrs.get("attested_passkey"),
            ))
        }
    }
}

// --- 3. 統合層 (Facade) : 全体を繋ぐスッキリした流れ ---

/// 公開関数: ライフタイム問題を回避しつつ、論理的な流れを記述する
pub async fn is_new_admin_webauthn_pending(client: &KanidmClient) -> AppResult<bool> {
    let default_admin = "idm_admin";

    // 1. メンバー取得
    match get_admin_members(client).await {
        Err(e) => Err(e),
        Ok(members) => {
            // 2. ターゲット抽出（ロジックを利用）
            let targets = filter_new_admins(&members, default_admin);

            // 3. 通信の実行（clientの参照を直接使うことでライフタイム問題を回避）
            match (members.len(), targets.is_empty()) {
                (2, false) => {
                    let checks = targets.iter().map(|id| has_webauthn_registrations(client, id));
                    match try_join_all(checks).await {
                        Err(e) => Err(e),
                        Ok(results) => Ok(is_all_pending_logic(results)),
                    }
                }
                _ => Ok(false),
            }
        }
    }
}

/// 管理者がWebAuthn（認証デバイス）を登録済みか判定する
pub async fn admin_has_webauthn(client: &KanidmClient) -> AppResult<bool> {
    match count_admins(client).await {
        Err(e) => Err(AppError::Other(format!("Failed to check admin count: {}", e))),
        // 1人以下の場合は、初期管理者のみなので「WebAuthn保持者はいない」とみなす
        Ok(count) if count <= 1 => Ok(false),
        // 2人いる場合は、新管理者が登録を終えているかを確認
        Ok(_) => {
            // 登録待ち（pending）なら、保持（has）は false
            let pending = is_new_admin_webauthn_pending(client).await.unwrap_or(false);
            Ok(!pending)
        }
    }
}
