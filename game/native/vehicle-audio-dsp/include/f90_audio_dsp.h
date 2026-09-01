#ifndef F90_AUDIO_DSP_H
#define F90_AUDIO_DSP_H

#include <stdint.h>

#if defined(_WIN32) && defined(F90_AUDIO_DSP_SHARED)
#  if defined(F90_AUDIO_DSP_EXPORTS)
#    define F90_AUDIO_DSP_API __declspec(dllexport)
#  else
#    define F90_AUDIO_DSP_API __declspec(dllimport)
#  endif
#else
#  define F90_AUDIO_DSP_API
#endif

#define F90_AUDIO_DSP_ABI_VERSION 2u
#define F90_DSP_MAX_BLOCK_SAMPLES        4096u
#define F90_DSP_MAX_EVENTS_PER_BLOCK     512u
#define F90_DSP_BUILD_SOURCE_LEN         40u
#define F90_DSP_MAX_CYLINDERS            5u
#define F90_DSP_MAX_BANKS                2u
#define F90_DSP_MAX_CHANNELS             2u

#define F90_DSP_MIN_CONFIG_SIZE          24u

#define F90_DSP_OK                       0
#define F90_DSP_ERR_NULL_ARGUMENT        1
#define F90_DSP_ERR_INVALID_ABI          2
#define F90_DSP_ERR_BUILD_MISMATCH       3
#define F90_DSP_ERR_UNSUPPORTED_SAMPLE_RATE 4
#define F90_DSP_ERR_BLOCK_TOO_LARGE      5
#define F90_DSP_ERR_EVENT_OVERFLOW       6
#define F90_DSP_ERR_NONFINITE_INPUT      7
#define F90_DSP_ERR_INVALID_STATE        8
#define F90_DSP_ERR_NOT_SUPPORTED        9

typedef struct f90_dsp_instance f90_dsp_instance;
typedef f90_dsp_instance* f90_dsp_handle;

typedef struct f90_dsp_config {
    uint32_t struct_size;
    uint32_t sample_rate;
    uint8_t channels;
    uint8_t simulated_cylinders;
    uint8_t bank_count;
    uint8_t flags;
    float half_block_offset_deg;
    float idle_rpm;
    float max_rpm;
} f90_dsp_config;

typedef struct f90_dsp_event {
    uint32_t sample_offset;
    uint8_t cylinder;
    uint8_t bank;
    uint16_t reserved;
    float crank_phase_deg;
    float pressure;
    float pressure_derivative;
    float energy;
    float cycle_variation;
    float event_pad;
} f90_dsp_event;

typedef struct f90_dsp_event_block {
    uint64_t stream_block_id;
    uint32_t event_count;
    uint32_t block_samples;
    f90_dsp_event events[F90_DSP_MAX_EVENTS_PER_BLOCK];
} f90_dsp_event_block;

typedef struct f90_dsp_controls {
    float rpm;
    float throttle;
    float load;
    float tc_cut;
    float master_gain;
    uint8_t lod;
    uint8_t bypass;
    uint8_t reserved0;
    uint8_t reserved1;
} f90_dsp_controls;

typedef struct f90_dsp_diagnostics {
    uint32_t struct_size;
    uint32_t blocks_processed;
    uint32_t samples_processed;
    uint32_t events_received;
    uint32_t events_dropped;
    uint32_t nonfinite_outputs;
    uint32_t nonfinite_inputs;
    uint32_t rt_violations;
    uint32_t last_error_code;
    uint32_t reserved;
} f90_dsp_diagnostics;

#ifdef __cplusplus
extern "C" {
#endif

F90_AUDIO_DSP_API uint32_t f90_dsp_abi_version(void) noexcept;
F90_AUDIO_DSP_API const char* f90_dsp_build_source(void) noexcept;
F90_AUDIO_DSP_API int32_t f90_dsp_check_build_source(const char* expected) noexcept;
F90_AUDIO_DSP_API int32_t f90_dsp_create(const f90_dsp_config* cfg, f90_dsp_handle* out) noexcept;
F90_AUDIO_DSP_API int32_t f90_dsp_process(f90_dsp_handle h, const f90_dsp_event_block* ev, const f90_dsp_controls* ctl, float* out_left, float* out_right, uint32_t frames) noexcept;
F90_AUDIO_DSP_API int32_t f90_dsp_reset(f90_dsp_handle h) noexcept;
F90_AUDIO_DSP_API int32_t f90_dsp_get_diagnostics(f90_dsp_handle h, f90_dsp_diagnostics* out) noexcept;
F90_AUDIO_DSP_API void f90_dsp_destroy(f90_dsp_handle h) noexcept;

#ifdef __cplusplus
}
#endif

#ifdef __cplusplus
static_assert(sizeof(f90_dsp_config) == 24, "f90_dsp_config layout");
static_assert(alignof(f90_dsp_config) == 4, "f90_dsp_config alignment");
static_assert(sizeof(f90_dsp_event) == 32, "f90_dsp_event layout");
static_assert(alignof(f90_dsp_event) == 4, "f90_dsp_event alignment");
static_assert(sizeof(f90_dsp_event_block) == 16 + 512 * 32, "f90_dsp_event_block layout");
static_assert(alignof(f90_dsp_event_block) == 8, "f90_dsp_event_block alignment");
static_assert(sizeof(f90_dsp_controls) == 24, "f90_dsp_controls layout");
static_assert(alignof(f90_dsp_controls) == 4, "f90_dsp_controls alignment");
static_assert(sizeof(f90_dsp_diagnostics) == 40, "f90_dsp_diagnostics layout");
static_assert(alignof(f90_dsp_diagnostics) == 4, "f90_dsp_diagnostics alignment");
#endif

#endif
