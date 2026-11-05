// core_api.cpp — C API implementation: turns C calls into C++ objects.
#include "subquest/core_api.h"

#include <memory>
#include <mutex>
#include <string>

#include "capture/wgc_capture.h"
#include "capture/window_list.h"
#include "ocr/win_ocr_engine.h"
#include "overlay/overlay_window.h"
#include "pipeline/pipeline.h"
#include "pipeline/preview_stream.h"

using namespace subquest;

namespace {

// Global core state: one session at a time is enough for one game at a time.
struct CoreState {
    std::mutex mutex;
    std::unique_ptr<Pipeline> pipeline;
    std::unique_ptr<PreviewStream> preview;
    std::unique_ptr<OverlayWindow> overlay;
    std::string last_error;
};

// Single instance created on first use (avoids static initialization order issues).
CoreState& state() {
    static CoreState s;
    return s;
}

// Stores the error message and returns the code, for use as "return fail(...)".
SqStatus fail(SqStatus code, const char* message) {
    state().last_error = message;
    return code;
}

} // namespace

// ABI version; Rust refuses to load a DLL with a different version.
extern "C" SQ_API uint32_t sq_api_version(void) { return SQ_API_VERSION; }

// Calls the callback once per capturable window.
extern "C" SQ_API SqStatus sq_list_windows(SqWindowCallback callback, void* user_data) {
    if (!callback) return fail(SQ_ERR_INVALID_ARG, "null callback");
    for (const WindowInfo& w : list_capturable_windows())
        callback(w.handle, w.title.c_str(), w.process.c_str(), user_data);
    return SQ_OK;
}

// Builds the pipeline with the Windows implementations and starts the session.
extern "C" SQ_API SqStatus sq_start(const SqSessionConfig* config, SqTextCallback callback, void* user_data) {
    if (!config || !callback || !config->ocr_language) return fail(SQ_ERR_INVALID_ARG, "null config or callback");

    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (s.pipeline && s.pipeline->running()) return fail(SQ_ERR_ALREADY_RUNNING, "session already running");

    PipelineConfig pc;
    pc.window_handle = config->window_handle;
    pc.region = {config->region.x, config->region.y, config->region.width, config->region.height};
    pc.ocr_language = config->ocr_language;
    if (config->settle_frames > 0) pc.detector.settle_frames = config->settle_frames;
    if (config->change_threshold > 0.0f) pc.detector.change_threshold = config->change_threshold;

    s.pipeline = std::make_unique<Pipeline>(std::make_unique<WgcCapture>(), std::make_unique<WinOcrEngine>());
    if (!s.overlay) s.overlay = std::make_unique<OverlayWindow>();

    const StartError result = s.pipeline->start(pc, [callback, user_data](const OcrResult& r) {
        callback(r.full_text.c_str(), r.frame_id, r.elapsed_ms, user_data);
    });
    switch (result) {
        case StartError::None: return SQ_OK;
        case StartError::AlreadyRunning: return fail(SQ_ERR_ALREADY_RUNNING, "session already running");
        case StartError::Ocr: return fail(SQ_ERR_NOT_IMPLEMENTED, "OCR not implemented yet");
        case StartError::Capture: return fail(SQ_ERR_PLATFORM, s.pipeline->capture_error().c_str());
    }
    return fail(SQ_ERR_PLATFORM, "unknown error");
}

// Ends the active session.
extern "C" SQ_API SqStatus sq_stop(void) {
    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (!s.pipeline || !s.pipeline->running()) return fail(SQ_ERR_NOT_RUNNING, "no active session");
    s.pipeline->stop();
    return SQ_OK;
}

// Starts the live window preview (8 fps is enough to pick the area).
extern "C" SQ_API SqStatus sq_preview_start(uint64_t window_handle, int32_t max_width,
                                            SqPreviewCallback callback, void* user_data) {
    if (!callback || max_width <= 0) return fail(SQ_ERR_INVALID_ARG, "null callback or invalid width");

    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (s.preview && s.preview->running()) return fail(SQ_ERR_ALREADY_RUNNING, "preview already running");

    s.preview = std::make_unique<PreviewStream>(std::make_unique<WgcCapture>(8));
    const bool ok = s.preview->start(window_handle, max_width,
        [callback, user_data](const RgbaImage& img, int32_t source_width, int32_t source_height) {
            callback(img.pixels.data(), img.width, img.height, source_width, source_height, user_data);
        });
    if (!ok) return fail(SQ_ERR_PLATFORM, s.preview->error().c_str());
    return SQ_OK;
}

// Stops the preview; once this returns, the callback is no longer called.
extern "C" SQ_API SqStatus sq_preview_stop(void) {
    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (!s.preview || !s.preview->running()) return fail(SQ_ERR_NOT_RUNNING, "preview is not running");
    s.preview->stop();
    return SQ_OK;
}

// Hands the finished translation to the overlay.
extern "C" SQ_API SqStatus sq_overlay_set_text(const char* utf8_text) {
    if (!utf8_text) return fail(SQ_ERR_INVALID_ARG, "null text");
    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (!s.overlay) s.overlay = std::make_unique<OverlayWindow>();
    s.overlay->set_text(utf8_text);
    return SQ_OK;
}

// Shows or hides the overlay (user's global hotkey).
extern "C" SQ_API SqStatus sq_overlay_set_visible(int32_t visible) {
    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (!s.overlay) return fail(SQ_ERR_NOT_RUNNING, "overlay not created yet");
    s.overlay->set_visible(visible != 0);
    return SQ_OK;
}

// Last error message stored by fail().
extern "C" SQ_API const char* sq_last_error(void) { return state().last_error.c_str(); }
