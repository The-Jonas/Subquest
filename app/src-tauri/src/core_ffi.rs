//! Ponte com a `subquest_core.dll` (C++), carregada em tempo de execução.
//!
//! Por que carregar dinamicamente (libloading) em vez de linkar?
//! - O app abre mesmo sem a DLL e mostra um erro amigável em vez de não iniciar.
//! - Dá para trocar a DLL em dev sem recompilar o Rust.
//!
//! As assinaturas abaixo DEVEM bater com `core/include/subquest/core_api.h`.

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};

use libloading::Library;

/// Versão da ABI que este código espera (`SQ_API_VERSION` no header).
pub const EXPECTED_API_VERSION: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct SqRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[repr(C)]
pub struct SqSessionConfig {
    pub window_handle: u64,
    pub region: SqRect,
    pub ocr_language: *const c_char,
    pub settle_frames: u32,
    pub change_threshold: f32,
}

pub type SqTextCallback = unsafe extern "C" fn(*const c_char, u64, f64, *mut c_void);
pub type SqWindowCallback = unsafe extern "C" fn(u64, *const c_char, *const c_char, *mut c_void);

type FnApiVersion = unsafe extern "C" fn() -> u32;
type FnListWindows = unsafe extern "C" fn(SqWindowCallback, *mut c_void) -> i32;
type FnStart = unsafe extern "C" fn(*const SqSessionConfig, SqTextCallback, *mut c_void) -> i32;
type FnStop = unsafe extern "C" fn() -> i32;
type FnSetText = unsafe extern "C" fn(*const c_char) -> i32;
type FnSetVisible = unsafe extern "C" fn(i32) -> i32;
type FnLastError = unsafe extern "C" fn() -> *const c_char;

/// Janela capturável, como o núcleo a reporta.
#[derive(Clone, Debug, serde::Serialize)]
pub struct WindowEntry {
    pub handle: u64,
    pub title: String,
    pub process: String,
}

/// Biblioteca carregada + ponteiros para as funções exportadas.
/// A `Library` precisa viver enquanto os ponteiros forem usados.
pub struct Core {
    _lib: Library,
    api_version: FnApiVersion,
    list_windows: FnListWindows,
    start: FnStart,
    stop: FnStop,
    overlay_set_text: FnSetText,
    overlay_set_visible: FnSetVisible,
    last_error: FnLastError,
}

/// Lê um símbolo da DLL e copia o ponteiro de função (falha com o nome do símbolo).
unsafe fn symbol<T: Copy>(lib: &Library, name: &str) -> Result<T, String> {
    lib.get::<T>(name.as_bytes())
        .map(|s| *s)
        .map_err(|e| format!("símbolo `{name}` não encontrado: {e}"))
}

impl Core {
    /// Carrega a DLL do caminho dado, resolve todos os símbolos e confere a versão da ABI.
    pub fn load(path: &Path) -> Result<Self, String> {
        // SAFETY: a DLL é nossa e as assinaturas seguem o core_api.h.
        unsafe {
            let lib = Library::new(path)
                .map_err(|e| format!("não consegui abrir {}: {e}", path.display()))?;
            let core = Core {
                api_version: symbol(&lib, "sq_api_version")?,
                list_windows: symbol(&lib, "sq_list_windows")?,
                start: symbol(&lib, "sq_start")?,
                stop: symbol(&lib, "sq_stop")?,
                overlay_set_text: symbol(&lib, "sq_overlay_set_text")?,
                overlay_set_visible: symbol(&lib, "sq_overlay_set_visible")?,
                last_error: symbol(&lib, "sq_last_error")?,
                _lib: lib,
            };
            let version = (core.api_version)();
            if version != EXPECTED_API_VERSION {
                return Err(format!(
                    "versão da ABI incompatível: DLL {version}, app {EXPECTED_API_VERSION}"
                ));
            }
            Ok(core)
        }
    }

    /// Versão da ABI informada pela DLL.
    pub fn api_version(&self) -> u32 {
        unsafe { (self.api_version)() }
    }

    /// Lista as janelas capturáveis, na ordem em que estão na tela (a da frente primeiro).
    pub fn list_windows(&self) -> Result<Vec<WindowEntry>, String> {
        // Callback em C que empurra cada janela no Vec recebido via user_data.
        unsafe extern "C" fn collect(
            handle: u64,
            title: *const c_char,
            process: *const c_char,
            user: *mut c_void,
        ) {
            let out = &mut *(user as *mut Vec<WindowEntry>);
            out.push(WindowEntry {
                handle,
                title: CStr::from_ptr(title).to_string_lossy().into_owned(),
                process: CStr::from_ptr(process).to_string_lossy().into_owned(),
            });
        }
        let mut windows: Vec<WindowEntry> = Vec::new();
        let status = unsafe { (self.list_windows)(collect, &mut windows as *mut _ as *mut c_void) };
        self.check(status)?;
        Ok(windows)
    }

    /// Inicia a sessão. `user_data` é repassado intacto para o callback na thread de OCR.
    pub fn start(
        &self,
        window_handle: u64,
        region: SqRect,
        ocr_language: &str,
        callback: SqTextCallback,
        user_data: *mut c_void,
    ) -> Result<(), String> {
        let lang = CString::new(ocr_language).map_err(|e| e.to_string())?;
        let config = SqSessionConfig {
            window_handle,
            region,
            ocr_language: lang.as_ptr(),
            settle_frames: 0,      // 0 = usar o padrão do núcleo
            change_threshold: 0.0, // idem
        };
        let status = unsafe { (self.start)(&config, callback, user_data) };
        self.check(status)
    }

    /// Para a sessão ativa.
    pub fn stop(&self) -> Result<(), String> {
        let status = unsafe { (self.stop)() };
        self.check(status)
    }

    /// Mostra a tradução no overlay.
    pub fn overlay_set_text(&self, text: &str) -> Result<(), String> {
        let text = CString::new(text).map_err(|e| e.to_string())?;
        let status = unsafe { (self.overlay_set_text)(text.as_ptr()) };
        self.check(status)
    }

    /// Mostra ou esconde o overlay.
    pub fn overlay_set_visible(&self, visible: bool) -> Result<(), String> {
        let status = unsafe { (self.overlay_set_visible)(visible as i32) };
        self.check(status)
    }

    /// Converte um SqStatus em Result, anexando a mensagem de sq_last_error.
    fn check(&self, status: i32) -> Result<(), String> {
        if status == 0 {
            return Ok(());
        }
        let message = unsafe { CStr::from_ptr((self.last_error)()) }
            .to_string_lossy()
            .into_owned();
        Err(format!("núcleo retornou {status}: {message}"))
    }
}

/// Procura a DLL: primeiro ao lado do executável/recursos (instalado), depois em `src-tauri/bin` (dev).
pub fn candidate_paths(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let file = libloading::library_filename("subquest_core");
    let mut paths = Vec::new();
    if let Some(dir) = resource_dir {
        paths.push(dir.join(&file));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join(&file));
        }
    }
    paths.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("bin")
            .join(&file),
    );
    paths
}
