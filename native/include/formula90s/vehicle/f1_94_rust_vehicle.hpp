#pragma once

#include "formula90s/vehicle/formula90_physics.h"
#include "formula90s/core/f90_core.h"
#include "formula90s/sim/f90_sim_bridge.h"

#include <godot_cpp/classes/rigid_body3d.hpp>
#include <godot_cpp/classes/physics_direct_body_state3d.hpp>
#include <godot_cpp/classes/physics_server3d.hpp>
#include <godot_cpp/classes/ray_cast3d.hpp>
#include <godot_cpp/classes/input.hpp>
#include <godot_cpp/classes/engine.hpp>
#include <godot_cpp/variant/dictionary.hpp>
#include <godot_cpp/variant/packed_float64_array.hpp>
#include <godot_cpp/variant/quaternion.hpp>
#include <godot_cpp/variant/transform3d.hpp>
#include <godot_cpp/variant/vector3.hpp>

namespace godot {

class F90Core;


// C-ABI type aliases matching formula90_physics.h
using FfiRaycastHit = F90RaycastHit;
using FfiTriRaycastSample = F90TriRaycastSample;
using FfiVehicleInput = F90VehicleInput;
using FfiBodyKinematics = F90BodyKinematics;
using FfiForceTorqueOutput = F90ForceTorqueOutput;
using FfiTelemetryOutput = F90TelemetryOutput;

// Rust FFI Function Pointers (C-ABI)
typedef void *(*FnPhysicsCreateDefault)(void);
typedef void *(*FnPhysicsCreateFromJson)(const uint8_t *json_utf8, uint32_t json_len, uint8_t *error_buffer, uint32_t error_buffer_len);
typedef void *(*FnPhysicsCreateWithPos)(double pos_x, double pos_y, double pos_z, double yaw_rad);
typedef void (*FnPhysicsReset)(void *sim_ptr, double pos_x, double pos_y, double pos_z, double yaw_rad);
typedef void (*FnPhysicsSolveForces)(
	void *sim_ptr,
	const F90BodyKinematics *body_ptr,
	const F90VehicleInput *input_ptr,
	const F90TriRaycastSample samples_ptr[4],
	double dt,
	F90ForceTorqueOutput *out_forces,
	F90TelemetryOutput *out_telem
);
typedef void (*FnPhysicsStep)(
	void *sim_ptr,
	const F90VehicleInput *input_ptr,
	const F90TriRaycastSample samples_ptr[4],
	double dt,
	F90TelemetryOutput *out_telem
);
typedef void (*FnPhysicsGetWheelAnchorLocal)(void *sim, uint32_t wheel_idx, double *x, double *y, double *z);
typedef double (*FnPhysicsGetTriRaySpan)(void *sim, uint32_t wheel_idx);
typedef double (*FnPhysicsGetRayLength)(void *sim, uint32_t wheel_idx);
typedef double (*FnPhysicsGetVehicleMass)(void *sim);
typedef double (*FnPhysicsGetDefaultSpawnHeight)(void *sim);
typedef void (*FnPhysicsGetCenterOfMassLocal)(void *sim, double *x, double *y, double *z);
typedef void (*FnPhysicsDestroy)(void *sim);
typedef uint32_t (*FnPhysicsAbiVersion)(void);
typedef const char *(*FnPhysicsBuildSha)(void);
typedef bool (*FnPhysicsGetRuntimeConfig)(void *sim, F90RuntimeConfig *out_config);
typedef bool (*FnPhysicsApplyRuntimeConfig)(void *sim, const F90RuntimeConfig *config);

class F194RustVehicle : public RigidBody3D {
	GDCLASS(F194RustVehicle, RigidBody3D)

private:
	void *sim_ptr_ = nullptr;
	void *dll_handle_ = nullptr;
	bool inertia_initialized_ = false;
	String physics_config_path_ = "res://data/vehicles/f1_2026_2008/f1_2026_2008_physics.json";

