#ifndef FORMULA90_PHYSICS_H
#define FORMULA90_PHYSICS_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct F90RaycastHit {
    bool is_colliding;
    double distance;
    double point_x, point_y, point_z;
    double normal_x, normal_y, normal_z;
    uint32_t surface_type;
} F90RaycastHit;

typedef struct F90TriRaycastSample {
    F90RaycastHit inner;
    F90RaycastHit center;
    F90RaycastHit outer;
} F90TriRaycastSample;

typedef struct F90VehicleInput {
    double throttle;
    double steering;
    double brake;
    double handbrake;
    double clutch;
    int32_t gear_request;
} F90VehicleInput;

typedef struct F90BodyKinematics {
    double pos_x, pos_y, pos_z;
    double rot_quat_x, rot_quat_y, rot_quat_z, rot_quat_w;
    double lin_vel_x, lin_vel_y, lin_vel_z;
    double ang_vel_x, ang_vel_y, ang_vel_z;
} F90BodyKinematics;

typedef struct F90ForceTorqueOutput {
    double force_x, force_y, force_z;
    double torque_x, torque_y, torque_z;
} F90ForceTorqueOutput;

typedef struct F90TelemetryOutput {
    double sim_time;
    double speed_kmh;
    double rpm;
    int32_t gear;
    double engine_torque;
    double clutch_engagement;
    double throttle;
    double brake;
    double steer;
    double pos_x, pos_y, pos_z;
    double rot_quat_x, rot_quat_y, rot_quat_z, rot_quat_w;
    double lin_vel_x, lin_vel_y, lin_vel_z;
    double ang_vel_x, ang_vel_y, ang_vel_z;
    double lat_g, long_g, vert_g;
    double fl_comp_mm, fr_comp_mm, rl_comp_mm, rr_comp_mm;
    double fl_spin, fr_spin, rl_spin, rr_spin;
    double fl_slip, fr_slip, rl_slip, rr_slip;
    double steer_angle_rad;
} F90TelemetryOutput;

#define F1_94_PHYSICS_ABI_VERSION 3

typedef struct F90RuntimeConfig {
    double vehicle_mass;
    double front_brake_bias;
    double max_steering_angle;
    double max_torque;
    double coefficient_of_drag;
    double frontal_area;
    double air_density;
    double steering_exponent;
    double steering_speed;
    double countersteer_speed;
    bool automatic_transmission;
} F90RuntimeConfig;

uint32_t f1_94_physics_abi_version(void);
const char *f1_94_physics_build_sha(void);

bool f1_94_physics_get_runtime_config(void *sim, F90RuntimeConfig *out_config);
bool f1_94_physics_apply_runtime_config(void *sim, const F90RuntimeConfig *config);

void *f1_94_physics_create_default(void);
void *f1_94_physics_create_with_pos(double pos_x, double pos_y, double pos_z, double yaw_rad);
void f1_94_physics_reset(void *sim, double pos_x, double pos_y, double pos_z, double yaw_rad);

/* Recommended Godot path. Gravity is not included in out_forces. */
void f1_94_physics_solve_forces(
    void *sim,
    const F90BodyKinematics *body,
    const F90VehicleInput *input,
    const F90TriRaycastSample samples[4],
    double dt,
    F90ForceTorqueOutput *out_forces,
    F90TelemetryOutput *out_telem
);

/* Legacy standalone integrator. */
void f1_94_physics_step(
    void *sim,
    const F90VehicleInput *input,
    const F90TriRaycastSample samples[4],
    double dt,
    F90TelemetryOutput *out_telem
);

void f1_94_physics_get_wheel_anchor_local(void *sim, uint32_t wheel_idx, double *x, double *y, double *z);
double f1_94_physics_get_tri_ray_span(void *sim, uint32_t wheel_idx);
double f1_94_physics_get_ray_length(void *sim, uint32_t wheel_idx);
double f1_94_physics_get_vehicle_mass(void *sim);
double f1_94_physics_get_default_spawn_height(void *sim);
void f1_94_physics_get_center_of_mass_local(void *sim, double *x, double *y, double *z);
void f1_94_physics_destroy(void *sim);

#ifdef __cplusplus
}
#endif
#endif
