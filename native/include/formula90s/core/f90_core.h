#pragma once
#include <cstdint>
#include "formula90s/sim/f90_sim_bridge.h"
#include "formula90s/vehicle/formula90_physics.h" // F90RuntimeConfig (mirror of FfiRuntimeConfig)

// C-API mirror of the orchestrator facade (`game/core/src/ffi.rs`, `formula90_core.dll`).
// ABI v2: `f90_core_step` carries the FULL aids mask (`aids_mask`, 8 bits) instead of
// a `toggle_tc` pulse, and `f90_core_apply_runtime_config` was added.
// Field order and types MUST match the Rust `#[repr(C)]` structs (they are locked by
// `ffi.rs::layout_tests` and the static_asserts in `f90_core.hpp`). Reuses
// `CSimTriRaycastSample` (mirrored in f90_sim_bridge.h) as the sample input, so the
// whole facade uses ONE family of structs.
#ifdef __cplusplus
extern "C" {
#endif

// Single output block of `f90_core_step`. MUST mirror `F90CoreFrameOut` in
// `game/core/src/ffi.rs` (offset-locked by tests + header static_asserts).
typedef struct F90CoreFrameOut {
    // physics
    double force_x, force_y, force_z;
    double torque_x, torque_y, torque_z;
    // compact telemetry
    double speed_kmh, rpm;
    int32_t gear;
    double steer, throttle;
    double lat_g, long_g, vert_g;
    double fl_comp_mm, fr_comp_mm, rl_comp_mm, rr_comp_mm;
    double front_slip, rear_slip;
    double tc_active; // 0.0 / 1.0
    double drive_torque;
    // pose / velocity
    double px, py, pz, yaw;
    double lvx, lvy, lvz, avx, avy, avz;
    // audio presentation readouts
    int32_t surface_code;      // 0=asphalt, 1=rumble, 2=grass, 3=sand
    int32_t active_bed_code;   // 0=none, 1=rumble, 2=grass, 3=sand
    int32_t trigger_code;      // -1 none, else legacy one-shot code 0..10
    float last_norm;
    double last_rpm;
    float last_throttle;
    double last_speed_kph;
    float last_slip;
    float last_engine_gain;
    float weights[5];
    float pitches[5];
} F90CoreFrameOut;

typedef uint32_t (*FnCoreAbiVersion)(void);
typedef const char *(*FnCoreBuildSha)(void);
typedef void *(*FnCoreCreate)(const char *opts_json, uint8_t *err_buf, uint32_t err_len);
typedef void (*FnCoreDestroy)(void *core);
typedef uint32_t (*FnCoreSpawn)(void *core);
typedef void (*FnCoreReset)(void *core, double x, double y, double z, double yaw);
typedef bool (*FnCoreApplyRuntimeConfig)(void *core, uint32_t id, const F90RuntimeConfig *config);
typedef void (*FnCoreStep)(void *core, uint32_t id,
    double x, double y, double z, double qx, double qy, double qz, double qw,
    double lx, double ly, double lz, double ax, double ay, double az,
    double throttle, double brake, double steer, double handbrake, double clutch,
    int8_t gear_request, uint32_t aids_mask, double dt,
    const CSimTriRaycastSample *samples, F90CoreFrameOut *out);
typedef uint32_t (*FnCoreAudioRender)(void *core, float *out_l, float *out_r, uint32_t n);
typedef bool (*FnCoreAudioTrigger)(void *core, int32_t code);
typedef void (*FnCoreAudioReadouts)(void *core, F90CoreFrameOut *out);
typedef uint32_t (*FnCoreSnapshot)(void *core, uint8_t *out, uint32_t cap, uint32_t *out_len);

#ifdef __cplusplus
}
#endif
