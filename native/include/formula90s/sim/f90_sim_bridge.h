#pragma once
#include <cstdint>

// C-API mirror of `game_sim`'s `c_abi.rs` `#[repr(C)]` structs. Field order and
// types MUST match the Rust side. `bool` is 1-byte on this toolchain, matching
// Rust's `bool` (consistent with the existing formula90_physics.h FFI).
#ifdef __cplusplus
extern "C" {
#endif

typedef struct CSimRaycastHit {
    bool is_colliding;
    double distance;
    double px, py, pz;
    double nx, ny, nz;
    uint32_t surface;
} CSimRaycastHit;

typedef struct CSimTriRaycastSample {
    CSimRaycastHit inner;
    CSimRaycastHit center;
    CSimRaycastHit outer;
} CSimTriRaycastSample;

typedef struct CSimPose {
    double x, y, z, yaw;
} CSimPose;

typedef struct CSimTelemetry {
    double speed_kmh, rpm, gear, steer, lat_g, long_g, vert_g;
    double fl_comp_mm, fr_comp_mm, rl_comp_mm, rr_comp_mm;
    double front_slip, rear_slip, tc_active, drive_torque;
} CSimTelemetry;

typedef void *(*FnSimWorldCreate)(double dt);
typedef void (*FnSimWorldDestroy)(void *world);
typedef uint32_t (*FnSimWorldSpawnFromJson)(void *world, const char *json_path);
typedef uint32_t (*FnSimWorldSpawnCanonical)(void *world);
typedef void (*FnSimWorldSetInput)(void *world, uint32_t id, double throttle, double brake,
    double steer, double handbrake, double clutch, int8_t gear_request, bool toggle_tc);
typedef void (*FnSimWorldStep)(void *world);
typedef void (*FnSimWorldSetPose)(void *world, uint32_t id, double x, double y, double z, double yaw);
typedef void (*FnSimWorldSetPoseAndVelocity)(void *world, uint32_t id, double x, double y, double z, double yaw,
    double lx, double ly, double lz, double ax, double ay, double az);
typedef void (*FnSimWorldStepWithSamples)(void *world, uint32_t id, double throttle, double brake,
    double steer, double handbrake, double clutch, int8_t gear_request, bool toggle_tc, double dt,
    const CSimTriRaycastSample *samples);
typedef void (*FnSimWorldSolveExternal)(void *world, uint32_t id, double x, double y, double z, double yaw,
    double lx, double ly, double lz, double ax, double ay, double az, double throttle, double brake,
    double steer, double handbrake, double clutch, int8_t gear_request, bool toggle_tc, double dt,
    const CSimTriRaycastSample *samples, double *out_force, double *out_torque);
typedef void (*FnSimWorldPose)(void *world, uint32_t id, CSimPose *out);
typedef void (*FnSimWorldTelemetry)(void *world, uint32_t id, CSimTelemetry *out);
typedef void (*FnSimWorldFlatSamples)(void *world, uint32_t id, CSimTriRaycastSample *out);

#ifdef __cplusplus
}
#endif
