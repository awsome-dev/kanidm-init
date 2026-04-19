use clap::Parser;
use kanidm_init::{conf::{determine_config_path, load_kanidm_config, load_bootstrap_config}};
use kanidm_init::client::prepare_admin_client;
use kanidm_init::error::{AppResult};
use kanidm_init::logic::execute_bootstrap_flow;

use std::os::unix::process::CommandExt;
use std::process::Command;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    config_path: Option<String>,
    #[arg(long, default_value = "config.toml")]
    setup_config: String,
    #[arg(long, default_value = "idm_admin")]
    account: String,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();

    // 1. パスと設定の決定
    let config_path = determine_config_path(cli.config_path);
    let k_conf = load_kanidm_config(&config_path)?;
    let b_conf = load_bootstrap_config(&cli.setup_config)?;

    let final_config_path = config_path.clone();
    
    // 2. クライアントの準備 (Recovery Code発行を伴う)
    let client = prepare_admin_client(&config_path, &cli.account, &k_conf).await?;

    // 3. ブートストラップの実行
    execute_bootstrap_flow(client, k_conf, b_conf).await?;

    // 4. kanidmd server へのプロセス移譲 (シェルを介さず PID 1 を継承)
    println!("Starting kanidmd server...");
    let err = Command::new("/sbin/kanidmd")
        .args(&["server", "--config-path", &final_config_path])
        .exec();

    panic!("Failed to execute /sbin/kanidmd: {}", err);
}
