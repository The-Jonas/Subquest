#include "capture/wgc_capture.h"

#ifdef _WIN32

#include <algorithm>
#include <chrono>
#include <cstring>
#include <future>
#include <mutex>
#include <stdexcept>
#include <thread>

#include <unknwn.h> // precisa vir antes do C++/WinRT para o com_ptr funcionar com COM clássico
#include <windows.h>
#include <d3d11.h>
#include <dxgi.h>
#include <windows.graphics.capture.interop.h>
#include <windows.graphics.directx.direct3d11.interop.h>

#include <winrt/base.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Graphics.Capture.h>
#include <winrt/Windows.Graphics.DirectX.h>
#include <winrt/Windows.Graphics.DirectX.Direct3D11.h>

namespace wgc = winrt::Windows::Graphics::Capture;
namespace wdx = winrt::Windows::Graphics::DirectX;

namespace subquest {

namespace {

// Cria o dispositivo D3D11 (GPU padrão) com suporte a BGRA, exigido pela captura.
void create_d3d_device(winrt::com_ptr<ID3D11Device>& device, winrt::com_ptr<ID3D11DeviceContext>& context) {
    winrt::check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
                                           D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0,
                                           D3D11_SDK_VERSION, device.put(), nullptr, context.put()));
}

// Embrulha o dispositivo D3D11 no tipo do WinRT que o frame pool espera.
wdx::Direct3D11::IDirect3DDevice to_winrt_device(const winrt::com_ptr<ID3D11Device>& device) {
    auto dxgi = device.as<IDXGIDevice>();
    winrt::com_ptr<::IInspectable> inspectable;
    winrt::check_hresult(CreateDirect3D11DeviceFromDXGIDevice(dxgi.get(), inspectable.put()));
    return inspectable.as<wdx::Direct3D11::IDirect3DDevice>();
}

// Cria o "item de captura" a partir do HWND do jogo.
wgc::GraphicsCaptureItem create_item_for_window(HWND hwnd) {
    auto interop = winrt::get_activation_factory<wgc::GraphicsCaptureItem, IGraphicsCaptureItemInterop>();
    wgc::GraphicsCaptureItem item{nullptr};
    winrt::check_hresult(interop->CreateForWindow(hwnd, winrt::guid_of<wgc::IGraphicsCaptureItem>(),
                                                  winrt::put_abi(item)));
    return item;
}

// Extrai a textura D3D11 de dentro da superfície WinRT entregue a cada quadro.
winrt::com_ptr<ID3D11Texture2D> surface_texture(const wdx::Direct3D11::IDirect3DSurface& surface) {
    auto access = surface.as<::Windows::Graphics::DirectX::Direct3D11::IDirect3DDxgiInterfaceAccess>();
    winrt::com_ptr<ID3D11Texture2D> texture;
    winrt::check_hresult(access->GetInterface(winrt::guid_of<ID3D11Texture2D>(), texture.put_void()));
    return texture;
}

// Converte o texto de erro do WinRT (UTF-16) para UTF-8.
std::string hresult_text(const winrt::hresult_error& e) {
    return winrt::to_string(e.message());
}

} // namespace

// Estado interno da captura. Tudo que é D3D/WinRT mora aqui.
struct WgcCapture::Impl {
    std::thread thread;           // dona do apartamento COM (MTA) e da sessão
    HANDLE stop_event = nullptr;  // sinaliza para a thread encerrar

    std::mutex mutex;             // protege tudo abaixo (callback x stop x set_region)
    bool running = false;
    Rect region;
    FrameCallback on_frame;
    std::chrono::steady_clock::duration min_interval{};
    std::chrono::steady_clock::time_point last_emit{};
    uint64_t next_id = 1;

    winrt::com_ptr<ID3D11Device> device;
    winrt::com_ptr<ID3D11DeviceContext> context;
    winrt::com_ptr<ID3D11Texture2D> staging; // textura que a CPU consegue ler
    D3D11_TEXTURE2D_DESC staging_desc{};
    wdx::Direct3D11::IDirect3DDevice winrt_device{nullptr};
    wgc::GraphicsCaptureItem item{nullptr};
    wgc::Direct3D11CaptureFramePool pool{nullptr};
    wgc::GraphicsCaptureSession session{nullptr};
    wgc::Direct3D11CaptureFramePool::FrameArrived_revoker frame_arrived;
    winrt::Windows::Graphics::SizeInt32 last_size{};

