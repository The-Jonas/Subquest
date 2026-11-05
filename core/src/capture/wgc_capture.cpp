#include "capture/wgc_capture.h"

#ifdef _WIN32

#include <algorithm>
#include <chrono>
#include <cstring>
#include <future>
#include <mutex>
#include <stdexcept>
#include <thread>

#include <unknwn.h> // must come before C++/WinRT so com_ptr works with classic COM
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

// Creates the D3D11 device (default GPU) with BGRA support, required by capture.
void create_d3d_device(winrt::com_ptr<ID3D11Device>& device, winrt::com_ptr<ID3D11DeviceContext>& context) {
    winrt::check_hresult(D3D11CreateDevice(nullptr, D3D_DRIVER_TYPE_HARDWARE, nullptr,
                                           D3D11_CREATE_DEVICE_BGRA_SUPPORT, nullptr, 0,
                                           D3D11_SDK_VERSION, device.put(), nullptr, context.put()));
}

// Wraps the D3D11 device in the WinRT type the frame pool expects.
wdx::Direct3D11::IDirect3DDevice to_winrt_device(const winrt::com_ptr<ID3D11Device>& device) {
    auto dxgi = device.as<IDXGIDevice>();
    winrt::com_ptr<::IInspectable> inspectable;
    winrt::check_hresult(CreateDirect3D11DeviceFromDXGIDevice(dxgi.get(), inspectable.put()));
    return inspectable.as<wdx::Direct3D11::IDirect3DDevice>();
}

// Creates the "capture item" from the game's HWND.
wgc::GraphicsCaptureItem create_item_for_window(HWND hwnd) {
    auto interop = winrt::get_activation_factory<wgc::GraphicsCaptureItem, IGraphicsCaptureItemInterop>();
    wgc::GraphicsCaptureItem item{nullptr};
    winrt::check_hresult(interop->CreateForWindow(hwnd, winrt::guid_of<wgc::IGraphicsCaptureItem>(),
                                                  winrt::put_abi(item)));
    return item;
}

// Pulls the D3D11 texture out of the WinRT surface delivered with each frame.
winrt::com_ptr<ID3D11Texture2D> surface_texture(const wdx::Direct3D11::IDirect3DSurface& surface) {
    auto access = surface.as<::Windows::Graphics::DirectX::Direct3D11::IDirect3DDxgiInterfaceAccess>();
    winrt::com_ptr<ID3D11Texture2D> texture;
    winrt::check_hresult(access->GetInterface(winrt::guid_of<ID3D11Texture2D>(), texture.put_void()));
    return texture;
}

// Converts WinRT error text (UTF-16) to UTF-8.
std::string hresult_text(const winrt::hresult_error& e) {
    return winrt::to_string(e.message());
}

} // namespace

// Internal capture state. Everything D3D/WinRT lives here.
struct WgcCapture::Impl {
    std::thread thread;           // owns the COM apartment (MTA) and the session
    HANDLE stop_event = nullptr;  // tells the thread to finish

    std::mutex mutex;             // guards everything below (callback vs stop vs set_region)
    bool running = false;
    Rect region;
    FrameCallback on_frame;
    std::chrono::steady_clock::duration min_interval{};
    std::chrono::steady_clock::time_point last_emit{};
    uint64_t next_id = 1;

    winrt::com_ptr<ID3D11Device> device;
    winrt::com_ptr<ID3D11DeviceContext> context;
    winrt::com_ptr<ID3D11Texture2D> staging; // texture the CPU can read
    D3D11_TEXTURE2D_DESC staging_desc{};
    wdx::Direct3D11::IDirect3DDevice winrt_device{nullptr};
    wgc::GraphicsCaptureItem item{nullptr};
    wgc::Direct3D11CaptureFramePool pool{nullptr};
    wgc::GraphicsCaptureSession session{nullptr};
    wgc::Direct3D11CaptureFramePool::FrameArrived_revoker frame_arrived;
    winrt::Windows::Graphics::SizeInt32 last_size{};

    void thread_main(HWND hwnd, std::promise<std::string> ready); // sets up, waits for stop and tears down
    void on_frame_arrived(const wgc::Direct3D11CaptureFramePool& sender); // crops, copies and delivers one frame
    void ensure_staging(UINT width, UINT height, DXGI_FORMAT format);    // (re)creates the readback texture at the right size
};

