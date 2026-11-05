#include "pipeline/pipeline.h"

namespace subquest {

// Recebe as implementações concretas (injeção de dependência facilita testes).
Pipeline::Pipeline(std::unique_ptr<ICaptureSource> capture, std::unique_ptr<IOcrEngine> ocr)
    : capture_(std::move(capture)), ocr_(std::move(ocr)) {}

// Para tudo antes de destruir para não deixar threads soltas.
Pipeline::~Pipeline() { stop(); }

// Carrega o OCR, sobe a thread de OCR e só então liga a captura.
// A ordem importa: a captura não pode entregar quadros para ninguém.
StartError Pipeline::start(const PipelineConfig& config, TextCallback on_text) {
    if (running_) return StartError::AlreadyRunning;
    if (!ocr_->load(config.ocr_language)) return StartError::Ocr;

    detector_ = ChangeDetector(config.detector);
    ocr_slot_.reset();
    on_text_ = std::move(on_text);
    last_text_.clear();
    running_ = true;
    ocr_thread_ = std::thread(&Pipeline::ocr_loop, this);

    const bool ok = capture_->start(config.window_handle, config.region,
                                    [this](Frame&& f) { on_frame(std::move(f)); });
    if (!ok) {
        stop();
        return StartError::Capture;
    }
    return StartError::None;
}

// Desliga a captura primeiro (para de produzir), depois fecha a vaga e espera o OCR.
void Pipeline::stop() {
    if (!running_.exchange(false)) return;
    capture_->stop();
    ocr_slot_.close();
    if (ocr_thread_.joinable()) ocr_thread_.join();
}

// Só repassa para o OCR os quadros que o detector aprovar.
void Pipeline::on_frame(Frame&& frame) {
    if (!running_) return;
    if (detector_.feed(frame)) ocr_slot_.put(std::move(frame));
}

// Consome a vaga até a sessão fechar; ignora texto vazio ou repetido.
void Pipeline::ocr_loop() {
    while (auto frame = ocr_slot_.take()) {
        OcrResult result = ocr_->recognize(*frame);
        if (result.full_text.empty() || result.full_text == last_text_) continue;
        last_text_ = result.full_text;
        if (on_text_) on_text_(result);
    }
}

} // namespace subquest
