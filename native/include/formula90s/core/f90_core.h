#pragma once
#include <cstdint>
#include "formula90s/sim/f90_sim_bridge.h"
#include "formula90s/vehicle/formula90_physics.h" // F90RuntimeConfig (mirror of FfiRuntimeConfig)

// C-API mirror of the orchestrator facade (game/crates/formula90-core/src/ffi.rs,
// formula90_core.dll). ABI v11 appends powertrain diagnostics used by CSV telemetry.
// Field order and types MUST match the Rust #[repr(C)] structs (they are locked by
// ffi.rs::layout_tests and the static_asserts in f90_core.hpp). Reuses
// CSimTriRaycastSample (mirrored in f90_sim_bridge.h) as the sample input, so the
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
    // tire pressure + thermal (per wheel, WheelIndex order FL/FR/RL/RR)
    double tire_pressure_kpa[4];
    double tire_tread_inner_c[4];
    double tire_tread_center_c[4];
    double tire_tread_outer_c[4];
    double tire_carcass_c[4];
    double tire_gas_c[4];
    double brake_disc_c[4];
    double brake_caliper_c[4];
    double brake_hub_c[4];
    double brake_rim_c[4];
    double brake_efficiency[4];
    double duct_mass_flow_kg_s[4];
    double duct_drag_n[4];
    double total_brake_duct_drag_n;
    double brake_optimal_min_c;
    double brake_optimal_max_c;
    double brake_fade_start_c;
    double brake_critical_c;
    double brake_torque_nm[4];
    double brake_spin_pre_rad_s[4];
    double brake_spin_post_rad_s[4];
    double brake_power_w[4];
    double brake_energy_j[4];
    double brake_disc_bulk_c[4];
    double brake_surface_capacity_j_k[4];
    double brake_bulk_capacity_j_k[4];
    double brake_surface_bulk_w_k[4];
    double brake_natural_cooling_w_k[4];
    double brake_speed_cooling_w_k[4];
    double brake_surface_to_bulk_heat_w[4];
    double underfloor_clearance_m[5];
    uint32_t underfloor_valid_mask;
    int32_t underfloor_scrape_phase;
    double underfloor_min_clearance_m;
    double underfloor_rake_rad;
    double underfloor_roll_rad;
    double underfloor_contact_confidence;
    double underfloor_scrape_intensity;
    float audio_scrape_gain;
    float audio_scrape_pitch;
    double audio_scrape_cursor;
    double underfloor_compression_m[5];
    double underfloor_closing_speed_m_s[5];
    double underfloor_normal_force_n[5];
    int32_t underfloor_bottoming_phase[5];
    uint32_t underfloor_active_probe_mask;
    double underfloor_total_normal_force_n;
    double underfloor_max_probe_force_n;
    double underfloor_force_center_local[3];
    double underfloor_bottoming_torque[3];
    double underfloor_dissipated_energy_j;
    double underfloor_rigid_contact_blend;
    double aero_total_downforce_n;
    double aero_raw_downforce_n;
    double aero_front_downforce_n;
    double aero_floor_downforce_n;
    double aero_rear_downforce_n;
    double aero_drag_n;
    double aero_front_wing_angle_deg;
    double aero_rear_wing_angle_deg;
    double aero_front_wing_cl;
    double aero_rear_wing_cl;
    double aero_floor_height_factor;
    double aero_floor_rake_factor;
    double aero_floor_seal_factor;
    double aero_diffuser_expansion_deg;
    double aero_diffuser_stall_factor;
    double aero_global_limit_factor;
    double aero_load_ratio;
    double aero_balance_front;
    // Append-only ABI 11 powertrain diagnostics.
    double wheel_drive_torque_nm[4];
    double tc_cut_ratio;
    double net_drive_power_w;
    // Append-only ABI 12 expanded traction-control diagnostics.
    double tc_enabled;
    double tc_eligible;
    double tc_gear_authority;
    double tc_slip_target;
    double tc_raw_cut_ratio;
    double tc_slip_ratio[4];
    double wheel_drive_torque_pre_tc_nm[4];
    double pre_tc_drive_power_w;
} F90CoreFrameOut;

typedef struct F90UnderfloorRayHit {
    double valid, clearance_m;
    double point_x, point_y, point_z;
    double normal_x, normal_y, normal_z;
    double surface_code;
} F90UnderfloorRayHit;

typedef struct F90UnderfloorSample {
    F90UnderfloorRayHit rays[5];
    double rigid_confirmed;
    double rigid_local_x, rigid_local_y, rigid_local_z;
    double rigid_normal_impulse_ns;
    double rigid_tangential_speed_m_s;
} F90UnderfloorSample;

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
    const CSimTriRaycastSample *samples, const F90UnderfloorSample *underfloor,
    F90CoreFrameOut *out);
typedef uint32_t (*FnCoreAudioRender)(void *core, float *out_l, float *out_r, uint32_t n);
typedef bool (*FnCoreAudioTrigger)(void *core, int32_t code);
typedef void (*FnCoreAudioReadouts)(void *core, F90CoreFrameOut *out);
typedef uint32_t (*FnCoreAudioCommandsJson)(void *core, uint8_t *out, uint32_t cap);
typedef void (*FnCoreAudioCollision)(void *core, uint32_t kind, float normal_speed_m_s,
    float tangential_speed_m_s, float impulse_ns);
typedef void (*FnCoreAudioSetListenerDistance)(void *core, float distance_m);
typedef void (*FnCoreAudioSetMuteMask)(void *core, uint8_t mask);
typedef void (*FnCoreAudioSetRecordPath)(void *core, const char *path);
typedef uint32_t (*FnCoreSnapshot)(void *core, uint8_t *out, uint32_t cap, uint32_t *out_len);

#ifdef __cplusplus
}
#endif