	// FFI function pointers
	FnPhysicsAbiVersion fn_abi_version_ = nullptr;
	FnPhysicsBuildSha fn_build_sha_ = nullptr;
	FnPhysicsGetRuntimeConfig fn_get_runtime_config_ = nullptr;
	FnPhysicsApplyRuntimeConfig fn_apply_runtime_config_ = nullptr;
	FnPhysicsCreateDefault fn_create_default_ = nullptr;
	FnPhysicsCreateFromJson fn_create_from_json_ = nullptr;
	FnPhysicsCreateWithPos fn_create_with_pos_ = nullptr;
	FnPhysicsReset fn_reset_ = nullptr;
	FnPhysicsSolveForces fn_solve_forces_ = nullptr;
	FnPhysicsStep fn_step_ = nullptr;
	FnPhysicsGetWheelAnchorLocal fn_get_anchor_ = nullptr;
	FnPhysicsGetTriRaySpan fn_get_tri_span_ = nullptr;
	FnPhysicsGetRayLength fn_get_ray_length_ = nullptr;
	FnPhysicsGetVehicleMass fn_get_vehicle_mass_ = nullptr;
	FnPhysicsGetDefaultSpawnHeight fn_get_default_spawn_height_ = nullptr;
	FnPhysicsGetCenterOfMassLocal fn_get_center_of_mass_local_ = nullptr;
	FnPhysicsDestroy fn_destroy_ = nullptr;

	// Visual NodePaths
	NodePath chassis_node_path_;
	NodePath front_left_wheel_node_path_;
	NodePath front_right_wheel_node_path_;
	NodePath rear_left_wheel_node_path_;
	NodePath rear_right_wheel_node_path_;

	Node3D *chassis_node_ = nullptr;
	Node3D *wheel_nodes_[4] = { nullptr, nullptr, nullptr, nullptr }; // FL, FR, RL, RR
	Vector3 wheel_base_positions_[4];

	// 12 RayCast3D children (3 per wheel: Inner, Center, Outer)
	RayCast3D *raycasts_[4][3] = {}; // [wheel 0..3][inner=0, center=1, outer=2]
	// Five synchronized underfloor probes: front-left, front-right, floor center,
	// diffuser throat and diffuser exit.
	RayCast3D *underfloor_raycasts_[5] = {};

	// Player Inputs / Overrides
	bool enable_player_input_ = true;
	double throttle_amount_ = 0.0;
	double steering_input_ = 0.0;
	double brake_amount_ = 0.0;
	double handbrake_amount_ = 0.0;
	double clutch_amount_ = 0.0;
	int gear_request_ = -2;
	bool automatic_transmission_ = false;

	// Telemetry State
	double speed_ms_ = 0.0;
	double speed_kmh_ = 0.0;
	double motor_rpm_ = 4500.0;
	int current_gear_ = 1;
	double engine_torque_ = 0.0;
	double clutch_engagement_ = 1.0;
	double clutch_torque_ = 0.0;
	double true_steering_amount_ = 0.0;
	double steer_angle_rad_ = 0.0;

	Vector3 lin_vel_ = Vector3();
	Vector3 ang_vel_ = Vector3();
	double lat_g_ = 0.0;
	double long_g_ = 0.0;
	double vert_g_ = 0.0;

	double wheel_compressions_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double wheel_spins_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double wheel_slips_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double wheel_drive_torques_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double wheel_drive_torques_pre_tc_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double tc_slip_ratio_[4] = { 0.0, 0.0, 0.0, 0.0 };
	bool tc_enabled_ = false;
	bool tc_eligible_ = false;
	bool tc_intervening_ = false;
	double tc_gear_authority_ = 0.0;
	double tc_slip_target_ = 0.0;
	double tc_raw_cut_ratio_ = 0.0;
	double tc_cut_ratio_ = 0.0;
	double pre_tc_drive_power_w_ = 0.0;
	double net_drive_power_w_ = 0.0;
	double wheel_normal_forces_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double wheel_angles_[4] = { 0.0, 0.0, 0.0, 0.0 };

	// Tire pressure + thermal telemetry (WheelIndex order FL/FR/RL/RR).
	double tire_pressure_kpa_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double tire_tread_inner_c_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double tire_tread_center_c_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double tire_tread_outer_c_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double tire_carcass_c_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double tire_gas_c_[4] = { 0.0, 0.0, 0.0, 0.0 };

