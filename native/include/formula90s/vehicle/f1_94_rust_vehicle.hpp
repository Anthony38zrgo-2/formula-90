#pragma once

#include <godot_cpp/classes/rigid_body3d.hpp>
#include <godot_cpp/classes/ray_cast3d.hpp>
#include <godot_cpp/classes/input.hpp>
#include <godot_cpp/classes/engine.hpp>
#include <godot_cpp/variant/packed_float64_array.hpp>
#include <godot_cpp/variant/quaternion.hpp>
#include <godot_cpp/variant/transform3d.hpp>
#include <godot_cpp/variant/vector3.hpp>

namespace godot {

struct FfiRaycastHit {
	bool is_colliding;
	double distance;
	double point_x;
	double point_y;
	double point_z;
	double normal_x;
	double normal_y;
	double normal_z;
	uint32_t surface_type; // 0=Road, 1=Curb, 2=Dirt, 3=Grass, 4=Gravel, 5=Sand, 6=Wall, 7=Metal
};

struct FfiTriRaycastSample {
	FfiRaycastHit inner;
	FfiRaycastHit center;
	FfiRaycastHit outer;
};

struct FfiVehicleInput {
	double throttle;
	double steering;
	double brake;
	double handbrake;
	double clutch;
	int32_t gear_request; // -2 = None, -1 = Reverse, 0 = Neutral, 1..6 = Forward gears
};

struct FfiTelemetryOutput {
	double sim_time;
	double speed_kmh;
	double rpm;
	int32_t gear;
	double engine_torque;
	double clutch_engagement;
	double throttle;
	double brake;
	double steer;

	// Chassis 3D Pose
	double pos_x;
	double pos_y;
	double pos_z;
	double rot_quat_x;
	double rot_quat_y;
	double rot_quat_z;
	double rot_quat_w;

	// Chassis 3D Motion
	double lin_vel_x;
	double lin_vel_y;
	double lin_vel_z;
	double ang_vel_x;
	double ang_vel_y;
	double ang_vel_z;
	double lat_g;
	double long_g;
	double vert_g;

	// Wheel Compressions (mm)
	double fl_comp_mm;
	double fr_comp_mm;
	double rl_comp_mm;
	double rr_comp_mm;

	// Wheel Spin Angular Velocities (rad/s)
	double fl_spin;
	double fr_spin;
	double rl_spin;
	double rr_spin;

	// Wheel Slip Ratios
	double fl_slip;
	double fr_slip;
	double rl_slip;
	double rr_slip;

	// Wheel Steer Angle (rad)
	double steer_angle_rad;
};

// Rust FFI Function Pointers
typedef void *(*FnPhysicsCreateWithPos)(double pos_x, double pos_y, double pos_z, double yaw_rad);
typedef void (*FnPhysicsReset)(void *sim_ptr, double pos_x, double pos_y, double pos_z, double yaw_rad);
typedef void (*FnPhysicsStep)(void *sim_ptr, const FfiVehicleInput *input_ptr, const FfiTriRaycastSample *samples_ptr, double dt, FfiTelemetryOutput *out_telem);
typedef void (*FnPhysicsGetWheelAnchorLocal)(const void *sim_ptr, uint32_t wheel_idx, double *out_x, double *out_y, double *out_z);
typedef double (*FnPhysicsGetTriRaySpan)(const void *sim_ptr, uint32_t wheel_idx);
typedef void (*FnPhysicsDestroy)(void *sim_ptr);

class F194RustVehicle : public RigidBody3D {
	GDCLASS(F194RustVehicle, RigidBody3D)

private:
	void *sim_ptr_ = nullptr;
	void *dll_handle_ = nullptr;

	// FFI function pointers
	FnPhysicsCreateWithPos fn_create_with_pos_ = nullptr;
	FnPhysicsReset fn_reset_ = nullptr;
	FnPhysicsStep fn_step_ = nullptr;
	FnPhysicsGetWheelAnchorLocal fn_get_anchor_ = nullptr;
	FnPhysicsGetTriRaySpan fn_get_tri_span_ = nullptr;
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
	RayCast3D *raycasts_[4][3]; // [wheel 0..3][inner=0, center=1, outer=2]

