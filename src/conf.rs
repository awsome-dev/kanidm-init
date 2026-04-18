use serde::Deserialize;

/// Kanidm 本体設定 (server.toml) に対応する構造
#[derive(Deserialize, Debug, Clone)]
pub struct KanidmConfig {
    pub version: String,
    pub bindaddress: String,
    pub db_path: String,
    pub tls_chain: String,
    pub tls_key: String,
    pub domain: String,
    pub origin: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BootstrapConfig {
    #[serde(default = "default_person")]
    pub person: String,

    #[serde(default = "default_display_person_name")]
    pub display_person_name: String,

    #[serde(default = "default_app_name")]
    pub app_name: String,

    #[serde(default = "default_display_app_name")]
    pub display_app_name: String,

    #[serde(default = "default_callback_url")]
    pub callback_url: String,

    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,

    #[serde(default = "default_readme_dir")]
    pub readme_dir: String,
}

fn default_readme_dir() -> String {
    ".".to_string()
}

// --- デフォルト値の定義 ---

fn default_person() -> String {
    "default_idm_admin".to_string()
}

fn default_display_person_name() -> String {
    "Default Administrator".to_string()
}

fn default_app_name() -> String {
    "internal_admin_portal".to_string()
}

fn default_display_app_name() -> String {
    "Internal Admin Portal".to_string()
}

fn default_callback_url() -> String {
    "https://admin.idm.example.internal/callback".to_string()
}

fn default_scopes() -> Vec<String> {
    vec![
        "email".to_string(),
        "profile".to_string(),
        "openid".to_string(),
    ]
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            person: default_person(),
            display_person_name: default_display_person_name(),
            app_name: default_app_name(),
            display_app_name: default_display_app_name(),
            callback_url: default_callback_url(),
            scopes: default_scopes(),
            readme_dir: default_readme_dir(),
        }
    }
}
