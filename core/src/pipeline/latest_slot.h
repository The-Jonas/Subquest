// latest_slot.h — a one-item "queue": a new item replaces the old one.
//
// This is the heart of low latency: if OCR is still busy and three new frames
// arrive, only the NEWEST waits in the slot. The others are dropped instead of
// forming a queue that would make the translation fall further and further behind.
#pragma once

#include <condition_variable>
#include <mutex>
#include <optional>

namespace subquest {

template <typename T>
class LatestSlot {
public:
    // Puts an item in the slot, dropping whatever was there; wakes up the waiter.
    void put(T item) {
        {
            std::lock_guard lock(mutex_);
            item_ = std::move(item);
        }
        cv_.notify_one();
    }

    // Blocks until there is an item (returns it) or until close() (returns empty).
    std::optional<T> take() {
        std::unique_lock lock(mutex_);
        cv_.wait(lock, [this] { return item_.has_value() || closed_; });
        if (!item_.has_value()) return std::nullopt;
        std::optional<T> out = std::move(item_);
        item_.reset();
        return out;
    }

    // Releases every thread blocked in take(); used when the session ends.
    void close() {
        {
            std::lock_guard lock(mutex_);
            closed_ = true;
        }
        cv_.notify_all();
    }

    // Reopens the slot for a new session.
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
