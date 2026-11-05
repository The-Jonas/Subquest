#include "capture/wgc_capture.h"

namespace subquest {

// Construtor vazio: os recursos D3D/WinRT só são criados em start().
WgcCapture::WgcCapture() = default;

// Garante que a captura pare antes de destruir o objeto.
WgcCapture::~WgcCapture() { stop(); }

// TODO(fase 1): implementar a captura real. Roteiro:
//  1. Criar um ID3D11Device (D3D11CreateDevice, flag BGRA_SUPPORT) e embrulhar
//     num IDirect3DDevice do WinRT (CreateDirect3D11DeviceFromDXGIDevice).
//  2. Criar o GraphicsCaptureItem a partir do HWND via IGraphicsCaptureItemInterop::CreateForWindow.
//  3. Direct3D11CaptureFramePool::CreateFreeThreaded(device, B8G8R8A8, 2, size) e
//     uma sessão; desligar a borda amarela (IsBorderRequired = false, Win11).
//  4. No evento FrameArrived: CopySubresourceRegion da textura só na `region_`
//     para uma textura STAGING pequena -> Map -> copiar para Frame::bgra.
//     Recortar na GPU antes de copiar é o que mantém o custo baixo.
//  5. Chamar on_frame_ com o Frame (id e timestamp preenchidos).
// Por enquanto retorna false para o núcleo informar SQ_ERR_NOT_IMPLEMENTED.
bool WgcCapture::start(uint64_t window_handle, Rect region, FrameCallback on_frame) {
    (void)window_handle;
    set_region(region);
    on_frame_ = std::move(on_frame);
    return false;
}

// Encerra a sessão de captura e libera o frame pool.
void WgcCapture::stop() {
    running_ = false;
}

// Atualiza a região lida a cada FrameArrived; não precisa reiniciar a sessão.
void WgcCapture::set_region(Rect region) {
    std::lock_guard lock(region_mutex_);
    region_ = region;
}

} // namespace subquest
