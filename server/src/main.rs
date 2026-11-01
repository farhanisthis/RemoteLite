use anyhow::Result;
use remotelite_server::run_signaling_server;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,remotelite_server=debug".into()),
        )
        .init();

    let args: Vec<String> = env::args().collect();
    let bind_addr = env::var("PORT")
        .map(|p| format!("0.0.0.0:{}", p))
        .unwrap_or_else(|_| {
            args.get(1)
                .cloned()
                .unwrap_or_else(|| "0.0.0.0:9090".to_string())
        });

    println!("Starting RemoteLite Standalone Server on {}...", bind_addr);
    run_signaling_server(&bind_addr).await
}