	// Player Inputs / Overrides
	bool enable_player_input_ = true;
	double throttle_amount_ = 0.0;
	double steering_input_ = 0.0;
	double brake_amount_ = 0.0;
	double handbrake_amount_ = 0.0;
	double clutch_amount_ = 0.0;
	int gear_request_ = -2;
	bool automatic_transmission_ = true;

	// Telemetry State
	double speed_ms_ = 0.0;
	double speed_kmh_ = 0.0;
	double motor_rpm_ = 4500.0;
	int current_gear_ = 1;
	double engine_torque_ = 0.0;
	double clutch_engagement_ = 1.0;
	double true_steering_amount_ = 0.0;

	Vector3 lin_vel_ = Vector3();
	Vector3 ang_vel_ = Vector3();
	double lat_g_ = 0.0;
	double long_g_ = 0.0;
	double vert_g_ = 0.0;

	double wheel_compressions_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double wheel_spins_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double wheel_slips_[4] = { 0.0, 0.0, 0.0, 0.0 };
	double wheel_angles_[4] = { 0.0, 0.0, 0.0, 0.0 };

	bool load_rust_dll();
	void unload_rust_dll();
	void setup_raycasts();
	void update_wheel_visuals(double delta);
	uint32_t detect_surface_type(const RayCast3D *ray);

protected:
	static void _bind_methods();

public:
	F194RustVehicle();
	~F194RustVehicle() override;

	void _ready() override;
	void _physics_process(double delta) override;
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

	void set_automatic_transmission(bool p_val) { automatic_transmission_ = p_val; }
	bool get_automatic_transmission() const { return automatic_transmission_; }

	// Telemetry Getters
	double get_speed() const { return speed_ms_; }
	double get_speed_kmh() const { return speed_kmh_; }
	double get_motor_rpm() const { return motor_rpm_; }
	int get_current_gear() const { return current_gear_; }
	double get_engine_torque() const { return engine_torque_; }
	double get_clutch_engagement() const { return clutch_engagement_; }
	double get_true_steering_amount() const { return true_steering_amount_; }

	double get_lat_g() const { return lat_g_; }
	double get_long_g() const { return long_g_; }
	double get_vert_g() const { return vert_g_; }

	Vector3 get_linear_velocity() const { return lin_vel_; }
	Vector3 get_angular_velocity() const { return ang_vel_; }

	PackedFloat64Array get_wheel_compressions() const;
	PackedFloat64Array get_wheel_spins() const;
	PackedFloat64Array get_wheel_slips() const;

	// Tuning panel properties (for handling_tuning_panel.gd compatibility)
	double motor_drag_ = 0.006;
	double max_torque_ = 340.0;
	double brake_force_multiplier_ = 1.0;
	double front_brake_bias_ = 0.58;
	double stability_yaw_strength_ = 5.25;
	bool enable_stability_ = true;
	double steering_exponent_ = 1.50;
	double steering_speed_ = 4.25;
	double countersteer_speed_ = 11.0;

	// Tuning Getters/Setters
	void set_motor_drag(double v) { motor_drag_ = v; }
	double get_motor_drag() const { return motor_drag_; }
	void set_max_torque(double v) { max_torque_ = v; }
	double get_max_torque() const { return max_torque_; }
	void set_brake_force_multiplier(double v) { brake_force_multiplier_ = v; }
	double get_brake_force_multiplier() const { return brake_force_multiplier_; }
	void set_front_brake_bias(double v) { front_brake_bias_ = v; }
	double get_front_brake_bias() const { return front_brake_bias_; }
	void set_stability_yaw_strength(double v) { stability_yaw_strength_ = v; }
	double get_stability_yaw_strength() const { return stability_yaw_strength_; }
	void set_enable_stability(bool v) { enable_stability_ = v; }
	bool get_enable_stability() const { return enable_stability_; }
	void set_steering_exponent(double v) { steering_exponent_ = v; }
	double get_steering_exponent() const { return steering_exponent_; }
	void set_steering_speed(double v) { steering_speed_ = v; }
	double get_steering_speed() const { return steering_speed_; }
	void set_countersteer_speed(double v) { countersteer_speed_ = v; }
	double get_countersteer_speed() const { return countersteer_speed_; }

	void reset_vehicle(const Vector3 &p_pos, double p_yaw_rad);
};

} // namespace godot
