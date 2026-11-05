// capture_source.h — interface de qualquer fonte de quadros.
//
// A implementação real é a WgcCapture (Windows.Graphics.Capture). Ter uma
// interface permite trocar por uma fonte falsa nos testes (imagens do disco).
#pragma once

#include <cstdint>
#include <functional>
#include <string>

#include "common/types.h"

namespace subquest {

using FrameCallback = std::function<void(Frame&&)>; // chamado na thread de captura

class ICaptureSource {
public:
    virtual ~ICaptureSource() = default;

    virtual bool start(uint64_t window_handle, Rect region, FrameCallback on_frame) = 0; // começa a entregar quadros já recortados em `region`
    virtual void stop() = 0;                                                         // para e espera a thread de captura terminar
    virtual void set_region(Rect region) = 0;                                        // troca a região sem reiniciar a captura
    virtual const std::string& error() const = 0;                                    // motivo da última falha de start(), em texto
};

} // namespace subquest
