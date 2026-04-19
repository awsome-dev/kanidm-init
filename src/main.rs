use clap::Parser;
use kanidm_init::{conf::{determine_config_path, load_kanidm_config, load_bootstrap_config}};
use kanidm_init::client::prepare_admin_client;
use kanidm_init::error::{AppResult};
use kanidm_init::logic::execute_bootstrap_flow;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    config_path: Option<String>,
    #[arg(long, default_value = "config.toml")]
    setup_config_path: String,
    #[arg(long, default_value = "idm_admin")]
    account: String,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();

    // 1. パスと設定の決定
    let config_path = determine_config_path(cli.config_path);
    let k_conf = load_kanidm_config(&config_path)?;
    let b_conf = load_bootstrap_config(&cli.setup_config_path)?;

    // 2. クライアントの準備 (Recovery Code発行を伴う)
    let client = prepare_admin_client(&config_path, &cli.account, &k_conf).await?;

    // 3. ブートストラップ儀式の実行
    execute_bootstrap_flow(client, k_conf, b_conf).await
}