    void thread_main(HWND hwnd, std::promise<std::string> ready); // monta, espera o stop e desmonta
    void on_frame_arrived(const wgc::Direct3D11CaptureFramePool& sender); // recorta, copia e entrega um quadro
    void ensure_staging(UINT width, UINT height, DXGI_FORMAT format);    // (re)cria a textura de leitura no tamanho certo
};

// Garante uma textura STAGING do tamanho da região; recria só quando o tamanho muda.
void WgcCapture::Impl::ensure_staging(UINT width, UINT height, DXGI_FORMAT format) {
    if (staging && staging_desc.Width == width && staging_desc.Height == height) return;
    D3D11_TEXTURE2D_DESC desc{};
    desc.Width = width;
    desc.Height = height;
    desc.MipLevels = 1;
    desc.ArraySize = 1;
    desc.Format = format;
    desc.SampleDesc.Count = 1;
    desc.Usage = D3D11_USAGE_STAGING;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
    staging = nullptr;
    winrt::check_hresult(device->CreateTexture2D(&desc, nullptr, staging.put()));
    staging_desc = desc;
}

// Chamado numa thread do sistema a cada quadro novo. Respeita o limite de FPS,
// recorta a região NA GPU (CopySubresourceRegion) e só então copia para a CPU:
// é isso que mantém o custo baixo mesmo com o jogo em 4K.
void WgcCapture::Impl::on_frame_arrived(const wgc::Direct3D11CaptureFramePool& sender) {
    auto frame = sender.TryGetNextFrame();
    if (!frame) return;

    std::lock_guard lock(mutex);
    if (!running) return;

    // Janela redimensionada: recria o pool no novo tamanho (o quadro atual ainda serve).
    const auto content = frame.ContentSize();
    if (content.Width != last_size.Width || content.Height != last_size.Height) {
        last_size = content;
        sender.Recreate(winrt_device, wdx::DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, content);
    }

    const auto now = std::chrono::steady_clock::now();
    if (now - last_emit < min_interval) return;
    last_emit = now;

    try {
        auto texture = surface_texture(frame.Surface());
        D3D11_TEXTURE2D_DESC desc{};
        texture->GetDesc(&desc);

        // A textura pode ser maior que o conteúdo; vale o menor dos dois.
        const int32_t full_w = std::min<int32_t>(content.Width, int32_t(desc.Width));
        const int32_t full_h = std::min<int32_t>(content.Height, int32_t(desc.Height));

        Rect r = region.empty() ? Rect{0, 0, full_w, full_h} : region;
        r.x = std::clamp(r.x, 0, std::max(0, full_w - 1));
        r.y = std::clamp(r.y, 0, std::max(0, full_h - 1));
        r.width = std::min(r.width, full_w - r.x);
        r.height = std::min(r.height, full_h - r.y);
        if (r.width <= 0 || r.height <= 0) return;

        ensure_staging(UINT(r.width), UINT(r.height), desc.Format);
        D3D11_BOX box{UINT(r.x), UINT(r.y), 0, UINT(r.x + r.width), UINT(r.y + r.height), 1};
        context->CopySubresourceRegion(staging.get(), 0, 0, 0, 0, texture.get(), 0, &box);

        D3D11_MAPPED_SUBRESOURCE mapped{};
        winrt::check_hresult(context->Map(staging.get(), 0, D3D11_MAP_READ, 0, &mapped));
        Frame out;
        out.width = r.width;
        out.height = r.height;
        out.stride = r.width * 4;
        out.bgra.resize(size_t(out.stride) * out.height);
        for (int32_t y = 0; y < out.height; ++y) {
            const auto* src = static_cast<const uint8_t*>(mapped.pData) + size_t(y) * mapped.RowPitch;
            std::memcpy(out.bgra.data() + size_t(y) * out.stride, src, size_t(out.stride));
        }
        context->Unmap(staging.get(), 0);

        out.id = next_id++;
        out.timestamp_us = uint64_t(std::chrono::duration_cast<std::chrono::microseconds>(
                                        now.time_since_epoch()).count());
        if (on_frame) on_frame(std::move(out));
    } catch (const winrt::hresult_error&) {
        // Quadro perdido (ex.: dispositivo da GPU resetado). O próximo tenta de novo.
    }
}

