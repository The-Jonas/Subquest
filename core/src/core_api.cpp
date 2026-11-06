// core_api.cpp — C API implementation: turns C calls into C++ objects.
#include "subquest/core_api.h"

#include <memory>
#include <mutex>
#include <string>
#include <vector>

#include "capture/wgc_capture.h"
#include "capture/window_list.h"
#include "ocr/ocr_factory.h"
#include "pipeline/pipeline.h"
#include "pipeline/preview_stream.h"

using namespace subquest;

namespace {

// Global core state: one session at a time is enough for one game at a time.
struct CoreState {
    std::mutex mutex;
    std::unique_ptr<Pipeline> pipeline;
    std::unique_ptr<PreviewStream> preview;
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

// Reports where the game window is, so the app can place the overlay over it.
extern "C" SQ_API SqStatus sq_window_bounds(uint64_t window_handle, SqRect* out_bounds, int32_t* out_minimized) {
    if (!out_bounds || !out_minimized) return fail(SQ_ERR_INVALID_ARG, "null output pointer");
    WindowBounds b;
    if (!window_bounds(window_handle, b)) return fail(SQ_ERR_INVALID_ARG, "the window no longer exists");
    *out_bounds = {b.x, b.y, b.width, b.height};
    *out_minimized = b.minimized ? 1 : 0;
    return SQ_OK;
}

// Builds the pipeline (Windows capture + the chosen OCR engine) and starts the session.
extern "C" SQ_API SqStatus sq_start(const SqSessionConfig* config, SqTextCallback callback, void* user_data) {
    if (!config || !callback || !config->ocr_language) return fail(SQ_ERR_INVALID_ARG, "null config or callback");

    CoreState& s = state();
    std::lock_guard lock(s.mutex);
    if (s.pipeline && s.pipeline->running()) return fail(SQ_ERR_ALREADY_RUNNING, "session already running");

    if (config->area_count > SQ_MAX_AREAS || (config->area_count > 0 && !config->areas))
        return fail(SQ_ERR_INVALID_ARG, "invalid areas");

    PipelineConfig pc;
    pc.window_handle = config->window_handle;
    for (uint32_t i = 0; i < config->area_count; ++i) {
        const SqRect& a = config->areas[i];
        pc.areas.push_back({a.x, a.y, a.width, a.height});
    }
    pc.ocr_language = config->ocr_language;
    if (config->settle_frames > 0) pc.detector.settle_frames = config->settle_frames;
    if (config->change_threshold > 0.0f) pc.detector.change_threshold = config->change_threshold;

    const std::string engine_name = config->ocr_engine ? config->ocr_engine : "";
    std::unique_ptr<IOcrEngine> ocr = make_ocr_engine(engine_name);
    if (!ocr) return fail(SQ_ERR_OCR_ENGINE, ("unknown OCR engine '" + engine_name + "'").c_str());

    s.pipeline = std::make_unique<Pipeline>(std::make_unique<WgcCapture>(), std::move(ocr));

    const StartError result = s.pipeline->start(pc, [callback, user_data](const OcrResult& r) {
        // Plain C view of the result; the strings stay owned by `r` during the call.
        std::vector<std::vector<SqWord>> words(r.lines.size());
        std::vector<SqLine> lines;
        lines.reserve(r.lines.size());
        for (size_t i = 0; i < r.lines.size(); ++i) {
            const OcrLine& l = r.lines[i];
            for (const OcrWord& w : l.words)
                words[i].push_back({w.text.c_str(), {w.box.x, w.box.y, w.box.width, w.box.height}});
            lines.push_back({l.text.c_str(), {l.box.x, l.box.y, l.box.width, l.box.height}, l.background_rgb,
                             l.text_rgb, words[i].data(), uint32_t(words[i].size())});
        }
        const SqOcrEvent event{r.full_text.c_str(), lines.data(), uint32_t(lines.size()), r.frame_id,
                               r.area_index, r.elapsed_ms, r.total_ms, r.frame_width, r.frame_height,
                               r.background_rgb, r.text_rgb,
                               r.image ? r.image->bgra.data() : nullptr,
                               r.image ? r.image->origin_x : 0, r.image ? r.image->origin_y : 0,
                               r.image ? r.image->width : 0, r.image ? r.image->height : 0,
                               r.image ? r.image->stride : 0};
        callback(&event, user_data);
    });
    switch (result) {
        case StartError::None: return SQ_OK;
        case StartError::AlreadyRunning: return fail(SQ_ERR_ALREADY_RUNNING, "session already running");
        case StartError::OcrLanguage: return fail(SQ_ERR_OCR_LANGUAGE, s.pipeline->ocr_error().c_str());
        case StartError::Ocr: return fail(SQ_ERR_PLATFORM, s.pipeline->ocr_error().c_str());
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

    s.preview = std::make_unique<PreviewStream>(std::make_unique<WgcCapture>(8, false)); // a preview needs no repeats
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

// Last error message stored by fail().
extern "C" SQ_API const char* sq_last_error(void) { return state().last_error.c_str(); }