	// Brake thermal + duct telemetry (WheelIndex order FL/FR/RL/RR).
	double brake_disc_c_[4] = { 0.0, 0.0, 0.0, 0.0 };
	// Lumped rotor cooling diagnostics (compact brake model).
	double brake_natural_cooling_w_k_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double brake_speed_cooling_w_k_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double brake_rim_c_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double brake_efficiency_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double duct_mass_flow_kg_s_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double duct_drag_n_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double brake_optimal_min_c_ = 400.0;
	double brake_optimal_max_c_ = 800.0;
	double brake_fade_start_c_ = 900.0;
	double brake_critical_c_ = 1100.0;
	double brake_torque_nm_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double brake_spin_pre_rad_s_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double brake_spin_post_rad_s_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double brake_power_w_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double brake_energy_j_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double underfloor_clearance_m_[5] = { 0.35, 0.35, 0.35, 0.35, 0.35 };
	uint32_t underfloor_valid_mask_ = 0;
	int underfloor_scrape_phase_ = 0;
	double underfloor_min_clearance_m_ = 0.35;
	double underfloor_rake_rad_ = 0.0;
	double underfloor_roll_rad_ = 0.0;
	double underfloor_contact_confidence_ = 0.0;
	double underfloor_scrape_intensity_ = 0.0;
	double audio_scrape_gain_ = 0.0;
	double audio_scrape_pitch_ = 1.0;
	double audio_scrape_cursor_ = 0.0;
	double underfloor_compression_m_[5] = {};
	double underfloor_closing_speed_m_s_[5] = {};
	double underfloor_normal_force_n_[5] = {};
	int32_t underfloor_bottoming_phase_[5] = {};
	uint32_t underfloor_active_probe_mask_ = 0;
	double underfloor_total_normal_force_n_ = 0.0;
	double underfloor_max_probe_force_n_ = 0.0;
	double underfloor_force_center_local_[3] = {};
	double underfloor_bottoming_torque_[3] = {};
	double underfloor_dissipated_energy_j_ = 0.0;
	double underfloor_rigid_contact_blend_ = 0.0;
	double aero_telemetry_[17] = {};

	bool load_rust_dll();
	void unload_rust_dll();
	void setup_raycasts();
	void update_wheel_visuals(double delta);
	uint32_t detect_surface_type(const RayCast3D *ray) const;

protected:
	static void _bind_methods();
	void _notification(int p_what);

public:
	F194RustVehicle();
	~F194RustVehicle() override;

	void set_physics_config_path(const String &p_path) { physics_config_path_ = p_path; }
	String get_physics_config_path() const { return physics_config_path_; }

	void _ready() override;
	void _integrate_forces(PhysicsDirectBodyState3D *p_state) override;
	void solve_forces_for_state(PhysicsDirectBodyState3D *p_state);
	void _exit_tree() override;

	// NodePath Getters/Setters
	void set_chassis_node(const NodePath &p_path) { chassis_node_path_ = p_path; }
	NodePath get_chassis_node() const { return chassis_node_path_; }

	void set_front_left_wheel_node(const NodePath &p_path) { front_left_wheel_node_path_ = p_path; }
	NodePath get_front_left_wheel_node() const { return front_left_wheel_node_path_; }

	void set_front_right_wheel_node(const NodePath &p_path) { front_right_wheel_node_path_ = p_path; }
	NodePath get_front_right_wheel_node() const { return front_right_wheel_node_path_; }

	void set_rear_left_wheel_node(const NodePath &p_path) { rear_left_wheel_node_path_ = p_path; }
	NodePath get_rear_left_wheel_node() const { return rear_left_wheel_node_path_; }

	void set_rear_right_wheel_node(const NodePath &p_path) { rear_right_wheel_node_path_ = p_path; }
	NodePath get_rear_right_wheel_node() const { return rear_right_wheel_node_path_; }

	// Input / Control Getters/Setters
	void set_enable_player_input(bool p_enable) { enable_player_input_ = p_enable; }
	bool get_enable_player_input() const { return enable_player_input_; }

	void set_throttle_amount(double p_val) { throttle_amount_ = p_val; }
	double get_throttle_amount() const { return throttle_amount_; }

	void set_steering_input(double p_val) { steering_input_ = p_val; }
	double get_steering_input() const { return steering_input_; }

	void set_brake_amount(double p_val) { brake_amount_ = p_val; }
	double get_brake_amount() const { return brake_amount_; }

	void set_handbrake_amount(double p_val) { handbrake_amount_ = p_val; }
	double get_handbrake_amount() const { return handbrake_amount_; }

	void set_clutch_amount(double p_val) { clutch_amount_ = p_val; }
	double get_clutch_amount() const { return clutch_amount_; }

	void set_gear_request(int p_val) { gear_request_ = p_val; }
	int get_gear_request() const { return gear_request_; }

	void set_automatic_transmission(bool p_val);
	bool get_automatic_transmission() const;

