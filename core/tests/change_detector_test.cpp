// Teste mínimo do ChangeDetector, sem framework: roda em qualquer SO.
#include <cstdio>
#include <cstdlib>

#include "vision/change_detector.h"

using namespace subquest;

// Gera um quadro cinza uniforme com um retângulo claro simulando um texto.
static Frame make_frame(int w, int h, int text_width, uint8_t bg = 30) {
    Frame f;
    f.width = w;
    f.height = h;
    f.stride = w * 4;
    f.bgra.assign(size_t(f.stride) * h, bg);
    for (int y = h / 3; y < 2 * h / 3; ++y)
        for (int x = 10; x < 10 + text_width && x < w; ++x)
            for (int c = 0; c < 3; ++c) f.bgra[size_t(y) * f.stride + x * 4 + c] = 230;
    return f;
}

static int failures = 0;

// Confere uma condição e registra a falha com uma descrição.
static void check(bool ok, const char* what) {
    std::printf("[%s] %s\n", ok ? " ok " : "FAIL", what);
    if (!ok) ++failures;
}

int main() {
    ChangeDetector det; // settle_frames = 3

    Frame a = make_frame(640, 120, 200);
    check(!det.feed(a), "1o quadro nao dispara (sem historico)");
    check(!det.feed(a), "2o quadro igual ainda nao dispara");
    check(det.feed(a), "3o quadro igual dispara (estabilizou)");
    check(!det.feed(a), "4o quadro igual nao dispara de novo");

    // Efeito máquina de escrever: o texto cresce a cada quadro.
    check(!det.feed(make_frame(640, 120, 260)), "texto crescendo nao dispara");
    check(!det.feed(make_frame(640, 120, 320)), "texto crescendo nao dispara (2)");
    Frame b = make_frame(640, 120, 400);
    check(!det.feed(b), "texto parou: 1o quadro parado");
    check(!det.feed(b), "texto parou: 2o quadro parado");
    check(det.feed(b), "texto parou: 3o quadro dispara");

    // Mesmo texto voltando depois de um piscar (ex.: cursor) não dispara de novo.
    det.feed(make_frame(640, 120, 400, 60));
    det.feed(b); det.feed(b);
    check(!det.feed(b), "mesmo texto apos um piscar nao repete o OCR");

    std::printf("%s\n", failures ? "FALHOU" : "TUDO OK");
    return failures ? EXIT_FAILURE : EXIT_SUCCESS;
}
