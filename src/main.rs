use anyhow::Result;
use eframe::egui;
use remotelite::app::RemoteLiteApp;
use remotelite::config::AppConfig;
use remotelite::server::run_signaling_server;
use std::env;

fn main() -> Result<()> {
    // Install default cryptographic provider for Rustls (required for wss:// connections)
    let _ = rustls::crypto::ring::default_provider().install_default();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,remotelite=debug".into()),
        )
        .init();

    let args: Vec<String> = env::args().collect();

    // Mode 1: Dedicated Relay & Signaling Server mode via CLI
    if args.iter().any(|arg| arg == "--server" || arg == "-s") {
        let bind_addr = args
            .get(2)
            .map(|s| s.as_str())
            .unwrap_or("0.0.0.0:9090");
        
        println!("Starting InstantLink Signaling & Relay Server on {}...", bind_addr);
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        return rt.block_on(run_signaling_server(bind_addr));
    }

    // Mode 2: GUI Application
    // Initialize persistent background tokio runtime for network & capture tasks
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _guard = rt.enter();

    // Load or generate persistent 9-digit device configuration
    let config = AppConfig::load_or_create()?;
    tracing::info!("Loaded device configuration with ID: {}", config.device_id);

    let viewport = egui::ViewportBuilder::default()
        .with_title("InstantLink")
        .with_inner_size([1040.0, 680.0])
        .with_min_inner_size([920.0, 600.0]);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "InstantLink",
        native_options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.window_rounding = 12.0.into();
            visuals.panel_fill = egui::Color32::from_rgb(11, 35, 38);
            cc.egui_ctx.set_visuals(visuals);

            Ok(Box::new(RemoteLiteApp::new(cc, config)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("EFrame error: {:?}", e))?;

    Ok(())
}
