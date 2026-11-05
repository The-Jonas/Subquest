// Hides the extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Entry point: all the logic lives in the lib (Tauri v2 convention).
fn main() {
    subquest_lib::run()
}
