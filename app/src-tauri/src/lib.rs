//! Subquest — lado Rust do app: estado, comandos chamados pela interface e
//! ponte com o núcleo em C++.

// TODO(fase 1): remover os allow(dead_code) quando overlay e tradução forem ligados ao fluxo.
#[allow(dead_code)]
mod core_ffi;
mod settings;
#[allow(dead_code)]
mod translate;

use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use core_ffi::{Core, SqRect, WindowEntry};
use settings::Settings;

/// Estado compartilhado entre os comandos.
struct AppState {
    core: Result<Core, String>, // Err = DLL ausente ou incompatível (o app abre mesmo assim)
    settings: Mutex<Settings>,
    config_dir: PathBuf,
    session_user_data: Mutex<Option<*mut AppHandle>>, // AppHandle emprestado ao núcleo durante a sessão
}

// SAFETY: o ponteiro só é criado/liberado com o Mutex travado, e o AppHandle é Send + Sync.
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

/// Situação do núcleo, para a interface mostrar um aviso claro quando faltar a DLL.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoreStatus {
    loaded: bool,
    api_version: Option<u32>,
    error: Option<String>,
}

/// Fala reconhecida pelo OCR, enviada à interface como evento.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OcrTextEvent {
    text: String,
    frame_id: u64,
    ocr_ms: f64,
}

/// Retorna o núcleo carregado ou a mensagem de por que não carregou.
fn core(state: &AppState) -> Result<&Core, String> {
    state.core.as_ref().map_err(Clone::clone)
}

/// Informa se a DLL do núcleo foi carregada.
#[tauri::command]
fn core_status(state: State<'_, AppState>) -> CoreStatus {
    match &state.core {
        Ok(c) => CoreStatus {
            loaded: true,
            api_version: Some(c.api_version()),
            error: None,
        },
        Err(e) => CoreStatus {
            loaded: false,
            api_version: None,
            error: Some(e.clone()),
        },
    }
}

/// Lista as janelas abertas para o usuário escolher o jogo.
#[tauri::command]
fn list_windows(state: State<'_, AppState>) -> Result<Vec<WindowEntry>, String> {
    core(&state)?.list_windows()
}

/// Callback chamado pelo núcleo (thread de OCR): repassa o texto para a interface.
/// TODO(fase 1): aqui entra a tradução (cache -> tradutor -> sq_overlay_set_text).
unsafe extern "C" fn on_ocr_text(
    text: *const c_char,
    frame_id: u64,
    ocr_ms: f64,
    user: *mut c_void,
) {
    let app = &*(user as *const AppHandle);
    let text = CStr::from_ptr(text).to_string_lossy().into_owned();
    let _ = app.emit(
        "ocr-text",
        OcrTextEvent {
            text,
            frame_id,
            ocr_ms,
        },
    );
}

/// Começa a traduzir a janela escolhida, na região escolhida.
#[tauri::command]
fn start_session(
    app: AppHandle,
    state: State<'_, AppState>,
    window_handle: u64,
    region: Option<(i32, i32, i32, i32)>,
) -> Result<(), String> {
    let core = core(&state)?;
    let language = state.settings.lock().unwrap().source_language.clone();
    let (x, y, width, height) = region.unwrap_or_default();

    let mut slot = state.session_user_data.lock().unwrap();
    let user = Box::into_raw(Box::new(app));
    let result = core.start(
        window_handle,
        SqRect {
            x,
            y,
            width,
            height,
        },
        &language,
        on_ocr_text,
        user as *mut c_void,
    );
    match result {
        Ok(()) => *slot = Some(user),
        // SAFETY: o núcleo recusou a sessão, então não guardou o ponteiro.
        Err(_) => drop(unsafe { Box::from_raw(user) }),
    }
    result
}

/// Para a sessão e devolve o AppHandle emprestado ao núcleo.
#[tauri::command]
fn stop_session(state: State<'_, AppState>) -> Result<(), String> {
    let result = core(&state)?.stop();
    if let Some(user) = state.session_user_data.lock().unwrap().take() {
        // SAFETY: sq_stop já esperou a thread de OCR terminar; ninguém mais usa o ponteiro.
        drop(unsafe { Box::from_raw(user) });
    }
    result
}

/// Devolve as configurações atuais.
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

/// Substitui e grava as configurações.
#[tauri::command]
fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    settings::save(&state.config_dir, &settings)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}

/// Monta e roda o app: carrega o núcleo e as configurações e registra os comandos.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let resource_dir = app.path().resource_dir().ok();

            let mut errors = Vec::new();
            let mut loaded = None;
            for path in core_ffi::candidate_paths(resource_dir) {
                match Core::load(&path) {
                    Ok(c) => {
                        loaded = Some(c);
                        break;
                    }
                    Err(e) => errors.push(e),
                }
            }
            let core = loaded.ok_or_else(|| errors.join("\n"));

            app.manage(AppState {
                core,
                settings: Mutex::new(settings::load(&config_dir)),
                config_dir,
                session_user_data: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            core_status,
            list_windows,
            start_session,
            stop_session,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o Subquest");
}
