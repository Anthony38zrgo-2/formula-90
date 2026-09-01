#include "dsp_instance.h"

#include <atomic>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <new>

// The Faust-generated class uses constructs (unreferenced parameters, an override
// whose signature differs only by const, etc.) that trip /W4 /WX. It is third
// party generated code, so suppress those specific warnings around the include
// rather than weakening the warning level of our own sources.
#pragma warning(push)
#pragma warning(disable: 4100 4127 4189 4244 4305 4389 4458 4701 4702 4800 4996 4373 4150)
#include "post_combustion.h"
#pragma warning(pop)

namespace f90dsp {

namespace {
std::atomic<uint64_t> g_alloc_total{0};
std::atomic<uint64_t> g_alloc_count{0};
std::atomic<uint64_t> g_free_count{0};

// Denormal flush threshold (kept from Fase 3): values below this are forced to
// exactly zero so tails do not leave subnormal noise. Deterministic, so it does
// not affect bit-equality across block sizes.
constexpr float kFlushThreshold = 1.0e-30f;

// ABI v2 intentionally transports pressure_derivative in physical
// pressure-units/second. Faust, however, consumes an audio-rate excitation.
// Keep the ABI physical and perform the unit conversion exactly once here.
//
// The current V10 model produces roughly 6k..30k pressure-units/s over the
// validated sweep. 30k therefore maps to ~1.0 acoustic excitation. Values above
// the model envelope are bounded so malformed/experimental profiles cannot drive
// the modal bank into permanent saturation.
constexpr float kPressureDerivativeReferencePerSecond = 30000.0f;
constexpr float kMaxNormalizedEventExcitation = 1.5f;
constexpr float kMaxSummedExcitation = 2.5f;

inline float clampf(float value, float lo, float hi) noexcept {
    return value < lo ? lo : (value > hi ? hi : value);
}

inline float normalize_pressure_derivative(float physical_per_second) noexcept {
    return clampf(
        physical_per_second / kPressureDerivativeReferencePerSecond,
        -kMaxNormalizedEventExcitation,
        kMaxNormalizedEventExcitation
    );
}
} // namespace

uint64_t rt_alloc_total() noexcept { return g_alloc_total.load(std::memory_order_relaxed); }
uint64_t rt_alloc_count() noexcept { return g_alloc_count.load(std::memory_order_relaxed); }
uint64_t rt_free_count() noexcept { return g_free_count.load(std::memory_order_relaxed); }
void rt_alloc_mark_alloc() noexcept {
    g_alloc_total.fetch_add(1, std::memory_order_relaxed);
    g_alloc_count.fetch_add(1, std::memory_order_relaxed);
}
void rt_alloc_mark_free() noexcept {
    g_alloc_total.fetch_sub(1, std::memory_order_relaxed);
    g_free_count.fetch_add(1, std::memory_order_relaxed);
}

Instance::Instance(const f90_dsp_config& cfg) noexcept : cfg_(cfg) {
    faust_ = new (std::nothrow) post_combustion();
    if (faust_) {
        faust_->init(static_cast<int>(cfg_.sample_rate));
    }
}

Instance::~Instance() noexcept {
    delete faust_;
}

void Instance::count_violation() {
    if (diag_.rt_violations < 0xFFFFFFFFu) {
        diag_.rt_violations += 1;
    }
}

int32_t Instance::process(const f90_dsp_event_block& ev, const f90_dsp_controls& ctl,
                          float* out_left, float* out_right, uint32_t frames) noexcept {
    const uint64_t alloc_before = rt_alloc_count();
    const uint64_t free_before = rt_free_count();
    int32_t rc = F90_DSP_OK;
    if (ev.event_count > F90_DSP_MAX_EVENTS_PER_BLOCK) {
        diag_.events_dropped += ev.event_count - F90_DSP_MAX_EVENTS_PER_BLOCK;
        diag_.last_error_code = F90_DSP_ERR_EVENT_OVERFLOW;
        for (uint32_t f = 0; f < frames; ++f) { out_left[f] = 0.0f; out_right[f] = 0.0f; }
        return F90_DSP_ERR_EVENT_OVERFLOW;
    }
    if (has_stream_block_id_ && ev.stream_block_id <= last_stream_block_id_) {
        diag_.last_error_code = F90_DSP_ERR_INVALID_STATE;
        for (uint32_t f = 0; f < frames; ++f) { out_left[f] = 0.0f; out_right[f] = 0.0f; }
        return F90_DSP_ERR_INVALID_STATE;
    }
    const uint32_t used = ev.event_count;

    bool invalid = false;
    bool nonfinite_event = false;
    uint32_t prev_offset = 0;
    uint8_t prev_bank = 0;
    uint8_t prev_cylinder = 0;
    uint32_t valid_count = 0;
    float derivs[F90_DSP_MAX_EVENTS_PER_BLOCK];
    uint32_t offsets[F90_DSP_MAX_EVENTS_PER_BLOCK];
    for (uint32_t i = 0; i < used; ++i) {
        const f90_dsp_event& e = ev.events[i];
        const bool finite = std::isfinite(e.crank_phase_deg) && std::isfinite(e.pressure) &&
                            std::isfinite(e.pressure_derivative) && std::isfinite(e.energy) &&
                            std::isfinite(e.cycle_variation);
        if (!finite || e.sample_offset >= ev.block_samples || e.bank >= cfg_.bank_count ||
            e.cylinder >= cfg_.simulated_cylinders) {
            nonfinite_event = !finite;
            invalid = true;
            break;
        }
        if (valid_count > 0 &&
            (e.sample_offset < prev_offset ||
             (e.sample_offset == prev_offset && e.bank < prev_bank) ||
             (e.sample_offset == prev_offset && e.bank == prev_bank && e.cylinder < prev_cylinder))) {
            invalid = true;
            break;
        }
        prev_offset = e.sample_offset;
        prev_bank = e.bank;
        prev_cylinder = e.cylinder;
        // ABI v2 carries a physical derivative (pressure-units/s), not audio
        // amplitude. Normalize at this single boundary before Faust. Keeping
        // the conversion here prevents a unit mismatch while preserving the ABI
        // and keeps every upstream producer mechanically meaningful.
        derivs[valid_count] = normalize_pressure_derivative(e.pressure_derivative);
        offsets[valid_count] = e.sample_offset;
        valid_count += 1;
    }

    if (invalid) {
        rc = nonfinite_event ? F90_DSP_ERR_NONFINITE_INPUT : F90_DSP_ERR_INVALID_STATE;
        if (nonfinite_event && diag_.nonfinite_inputs < 0xFFFFFFFFu) {
            diag_.nonfinite_inputs += 1;
        }
        for (uint32_t f = 0; f < frames; ++f) {
            out_left[f] = 0.0f;
            out_right[f] = 0.0f;
        }
        diag_.last_error_code = static_cast<uint32_t>(rc);
        diag_.blocks_processed += 1;
        diag_.samples_processed += frames;
        return rc;
    }
    last_stream_block_id_ = ev.stream_block_id;
    has_stream_block_id_ = true;

    // Rust is the temporal authority: every event (both banks) carries its exact
    // sample_offset. The DLL only triggers what it receives, exactly at
    // sample_offset (Fase 2.1/3.1/3.2).
    const float master_gain = ctl.master_gain < 0.0f ? 0.0f : (ctl.master_gain > 4.0f ? 4.0f : ctl.master_gain);
    const bool bypass = ctl.bypass != 0;
    // Event derivative already contains the physical event amplitude. `load`
    // therefore acts only as a secondary acoustic/timbre control; multiplying by
    // raw throttle here would square the load response and collapse the layer on
    // lift-off. Keep a floor so coast/idle events retain their resonant tail.
    const float load = clampf(ctl.load, 0.0f, 1.0f);
    const float level = 0.55f + 0.45f * load;
    const float intake = 0.0f;

    // Per-frame Faust I/O buffers (1 sample each).
    float in_buf[3] = {0.0f, level, intake};
    float* in_ptrs[3] = {&in_buf[0], &in_buf[1], &in_buf[2]};
    float out_buf = 0.0f;
    float* out_ptrs[1] = {&out_buf};

    uint32_t ei = 0;
    for (uint32_t f = 0; f < frames; ++f) {
        float exc = 0.0f;
        while (ei < valid_count && offsets[ei] == f) {
            if (diag_.events_received < 0xFFFFFFFFu) {
                diag_.events_received += 1;
            }
            exc += derivs[ei];
            ++ei;
        }
        exc = clampf(exc, -kMaxSummedExcitation, kMaxSummedExcitation);
        // Advance ALL Faust DSP state every frame, even during bypass, so filters
        // and envelopes keep running and no stale voice replays later (Fase 3.3).
        // Only the audible contribution is gated by bypass.
        in_buf[0] = exc;
        in_buf[1] = level;
        in_buf[2] = intake;
        if (faust_) {
            faust_->compute(1, in_ptrs, out_ptrs);
        }
        float y = out_buf;
        if (!std::isfinite(y)) {
            y = 0.0f;
            if (diag_.nonfinite_outputs < 0xFFFFFFFFu) {
                diag_.nonfinite_outputs += 1;
            }
        } else if (y > 1.0f) {
            y = 1.0f;
        } else if (y < -1.0f) {
            y = -1.0f;
        }
        if (y != 0.0f && std::fabs(y) < kFlushThreshold) {
            y = 0.0f;
        }
        const float g = bypass ? 0.0f : y * master_gain;
        // Dual-mono: the single Faust output is written to BOTH channels
        // identically (Fase 3 dual-mono invariant).
        out_left[f] = g;
        out_right[f] = g;
    }

    diag_.last_error_code = static_cast<uint32_t>(rc);
    diag_.blocks_processed += 1;
    diag_.samples_processed += frames;

    if (rt_alloc_count() != alloc_before || rt_free_count() != free_before) {
        count_violation();
    }
    return rc;
}

void Instance::reset() noexcept {
    if (faust_) {
        faust_->instanceClear();
    }
    const uint32_t declared = diag_.struct_size;
    diag_ = f90_dsp_diagnostics{};
    diag_.struct_size = declared;
    last_stream_block_id_ = 0;
    has_stream_block_id_ = false;
}

void Instance::fill_diagnostics(f90_dsp_diagnostics& out) const noexcept {
    const uint32_t declared = out.struct_size;
    f90_dsp_diagnostics tmp = diag_;
    tmp.struct_size = declared;
    out = tmp;
}

} // namespace f90dsp