	// Telemetry Getters
	double get_speed() const { return speed_ms_; }
	double get_speed_kmh() const { return speed_kmh_; }
	double get_motor_rpm() const { return motor_rpm_; }
	int get_current_gear() const { return current_gear_; }
	double get_engine_torque() const { return engine_torque_; }
	double get_clutch_engagement() const { return clutch_engagement_; }
	double get_clutch_torque() const { return clutch_torque_; }
	double get_true_steering_amount() const { return true_steering_amount_; }
	double get_steer_angle_rad() const { return steer_angle_rad_; }

	double get_lat_g() const { return lat_g_; }
	double get_long_g() const { return long_g_; }
	double get_vert_g() const { return vert_g_; }

	Vector3 get_linear_velocity() const { return lin_vel_; }
	Vector3 get_angular_velocity() const { return ang_vel_; }

	PackedFloat64Array get_wheel_compressions() const;
	PackedFloat64Array get_wheel_spins() const;
	PackedFloat64Array get_wheel_slips() const;
	PackedInt64Array get_wheel_surface_types() const;
	PackedFloat64Array get_drive_torques() const;
	Dictionary get_powertrain_state_snapshot() const;
	PackedFloat64Array get_normal_forces() const;

	// Dimension & Anchor Queries
	double get_vehicle_mass_value() const;
	Vector3 get_center_of_mass_local_value() const;
	Vector3 get_wheel_anchor_local_value(int p_wheel_idx) const;
	double get_tri_ray_span_value(int p_wheel_idx) const;
	double get_ray_length_value(int p_wheel_idx) const;
	double get_default_spawn_height_value() const;

	// Tuning panel properties (for handling_tuning_panel.gd compatibility)
	double motor_drag_ = 0.006;
	double max_torque_ = 455.0;
	double brake_force_multiplier_ = 1.0;
	double front_brake_bias_ = 0.58;
	double stability_yaw_strength_ = 5.25;
	bool enable_stability_ = true;
	double steering_exponent_ = 1.50;
	double steering_speed_ = 4.25;
	double countersteer_speed_ = 11.0;
	double max_steering_angle_ = 0.436332;
	double coefficient_of_drag_ = 0.78;
	double frontal_area_ = 1.25;
	double air_density_ = 1.225;
	double idle_rpm_ = 4500.0;
	double max_rpm_ = 15000.0;
	double vehicle_mass_ = 575.0;

	// Differential (Salisbury Clutch-Pack LSD) — runtime tunable.
	// Defaults mirror VehicleConfig::f1_94_canonical() (Fase 2 B calibration).
	double diff_preload_ = 120.0;
	double diff_power_ramp_angle_deg_ = 65.0;
	double diff_coast_ramp_angle_deg_ = 75.0;
	double diff_clutches_ = 4.0;
	double diff_clutch_friction_coeff_ = 0.15;
	// GEVP-facing alias: rear_locking_differential_engage_torque (gearbox_spec.gd).
	// -1.0 = unset (use Salisbury params above). >=0 maps to preload=value, mu=0.
	double rear_locking_differential_engage_torque_ = -1.0;

	// Runtime aids enable mask (see formula90_physics.h F90RuntimeConfig.aids_enabled_mask).
	uint32_t aids_enabled_mask_ = 0;

	// Inertia & Suspension parameters from authoritative JSON / Rust
	double inertia_multiplier_x_ = 1.05;
	double inertia_multiplier_y_ = 1.15;
	double inertia_multiplier_z_ = 1.05;

	double suspension_front_spring_length_ = 0.250;
	double suspension_rear_spring_length_ = 0.180;
	double suspension_front_resting_ratio_ = 0.280;
	double suspension_rear_resting_ratio_ = 0.350;

	void apply_runtime_config();
	void sync_runtime_config_from_rust();

	// Tuning Getters/Setters
	void set_motor_drag(double v) { motor_drag_ = v; }
	double get_motor_drag() const { return motor_drag_; }
	void set_max_torque(double v);
	double get_max_torque() const;
	void set_brake_force_multiplier(double v) { brake_force_multiplier_ = v; }
	double get_brake_force_multiplier() const { return brake_force_multiplier_; }
	void set_front_brake_bias(double v);
	double get_front_brake_bias() const;
	void set_stability_yaw_strength(double v) { stability_yaw_strength_ = v; }
	double get_stability_yaw_strength() const { return stability_yaw_strength_; }
	void set_enable_stability(bool v) { enable_stability_ = v; }
	bool get_enable_stability() const { return enable_stability_; }
	void set_steering_exponent(double v);
	double get_steering_exponent() const;
	void set_steering_speed(double v);
	double get_steering_speed() const;
	void set_countersteer_speed(double v);
	double get_countersteer_speed() const;
	void set_max_steering_angle(double v);
	double get_max_steering_angle() const;
	void set_coefficient_of_drag(double v);
	double get_coefficient_of_drag() const;
	void set_frontal_area(double v);
	double get_frontal_area() const;
	void set_air_density(double v);
	double get_air_density() const;
	void set_idle_rpm(double v) { idle_rpm_ = v; }
	double get_idle_rpm() const { return idle_rpm_; }
	void set_max_rpm(double v) { max_rpm_ = v; }
	double get_max_rpm() const { return max_rpm_; }
	void set_vehicle_mass(double v);
	double get_vehicle_mass() const;

