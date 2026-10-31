// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn get_host_status() -> String {
    "online".to_string()
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {:?}", info);
        let _ = std::fs::write("tauri_debug.log", &msg);
    }));

    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_host_status])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        let err_msg = format!("Tauri run error: {:?}", e);
        let _ = std::fs::write("tauri_debug.log", &err_msg);
    }
}
