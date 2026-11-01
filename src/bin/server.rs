use anyhow::Result;
use remotelite::server::run_signaling_server;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,remotelite=debug".into()),
        )
        .init();

    let args: Vec<String> = env::args().collect();
    let bind_addr = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("0.0.0.0:9090");

    println!("Starting RemoteLite Standalone Server on {}...", bind_addr);
    run_signaling_server(bind_addr).await
}
