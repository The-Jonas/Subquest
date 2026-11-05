#include "vision/change_detector.h"

#include <algorithm>
#include <cstdlib>

namespace subquest {

// Guarda a configuração; o histórico começa vazio.
ChangeDetector::ChangeDetector(ChangeDetectorConfig config) : config_(config) {}

// Limpa o histórico para que o próximo quadro estável sempre dispare.
void ChangeDetector::reset() {
    previous_.clear();
    last_fired_.clear();
    still_count_ = 0;
}

// Média da diferença absoluta pixel a pixel, normalizada para 0..1.
float ChangeDetector::mean_abs_diff(const std::vector<uint8_t>& a, const std::vector<uint8_t>& b) {
    if (a.size() != b.size() || a.empty()) return 1.0f;
    uint64_t sum = 0;
    for (size_t i = 0; i < a.size(); ++i) {
        sum += static_cast<uint64_t>(std::abs(int(a[i]) - int(b[i])));
    }
    return float(sum) / (float(a.size()) * 255.0f);
}

// Reduz o quadro por média de blocos (box filter) e converte BGRA para luminância.
// Média de blocos, e não amostragem de 1 pixel, para que ruído e anti-aliasing não
// pareçam "mudança".
std::vector<uint8_t> ChangeDetector::make_thumbnail(const Frame& frame) const {
    const int tw = config_.thumb_width;
    const int th = config_.thumb_height;
    std::vector<uint8_t> thumb(size_t(tw) * th, 0);
    if (frame.width <= 0 || frame.height <= 0) return thumb;

    for (int ty = 0; ty < th; ++ty) {
        const int y0 = ty * frame.height / th;
        const int y1 = std::max(y0 + 1, (ty + 1) * frame.height / th);
        for (int tx = 0; tx < tw; ++tx) {
            const int x0 = tx * frame.width / tw;
            const int x1 = std::max(x0 + 1, (tx + 1) * frame.width / tw);
            uint64_t acc = 0;
            uint32_t count = 0;
            for (int y = y0; y < y1 && y < frame.height; ++y) {
                const uint8_t* row = frame.bgra.data() + size_t(y) * frame.stride;
                for (int x = x0; x < x1 && x < frame.width; ++x) {
                    const uint8_t* px = row + size_t(x) * 4;
                    // Luminância aproximada (BT.601) em inteiros: 0.114 B + 0.587 G + 0.299 R
                    acc += (29u * px[0] + 150u * px[1] + 77u * px[2]) >> 8;
                    ++count;
                }
            }
            thumb[size_t(ty) * tw + tx] = count ? uint8_t(acc / count) : 0;
        }
    }
    return thumb;
}

// Máquina de estados: conta quadros parados; ao atingir settle_frames, dispara
// só se a imagem estável for diferente da que disparou da última vez.
bool ChangeDetector::feed(const Frame& frame) {
    std::vector<uint8_t> thumb = make_thumbnail(frame);

    if (previous_.empty()) {
        previous_ = std::move(thumb);
        still_count_ = 1;
        return false;
    }

    const float diff_prev = mean_abs_diff(thumb, previous_);
    still_count_ = (diff_prev <= config_.still_threshold) ? still_count_ + 1 : 1;
    previous_ = thumb;

    if (still_count_ != config_.settle_frames) return false; // dispara só uma vez por período parado

    const bool first_fire = last_fired_.empty();
    if (!first_fire && mean_abs_diff(thumb, last_fired_) < config_.change_threshold) return false;

    last_fired_ = std::move(thumb);
    return true;
}

} // namespace subquest
