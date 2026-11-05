// latest_slot.h — a one-item "queue": a new item replaces the old one.
//
// This is the heart of low latency: if OCR is still busy and three new frames
// arrive, only the NEWEST waits in the slot. The others are dropped instead of
// forming a queue that would make the translation fall further and further behind.
#pragma once

#include <condition_variable>
#include <mutex>
#include <optional>
#include <utility>
#include <vector>

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

// Several LatestSlots behind one wait: one slot per screen area. Each area
// keeps only its newest item, and take() serves the areas in turn, so a busy
// area (an animated background) can't starve a quiet one (a name label).
template <typename T>
class LatestSlots {
public:
    // Empties the slots and sets how many there are (a new session).
    void reset(size_t count) {
        std::lock_guard lock(mutex_);
        items_.assign(count, std::nullopt);
        next_ = 0;
        closed_ = false;
    }

    // Puts an item in slot `index`, dropping what was there; wakes up the waiter.
    void put(size_t index, T item) {
        {
            std::lock_guard lock(mutex_);
            if (index >= items_.size()) return;
            items_[index] = std::move(item);
        }
        cv_.notify_one();
    }

    // Blocks until some slot has an item (returns it with its index) or until close().
    std::optional<std::pair<size_t, T>> take() {
        std::unique_lock lock(mutex_);
        cv_.wait(lock, [this] { return closed_ || any(); });
        if (closed_) return std::nullopt;
        for (size_t step = 0; step < items_.size(); ++step) {
            const size_t i = (next_ + step) % items_.size();
            if (!items_[i]) continue;
            next_ = i + 1; // the next take() starts after this area
            std::pair<size_t, T> out{i, std::move(*items_[i])};
            items_[i].reset();
            return out;
        }
        return std::nullopt; // unreachable: any() was true
    }

    // Releases every thread blocked in take(); used when the session ends.
    void close() {
        {
            std::lock_guard lock(mutex_);
            closed_ = true;
        }
        cv_.notify_all();
    }

private:
    // True when at least one slot holds an item (mutex held).
    bool any() const {
        for (const auto& item : items_)
            if (item) return true;
        return false;
    }

    std::mutex mutex_;
    std::condition_variable cv_;
    std::vector<std::optional<T>> items_;
    size_t next_ = 0;
    bool closed_ = false;
};

} // namespace subquest
