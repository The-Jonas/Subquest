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

#define SQ_API_VERSION 14u /* bump whenever the ABI changes */

typedef enum SqStatus {
    SQ_OK = 0,
    SQ_ERR_INVALID_ARG = 1,     /* null pointer or value out of range */
    SQ_ERR_ALREADY_RUNNING = 2, /* sq_start called while a session is active */
    SQ_ERR_NOT_RUNNING = 3,     /* sq_stop with no active session */
    SQ_ERR_PLATFORM = 4,        /* a Windows API failed (capture, OCR...) */
    SQ_ERR_NOT_IMPLEMENTED = 5, /* feature still a stub */
    SQ_ERR_OCR_LANGUAGE = 6,    /* the OCR engine can't read the requested language (not installed) */
    SQ_ERR_OCR_ENGINE = 7       /* no OCR engine with the requested name */
} SqStatus;

typedef struct SqRect {
    int32_t x;      /* in pixels of the captured image (same as the preview, before downscaling) */
    int32_t y;
    int32_t width;
    int32_t height;
} SqRect;

#define SQ_MAX_AREAS 16u /* most areas one session reads */

typedef struct SqSessionConfig {
    uint64_t window_handle;      /* game HWND, cast to an integer */
    const SqRect* areas;         /* screen areas to read; width/height = 0 => whole window */
    uint32_t area_count;         /* 0 => one area, the whole window */
    const char* ocr_language;    /* tag BCP-47: "en", "ja", "zh-Hans", "ko" */
    const char* ocr_engine;      /* "windows" (Windows.Media.Ocr); NULL or "" = the default */
    uint32_t settle_frames;      /* identical frames in a row before running OCR (default 3) */
    float change_threshold;      /* minimum mean difference (0..1) that counts as a change (default 0.02) */
} SqSessionConfig;

/* One word of a line, so a word can be picked over the game (glossary). */
typedef struct SqWord {
    const char* utf8_text;
    SqRect box;              /* in window capture pixels, like the line */
} SqWord;

/* One recognized line: where it is and what it looks like, so the overlay can
 * draw its translation in the same place, size and colors. */
typedef struct SqLine {
    const char* utf8_text;
    SqRect box;              /* in window capture pixels (the whole window, not the area) */
    uint32_t background_rgb; /* color right around the line (0xRRGGBB) */
    uint32_t text_rgb;       /* color of the line's letters (0xRRGGBB) */
    const SqWord* words;     /* the line's words, in reading order (boxes estimated when the engine has none) */
    uint32_t word_count;
} SqLine;

/* Everything about one piece of NEW text. Pointers are valid only during the callback. */
typedef struct SqOcrEvent {
    const char* utf8_text;   /* all lines joined by '\n' */
    const SqLine* lines;
    uint32_t line_count;
    uint64_t frame_id;       /* unique and increasing for every event */
    uint32_t area_index;     /* index in SqSessionConfig.areas */
    double ocr_ms;           /* recognition only */
    double total_ms;         /* from frame capture until the text was ready */
    int32_t frame_width;     /* size of the whole window in capture pixels, to scale line boxes */
    int32_t frame_height;
    uint32_t background_rgb; /* whole area: box color (0xRRGGBB) */
    uint32_t text_rgb;       /* whole area: text color (0xRRGGBB) */
    /* The area's pixels (BGRA, rows `image_stride` bytes apart), so the app can
     * keep a picture of a line. (image_x, image_y) is the area's top-left in
     * window pixels, like the line boxes. Valid only during the callback. */
    const uint8_t* image_bgra;
    int32_t image_x;
    int32_t image_y;
    int32_t image_width;
    int32_t image_height;
    int32_t image_stride;
    double settle_ms;        /* waited for the text to stop changing before reading it */
    /* The area's picture with the text taken out (BGRA, clean_box.width * 4
     * bytes per row), so the original lines can be hidden with the game's own
     * background. NULL when there is none. Valid only during the callback. */
    const uint8_t* clean_bgra;
    SqRect clean_box;        /* where it goes, in window pixels */
    /* Pictures inside the lines (a gamepad button, a key): the text has "{1}"
     * where the first one was, "{2}" for the second... Window pixels; their
     * pixels are in image_bgra. */
    const SqRect* icons;
    uint32_t icon_count;
} SqOcrEvent;

/* Called on the OCR thread when an area shows NEW text, and once with empty
   text (utf8_text "" and line_count 0) when that area's text disappears. */
typedef void (*SqTextCallback)(const SqOcrEvent* event, void* user_data);

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
SQ_API const char* sq_last_error(void);                                   /* last error message (valid until the next call) */

#ifdef __cplusplus
}
#endif

#endif /* SUBQUEST_CORE_API_H */
