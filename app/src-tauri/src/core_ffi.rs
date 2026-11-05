//! Bridge to `subquest_core.dll` (C++), loaded at runtime.
//!
//! Why load it dynamically (libloading) instead of linking?
//! - The app opens even without the DLL and shows a friendly notice instead of failing to start.
//! - The DLL can be swapped in dev without recompiling Rust.
//!
//! The signatures below MUST match `core/include/subquest/core_api.h`.

use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};

use libloading::Library;

use crate::error::AppError;

/// ABI version this code expects (`SQ_API_VERSION` in the header).
pub const EXPECTED_API_VERSION: u32 = 3;

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

pub type SqTextCallback = unsafe extern "C" fn(*const c_char, u64, f64, f64, *mut c_void);
pub type SqWindowCallback = unsafe extern "C" fn(u64, *const c_char, *const c_char, *mut c_void);
pub type SqPreviewCallback = unsafe extern "C" fn(*const u8, i32, i32, i32, i32, *mut c_void);

type FnApiVersion = unsafe extern "C" fn() -> u32;
type FnListWindows = unsafe extern "C" fn(SqWindowCallback, *mut c_void) -> i32;
type FnStart = unsafe extern "C" fn(*const SqSessionConfig, SqTextCallback, *mut c_void) -> i32;
type FnStop = unsafe extern "C" fn() -> i32;
type FnPreviewStart = unsafe extern "C" fn(u64, i32, SqPreviewCallback, *mut c_void) -> i32;
type FnPreviewStop = unsafe extern "C" fn() -> i32;
type FnSetText = unsafe extern "C" fn(*const c_char) -> i32;
type FnSetVisible = unsafe extern "C" fn(i32) -> i32;
type FnLastError = unsafe extern "C" fn() -> *const c_char;

/// A capturable window, as reported by the core.
#[derive(Clone, Debug, serde::Serialize)]
pub struct WindowEntry {
    pub handle: u64,
    pub title: String,
    pub process: String,
}

/// Loaded library + pointers to the exported functions.
/// The `Library` must outlive every use of those pointers.
pub struct Core {
    _lib: Library,
    api_version: FnApiVersion,
    list_windows: FnListWindows,
    start: FnStart,
    stop: FnStop,
    preview_start: FnPreviewStart,
    preview_stop: FnPreviewStop,
    overlay_set_text: FnSetText,
    overlay_set_visible: FnSetVisible,
    last_error: FnLastError,
}

/// Reads a DLL symbol and copies the function pointer (fails with the symbol name).
unsafe fn symbol<T: Copy>(lib: &Library, name: &str) -> Result<T, String> {
    lib.get::<T>(name.as_bytes())
        .map(|s| *s)
        .map_err(|e| format!("symbol `{name}` not found: {e}"))
}

impl Core {
    /// Loads the DLL from the given path, resolves every symbol and checks the ABI version.
    pub fn load(path: &Path) -> Result<Self, String> {
        // SAFETY: the DLL is ours and the signatures follow core_api.h.
        unsafe {
            let lib = Library::new(path)
                .map_err(|e| format!("could not open {}: {e}", path.display()))?;
            // The version is checked before the other symbols: an old DLL
            // wouldn't have the new functions and a "symbol" error would be confusing.
            let api_version: FnApiVersion = symbol(&lib, "sq_api_version")?;
            let version = api_version();
            if version != EXPECTED_API_VERSION {
                return Err(format!(
                    "incompatible ABI version: DLL {version}, app {EXPECTED_API_VERSION} (rebuild the core folder)"
                ));
            }
            Ok(Core {
                api_version,
                list_windows: symbol(&lib, "sq_list_windows")?,
                start: symbol(&lib, "sq_start")?,
                stop: symbol(&lib, "sq_stop")?,
                preview_start: symbol(&lib, "sq_preview_start")?,
                preview_stop: symbol(&lib, "sq_preview_stop")?,
                overlay_set_text: symbol(&lib, "sq_overlay_set_text")?,
                overlay_set_visible: symbol(&lib, "sq_overlay_set_visible")?,
                last_error: symbol(&lib, "sq_last_error")?,
                _lib: lib,
            })
        }
    }

    /// ABI version reported by the DLL.
    pub fn api_version(&self) -> u32 {
        unsafe { (self.api_version)() }
    }

    /// Lists capturable windows in on-screen order (front-most first).
    pub fn list_windows(&self) -> Result<Vec<WindowEntry>, AppError> {
        // C callback that pushes each window into the Vec passed via user_data.
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

    /// Starts the session. `user_data` reaches the callback on the OCR thread untouched.
    pub fn start(
        &self,
        window_handle: u64,
        region: SqRect,
        ocr_language: &str,
        callback: SqTextCallback,
        user_data: *mut c_void,
    ) -> Result<(), AppError> {
        let lang = CString::new(ocr_language).map_err(|e| AppError::other(e.to_string()))?;
        let config = SqSessionConfig {
            window_handle,
            region,
            ocr_language: lang.as_ptr(),
            settle_frames: 0,      // 0 = use the core default
            change_threshold: 0.0, // same
        };
        let status = unsafe { (self.start)(&config, callback, user_data) };
        self.check(status)
    }

    /// Stops the active session.
    pub fn stop(&self) -> Result<(), AppError> {
        let status = unsafe { (self.stop)() };
        self.check(status)
    }

    /// Starts the live preview. `user_data` reaches every callback call untouched.
    pub fn preview_start(
        &self,
        window_handle: u64,
        max_width: i32,
        callback: SqPreviewCallback,
        user_data: *mut c_void,
    ) -> Result<(), AppError> {
        let status = unsafe { (self.preview_start)(window_handle, max_width, callback, user_data) };
        self.check(status)
    }

    /// Stops the preview; once this returns, the callback is no longer called.
    pub fn preview_stop(&self) -> Result<(), AppError> {
        let status = unsafe { (self.preview_stop)() };
        self.check(status)
    }

    /// Shows the translation in the overlay.
    pub fn overlay_set_text(&self, text: &str) -> Result<(), AppError> {
        let text = CString::new(text).map_err(|e| AppError::other(e.to_string()))?;
        let status = unsafe { (self.overlay_set_text)(text.as_ptr()) };
        self.check(status)
    }

    /// Shows or hides the overlay.
    pub fn overlay_set_visible(&self, visible: bool) -> Result<(), AppError> {
        let status = unsafe { (self.overlay_set_visible)(visible as i32) };
        self.check(status)
    }

    /// Turns an SqStatus into a Result, attaching sq_last_error's message as the detail.
    fn check(&self, status: i32) -> Result<(), AppError> {
        if status == 0 {
            return Ok(());
        }
        let message = unsafe { CStr::from_ptr((self.last_error)()) }
            .to_string_lossy()
            .into_owned();
        Err(AppError::from_status(status, message))
    }
}

/// Looks for the DLL: first next to the executable/resources (installed), then in `src-tauri/bin` (dev).
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
