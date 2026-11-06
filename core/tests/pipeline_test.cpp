// Test for the whole pipeline with a fake capture and a fake OCR engine, no
// framework: frames go in, and what reaches the app comes out. This is the
// contract every OCR engine relies on (see ocr_engine.h).
#include <chrono>
#include <condition_variable>
#include <cstdio>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

#include "capture/capture_source.h"
#include "ocr/ocr_engine.h"
#include "pipeline/pipeline.h"

using namespace subquest;

static int failures = 0;

// Checks a condition and records a failure with a description.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

// Capture that delivers only the frames the test pushes.
class FakeCapture final : public ICaptureSource {
public:
    bool start(uint64_t, Rect region, FrameCallback on_frame) override {
        region_ = region;
        on_frame_ = std::move(on_frame);
        return true;
    }
    void stop() override {}
    void set_region(Rect region) override { region_ = region; }
    const std::string& error() const override { return error_; }

    // Delivers `count` identical frames of one gray level, cut to the captured
    // region like the real capture does.
    void show(uint8_t gray, int count = 4) {
        for (int i = 0; i < count; ++i) {
            Frame f;
            f.width = 256;
            f.height = 64;
            f.stride = f.width * 4;
            f.bgra.assign(size_t(f.stride) * f.height, gray);
            f.id = ++next_id_;
            f.timestamp_us = next_id_ * 33'000; // 30 frames per second
            f.origin_x = region_.x;
            f.origin_y = region_.y;
            on_frame_(std::move(f));
        }
    }

    Rect region_;

private:
    FrameCallback on_frame_;
    std::string error_;
    uint64_t next_id_ = 0;
};

// Engine that "reads" the gray level: dark = nothing, 120 = a menu item with a
// stray "(" (a crescent in the art), 240 = a sentence.
class FakeOcr final : public IOcrEngine {
public:
    OcrLoadError load(const std::string& language) override {
        return language == "xx" ? OcrLoadError::LanguageMissing : OcrLoadError::None;
    }
    OcrResult recognize(const Frame& frame) override {
        OcrResult result;
        const uint8_t gray = frame.bgra.empty() ? 0 : frame.bgra[0];
        if (gray == 120) result.lines.push_back({"( PLAY", {}, {}, {}, {{"(", {10, 5, 8, 20}}, {"PLAY", {30, 5, 60, 20}}}});
        if (gray == 240) result.lines.push_back({"Hello there", {}, {}, {}, {}}); // no word boxes
        return result;
    }
    const char* name() const override { return "fake"; }
    const std::string& error() const override { return error_; }

private:
    std::string error_;
};

// Collects what the pipeline sends to the app, from the OCR thread.
struct Received {
    std::mutex mutex;
    std::condition_variable changed;
    std::vector<OcrResult> results;

    void add(const OcrResult& r) {
        std::lock_guard lock(mutex);
        OcrResult copy = r;
        copy.image = nullptr; // only valid during the callback
        results.push_back(std::move(copy));
        changed.notify_all();
    }
    // Waits until `count` results arrived (or a second passed).
    bool wait_for(size_t count) {
        std::unique_lock lock(mutex);
        return changed.wait_for(lock, std::chrono::seconds(1), [&] { return results.size() >= count; });
    }
    size_t size() {
        std::lock_guard lock(mutex);
        return results.size();
    }
};

int main() {
    {
        auto capture = std::make_unique<FakeCapture>();
        FakeCapture* screen = capture.get();
        Pipeline pipeline(std::move(capture), std::make_unique<FakeOcr>());
        Received got;
        PipelineConfig config;
        config.areas = {Rect{100, 400, 256, 64}};
        const StartError started = pipeline.start(config, [&](const OcrResult& r) { got.add(r); });
        check(started == StartError::None, "a session starts with any engine");
        check(screen->region_.x == 100 && screen->region_.y == 400, "the capture is asked for the area's rectangle");

        screen->show(120);
        check(got.wait_for(1), "new text on screen reaches the app");
        const OcrResult menu = got.results[0];
        check(menu.full_text == "PLAY", "the engine's lines are tidied (the stray '(' goes)");
        check(menu.lines.size() == 1 && menu.lines[0].box.x == 130 && menu.lines[0].box.y == 405,
              "line boxes leave in window pixels (area origin added), without the stray mark");
        check(menu.lines[0].words.size() == 1 && menu.lines[0].words[0].box.x == 130,
              "word boxes too, so a word can be picked over the game");
        check(menu.clean_rect.x == 100 && menu.clean_rect.width == 256 &&
                  menu.clean.size() == size_t(256) * 64 * 4,
              "the area's picture without the text comes along, in window pixels");
        check(menu.settle_ms > 60 && menu.settle_ms < 70,
              "the wait for the text to stand still is measured (3 still frames at 30 fps = 66 ms)");

        screen->show(120);
        screen->show(120);
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        check(got.size() == 1, "the same text still on screen is not sent again");

        screen->show(240);
        check(got.wait_for(2) && got.results[1].full_text == "Hello there",
              "an engine without word boxes works the same");
        check(got.results[1].frame_id > menu.frame_id, "every result gets a new, increasing id");

        screen->show(0);
        check(got.wait_for(3) && got.results[2].full_text.empty() && got.results[2].lines.empty(),
              "text that disappears is sent once as empty, so the translation hides");
        pipeline.stop();
        check(!pipeline.running(), "the session stops");
    }
    {
        Pipeline pipeline(std::make_unique<FakeCapture>(), std::make_unique<FakeOcr>());
        PipelineConfig config;
        config.ocr_language = "xx";
        check(pipeline.start(config, [](const OcrResult&) {}) == StartError::OcrLanguage,
              "an engine without the language stops the session from starting, with the reason");
    }

    std::printf(failures ? "%d FAILURE(S)\n" : "ALL OK\n", failures);
    return failures ? 1 : 0;
}
