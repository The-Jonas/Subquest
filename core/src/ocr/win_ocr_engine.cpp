#include "ocr/win_ocr_engine.h"

#ifdef _WIN32

#include <algorithm>
#include <chrono>
#include <cstring>
#include <cwctype>
#include <string>

#include <unknwn.h>
#include <windows.h>

#include <winrt/base.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.Globalization.h>
#include <winrt/Windows.Graphics.Imaging.h>
#include <winrt/Windows.Media.Ocr.h>
#include <winrt/Windows.Storage.Streams.h>

#include "vision/image_ops.h"

namespace ocr = winrt::Windows::Media::Ocr;
namespace imaging = winrt::Windows::Graphics::Imaging;

namespace subquest {

namespace {

// Makes sure the calling thread can use WinRT. The OCR runs on threads we don't
// own the setup of (Tauri's runtime, the pipeline thread), so join the MTA once
// per thread; if the thread is already in some apartment, that's fine too.
void ensure_com() {
    thread_local bool done = false;
    if (done) return;
    const HRESULT hr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    done = SUCCEEDED(hr) || hr == RPC_E_CHANGED_MODE;
}

// Case-insensitive "starts with", used to match "en" against "en-US".
bool starts_with_ci(const std::wstring& text, const std::wstring& prefix) {
    if (prefix.size() > text.size()) return false;
    return std::equal(prefix.begin(), prefix.end(), text.begin(),
                      [](wchar_t a, wchar_t b) { return towlower(a) == towlower(b); });
}

// Union of the word rectangles, i.e. the box around a whole line.
Rect line_box(const ocr::OcrLine& line) {
    float left = 1e9f, top = 1e9f, right = -1e9f, bottom = -1e9f;
    for (const auto& word : line.Words()) {
        const auto r = word.BoundingRect();
        left = std::min(left, r.X);
        top = std::min(top, r.Y);
        right = std::max(right, r.X + r.Width);
        bottom = std::max(bottom, r.Y + r.Height);
    }
    if (right < left) return {};
    return {int32_t(left), int32_t(top), int32_t(right - left), int32_t(bottom - top)};
}

} // namespace

struct WinOcrEngine::Impl {
    ocr::OcrEngine engine{nullptr};
};

// Nothing WinRT is created until load().
WinOcrEngine::WinOcrEngine() : impl_(std::make_unique<Impl>()) {}

// Releases the WinRT engine.
WinOcrEngine::~WinOcrEngine() = default;

// Picks the first installed OCR language whose tag starts with `language`
// ("en" matches "en-US"). Fails with LanguageMissing when Windows has no OCR
// for it, which is common for English on a Portuguese Windows install.
OcrLoadError WinOcrEngine::load(const std::string& language) {
    ensure_com();
    try {
        const std::wstring wanted = winrt::to_hstring(language).c_str();
        for (const auto& lang : ocr::OcrEngine::AvailableRecognizerLanguages()) {
            if (starts_with_ci(std::wstring(lang.LanguageTag().c_str()), wanted)) {
                impl_->engine = ocr::OcrEngine::TryCreateFromLanguage(lang);
                if (impl_->engine) return OcrLoadError::None;
            }
        }
        error_ = "no Windows OCR language installed for '" + language + "'";
        return OcrLoadError::LanguageMissing;
    } catch (const winrt::hresult_error& e) {
        error_ = "Windows OCR failed to load: " + winrt::to_string(e.message());
        return OcrLoadError::Platform;
    }
}

// Copies the frame into a SoftwareBitmap, runs RecognizeAsync and converts the
// lines. Frames larger than the engine's limit are shrunk first. Blocking on
// .get() is fine here: this always runs on a background thread, never on a UI one.
OcrResult WinOcrEngine::recognize(const Frame& frame) {
    OcrResult result;
    result.frame_id = frame.id;
    if (!impl_->engine || frame.width <= 0 || frame.height <= 0) return result;
    ensure_com();

    const auto started = std::chrono::steady_clock::now();
    try {
        Frame resized;
        const int32_t max_dim = int32_t(ocr::OcrEngine::MaxImageDimension());
        const Frame& input = fit_within(frame, max_dim, resized) ? resized : frame;
        const double scale = double(frame.width) / double(input.width); // maps boxes back to frame pixels

        winrt::Windows::Storage::Streams::Buffer buffer(uint32_t(input.bgra.size()));
        std::memcpy(buffer.data(), input.bgra.data(), input.bgra.size());
        buffer.Length(uint32_t(input.bgra.size()));

        imaging::SoftwareBitmap bitmap(imaging::BitmapPixelFormat::Bgra8, input.width, input.height,
                                       imaging::BitmapAlphaMode::Premultiplied);
        bitmap.CopyFromBuffer(buffer);

        const auto ocr_result = impl_->engine.RecognizeAsync(bitmap).get();
        for (const auto& line : ocr_result.Lines()) {
            Rect box = line_box(line);
            box = {int32_t(box.x * scale), int32_t(box.y * scale), int32_t(box.width * scale),
                   int32_t(box.height * scale)};
            std::string text = winrt::to_string(line.Text());
            if (!result.full_text.empty()) result.full_text += '\n';
            result.full_text += text;
            result.lines.push_back({std::move(text), box});
        }
    } catch (const winrt::hresult_error&) {
        // A failed frame just yields no text; the next stable frame tries again.
    }
    result.elapsed_ms =
        std::chrono::duration<double, std::milli>(std::chrono::steady_clock::now() - started).count();
    return result;
}

} // namespace subquest

#else // ---------- outside Windows: only so the rest builds and tests ----------

namespace subquest {

struct WinOcrEngine::Impl {};

// Empty version: Windows OCR only exists on Windows.
WinOcrEngine::WinOcrEngine() : impl_(std::make_unique<Impl>()) {}

// Nothing to release.
WinOcrEngine::~WinOcrEngine() = default;

// Always fails outside Windows, with a clear message.
OcrLoadError WinOcrEngine::load(const std::string&) {
    error_ = "Windows OCR is only available on Windows";
    return OcrLoadError::Platform;
}

// Never called outside Windows (load() fails first).
OcrResult WinOcrEngine::recognize(const Frame& frame) {
    OcrResult result;
    result.frame_id = frame.id;
    return result;
}

} // namespace subquest

#endif
