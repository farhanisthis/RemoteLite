// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn get_host_status() -> String {
    "online".to_string()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_host_status])
        .run(tauri::generate_context!())
        .expect("error while running OpenRemote desktop application");
}