// Ensures a STAGING texture the size of the region; recreates it only when the size changes.
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

// Called on a system thread for every new frame. Honors the FPS cap, crops the
// region ON THE GPU (CopySubresourceRegion) and only then copies to the CPU:
// that is what keeps the cost low even with the game at 4K.
void WgcCapture::Impl::on_frame_arrived(const wgc::Direct3D11CaptureFramePool& sender) {
    auto frame = sender.TryGetNextFrame();
    if (!frame) return;

    std::lock_guard lock(mutex);
    if (!running) return;

    // Window resized: recreate the pool at the new size (the current frame is still usable).
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

        // The texture may be larger than the content; use the smaller of the two.
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
        out.origin_x = r.x;
        out.origin_y = r.y;
        out.source_width = full_w;
        out.source_height = full_h;
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
        // Frame lost (e.g. GPU device reset). The next one tries again.
    }
}

// The whole session lives on its own thread: initializes COM as MTA (Tauri's
// main thread is already STA because of the WebView), sets everything up,
// notifies start() through the promise, waits for stop and tears down in reverse.
void WgcCapture::Impl::thread_main(HWND hwnd, std::promise<std::string> ready) {
    winrt::init_apartment(winrt::apartment_type::multi_threaded);
    try {
        if (!wgc::GraphicsCaptureSession::IsSupported())
            throw std::runtime_error("Windows.Graphics.Capture is not supported on this Windows version");

        create_d3d_device(device, context);
        winrt_device = to_winrt_device(device);
        item = create_item_for_window(hwnd);
        last_size = item.Size();
        pool = wgc::Direct3D11CaptureFramePool::CreateFreeThreaded(
            winrt_device, wdx::DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, last_size);
        session = pool.CreateCaptureSession(item);

        // Optional: no cursor and no yellow border (the border can only be hidden on Windows 11).
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
        ready.set_value("failed to start capture: " + hresult_text(e));
        winrt::uninit_apartment();
        return;
    } catch (const std::exception& e) {
        ready.set_value(e.what());
        winrt::uninit_apartment();
        return;
    }

    WaitForSingleObject(stop_event, INFINITE);

    {
        std::lock_guard lock(mutex); // waits for an in-flight frame to finish
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

// Stores the FPS cap; nothing D3D is created until start().
WgcCapture::WgcCapture(uint32_t max_fps) : impl_(std::make_unique<Impl>()), max_fps_(max_fps ? max_fps : 30) {}

// Makes sure the capture thread has finished before destruction.
WgcCapture::~WgcCapture() { stop(); }

// Starts the capture thread and waits for it to report success (synchronous for the caller).
bool WgcCapture::start(uint64_t window_handle, Rect region, FrameCallback on_frame) {
    if (impl_->thread.joinable()) {
        error_ = "capture is already running";
        return false;
    }
    HWND hwnd = reinterpret_cast<HWND>(static_cast<uintptr_t>(window_handle));
    if (!IsWindow(hwnd)) {
        error_ = "the selected window no longer exists";
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

// Signals the thread and waits for it to tear the session down. Idempotent.
void WgcCapture::stop() {
    if (!impl_->thread.joinable()) return;
    SetEvent(impl_->stop_event);
    impl_->thread.join();
    CloseHandle(impl_->stop_event);
    impl_->stop_event = nullptr;
}

// Changes the region read on each frame; takes effect from the next frame.
void WgcCapture::set_region(Rect region) {
    std::lock_guard lock(impl_->mutex);
    impl_->region = region;
}

} // namespace subquest

#else // ---------- outside Windows: only so the rest builds and tests ----------

namespace subquest {

struct WgcCapture::Impl {};

// Empty version: capture only exists on Windows.
WgcCapture::WgcCapture(uint32_t max_fps) : impl_(std::make_unique<Impl>()), max_fps_(max_fps) {}

// Nothing to release.
WgcCapture::~WgcCapture() = default;

// Always fails outside Windows, with a clear message.
bool WgcCapture::start(uint64_t, Rect, FrameCallback) {
    error_ = "capture is only available on Windows";
    return false;
}

// Nothing to stop.
void WgcCapture::stop() {}

// Nothing to adjust.
void WgcCapture::set_region(Rect) {}

} // namespace subquest

#endif
