// ocr_factory.h — builds an OCR engine from its name (the app's "ocrEngine" setting).
#pragma once

#include <memory>
#include <string>

#include "ocr/ocr_engine.h"

namespace subquest {

std::unique_ptr<IOcrEngine> make_ocr_engine(const std::string& name); // nullptr for an unknown name; "" = the default
const char* default_ocr_engine();                                    // engine used when none is named

} // namespace subquest
