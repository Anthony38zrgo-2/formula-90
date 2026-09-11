#pragma once

#include "f90_audio_dsp.h"

#include <cstdint>

// The Faust-generated `post_combustion` DSP class lives in the global namespace
// (it is produced by faust, not wrapped in f90dsp). Declared here as an
// incomplete type so the Instance can hold a pointer without pulling in the full
// generated class in every translation unit.
class post_combustion;

namespace f90dsp {

uint64_t rt_alloc_total() noexcept;
uint64_t rt_alloc_count() noexcept;
uint64_t rt_free_count() noexcept;
void rt_alloc_mark_alloc() noexcept;
void rt_alloc_mark_free() noexcept;

class Instance {
public:
    explicit Instance(const f90_dsp_config& cfg) noexcept;
    ~Instance() noexcept;

    Instance(const Instance&) = delete;
    Instance& operator=(const Instance&) = delete;

    int32_t process(const f90_dsp_event_block& ev, const f90_dsp_controls& ctl,
                    float* out_left, float* out_right, uint32_t frames) noexcept;
    void reset() noexcept;
    void fill_diagnostics(f90_dsp_diagnostics& out) const noexcept;
    void count_nonfinite_input() {
        if (diag_.nonfinite_inputs < 0xFFFFFFFFu) {
            diag_.nonfinite_inputs += 1;
        }
    }
    const f90_dsp_config& config() const noexcept { return cfg_; }
    bool ready() const noexcept { return faust_ != nullptr; }

private:
    void count_violation();

    f90_dsp_config cfg_;
    f90_dsp_diagnostics diag_{};
    // One Faust post_combustion DSP instance holds all internal filter/modal
    // state across the whole audio stream (Fase 5). Declared as a pointer to an
    // incomplete type; the full class is only visible inside dsp_instance.cpp.
    class post_combustion* faust_ = nullptr;
    uint64_t last_stream_block_id_ = 0;
    bool has_stream_block_id_ = false;
};

} // namespace f90dsp
