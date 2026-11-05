// Esconde o console extra no Windows em builds de release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Ponto de entrada: toda a lógica fica na lib (padrão do Tauri v2).
fn main() {
    subquest_lib::run()
}