	// Differential tuning accessors
	void set_diff_preload(double v);
	double get_diff_preload() const;
	void set_diff_power_ramp_angle_deg(double v);
	double get_diff_power_ramp_angle_deg() const;
	void set_diff_coast_ramp_angle_deg(double v);
	double get_diff_coast_ramp_angle_deg() const;
	void set_diff_clutches(double v);
	double get_diff_clutches() const;
	void set_diff_clutch_friction_coeff(double v);
	double get_diff_clutch_friction_coeff() const;

	// GEVP-facing alias: rear_locking_differential_engage_torque (gearbox_spec.gd)
	void set_rear_locking_differential_engage_torque(double v);
	double get_rear_locking_differential_engage_torque() const;

	// Driving aids runtime mask (bit0=ABS, bit1=TC, bit2=stability, bit3=steering slip,
	// bit4=countersteer, bit5=auto-clutch, bit6=launch, bit7=brake-assist).
	void set_aids_enabled_mask(uint32_t v);
	uint32_t get_aids_enabled_mask() const;

	void reset_vehicle(const Vector3 &p_pos, double p_yaw_rad);

	// --- Snapshot-server bridge control: F90SimBridge drives this vehicle via the
	// authoritative Rust core. When bridge_controlled_, _integrate_forces is skipped
	// (the bridge sets body velocities from the core's predicted pose). ---
	bool bridge_controlled_ = false;

public:
	void set_bridge_controlled(bool p_v);
	bool is_bridge_controlled() const { return bridge_controlled_; }
	// Orchestrator facade (F90Core): drives physics and audio with a single handshake.
	F90Core *core_driver_ = nullptr;
	void set_core_driver(F90Core *p) { core_driver_ = p; }


	// Sample this vehicle's 12 RayCast3D children into the core's sample layout.
	void collect_core_samples(CSimTriRaycastSample p_samples[4]);
	void collect_underfloor_sample(F90UnderfloorSample *p_sample, PhysicsDirectBodyState3D *p_state);
	// Drive the body by velocity (Godot integrates + collides); core owns dynamics.
	void apply_core_motion(const Vector3 &p_lin_vel, const Vector3 &p_ang_vel);
	// Forward core telemetry into the vehicle's mirrors + wheel visuals.
	void apply_core_telemetry(const CSimTelemetry &p_telemetry, double p_dt);
	void set_core_powertrain_telemetry(const F90CoreFrameOut &p_frame);
	// Copy the six per-wheel tire arrays from the facade frame or the legacy
	// physics telemetry block (WheelIndex order FL/FR/RL/RR).
	void set_core_tire_telemetry(
		const double pressure_kpa[4],
		const double tread_inner_c[4],
		const double tread_center_c[4],
		const double tread_outer_c[4],
		const double carcass_c[4],
		const double gas_c[4]);
	void set_core_brake_telemetry(
		const double disc_c[4],
		const double rim_c[4],
		const double efficiency[4],
		const double duct_mass_flow_kg_s[4],
		const double duct_drag_n[4],
		double optimal_min_c,
		double optimal_max_c,
		double fade_start_c,
		double critical_c);
	void set_core_brake_energy_telemetry(
		const double torque_nm[4],
		const double spin_pre_rad_s[4],
		const double spin_post_rad_s[4],
		const double power_w[4],
		const double energy_j[4]);
	void set_core_brake_cooling_telemetry(
		const double natural_cooling_w_k[4],
		const double speed_cooling_w_k[4]);
	// Tire pressure + thermal snapshot for the HUD (no raw C structs leak out).
	godot::Dictionary get_tire_state_snapshot() const;
	godot::Dictionary get_brake_state_snapshot() const;
	godot::Dictionary get_underfloor_state_snapshot() const;
	void set_core_underfloor_telemetry(const F90CoreFrameOut &p_frame);
};

} // namespace godot
