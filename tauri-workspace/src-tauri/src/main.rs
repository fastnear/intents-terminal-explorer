#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Logging is initialized by tauri-plugin-log in lib.rs
    explorer_tauri::run();
}
