use clap::Parser;
use kanidm_init::{conf::{determine_config_path, load_kanidm_config, load_bootstrap_config}};
use kanidm_init::client::prepare_admin_client;
use kanidm_init::error::{AppResult};
use kanidm_init::logic::execute_bootstrap_flow;

use std::os::unix::process::CommandExt;
use std::process::Command as StdCommand;
use tokio::process::Command as TokioCommand;
use std::path::Path;
use std::time::Duration;
use std::process::Stdio;

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
    
    // --- 1. バックグラウンドで kanidmd を一時起動 ---
    println!("Launching temporary kanidmd for initialization...");
    let mut temp_server = TokioCommand::new("/sbin/kanidmd")
        .args(&["server", "--config-path", &final_config_path])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir("/data")
        .env("HOME", "/data")
        .spawn()
        .expect("Failed to launch temporary kanidmd");

    // --- 2. Unixソケットの出現を待機 (最大30秒) ---
    let socket_path = "/tmp/kanidmd.sock";
    let mut found = false;
    for i in 0..30 {
        if Path::new(socket_path).exists() {
            println!("Kanidm socket found. Proceeding with init.");
            found = true;
            break;
        }
        if i % 5 == 0 { println!("Waiting for kanidmd socket..."); }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    if found {        
        // 2. クライアントの準備 (Recovery Code発行を伴う)
        let client = prepare_admin_client(&config_path, &cli.account, &k_conf).await?;    
        // 3. ブートストラップの実行
        execute_bootstrap_flow(client, k_conf, b_conf).await?;
        println!("--- Bootstrapping Phase Completed Successfully ---");
    } else {
        eprintln!("Error: Timeout waiting for kanidmd socket.");
    }

    // --- 4. 一時サーバーを終了させ、本番サーバーへ exec ---
    println!("Restarting kanidmd as PID 1...");
    let _ = temp_server.kill().await; // 一時プロセスを停止

    let err = StdCommand::new("/sbin/kanidmd")
        .args(&["server", "--config-path", &final_config_path])
        .current_dir("/data")  // これを忘れると、最後のリスタートで Permission Denied になる
        .env("HOME", "/data")   // 環境も引き継ぐ
        .exec();

    panic!("Failed to final exec kanidmd: {}", err);

}
