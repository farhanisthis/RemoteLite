// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Duration;
use openremote_authorization::registry::TrustedDeviceRegistry;
use openremote_host_service::{HostConfig, HostService};
use openremote_protocol::pairing::{DevicePermissions, QrPairingPayload, TrustedDevice};
use openremote_protocol::types::{DeviceId, Platform};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

struct AppState {
    service: Arc<Mutex<HostService>>,
    signaling_url: Mutex<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HostInfo {
    pub device_id: String,
    pub display_name: String,
    pub public_key: String,
    pub storage_path: String,
    pub registry_path: String,
    pub signaling_url: String,
    pub lan_ip: String,
    pub is_online: bool,
    pub trusted_device_count: usize,
    pub active_session_count: usize,
}

fn get_lan_ip() -> String {
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if socket.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = socket.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

#[tauri::command]
fn get_host_info(state: State<AppState>) -> Result<HostInfo, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    let signaling_url = state.signaling_url.lock().map_err(|e| e.to_string())?.clone();
    let devices = service.list_trusted_devices().unwrap_or_default();
    let lan_ip = get_lan_ip();

    // Check if coordination server is running on port 3000
    let is_online = std::net::TcpStream::connect("127.0.0.1:3000").is_ok();

    Ok(HostInfo {
        device_id: service.device_id().to_string(),
        display_name: service.display_name().to_string(),
        public_key: service.public_key(),
        storage_path: "%ProgramData%\\OpenRemote\\identity.dat".to_string(),
        registry_path: "%ProgramData%\\OpenRemote\\trusted_devices.json".to_string(),
        signaling_url,
        lan_ip,
        is_online,
        trusted_device_count: devices.len(),
        active_session_count: 0,
    })
}

#[tauri::command]
fn get_trusted_devices(state: State<AppState>) -> Result<Vec<TrustedDevice>, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    service.list_trusted_devices().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_pairing_invitation(
    state: State<AppState>,
    server_url: Option<String>,
) -> Result<QrPairingPayload, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    let url = server_url.unwrap_or_else(|| state.signaling_url.lock().unwrap().clone());
    let invitation = service.create_pairing_invitation(&url, Duration::minutes(10));
    Ok(invitation)
}

#[tauri::command]
fn set_unattended_access(
    state: State<AppState>,
    device_id: String,
    enabled: bool,
) -> Result<(), String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    let uuid = Uuid::parse_str(&device_id).map_err(|e| e.to_string())?;
    service
        .set_unattended_access(&DeviceId::from_uuid(uuid), enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_device_permission(
    state: State<AppState>,
    device_id: String,
    permission: String,
    enabled: bool,
) -> Result<(), String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    let uuid = Uuid::parse_str(&device_id).map_err(|e| e.to_string())?;
    service
        .set_device_permission(&DeviceId::from_uuid(uuid), &permission, enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn revoke_device(state: State<AppState>, device_id: String) -> Result<bool, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    let uuid = Uuid::parse_str(&device_id).map_err(|e| e.to_string())?;
    service
        .revoke_device(&DeviceId::from_uuid(uuid))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_simulated_device(
    state: State<AppState>,
    display_name: String,
    platform: String,
    unattended: bool,
) -> Result<TrustedDevice, String> {
    let service = state.service.lock().map_err(|e| e.to_string())?;
    let platform_enum = match platform.to_lowercase().as_str() {
        "android" => Platform::Android,
        "ios" => Platform::Ios,
        "windows" => Platform::Windows,
        "linux" => Platform::Linux,
        _ => Platform::Android,
    };

    let device = TrustedDevice {
        device_id: DeviceId::new(),
        display_name,
        platform: platform_enum,
        public_key: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            uuid::Uuid::new_v4().as_bytes(),
        ),
        paired_at: chrono::Utc::now(),
        last_seen_at: Some(chrono::Utc::now()),
        unattended_access_enabled: unattended,
        permissions: DevicePermissions::default(),
    };

    let reg_arc = service.registry();
    let mut reg = reg_arc.lock().map_err(|e| e.to_string())?;
    reg.insert(device.clone()).map_err(|e| e.to_string())?;

    Ok(device)
}

#[tauri::command]
fn set_signaling_url(state: State<AppState>, url: String) -> Result<(), String> {
    let mut sig = state.signaling_url.lock().map_err(|e| e.to_string())?;
    *sig = url;
    Ok(())
}

fn main() {
    let config = HostConfig::default();
    let lan_ip = get_lan_ip();
    let default_sig_url = format!("ws://{}:3000/ws", lan_ip);

    let host_service = match HostService::init(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize host service: {}", e);
            // Fallback in case of permissions error
            let temp_config = HostConfig {
                storage_path: std::env::temp_dir().join("openremote_id.dat"),
                registry_path: std::env::temp_dir().join("openremote_trusted.json"),
                display_name: std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Host-PC".into()),
                signaling_url: None,
            };
            HostService::init(temp_config).expect("Fallback host service init must succeed")
        }
    };

    let app_state = AppState {
        service: Arc::new(Mutex::new(host_service)),
        signaling_url: Mutex::new(default_sig_url),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_host_info,
            get_trusted_devices,
            create_pairing_invitation,
            set_unattended_access,
            set_device_permission,
            revoke_device,
            add_simulated_device,
            set_signaling_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenRemote desktop application");
}