// Vida inteira da sessão numa thread própria: inicializa o COM em modo MTA
// (a thread principal do Tauri já é STA por causa do WebView), monta tudo,
// avisa start() pelo promise, espera o stop e desmonta na ordem inversa.
void WgcCapture::Impl::thread_main(HWND hwnd, std::promise<std::string> ready) {
    winrt::init_apartment(winrt::apartment_type::multi_threaded);
    try {
        if (!wgc::GraphicsCaptureSession::IsSupported())
            throw std::runtime_error("Windows.Graphics.Capture não é suportado neste Windows");

        create_d3d_device(device, context);
        winrt_device = to_winrt_device(device);
        item = create_item_for_window(hwnd);
        last_size = item.Size();
        pool = wgc::Direct3D11CaptureFramePool::CreateFreeThreaded(
            winrt_device, wdx::DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, last_size);
        session = pool.CreateCaptureSession(item);

        // Opcionais: sem cursor e sem a borda amarela (a borda só pode sumir no Windows 11).
        try { session.IsCursorCaptureEnabled(false); } catch (...) {}
        try { session.IsBorderRequired(false); } catch (...) {}

        frame_arrived = pool.FrameArrived(winrt::auto_revoke,
            [this](const wgc::Direct3D11CaptureFramePool& sender, const winrt::Windows::Foundation::IInspectable&) {
                on_frame_arrived(sender);
            });

        {
            std::lock_guard lock(mutex);
            running = true;
        }
        session.StartCapture();
        ready.set_value({});
    } catch (const winrt::hresult_error& e) {
        ready.set_value("falha ao iniciar a captura: " + hresult_text(e));
        winrt::uninit_apartment();
        return;
    } catch (const std::exception& e) {
        ready.set_value(e.what());
        winrt::uninit_apartment();
        return;
    }

    WaitForSingleObject(stop_event, INFINITE);

    {
        std::lock_guard lock(mutex); // espera um quadro em processamento terminar
        running = false;
    }
    frame_arrived.revoke();
    session.Close();
    pool.Close();
    session = nullptr;
    pool = nullptr;
    item = nullptr;
    staging = nullptr;
    winrt::uninit_apartment();
}

// Guarda o limite de FPS; nada de D3D é criado até start().
WgcCapture::WgcCapture(uint32_t max_fps) : impl_(std::make_unique<Impl>()), max_fps_(max_fps ? max_fps : 30) {}

// Garante que a thread de captura terminou antes de destruir o objeto.
WgcCapture::~WgcCapture() { stop(); }

// Sobe a thread de captura e espera ela dizer se deu certo (síncrono para quem chama).
bool WgcCapture::start(uint64_t window_handle, Rect region, FrameCallback on_frame) {
    if (impl_->thread.joinable()) {
        error_ = "captura já está rodando";
        return false;
    }
    HWND hwnd = reinterpret_cast<HWND>(static_cast<uintptr_t>(window_handle));
    if (!IsWindow(hwnd)) {
        error_ = "a janela escolhida não existe mais";
        return false;
    }

    {
        std::lock_guard lock(impl_->mutex);
        impl_->region = region;
        impl_->on_frame = std::move(on_frame);
        impl_->min_interval = std::chrono::microseconds(1'000'000 / max_fps_);
        impl_->last_emit = {};
    }
    impl_->stop_event = CreateEventW(nullptr, TRUE, FALSE, nullptr);

    std::promise<std::string> ready;
    auto result = ready.get_future();
    impl_->thread = std::thread(&Impl::thread_main, impl_.get(), hwnd, std::move(ready));
    error_ = result.get();
    if (!error_.empty()) {
        impl_->thread.join();
        CloseHandle(impl_->stop_event);
        impl_->stop_event = nullptr;
        return false;
    }
    return true;
}

// Sinaliza a thread e espera ela desmontar a sessão. Idempotente.
void WgcCapture::stop() {
    if (!impl_->thread.joinable()) return;
    SetEvent(impl_->stop_event);
    impl_->thread.join();
    CloseHandle(impl_->stop_event);
    impl_->stop_event = nullptr;
}

// Troca a região lida a cada quadro; vale a partir do próximo quadro.
void WgcCapture::set_region(Rect region) {
    std::lock_guard lock(impl_->mutex);
    impl_->region = region;
}

} // namespace subquest

#else // ---------- fora do Windows: só para compilar e testar o resto ----------

namespace subquest {

struct WgcCapture::Impl {};

// Versão vazia: a captura só existe no Windows.
WgcCapture::WgcCapture(uint32_t max_fps) : impl_(std::make_unique<Impl>()), max_fps_(max_fps) {}

// Nada a liberar.
WgcCapture::~WgcCapture() = default;

// Sempre falha fora do Windows, com uma mensagem clara.
bool WgcCapture::start(uint64_t, Rect, FrameCallback) {
    error_ = "captura disponível apenas no Windows";
    return false;
}

// Nada a parar.
void WgcCapture::stop() {}

// Nada a ajustar.
void WgcCapture::set_region(Rect) {}

} // namespace subquest

#endif
