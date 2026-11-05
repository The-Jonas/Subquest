/*
 * core_api.h — public C API of subquest_core.dll.
 *
 * This is the ONLY boundary between the C++ core and the Rust (Tauri) app.
 * C types only: no std::string, exceptions or classes cross the DLL.
 * Every string is null-terminated UTF-8.
 */
#ifndef SUBQUEST_CORE_API_H
#define SUBQUEST_CORE_API_H

#include <stdint.h>

#ifdef _WIN32
  #ifdef SUBQUEST_CORE_BUILD
    #define SQ_API __declspec(dllexport)
  #else
    #define SQ_API __declspec(dllimport)
  #endif
#else
  #define SQ_API __attribute__((visibility("default")))
#endif

#ifdef __cplusplus
extern "C" {
#endif

#define SQ_API_VERSION 4u /* bump whenever the ABI changes */

typedef enum SqStatus {
    SQ_OK = 0,
    SQ_ERR_INVALID_ARG = 1,     /* null pointer or value out of range */
    SQ_ERR_ALREADY_RUNNING = 2, /* sq_start called while a session is active */
    SQ_ERR_NOT_RUNNING = 3,     /* sq_stop with no active session */
    SQ_ERR_PLATFORM = 4,        /* a Windows API failed (capture, OCR...) */
    SQ_ERR_NOT_IMPLEMENTED = 5, /* feature still a stub */
    SQ_ERR_OCR_LANGUAGE = 6     /* Windows has no OCR installed for the requested language */
} SqStatus;

typedef struct SqRect {
    int32_t x;      /* in pixels of the captured image (same as the preview, before downscaling) */
    int32_t y;
    int32_t width;
    int32_t height;
} SqRect;

typedef struct SqSessionConfig {
    uint64_t window_handle;      /* game HWND, cast to an integer */
    SqRect region;               /* dialogue area; width/height = 0 => whole window */
    const char* ocr_language;    /* tag BCP-47: "en", "ja", "zh-Hans", "ko" */
    uint32_t settle_frames;      /* identical frames in a row before running OCR (default 3) */
    float change_threshold;      /* minimum mean difference (0..1) that counts as a change (default 0.02) */
} SqSessionConfig;

/* Called on the OCR thread when NEW text is recognized.
 * ocr_ms = recognition only; total_ms = from frame capture until the text was ready. */
typedef void (*SqTextCallback)(const char* utf8_text, uint64_t frame_id, double ocr_ms, double total_ms,
                               void* user_data);

/* Called for every preview frame: contiguous RGBA, width * height * 4 bytes.
 * source_width/height = real capture size, to convert coordinates. */
typedef void (*SqPreviewCallback)(const uint8_t* rgba, int32_t width, int32_t height,
                                  int32_t source_width, int32_t source_height, void* user_data);

/* Called once per visible window during sq_list_windows. */
typedef void (*SqWindowCallback)(uint64_t window_handle, const char* utf8_title, const char* utf8_process, void* user_data);

SQ_API uint32_t sq_api_version(void);                                    /* ABI version, so Rust can check compatibility */
SQ_API SqStatus sq_list_windows(SqWindowCallback callback, void* user_data); /* enumerates visible top-level windows */
SQ_API SqStatus sq_window_bounds(uint64_t window_handle, SqRect* out_bounds,
                                 int32_t* out_minimized);         /* window on screen (physical px), for the overlay */
SQ_API SqStatus sq_start(const SqSessionConfig* config, SqTextCallback callback, void* user_data); /* starts capture + OCR */
SQ_API SqStatus sq_stop(void);                                            /* stops the session and waits for its threads */
SQ_API SqStatus sq_preview_start(uint64_t window_handle, int32_t max_width,
                                 SqPreviewCallback callback, void* user_data); /* live preview (~8 fps) */
SQ_API SqStatus sq_preview_stop(void);                                    /* stops the preview and waits for capture to end */
SQ_API SqStatus sq_overlay_set_text(const char* utf8_text);               /* replaces the text shown in the overlay */
SQ_API SqStatus sq_overlay_set_visible(int32_t visible);                  /* shows (1) or hides (0) the overlay */
SQ_API const char* sq_last_error(void);                                   /* last error message (valid until the next call) */

#ifdef __cplusplus
}
#endif

#endif /* SUBQUEST_CORE_API_H */
