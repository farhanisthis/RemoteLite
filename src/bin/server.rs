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
    // Render provides the port in the PORT environment variable
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
