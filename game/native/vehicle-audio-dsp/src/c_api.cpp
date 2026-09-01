#include "f90_audio_dsp.h"

#include "dsp_instance.h"

#include <cmath>
#include <cstdint>
#include <cstring>
#include <new>

#if __has_include("build_source.h")
#  include "build_source.h"
#else
#  define F90_DSP_BUILD_SOURCE "undefined"
#endif
#ifndef F90_DSP_BUILD_SOURCE
#  define F90_DSP_BUILD_SOURCE "undefined"
#endif

namespace {

f90dsp::Instance* as_instance(f90_dsp_handle h) {
    return reinterpret_cast<f90dsp::Instance*>(h);
}

float clamp01(float v) {
    return v < 0.0f ? 0.0f : (v > 1.0f ? 1.0f : v);
}

bool sanitize_controls(const f90_dsp_controls& in, f90_dsp_controls& out) {
    out = in;
    const float rpm = in.rpm;
    const float throttle = in.throttle;
    const float load = in.load;
    const float tc_cut = in.tc_cut;
    const float master_gain = in.master_gain;
    const bool nonfinite = !std::isfinite(rpm) || !std::isfinite(throttle) ||
                           !std::isfinite(load) || !std::isfinite(tc_cut) ||
                           !std::isfinite(master_gain);
    if (!std::isfinite(rpm)) {
        out.rpm = 0.0f;
    }
    if (!std::isfinite(throttle)) {
        out.throttle = 0.0f;
    }
    if (!std::isfinite(load)) {
        out.load = 0.0f;
    }
    if (!std::isfinite(tc_cut)) {
        out.tc_cut = 0.0f;
    }
    if (!std::isfinite(master_gain)) {
        out.master_gain = 0.0f;
    }
    out.rpm = out.rpm < 0.0f ? 0.0f : (out.rpm > 1.0e5f ? 1.0e5f : out.rpm);
    out.throttle = clamp01(out.throttle);
    out.load = clamp01(out.load);
    out.tc_cut = clamp01(out.tc_cut);
    out.master_gain = out.master_gain < 0.0f ? 0.0f : (out.master_gain > 4.0f ? 4.0f : out.master_gain);
    return nonfinite;
}

} // namespace

extern "C" {

uint32_t f90_dsp_abi_version(void) noexcept {
    return F90_AUDIO_DSP_ABI_VERSION;
}

const char* f90_dsp_build_source(void) noexcept {
    return F90_DSP_BUILD_SOURCE;
}

int32_t f90_dsp_check_build_source(const char* expected) noexcept {
    if (!expected) {
        return F90_DSP_ERR_NULL_ARGUMENT;
    }
    if (std::strlen(expected) != F90_DSP_BUILD_SOURCE_LEN ||
        std::strcmp(expected, F90_DSP_BUILD_SOURCE) != 0) {
        return F90_DSP_ERR_BUILD_MISMATCH;
    }
    return F90_DSP_OK;
}

int32_t f90_dsp_create(const f90_dsp_config* cfg, f90_dsp_handle* out) noexcept {
    if (!cfg || !out) {
        return F90_DSP_ERR_NULL_ARGUMENT;
    }
    if (cfg->struct_size < F90_DSP_MIN_CONFIG_SIZE || cfg->struct_size > sizeof(f90_dsp_config)) {
        return F90_DSP_ERR_INVALID_ABI;
    }
    if (cfg->sample_rate != 44100) {
        return F90_DSP_ERR_UNSUPPORTED_SAMPLE_RATE;
    }
    if (cfg->simulated_cylinders < 1 || cfg->simulated_cylinders > F90_DSP_MAX_CYLINDERS ||
        cfg->bank_count < 1 || cfg->bank_count > F90_DSP_MAX_BANKS ||
        (cfg->channels != 1 && cfg->channels != 2) || cfg->flags != 0) {
        return F90_DSP_ERR_NOT_SUPPORTED;
    }
    f90dsp::Instance* inst = new (std::nothrow) f90dsp::Instance(*cfg);
    if (!inst || !inst->ready()) {
        delete inst;
        return F90_DSP_ERR_INVALID_STATE;
    }
    *out = reinterpret_cast<f90_dsp_handle>(inst);
    return F90_DSP_OK;
}

int32_t f90_dsp_process(f90_dsp_handle h, const f90_dsp_event_block* ev,
                        const f90_dsp_controls* ctl, float* out_left, float* out_right,
                        uint32_t frames) noexcept {
    if (!h || !ev || !ctl || !out_left || !out_right) {
        return F90_DSP_ERR_NULL_ARGUMENT;
    }
    if (frames > F90_DSP_MAX_BLOCK_SAMPLES) {
        return F90_DSP_ERR_BLOCK_TOO_LARGE;
    }
    if (ev->block_samples != frames) {
        return F90_DSP_ERR_INVALID_STATE;
    }
    f90dsp::Instance* inst = as_instance(h);
    f90_dsp_controls clean{};
    const bool nonfinite = sanitize_controls(*ctl, clean);
    if (nonfinite) {
        inst->count_nonfinite_input();
    }
    const int32_t proc_rc = inst->process(*ev, clean, out_left, out_right, frames);
    if (nonfinite && proc_rc == F90_DSP_OK) {
        return F90_DSP_ERR_NONFINITE_INPUT;
    }
    return proc_rc;
}

int32_t f90_dsp_reset(f90_dsp_handle h) noexcept {
    if (!h) {
        return F90_DSP_ERR_NULL_ARGUMENT;
    }
    as_instance(h)->reset();
    return F90_DSP_OK;
}

int32_t f90_dsp_get_diagnostics(f90_dsp_handle h, f90_dsp_diagnostics* out) noexcept {
    if (!h || !out) {
        return F90_DSP_ERR_NULL_ARGUMENT;
    }
    as_instance(h)->fill_diagnostics(*out);
    return F90_DSP_OK;
}

void f90_dsp_destroy(f90_dsp_handle h) noexcept {
    delete as_instance(h);
}

} // extern "C"
