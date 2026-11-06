#include "ocr/ocr_factory.h"

#include "ocr/win_ocr_engine.h"

namespace subquest {

// The engine every session uses unless another one is named.
const char* default_ocr_engine() { return "windows"; }

// One line per engine: adding an engine means adding it here.
std::unique_ptr<IOcrEngine> make_ocr_engine(const std::string& name) {
    const std::string& chosen = name.empty() ? std::string(default_ocr_engine()) : name;
    if (chosen == "windows") return std::make_unique<WinOcrEngine>();
    return nullptr;
}

} // namespace subquest
