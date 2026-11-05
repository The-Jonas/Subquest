// latest_slot.h — "fila" de uma posição só: o item novo substitui o antigo.
//
// É o coração da baixa latência: se o OCR ainda está ocupado e chegam três
// quadros novos, só o MAIS RECENTE espera na vaga. Os outros são descartados
// em vez de formar uma fila que deixaria a tradução cada vez mais atrasada.
#pragma once

#include <condition_variable>
#include <mutex>
#include <optional>

namespace subquest {

template <typename T>
class LatestSlot {
public:
    // Coloca um item na vaga, descartando o que estiver lá; acorda quem espera.
    void put(T item) {
        {
            std::lock_guard lock(mutex_);
            item_ = std::move(item);
        }
        cv_.notify_one();
    }

    // Bloqueia até ter um item (retorna-o) ou até close() (retorna vazio).
    std::optional<T> take() {
        std::unique_lock lock(mutex_);
        cv_.wait(lock, [this] { return item_.has_value() || closed_; });
        if (!item_.has_value()) return std::nullopt;
        std::optional<T> out = std::move(item_);
        item_.reset();
        return out;
    }

    // Libera todas as threads presas em take(); usado ao encerrar a sessão.
    void close() {
        {
            std::lock_guard lock(mutex_);
            closed_ = true;
        }
        cv_.notify_all();
    }

    // Reabre a vaga para uma nova sessão.
    void reset() {
        std::lock_guard lock(mutex_);
        item_.reset();
        closed_ = false;
    }

private:
    std::mutex mutex_;
    std::condition_variable cv_;
    std::optional<T> item_;
    bool closed_ = false;
};

} // namespace subquest
