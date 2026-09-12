#include "formula90s/vehicle/f1_94_rust_vehicle.hpp"
#include "formula90s/core/f90_core.h"
#include "formula90s/core/f90_core.hpp"


#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/classes/file_access.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/variant/utility_functions.hpp>

#include <cmath>
#include <utility>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

namespace godot {

void F194RustVehicle::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_physics_config_path", "path"), &F194RustVehicle::set_physics_config_path);
	ClassDB::bind_method(D_METHOD("get_physics_config_path"), &F194RustVehicle::get_physics_config_path);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "physics_config_path", PROPERTY_HINT_FILE, "*.json"), "set_physics_config_path", "get_physics_config_path");

	// NodePaths
	ClassDB::bind_method(D_METHOD("set_chassis_node", "path"), &F194RustVehicle::set_chassis_node);
	ClassDB::bind_method(D_METHOD("get_chassis_node"), &F194RustVehicle::get_chassis_node);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "chassis_node"), "set_chassis_node", "get_chassis_node");

	ClassDB::bind_method(D_METHOD("set_front_left_wheel_node", "path"), &F194RustVehicle::set_front_left_wheel_node);
	ClassDB::bind_method(D_METHOD("get_front_left_wheel_node"), &F194RustVehicle::get_front_left_wheel_node);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "front_left_wheel_node"), "set_front_left_wheel_node", "get_front_left_wheel_node");

	ClassDB::bind_method(D_METHOD("set_front_right_wheel_node", "path"), &F194RustVehicle::set_front_right_wheel_node);
	ClassDB::bind_method(D_METHOD("get_front_right_wheel_node"), &F194RustVehicle::get_front_right_wheel_node);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "front_right_wheel_node"), "set_front_right_wheel_node", "get_front_right_wheel_node");

	ClassDB::bind_method(D_METHOD("set_rear_left_wheel_node", "path"), &F194RustVehicle::set_rear_left_wheel_node);
	ClassDB::bind_method(D_METHOD("get_rear_left_wheel_node"), &F194RustVehicle::get_rear_left_wheel_node);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "rear_left_wheel_node"), "set_rear_left_wheel_node", "get_rear_left_wheel_node");

	ClassDB::bind_method(D_METHOD("set_rear_right_wheel_node", "path"), &F194RustVehicle::set_rear_right_wheel_node);
	ClassDB::bind_method(D_METHOD("get_rear_right_wheel_node"), &F194RustVehicle::get_rear_right_wheel_node);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "rear_right_wheel_node"), "set_rear_right_wheel_node", "get_rear_right_wheel_node");

	// Input Controls
	ClassDB::bind_method(D_METHOD("set_enable_player_input", "enable"), &F194RustVehicle::set_enable_player_input);
	ClassDB::bind_method(D_METHOD("get_enable_player_input"), &F194RustVehicle::get_enable_player_input);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "enable_player_input"), "set_enable_player_input", "get_enable_player_input");

	ClassDB::bind_method(D_METHOD("set_throttle_amount", "amount"), &F194RustVehicle::set_throttle_amount);
	ClassDB::bind_method(D_METHOD("get_throttle_amount"), &F194RustVehicle::get_throttle_amount);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "throttle_amount"), "set_throttle_amount", "get_throttle_amount");

	ClassDB::bind_method(D_METHOD("set_steering_input", "amount"), &F194RustVehicle::set_steering_input);
	ClassDB::bind_method(D_METHOD("get_steering_input"), &F194RustVehicle::get_steering_input);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "steering_input"), "set_steering_input", "get_steering_input");

	ClassDB::bind_method(D_METHOD("set_brake_amount", "amount"), &F194RustVehicle::set_brake_amount);
	ClassDB::bind_method(D_METHOD("get_brake_amount"), &F194RustVehicle::get_brake_amount);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "brake_amount"), "set_brake_amount", "get_brake_amount");

	ClassDB::bind_method(D_METHOD("set_handbrake_amount", "amount"), &F194RustVehicle::set_handbrake_amount);
	ClassDB::bind_method(D_METHOD("get_handbrake_amount"), &F194RustVehicle::get_handbrake_amount);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "handbrake_amount"), "set_handbrake_amount", "get_handbrake_amount");

	ClassDB::bind_method(D_METHOD("set_clutch_amount", "amount"), &F194RustVehicle::set_clutch_amount);
	ClassDB::bind_method(D_METHOD("get_clutch_amount"), &F194RustVehicle::get_clutch_amount);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "clutch_amount"), "set_clutch_amount", "get_clutch_amount");

	ClassDB::bind_method(D_METHOD("set_gear_request", "gear"), &F194RustVehicle::set_gear_request);
	ClassDB::bind_method(D_METHOD("get_gear_request"), &F194RustVehicle::get_gear_request);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "gear_request"), "set_gear_request", "get_gear_request");

	ClassDB::bind_method(D_METHOD("set_automatic_transmission", "enable"), &F194RustVehicle::set_automatic_transmission);
	ClassDB::bind_method(D_METHOD("get_automatic_transmission"), &F194RustVehicle::get_automatic_transmission);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "automatic_transmission"), "set_automatic_transmission", "get_automatic_transmission");

	// Telemetry & State
	ClassDB::bind_method(D_METHOD("get_speed"), &F194RustVehicle::get_speed);
	ClassDB::bind_method(D_METHOD("get_speed_kmh"), &F194RustVehicle::get_speed_kmh);
	ClassDB::bind_method(D_METHOD("get_motor_rpm"), &F194RustVehicle::get_motor_rpm);
	ClassDB::bind_method(D_METHOD("get_current_gear"), &F194RustVehicle::get_current_gear);
	ClassDB::bind_method(D_METHOD("get_engine_torque"), &F194RustVehicle::get_engine_torque);
	ClassDB::bind_method(D_METHOD("get_clutch_engagement"), &F194RustVehicle::get_clutch_engagement);
	ClassDB::bind_method(D_METHOD("get_clutch_torque"), &F194RustVehicle::get_clutch_torque);
	ClassDB::bind_method(D_METHOD("get_true_steering_amount"), &F194RustVehicle::get_true_steering_amount);
	ClassDB::bind_method(D_METHOD("get_steer_angle_rad"), &F194RustVehicle::get_steer_angle_rad);

	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "speed"), "", "get_speed");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "speed_kmh"), "", "get_speed_kmh");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "motor_rpm"), "", "get_motor_rpm");
	ADD_PROPERTY(PropertyInfo(Variant::INT, "current_gear"), "", "get_current_gear");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "engine_torque"), "", "get_engine_torque");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "clutch_engagement"), "", "get_clutch_engagement");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "clutch_torque"), "", "get_clutch_torque");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "true_steering_amount"), "", "get_true_steering_amount");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "steer_angle_rad"), "", "get_steer_angle_rad");

	ClassDB::bind_method(D_METHOD("get_lat_g"), &F194RustVehicle::get_lat_g);
	ClassDB::bind_method(D_METHOD("get_long_g"), &F194RustVehicle::get_long_g);
	ClassDB::bind_method(D_METHOD("get_vert_g"), &F194RustVehicle::get_vert_g);

	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "lat_g"), "", "get_lat_g");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "long_g"), "", "get_long_g");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "vert_g"), "", "get_vert_g");

	ClassDB::bind_method(D_METHOD("get_linear_velocity"), &F194RustVehicle::get_linear_velocity);
	ClassDB::bind_method(D_METHOD("get_angular_velocity"), &F194RustVehicle::get_angular_velocity);

	ClassDB::bind_method(D_METHOD("get_wheel_compressions"), &F194RustVehicle::get_wheel_compressions);
	ClassDB::bind_method(D_METHOD("get_wheel_spins"), &F194RustVehicle::get_wheel_spins);
	ClassDB::bind_method(D_METHOD("get_wheel_slips"), &F194RustVehicle::get_wheel_slips);
	ClassDB::bind_method(D_METHOD("get_wheel_surface_types"), &F194RustVehicle::get_wheel_surface_types);
	ClassDB::bind_method(D_METHOD("get_drive_torques"), &F194RustVehicle::get_drive_torques);
	ClassDB::bind_method(D_METHOD("get_powertrain_state_snapshot"), &F194RustVehicle::get_powertrain_state_snapshot);
	ClassDB::bind_method(D_METHOD("get_normal_forces"), &F194RustVehicle::get_normal_forces);

	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "wheel_compressions"), "", "get_wheel_compressions");
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "wheel_spins"), "", "get_wheel_spins");
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "wheel_slips"), "", "get_wheel_slips");
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_INT64_ARRAY, "wheel_surface_types"), "", "get_wheel_surface_types");
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "drive_torques"), "", "get_drive_torques");
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "normal_forces"), "", "get_normal_forces");

	// Rust Dimension and Anchor Queries
	ClassDB::bind_method(D_METHOD("get_vehicle_mass"), &F194RustVehicle::get_vehicle_mass_value);
	ClassDB::bind_method(D_METHOD("get_center_of_mass_local"), &F194RustVehicle::get_center_of_mass_local_value);
	ClassDB::bind_method(D_METHOD("get_wheel_anchor_local", "wheel_idx"), &F194RustVehicle::get_wheel_anchor_local_value);
	ClassDB::bind_method(D_METHOD("get_tri_ray_span", "wheel_idx"), &F194RustVehicle::get_tri_ray_span_value);
	ClassDB::bind_method(D_METHOD("get_ray_length", "wheel_idx"), &F194RustVehicle::get_ray_length_value);
	ClassDB::bind_method(D_METHOD("get_default_spawn_height"), &F194RustVehicle::get_default_spawn_height_value);

	// Tuning Parameters
	ClassDB::bind_method(D_METHOD("set_motor_drag", "drag"), &F194RustVehicle::set_motor_drag);
	ClassDB::bind_method(D_METHOD("get_motor_drag"), &F194RustVehicle::get_motor_drag);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "motor_drag"), "set_motor_drag", "get_motor_drag");

	ClassDB::bind_method(D_METHOD("set_max_torque", "torque"), &F194RustVehicle::set_max_torque);
	ClassDB::bind_method(D_METHOD("get_max_torque"), &F194RustVehicle::get_max_torque);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "max_torque"), "set_max_torque", "get_max_torque");

	ClassDB::bind_method(D_METHOD("set_brake_force_multiplier", "mult"), &F194RustVehicle::set_brake_force_multiplier);
	ClassDB::bind_method(D_METHOD("get_brake_force_multiplier"), &F194RustVehicle::get_brake_force_multiplier);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "brake_force_multiplier"), "set_brake_force_multiplier", "get_brake_force_multiplier");

	ClassDB::bind_method(D_METHOD("set_front_brake_bias", "bias"), &F194RustVehicle::set_front_brake_bias);
	ClassDB::bind_method(D_METHOD("get_front_brake_bias"), &F194RustVehicle::get_front_brake_bias);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "front_brake_bias"), "set_front_brake_bias", "get_front_brake_bias");

	ClassDB::bind_method(D_METHOD("set_stability_yaw_strength", "strength"), &F194RustVehicle::set_stability_yaw_strength);
	ClassDB::bind_method(D_METHOD("get_stability_yaw_strength"), &F194RustVehicle::get_stability_yaw_strength);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "stability_yaw_strength"), "set_stability_yaw_strength", "get_stability_yaw_strength");

	ClassDB::bind_method(D_METHOD("set_enable_stability", "enable"), &F194RustVehicle::set_enable_stability);
	ClassDB::bind_method(D_METHOD("get_enable_stability"), &F194RustVehicle::get_enable_stability);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "enable_stability"), "set_enable_stability", "get_enable_stability");

	ClassDB::bind_method(D_METHOD("set_steering_exponent", "exp"), &F194RustVehicle::set_steering_exponent);
	ClassDB::bind_method(D_METHOD("get_steering_exponent"), &F194RustVehicle::get_steering_exponent);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "steering_exponent"), "set_steering_exponent", "get_steering_exponent");

	ClassDB::bind_method(D_METHOD("set_steering_speed", "speed"), &F194RustVehicle::set_steering_speed);
	ClassDB::bind_method(D_METHOD("get_steering_speed"), &F194RustVehicle::get_steering_speed);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "steering_speed"), "set_steering_speed", "get_steering_speed");

	ClassDB::bind_method(D_METHOD("set_countersteer_speed", "speed"), &F194RustVehicle::set_countersteer_speed);
	ClassDB::bind_method(D_METHOD("get_countersteer_speed"), &F194RustVehicle::get_countersteer_speed);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "countersteer_speed"), "set_countersteer_speed", "get_countersteer_speed");

	// Differential (Salisbury LSD) tuning
	ClassDB::bind_method(D_METHOD("set_diff_preload", "preload"), &F194RustVehicle::set_diff_preload);
	ClassDB::bind_method(D_METHOD("get_diff_preload"), &F194RustVehicle::get_diff_preload);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "diff_preload"), "set_diff_preload", "get_diff_preload");

	ClassDB::bind_method(D_METHOD("set_diff_power_ramp_angle_deg", "deg"), &F194RustVehicle::set_diff_power_ramp_angle_deg);
	ClassDB::bind_method(D_METHOD("get_diff_power_ramp_angle_deg"), &F194RustVehicle::get_diff_power_ramp_angle_deg);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "diff_power_ramp_angle_deg"), "set_diff_power_ramp_angle_deg", "get_diff_power_ramp_angle_deg");

	ClassDB::bind_method(D_METHOD("set_diff_coast_ramp_angle_deg", "deg"), &F194RustVehicle::set_diff_coast_ramp_angle_deg);
	ClassDB::bind_method(D_METHOD("get_diff_coast_ramp_angle_deg"), &F194RustVehicle::get_diff_coast_ramp_angle_deg);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "diff_coast_ramp_angle_deg"), "set_diff_coast_ramp_angle_deg", "get_diff_coast_ramp_angle_deg");

	ClassDB::bind_method(D_METHOD("set_diff_clutches", "clutches"), &F194RustVehicle::set_diff_clutches);
	ClassDB::bind_method(D_METHOD("get_diff_clutches"), &F194RustVehicle::get_diff_clutches);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "diff_clutches"), "set_diff_clutches", "get_diff_clutches");

	ClassDB::bind_method(D_METHOD("set_diff_clutch_friction_coeff", "mu"), &F194RustVehicle::set_diff_clutch_friction_coeff);
	ClassDB::bind_method(D_METHOD("get_diff_clutch_friction_coeff"), &F194RustVehicle::get_diff_clutch_friction_coeff);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "diff_clutch_friction_coeff"), "set_diff_clutch_friction_coeff", "get_diff_clutch_friction_coeff");

	// GEVP-facing alias for rear_locking_differential_engage_torque (gearbox_spec.gd)
	ClassDB::bind_method(D_METHOD("set_rear_locking_differential_engage_torque", "torque"), &F194RustVehicle::set_rear_locking_differential_engage_torque);
	ClassDB::bind_method(D_METHOD("get_rear_locking_differential_engage_torque"), &F194RustVehicle::get_rear_locking_differential_engage_torque);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "rear_locking_differential_engage_torque"), "set_rear_locking_differential_engage_torque", "get_rear_locking_differential_engage_torque");

	ClassDB::bind_method(D_METHOD("set_aids_enabled_mask", "mask"), &F194RustVehicle::set_aids_enabled_mask);
	ClassDB::bind_method(D_METHOD("get_aids_enabled_mask"), &F194RustVehicle::get_aids_enabled_mask);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "aids_enabled_mask"), "set_aids_enabled_mask", "get_aids_enabled_mask");

	ClassDB::bind_method(D_METHOD("reset_vehicle", "pos", "yaw_rad"), &F194RustVehicle::reset_vehicle);
	ClassDB::bind_method(D_METHOD("solve_forces_for_state", "state"), &F194RustVehicle::solve_forces_for_state);
	ClassDB::bind_method(D_METHOD("get_tire_state_snapshot"), &F194RustVehicle::get_tire_state_snapshot);
	ClassDB::bind_method(D_METHOD("get_brake_state_snapshot"), &F194RustVehicle::get_brake_state_snapshot);
	ClassDB::bind_method(D_METHOD("get_underfloor_state_snapshot"), &F194RustVehicle::get_underfloor_state_snapshot);
}

F194RustVehicle::F194RustVehicle() {
	set_gravity_scale(1.0);
	set_mass(505.0);
	set_freeze_enabled(false);
	set_collision_layer(2);
	set_collision_mask(1);

	for (int w = 0; w < 4; ++w) {
		for (int r = 0; r < 3; ++r) {
			raycasts_[w][r] = nullptr;
		}
	}
}

F194RustVehicle::~F194RustVehicle() {
	unload_rust_dll();
}

bool F194RustVehicle::load_rust_dll() {
	if (dll_handle_ != nullptr) {
		return true;
	}

#ifdef _WIN32
	// List of candidate paths for the compiled Rust DLL
	Array candidate_paths;
	candidate_paths.append("res://addons/formula90s/bin/vehicle_physics_engine.windows.template_release.x86_64.dll");
	candidate_paths.append("res://addons/formula90s/bin/vehicle_physics_engine.windows.template_debug.x86_64.dll");
	candidate_paths.append("res://addons/formula90s/bin/vehicle_physics_engine.dll");
	candidate_paths.append("game/addons/formula90s/bin/vehicle_physics_engine.windows.template_release.x86_64.dll");
	candidate_paths.append("game/addons/formula90s/bin/vehicle_physics_engine.windows.template_debug.x86_64.dll");
	candidate_paths.append("game/addons/formula90s/bin/vehicle_physics_engine.dll");
	candidate_paths.append("vehicle_physics_engine.windows.template_release.x86_64.dll");
	candidate_paths.append("vehicle_physics_engine.windows.template_debug.x86_64.dll");
	candidate_paths.append("vehicle_physics_engine.dll");

	HMODULE hDll = nullptr;
	ProjectSettings *ps = ProjectSettings::get_singleton();
	String loaded_path = "";

	for (int i = 0; i < candidate_paths.size(); ++i) {
		String p = candidate_paths[i];
		String global_p = ps ? ps->globalize_path(p) : p;
		hDll = LoadLibraryW((LPCWSTR)global_p.utf16().get_data());
		if (hDll) {
			loaded_path = std::move(global_p);
			break;
		}
	}

	if (!hDll) {
		UtilityFunctions::printerr("[F194RustVehicle] Failed to load vehicle_physics_engine.dll from all candidates!");
		return false;
	}

	dll_handle_ = reinterpret_cast<void *>(hDll);

	fn_abi_version_ = reinterpret_cast<FnPhysicsAbiVersion>(GetProcAddress(hDll, "f1_94_physics_abi_version"));
	fn_build_sha_ = reinterpret_cast<FnPhysicsBuildSha>(GetProcAddress(hDll, "f1_94_physics_build_sha"));
	fn_get_runtime_config_ = reinterpret_cast<FnPhysicsGetRuntimeConfig>(GetProcAddress(hDll, "f1_94_physics_get_runtime_config"));
	fn_apply_runtime_config_ = reinterpret_cast<FnPhysicsApplyRuntimeConfig>(GetProcAddress(hDll, "f1_94_physics_apply_runtime_config"));
	fn_create_default_ = reinterpret_cast<FnPhysicsCreateDefault>(GetProcAddress(hDll, "f1_94_physics_create_default"));
	fn_create_from_json_ = reinterpret_cast<FnPhysicsCreateFromJson>(GetProcAddress(hDll, "f1_94_physics_create_from_json"));
	fn_create_with_pos_ = reinterpret_cast<FnPhysicsCreateWithPos>(GetProcAddress(hDll, "f1_94_physics_create_with_pos"));
	fn_reset_ = reinterpret_cast<FnPhysicsReset>(GetProcAddress(hDll, "f1_94_physics_reset"));
	fn_solve_forces_ = reinterpret_cast<FnPhysicsSolveForces>(GetProcAddress(hDll, "f1_94_physics_solve_forces"));
	fn_step_ = reinterpret_cast<FnPhysicsStep>(GetProcAddress(hDll, "f1_94_physics_step"));
	fn_get_anchor_ = reinterpret_cast<FnPhysicsGetWheelAnchorLocal>(GetProcAddress(hDll, "f1_94_physics_get_wheel_anchor_local"));
	fn_get_tri_span_ = reinterpret_cast<FnPhysicsGetTriRaySpan>(GetProcAddress(hDll, "f1_94_physics_get_tri_ray_span"));
	fn_get_ray_length_ = reinterpret_cast<FnPhysicsGetRayLength>(GetProcAddress(hDll, "f1_94_physics_get_ray_length"));
	fn_get_vehicle_mass_ = reinterpret_cast<FnPhysicsGetVehicleMass>(GetProcAddress(hDll, "f1_94_physics_get_vehicle_mass"));
	fn_get_default_spawn_height_ = reinterpret_cast<FnPhysicsGetDefaultSpawnHeight>(GetProcAddress(hDll, "f1_94_physics_get_default_spawn_height"));
	fn_get_center_of_mass_local_ = reinterpret_cast<FnPhysicsGetCenterOfMassLocal>(GetProcAddress(hDll, "f1_94_physics_get_center_of_mass_local"));
	fn_destroy_ = reinterpret_cast<FnPhysicsDestroy>(GetProcAddress(hDll, "f1_94_physics_destroy"));

	uint32_t abi_ver = fn_abi_version_ ? fn_abi_version_() : 0;
	const char *build_sha = fn_build_sha_ ? fn_build_sha_() : "unknown";

	UtilityFunctions::print(String("[F194Physics]\nDLL=") + loaded_path + "\nABI=" + String::num_int64(abi_ver) + "\nBUILD=" + String(build_sha));

	if (abi_ver != F1_94_PHYSICS_ABI_VERSION) {
		UtilityFunctions::printerr(String("[F194Physics] FATAL: ABI mismatch! Expected ") + String::num_int64(F1_94_PHYSICS_ABI_VERSION) + " but loaded DLL has " + String::num_int64(abi_ver));
		unload_rust_dll();
		return false;
	}

	if (!fn_solve_forces_ || !fn_destroy_) {
		UtilityFunctions::printerr("[F194RustVehicle] Missing required exported symbols in vehicle_physics_engine.dll!");
		unload_rust_dll();
		return false;
	}

	return true;
#else
	return false;
#endif
}

void F194RustVehicle::unload_rust_dll() {
	if (sim_ptr_ && fn_destroy_) {
		fn_destroy_(sim_ptr_);
		sim_ptr_ = nullptr;
	}
#ifdef _WIN32
	if (dll_handle_) {
		FreeLibrary((HMODULE)dll_handle_);
		dll_handle_ = nullptr;
	}
#endif
}

void F194RustVehicle::setup_raycasts() {
	const char *w_names[4] = { "FL", "FR", "RL", "RR" };
	const char *r_names[3] = { "In", "Mid", "Out" };

	for (int w = 0; w < 4; ++w) {
		double ax = 0.0, ay = 0.0, az = 0.0;
		if (fn_get_anchor_ && sim_ptr_) {
			fn_get_anchor_(sim_ptr_, (uint32_t)w, &ax, &ay, &az);
		} else {
			const Vector3 default_anchors[4] = {
				Vector3(-0.79625, 0.143973, -1.460326),
				Vector3(0.79625, 0.143973, -1.460326),
				Vector3(-0.762307, 0.123027, 1.460326),
				Vector3(0.762307, 0.123027, 1.460326)
			};
			ax = default_anchors[w].x;
			ay = default_anchors[w].y;
			az = default_anchors[w].z;
		}
		wheel_base_positions_[w] = Vector3(ax, ay, az);

		double span = (fn_get_tri_span_ && sim_ptr_) ? fn_get_tri_span_(sim_ptr_, (uint32_t)w) : ((w < 2) ? (0.305 * 0.40) : (0.380 * 0.40));
		double ray_length = (fn_get_ray_length_ && sim_ptr_) ? fn_get_ray_length_(sim_ptr_, (uint32_t)w) : 0.65;

		// For left wheels (FL=0, RL=2), inner is +X and outer is -X.
		// For right wheels (FR=1, RR=3), inner is -X and outer is +X.
		double x_offsets[3] = { (w % 2 == 0) ? span : -span, 0.0, (w % 2 == 0) ? -span : span };

		for (int r = 0; r < 3; ++r) {
			String node_name = String("RayCast_") + w_names[w] + "_" + r_names[r];
			RayCast3D *ray = Object::cast_to<RayCast3D>(find_child(node_name, true, false));

			if (!ray) {
				ray = memnew(RayCast3D);
				ray->set_name(node_name);
				add_child(ray);
			}

			Vector3 local_origin = wheel_base_positions_[w] + Vector3(x_offsets[r], 0.0, 0.0);
			ray->set_position(local_origin);
			ray->set_target_position(Vector3(0.0, -ray_length, 0.0));
			ray->set_enabled(true);
			ray->set_collide_with_areas(false);
			ray->set_collide_with_bodies(true);
			ray->add_exception(this);
			CollisionObject3D *parent_col = Object::cast_to<CollisionObject3D>(get_parent());
			if (parent_col) {
				ray->add_exception(parent_col);
			}

			raycasts_[w][r] = ray;
		}
	}

	const char *uf_names[5] = {
		"UnderfloorFrontLeft", "UnderfloorFrontRight", "UnderfloorCenter",
		"DiffuserThroat", "DiffuserExit"
	};
	const Vector3 uf_positions[5] = {
		Vector3(-0.45, -0.205, -0.90), Vector3(0.45, -0.205, -0.90),
		Vector3(0.0, -0.205, 0.0), Vector3(0.0, -0.165, 0.85),
		Vector3(0.0, -0.165, 1.65)
	};
	for (int i = 0; i < 5; ++i) {
		RayCast3D *ray = Object::cast_to<RayCast3D>(find_child(uf_names[i], true, false));
		if (!ray) {
			ray = memnew(RayCast3D);
			ray->set_name(uf_names[i]);
			add_child(ray);
		}
		ray->set_position(uf_positions[i]);
		ray->set_target_position(Vector3(0.0, -0.35, 0.0));
		ray->set_collision_mask(1);
		ray->set_enabled(true);
		ray->set_collide_with_areas(false);
		ray->set_collide_with_bodies(true);
		ray->add_exception(this);
		underfloor_raycasts_[i] = ray;
	}
}

void F194RustVehicle::_notification(int p_what) {
	if (p_what == NOTIFICATION_READY) {
		_ready();
	} else if (p_what == NOTIFICATION_EXIT_TREE) {
		_exit_tree();
	}
}

void F194RustVehicle::_ready() {
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}

	if (!load_rust_dll()) {
		UtilityFunctions::printerr("[F194RustVehicle] GDExtension failed to initialize Rust physics DLL!");
		return;
	}

	if (fn_create_from_json_) {
		const String json_path = physics_config_path_.is_empty()
			? String("res://data/vehicles/f1_2026_2008/f1_2026_2008_physics.json")
			: physics_config_path_;
		Ref<FileAccess> f = FileAccess::open(json_path, FileAccess::READ);
		if (f.is_valid()) {
			PackedByteArray bytes = f->get_buffer(f->get_length());
			f->close();
			if (bytes.size() > 0) {
				uint8_t err_buf[512] = {};
				void *json_sim = fn_create_from_json_(
					bytes.ptr(),
					(uint32_t)bytes.size(),
					err_buf,
					sizeof(err_buf));
				if (json_sim) {
					sim_ptr_ = json_sim;
					UtilityFunctions::print(String("[F194RustVehicle] Loaded physics from JSON: ") + json_path);
				} else {
					String err_msg = String(err_buf[0] ? reinterpret_cast<const char *>(err_buf) : "unknown error");
					UtilityFunctions::printerr(String("[F194RustVehicle] CRITICAL JSON PARSE ERROR: ") + err_msg + ". Falling back to default canonical configuration.");
				}
			}
		}
	}

	if (!sim_ptr_ && fn_create_default_) {
		sim_ptr_ = fn_create_default_();
		UtilityFunctions::printerr("[F194RustVehicle] WARNING: JSON physics load failed; falling back to canonical defaults.");
	} else if (!sim_ptr_ && fn_create_with_pos_) {
		Vector3 pos = get_global_position();
		Vector3 rot = get_global_rotation();
		sim_ptr_ = fn_create_with_pos_(pos.x, pos.y, pos.z, rot.y);
	}

	if (!sim_ptr_) {
		UtilityFunctions::printerr("[F194RustVehicle] Failed to create Rust VehicleSimulator instance!");
		return;
	}

	// 1. Dynamic RigidBody3D setup
	set_freeze_enabled(false);
	set_gravity_scale(1.0);
	set_contact_monitor(true);
	set_max_contacts_reported(4);

	// 2. Synchronize config from authoritative Rust backend
	sync_runtime_config_from_rust();

	// 3. Center of mass mode and position from Rust
	if (fn_get_center_of_mass_local_) {
		double com_x = 0.0, com_y = 0.0, com_z = 0.0;
		fn_get_center_of_mass_local_(sim_ptr_, &com_x, &com_y, &com_z);
		set_center_of_mass_mode(RigidBody3D::CENTER_OF_MASS_MODE_CUSTOM);
		set_center_of_mass(Vector3(com_x, com_y, com_z));
	}

	// 4. Setup 12 RayCast3D nodes (inner, center, outer per wheel)
	setup_raycasts();

	// Resolve visual wheel and chassis nodes from NodePaths
	if (!chassis_node_path_.is_empty()) {
		chassis_node_ = Object::cast_to<Node3D>(get_node_or_null(chassis_node_path_));
	}
	if (!front_left_wheel_node_path_.is_empty()) {
		wheel_nodes_[0] = Object::cast_to<Node3D>(get_node_or_null(front_left_wheel_node_path_));
	}
	if (!front_right_wheel_node_path_.is_empty()) {
		wheel_nodes_[1] = Object::cast_to<Node3D>(get_node_or_null(front_right_wheel_node_path_));
	}
	if (!rear_left_wheel_node_path_.is_empty()) {
		wheel_nodes_[2] = Object::cast_to<Node3D>(get_node_or_null(rear_left_wheel_node_path_));
	}
	if (!rear_right_wheel_node_path_.is_empty()) {
		wheel_nodes_[3] = Object::cast_to<Node3D>(get_node_or_null(rear_right_wheel_node_path_));
	}

	UtilityFunctions::print("[F194RustVehicle] Black-box Rust Physics Core GDExtension ready!");
}

uint32_t F194RustVehicle::detect_surface_type(const RayCast3D *ray) const {
	if (!ray || !ray->is_colliding()) {
		return 0; // Road
	}
	Object *collider = ray->get_collider();
	if (!collider) {
		return 0;
	}
	Node *node = Object::cast_to<Node>(collider);
	if (node) {
		if (node->is_in_group("Curb") || node->is_in_group("curb") || node->is_in_group("Kerb") || node->is_in_group("kerb")) {
			return 1;
		}
		if (node->is_in_group("Dirt") || node->is_in_group("dirt")) {
			return 2;
		}
		if (node->is_in_group("Grass") || node->is_in_group("grass") || node->is_in_group("Cesped") || node->is_in_group("cesped")) {
			return 3;
		}
		if (node->is_in_group("Gravel") || node->is_in_group("gravel") || node->is_in_group("Grava") || node->is_in_group("grava")) {
			return 4;
		}
		if (node->is_in_group("Sand") || node->is_in_group("sand") || node->is_in_group("Arena") || node->is_in_group("arena")) {
			return 5;
		}
		if (node->is_in_group("Wall") || node->is_in_group("wall") || node->is_in_group("Barrier") || node->is_in_group("barrier") || node->is_in_group("Guardrail") || node->is_in_group("guardrail")) {
			return 6;
		}
		if (node->is_in_group("Metal") || node->is_in_group("metal")) {
			return 7;
		}
		if (node->is_in_group("Road") || node->is_in_group("road") || node->is_in_group("Track") || node->is_in_group("track") || node->is_in_group("Asphalt") || node->is_in_group("asphalt")) {
			return 0;
		}

		// Fallback check by name
		String name = node->get_name().to_lower();
		if (name.contains("curb") || name.contains("kerb") || name.contains("piano")) {
			return 1;
		}
		if (name.contains("dirt")) {
			return 2;
		}
		if (name.contains("grass") || name.contains("cesped")) {
			return 3;
		}
		if (name.contains("gravel") || name.contains("grava")) {
			return 4;
		}
		if (name.contains("sand") || name.contains("arena")) {
			return 5;
		}
		if (name.contains("wall") || name.contains("guardrail") || name.contains("barrier")) {
			return 6;
		}
		if (name.contains("metal")) {
			return 7;
		}
	}
	return 0; // Default Road
}

void F194RustVehicle::_integrate_forces(PhysicsDirectBodyState3D *p_state) {
	if (bridge_controlled_) {
		// Snapshot-server wiring: the F90Core (orchestrator facade) or the legacy
		// F90Core owns the dynamics. It samples the raycasts and steps the
		// authoritative core inside this integrate callback (the context where
		// force_raycast_update() is guaranteed fresh) and applies the resulting body
		// force/torque via p_state.
		if (core_driver_ != nullptr) {
			core_driver_->drive_integrate(this, p_state);
		}
		return;
	}

	solve_forces_for_state(p_state);
}

void F194RustVehicle::solve_forces_for_state(PhysicsDirectBodyState3D *p_state) {
	if (bridge_controlled_) {
		return;
	}
	if (Engine::get_singleton()->is_editor_hint() || !sim_ptr_ || !fn_solve_forces_ || !p_state) {
		return;
	}

	// Synchronize Godot rigid-body inertia once (P1-F)
	if (!inertia_initialized_) {
		Vector3 inv_i = p_state->get_inverse_inertia();
		if (inv_i.x > 0.0F && inv_i.y > 0.0F && inv_i.z > 0.0F && std::isfinite(inv_i.x) && std::isfinite(inv_i.y) && std::isfinite(inv_i.z)) {
			Vector3 base_inertia(1.0F / inv_i.x, 1.0F / inv_i.y, 1.0F / inv_i.z);
			Vector3 configured_inertia(
				base_inertia.x * (float)inertia_multiplier_x_,
				base_inertia.y * (float)inertia_multiplier_y_,
				base_inertia.z * (float)inertia_multiplier_z_
			);
			if (configured_inertia.x > 0.0F && configured_inertia.y > 0.0F && configured_inertia.z > 0.0F &&
				std::isfinite(configured_inertia.x) && std::isfinite(configured_inertia.y) && std::isfinite(configured_inertia.z)) {
				PhysicsServer3D *ps = PhysicsServer3D::get_singleton();
				if (ps) {
					ps->body_set_param(get_rid(), PhysicsServer3D::BODY_PARAM_INERTIA, configured_inertia);
				}
				inertia_initialized_ = true;
				UtilityFunctions::print(String("[F194RustVehicle] Base inertia: ") + Variant(base_inertia).stringify() + " -> Configured inertia: " + Variant(configured_inertia).stringify());
			} else {
				UtilityFunctions::push_warning("[F194RustVehicle] Configured inertia calculation resulted in invalid values; retaining automatic inertia.");
			}
		} else {
			UtilityFunctions::push_warning("[F194RustVehicle] Base inertia tensor from Godot is invalid; retaining automatic inertia.");
		}
	}

	// 1. Extract 6-DOF transform, linear velocity, angular velocity, and delta time
	Transform3D gt = p_state->get_transform();
	Vector3 pos = gt.origin;
	Quaternion rot = gt.basis.get_rotation_quaternion();
	if (rot.is_finite() && rot.length_squared() > 1e-4) {
		rot.normalize();
	} else {
		rot = Quaternion();
	}

	Vector3 lin_vel = p_state->get_linear_velocity();
	Vector3 ang_vel = p_state->get_angular_velocity();
	double dt = (double)p_state->get_step();

	F90BodyKinematics kinematics = {};
	kinematics.pos_x = pos.x;
	kinematics.pos_y = pos.y;
	kinematics.pos_z = pos.z;
	kinematics.rot_quat_x = rot.x;
	kinematics.rot_quat_y = rot.y;
	kinematics.rot_quat_z = rot.z;
	kinematics.rot_quat_w = rot.w;
	kinematics.lin_vel_x = lin_vel.x;
	kinematics.lin_vel_y = lin_vel.y;
	kinematics.lin_vel_z = lin_vel.z;
	kinematics.ang_vel_x = ang_vel.x;
	kinematics.ang_vel_y = ang_vel.y;
	kinematics.ang_vel_z = ang_vel.z;

	// 2. Consume Driver Inputs stored on the vehicle node
	F90VehicleInput input = {};
	input.throttle = throttle_amount_;
	input.steering = steering_input_;
	input.brake = brake_amount_;
	input.handbrake = handbrake_amount_;
	input.clutch = clutch_amount_;
	input.gear_request = gear_request_;

	// 3. Sample 12 RayCast3Ds in Godot (global coordinates)
	F90TriRaycastSample samples[4] = {};
	for (int w = 0; w < 4; ++w) {
		RayCast3D *r_in = raycasts_[w][0];
		RayCast3D *r_mid = raycasts_[w][1];
		RayCast3D *r_out = raycasts_[w][2];

		auto sample_ray = [this](RayCast3D *ray, F90RaycastHit &hit) {
			hit.is_colliding = false;
			hit.distance = 0.65;
			hit.point_x = 0.0;
			hit.point_y = 0.0;
			hit.point_z = 0.0;
			hit.normal_x = 0.0;
			hit.normal_y = 1.0;
			hit.normal_z = 0.0;
			hit.surface_type = 0;

			if (ray) {
				ray->force_raycast_update();
				if (ray->is_colliding()) {
					hit.is_colliding = true;
					Vector3 pt = ray->get_collision_point();
					Vector3 n = ray->get_collision_normal();
					hit.distance = (pt - ray->get_global_position()).length();
					hit.point_x = pt.x;
					hit.point_y = pt.y;
					hit.point_z = pt.z;
					if (n.is_finite() && n.length_squared() > 1e-4) {
						n.normalize();
						hit.normal_x = n.x;
						hit.normal_y = n.y;
						hit.normal_z = n.z;
					}
					hit.surface_type = detect_surface_type(ray);
				}
			}
		};

		sample_ray(r_in, samples[w].inner);
		sample_ray(r_mid, samples[w].center);
		sample_ray(r_out, samples[w].outer);
	}

	// 4. Solve Forces via Rust C FFI
	F90ForceTorqueOutput out_forces = {};
	F90TelemetryOutput telem = {};
	fn_solve_forces_(sim_ptr_, &kinematics, &input, samples, dt, &out_forces, &telem);

	// 5. Apply Forces and Torques Directly (Godot integrates gravity and rigid motion)
	Vector3 force(out_forces.force_x, out_forces.force_y, out_forces.force_z);
	Vector3 torque(out_forces.torque_x, out_forces.torque_y, out_forces.torque_z);

	if (force.is_finite()) {
		p_state->apply_central_force(force);
	}
	if (torque.is_finite()) {
		p_state->apply_torque(torque);
	}

	// 6. Store Telemetry Variables
	lin_vel_ = lin_vel;
	ang_vel_ = ang_vel;

	speed_kmh_ = telem.speed_kmh;
	speed_ms_ = telem.speed_kmh / 3.6;
	motor_rpm_ = telem.rpm;
	current_gear_ = telem.gear;
	engine_torque_ = telem.engine_torque;
	clutch_engagement_ = telem.clutch_engagement;
	clutch_torque_ = telem.clutch_torque;
	true_steering_amount_ = telem.steer;
	steer_angle_rad_ = telem.steer_angle_rad;

	lat_g_ = telem.lat_g;
	long_g_ = telem.long_g;
	vert_g_ = telem.vert_g;

	wheel_compressions_[0] = telem.fl_comp_mm;
	wheel_compressions_[1] = telem.fr_comp_mm;
	wheel_compressions_[2] = telem.rl_comp_mm;
	wheel_compressions_[3] = telem.rr_comp_mm;

	wheel_spins_[0] = telem.fl_spin;
	wheel_spins_[1] = telem.fr_spin;
	wheel_spins_[2] = telem.rl_spin;
	wheel_spins_[3] = telem.rr_spin;

	wheel_slips_[0] = telem.fl_slip;
	wheel_slips_[1] = telem.fr_slip;
	wheel_slips_[2] = telem.rl_slip;
	wheel_slips_[3] = telem.rr_slip;

	wheel_drive_torques_[0] = telem.fl_drive_torque;
	wheel_drive_torques_[1] = telem.fr_drive_torque;
	wheel_drive_torques_[2] = telem.rl_drive_torque;
	wheel_drive_torques_[3] = telem.rr_drive_torque;
	for (int i = 0; i < 4; ++i) {
		wheel_drive_torques_pre_tc_[i] = telem.drive_torque_pre_tc_nm[i];
		tc_slip_ratio_[i] = telem.tc_slip_ratio[i];
	}
	tc_enabled_ = (telem.aids_enabled_mask & 2U) != 0;
	tc_eligible_ = telem.tc_eligible;
	tc_intervening_ = telem.tc_active;
	tc_gear_authority_ = telem.tc_gear_authority;
	tc_slip_target_ = telem.tc_slip_target;
	tc_raw_cut_ratio_ = telem.tc_raw_cut_ratio;
	tc_cut_ratio_ = telem.tc_cut_ratio;
	pre_tc_drive_power_w_ = telem.pre_tc_drive_power_w;
	net_drive_power_w_ = telem.net_drive_power_w;
	wheel_normal_forces_[0] = telem.fl_normal_force;
	wheel_normal_forces_[1] = telem.fr_normal_force;
	wheel_normal_forces_[2] = telem.rl_normal_force;
	wheel_normal_forces_[3] = telem.rr_normal_force;

	tire_pressure_kpa_[0] = telem.fl_pressure_kpa;
	tire_pressure_kpa_[1] = telem.fr_pressure_kpa;
	tire_pressure_kpa_[2] = telem.rl_pressure_kpa;
	tire_pressure_kpa_[3] = telem.rr_pressure_kpa;
	tire_tread_inner_c_[0] = telem.fl_tread_inner_c;
	tire_tread_inner_c_[1] = telem.fr_tread_inner_c;
	tire_tread_inner_c_[2] = telem.rl_tread_inner_c;
	tire_tread_inner_c_[3] = telem.rr_tread_inner_c;
	tire_tread_center_c_[0] = telem.fl_tread_center_c;
	tire_tread_center_c_[1] = telem.fr_tread_center_c;
	tire_tread_center_c_[2] = telem.rl_tread_center_c;
	tire_tread_center_c_[3] = telem.rr_tread_center_c;
	tire_tread_outer_c_[0] = telem.fl_tread_outer_c;
	tire_tread_outer_c_[1] = telem.fr_tread_outer_c;
	tire_tread_outer_c_[2] = telem.rl_tread_outer_c;
	tire_tread_outer_c_[3] = telem.rr_tread_outer_c;
	tire_carcass_c_[0] = telem.fl_carcass_c;
	tire_carcass_c_[1] = telem.fr_carcass_c;
	tire_carcass_c_[2] = telem.rl_carcass_c;
	tire_carcass_c_[3] = telem.rr_carcass_c;
	tire_gas_c_[0] = telem.fl_gas_c;
	tire_gas_c_[1] = telem.fr_gas_c;
	tire_gas_c_[2] = telem.rl_gas_c;
	tire_gas_c_[3] = telem.rr_gas_c;

	brake_disc_c_[0] = telem.fl_brake_disc_c;
	brake_disc_c_[1] = telem.fr_brake_disc_c;
	brake_disc_c_[2] = telem.rl_brake_disc_c;
	brake_disc_c_[3] = telem.rr_brake_disc_c;
	brake_rim_c_[0] = telem.fl_brake_rim_c;
	brake_rim_c_[1] = telem.fr_brake_rim_c;
	brake_rim_c_[2] = telem.rl_brake_rim_c;
	brake_rim_c_[3] = telem.rr_brake_rim_c;
	brake_efficiency_[0] = telem.fl_brake_efficiency;
	brake_efficiency_[1] = telem.fr_brake_efficiency;
	brake_efficiency_[2] = telem.rl_brake_efficiency;
	brake_efficiency_[3] = telem.rr_brake_efficiency;
	duct_mass_flow_kg_s_[0] = telem.fl_duct_mass_flow_kg_s;
	duct_mass_flow_kg_s_[1] = telem.fr_duct_mass_flow_kg_s;
	duct_mass_flow_kg_s_[2] = telem.rl_duct_mass_flow_kg_s;
	duct_mass_flow_kg_s_[3] = telem.rr_duct_mass_flow_kg_s;
	duct_drag_n_[0] = telem.fl_duct_drag_n;
	duct_drag_n_[1] = telem.fr_duct_drag_n;
	duct_drag_n_[2] = telem.rl_duct_drag_n;
	duct_drag_n_[3] = telem.rr_duct_drag_n;
	brake_optimal_min_c_ = telem.brake_optimal_min_c;
	brake_optimal_max_c_ = telem.brake_optimal_max_c;
	brake_fade_start_c_ = telem.brake_fade_start_c;
	brake_critical_c_ = telem.brake_critical_c;
	brake_torque_nm_[0] = telem.fl_brake_torque_nm;
	brake_torque_nm_[1] = telem.fr_brake_torque_nm;
	brake_torque_nm_[2] = telem.rl_brake_torque_nm;
	brake_torque_nm_[3] = telem.rr_brake_torque_nm;
	brake_spin_pre_rad_s_[0] = telem.fl_brake_spin_pre_rad_s;
	brake_spin_pre_rad_s_[1] = telem.fr_brake_spin_pre_rad_s;
	brake_spin_pre_rad_s_[2] = telem.rl_brake_spin_pre_rad_s;
	brake_spin_pre_rad_s_[3] = telem.rr_brake_spin_pre_rad_s;
	brake_spin_post_rad_s_[0] = telem.fl_brake_spin_post_rad_s;
	brake_spin_post_rad_s_[1] = telem.fr_brake_spin_post_rad_s;
	brake_spin_post_rad_s_[2] = telem.rl_brake_spin_post_rad_s;
	brake_spin_post_rad_s_[3] = telem.rr_brake_spin_post_rad_s;
	brake_power_w_[0] = telem.fl_brake_power_w;
	brake_power_w_[1] = telem.fr_brake_power_w;
	brake_power_w_[2] = telem.rl_brake_power_w;
	brake_power_w_[3] = telem.rr_brake_power_w;
	brake_energy_j_[0] = telem.fl_brake_energy_j;
	brake_energy_j_[1] = telem.fr_brake_energy_j;
	brake_energy_j_[2] = telem.rl_brake_energy_j;
	brake_energy_j_[3] = telem.rr_brake_energy_j;
	brake_natural_cooling_w_k_[0] = telem.fl_brake_natural_cooling_w_k;
	brake_natural_cooling_w_k_[1] = telem.fr_brake_natural_cooling_w_k;
	brake_natural_cooling_w_k_[2] = telem.rl_brake_natural_cooling_w_k;
	brake_natural_cooling_w_k_[3] = telem.rr_brake_natural_cooling_w_k;
	brake_speed_cooling_w_k_[0] = telem.fl_brake_speed_cooling_w_k;
	brake_speed_cooling_w_k_[1] = telem.fr_brake_speed_cooling_w_k;
	brake_speed_cooling_w_k_[2] = telem.rl_brake_speed_cooling_w_k;
	brake_speed_cooling_w_k_[3] = telem.rr_brake_speed_cooling_w_k;

	// 7. Visual Animation of Wheel Meshes
	update_wheel_visuals(dt);
}

void F194RustVehicle::update_wheel_visuals(double delta) {
	for (int w = 0; w < 4; ++w) {
		if (std::isfinite(wheel_spins_[w])) {
			wheel_angles_[w] += wheel_spins_[w] * delta;
		}
		Node3D *w_node = wheel_nodes_[w];
		if (!w_node) {
			continue;
		}

		// Vertical suspension displacement:
		// Hub Y = anchor_Y - spring_length + compression_m
		double spring_length = (w < 2) ? suspension_front_spring_length_ : suspension_rear_spring_length_;
		double resting_ratio = (w < 2) ? suspension_front_resting_ratio_ : suspension_rear_resting_ratio_;
		double anchor_y = wheel_base_positions_[w].y;
		double comp_m = std::isfinite(wheel_compressions_[w]) ? (wheel_compressions_[w] * 0.001) : (spring_length * resting_ratio);
		double visual_y = anchor_y - spring_length + comp_m;

		Vector3 pos = w_node->get_position();
		pos.y = visual_y;
		if (pos.is_finite()) {
			w_node->set_position(pos);
		}

		// Front steering and rolling rotations (-angle to roll forward along -Z)
		double steer = 0.0;
		if (w < 2) {
			steer = std::isfinite(steer_angle_rad_) ? steer_angle_rad_ : (true_steering_amount_ * max_steering_angle_);
		}
		double angle = std::isfinite(wheel_angles_[w]) ? wheel_angles_[w] : 0.0;
		w_node->set_rotation(Vector3(-angle, steer, 0.0));
	}
}

PackedFloat64Array F194RustVehicle::get_wheel_compressions() const {
	PackedFloat64Array arr;
	arr.resize(4);
	for (int i = 0; i < 4; ++i) {
		arr[i] = wheel_compressions_[i];
	}
	return arr;
}

PackedFloat64Array F194RustVehicle::get_wheel_spins() const {
	PackedFloat64Array arr;
	arr.resize(4);
	for (int i = 0; i < 4; ++i) {
		arr[i] = wheel_spins_[i];
	}
	return arr;
}

PackedFloat64Array F194RustVehicle::get_wheel_slips() const {
	PackedFloat64Array arr;
	arr.resize(4);
	for (int i = 0; i < 4; ++i) {
		arr[i] = wheel_slips_[i];
	}
	return arr;
}

// Returns the dominant surface code per wheel (0=Road/asphalt, 1=Curb, 2=Dirt,
// 3=Grass, 4=Gravel, 5=Sand, 6=Wall, 7=Metal) sampled from this wheel's 3
// tri-raycasts. Used by VehicleAudioControllerNative::detect_surface so the Rust
// vehicle reports real surface info instead of always falling back to "asphalt".
PackedInt64Array F194RustVehicle::get_wheel_surface_types() const {
	PackedInt64Array arr;
	arr.resize(4);
	for (int w = 0; w < 4; ++w) {
		int center = 0;
		int any_nonzero = 0;
		for (int r = 0; r < 3; ++r) {
			RayCast3D *ray = raycasts_[w][r];
			const uint32_t code = detect_surface_type(ray);
			if (r == 1) {
				center = (int)code;
			}
			if (code != 0 && any_nonzero == 0) {
				any_nonzero = (int)code;
			}
		}
		// Prefer the center ray; otherwise any non-road ray detected by inner/outer.
		arr[w] = (center != 0) ? center : any_nonzero;
	}
	return arr;
}

PackedFloat64Array F194RustVehicle::get_drive_torques() const {
	PackedFloat64Array arr;
	arr.resize(4);
	for (int i = 0; i < 4; ++i) {
		arr[i] = wheel_drive_torques_[i];
	}
	return arr;
}

Dictionary F194RustVehicle::get_powertrain_state_snapshot() const {
	Dictionary out;
	PackedFloat64Array torques;
	torques.resize(4);
	for (int i = 0; i < 4; ++i) {
		torques[i] = wheel_drive_torques_[i];
	}
	out["wheel_drive_torque_nm"] = torques;
	PackedFloat64Array pre_tc_torques;
	PackedFloat64Array tc_slips;
	pre_tc_torques.resize(4);
	tc_slips.resize(4);
	for (int i = 0; i < 4; ++i) {
		pre_tc_torques[i] = wheel_drive_torques_pre_tc_[i];
		tc_slips[i] = tc_slip_ratio_[i];
	}
	out["wheel_drive_torque_pre_tc_nm"] = pre_tc_torques;
	out["tc_slip_ratio"] = tc_slips;
	out["tc_enabled"] = tc_enabled_;
	out["tc_eligible"] = tc_eligible_;
	out["tc_intervening"] = tc_intervening_;
	out["tc_gear_authority"] = tc_gear_authority_;
	out["tc_slip_target"] = tc_slip_target_;
	out["tc_raw_cut_ratio"] = tc_raw_cut_ratio_;
	out["tc_cut_ratio"] = tc_cut_ratio_;
	out["pre_tc_drive_power_w"] = pre_tc_drive_power_w_;
	out["net_drive_power_w"] = net_drive_power_w_;
	return out;
}

PackedFloat64Array F194RustVehicle::get_normal_forces() const {
	PackedFloat64Array arr;
	arr.resize(4);
	for (int i = 0; i < 4; ++i) {
		arr[i] = wheel_normal_forces_[i];
	}
	return arr;
}

double F194RustVehicle::get_vehicle_mass_value() const {
	if (sim_ptr_ && fn_get_vehicle_mass_) {
		return fn_get_vehicle_mass_(sim_ptr_);
	}
	return get_mass();
}

Vector3 F194RustVehicle::get_center_of_mass_local_value() const {
	if (sim_ptr_ && fn_get_center_of_mass_local_) {
		double x = 0.0, y = 0.0, z = 0.0;
		fn_get_center_of_mass_local_(sim_ptr_, &x, &y, &z);
		return Vector3(x, y, z);
	}
	return get_center_of_mass();
}

Vector3 F194RustVehicle::get_wheel_anchor_local_value(int p_wheel_idx) const {
	if (p_wheel_idx < 0 || p_wheel_idx >= 4) {
		return Vector3();
	}
	if (sim_ptr_ && fn_get_anchor_) {
		double x = 0.0, y = 0.0, z = 0.0;
		fn_get_anchor_(sim_ptr_, (uint32_t)p_wheel_idx, &x, &y, &z);
		return Vector3(x, y, z);
	}
	return wheel_base_positions_[p_wheel_idx];
}

double F194RustVehicle::get_tri_ray_span_value(int p_wheel_idx) const {
	if (p_wheel_idx < 0 || p_wheel_idx >= 4) {
		return 0.12;
	}
	if (sim_ptr_ && fn_get_tri_span_) {
		return fn_get_tri_span_(sim_ptr_, (uint32_t)p_wheel_idx);
	}
	return (p_wheel_idx < 2) ? (0.305 * 0.40) : (0.380 * 0.40);
}

double F194RustVehicle::get_ray_length_value(int p_wheel_idx) const {
	if (p_wheel_idx < 0 || p_wheel_idx >= 4) {
		return 0.65;
	}
	if (sim_ptr_ && fn_get_ray_length_) {
		return fn_get_ray_length_(sim_ptr_, (uint32_t)p_wheel_idx);
	}
	return 0.65;
}

double F194RustVehicle::get_default_spawn_height_value() const {
	if (sim_ptr_ && fn_get_default_spawn_height_) {
		return fn_get_default_spawn_height_(sim_ptr_);
	}
	return 0.35;
}

void F194RustVehicle::apply_runtime_config() {
	F90RuntimeConfig cfg = {};
	cfg.vehicle_mass = vehicle_mass_;
	cfg.front_brake_bias = front_brake_bias_;
	cfg.max_steering_angle = max_steering_angle_;
	cfg.max_torque = max_torque_;
	cfg.coefficient_of_drag = coefficient_of_drag_;
	cfg.frontal_area = frontal_area_;
	cfg.air_density = air_density_;
	cfg.steering_exponent = steering_exponent_;
	cfg.steering_speed = steering_speed_;
	cfg.countersteer_speed = countersteer_speed_;
	cfg.automatic_transmission = automatic_transmission_;
	cfg.diff_preload = diff_preload_;
	cfg.diff_power_ramp_angle_deg = diff_power_ramp_angle_deg_;
	cfg.diff_coast_ramp_angle_deg = diff_coast_ramp_angle_deg_;
	cfg.diff_clutches = diff_clutches_;
	cfg.diff_clutch_friction_coeff = diff_clutch_friction_coeff_;
	cfg.aids_enabled_mask = aids_enabled_mask_;
	cfg.inertia_multiplier_x = inertia_multiplier_x_;
	cfg.inertia_multiplier_y = inertia_multiplier_y_;
	cfg.inertia_multiplier_z = inertia_multiplier_z_;
	cfg.suspension_front_spring_length = suspension_front_spring_length_;
	cfg.suspension_rear_spring_length = suspension_rear_spring_length_;
	cfg.suspension_front_resting_ratio = suspension_front_resting_ratio_;
	cfg.suspension_rear_resting_ratio = suspension_rear_resting_ratio_;

	if (sim_ptr_ && fn_apply_runtime_config_) {
		fn_apply_runtime_config_(sim_ptr_, &cfg);
	}
	// In bridge_controlled mode the facade's core is the sim that actually runs:
	// forward the SAME runtime config so every tunable JSON parameter keeps
	// applying at runtime (diff preload, aids mask, aero, ...).
	if (core_driver_ != nullptr) {
		core_driver_->apply_runtime_config(cfg);
	}
}

void F194RustVehicle::sync_runtime_config_from_rust() {
	if (!sim_ptr_ || !fn_get_runtime_config_) {
		return;
	}
	F90RuntimeConfig cfg = {};
	if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
		vehicle_mass_ = cfg.vehicle_mass;
		front_brake_bias_ = cfg.front_brake_bias;
		max_steering_angle_ = cfg.max_steering_angle;
		max_torque_ = cfg.max_torque;
		coefficient_of_drag_ = cfg.coefficient_of_drag;
		frontal_area_ = cfg.frontal_area;
		air_density_ = cfg.air_density;
		steering_exponent_ = cfg.steering_exponent;
		steering_speed_ = cfg.steering_speed;
		countersteer_speed_ = cfg.countersteer_speed;
		automatic_transmission_ = cfg.automatic_transmission;
		diff_preload_ = cfg.diff_preload;
		diff_power_ramp_angle_deg_ = cfg.diff_power_ramp_angle_deg;
		diff_coast_ramp_angle_deg_ = cfg.diff_coast_ramp_angle_deg;
		diff_clutches_ = cfg.diff_clutches;
		diff_clutch_friction_coeff_ = cfg.diff_clutch_friction_coeff;
		aids_enabled_mask_ = cfg.aids_enabled_mask;
		tc_enabled_ = (cfg.aids_enabled_mask & (1U << 1)) != 0;
		inertia_multiplier_x_ = cfg.inertia_multiplier_x;
		inertia_multiplier_y_ = cfg.inertia_multiplier_y;
		inertia_multiplier_z_ = cfg.inertia_multiplier_z;
		suspension_front_spring_length_ = cfg.suspension_front_spring_length;
		suspension_rear_spring_length_ = cfg.suspension_rear_spring_length;
		suspension_front_resting_ratio_ = cfg.suspension_front_resting_ratio;
		suspension_rear_resting_ratio_ = cfg.suspension_rear_resting_ratio;
		set_mass((float)vehicle_mass_);
	}
}

void F194RustVehicle::set_vehicle_mass(double v) {
	vehicle_mass_ = v;
	set_mass((float)v);
	apply_runtime_config();
}

double F194RustVehicle::get_vehicle_mass() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.vehicle_mass;
		}
	}
	return vehicle_mass_;
}

void F194RustVehicle::set_max_torque(double v) {
	max_torque_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_max_torque() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.max_torque;
		}
	}
	return max_torque_;
}

void F194RustVehicle::set_front_brake_bias(double v) {
	front_brake_bias_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_front_brake_bias() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.front_brake_bias;
		}
	}
	return front_brake_bias_;
}

void F194RustVehicle::set_steering_exponent(double v) {
	steering_exponent_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_steering_exponent() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.steering_exponent;
		}
	}
	return steering_exponent_;
}

void F194RustVehicle::set_steering_speed(double v) {
	steering_speed_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_steering_speed() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.steering_speed;
		}
	}
	return steering_speed_;
}

void F194RustVehicle::set_countersteer_speed(double v) {
	countersteer_speed_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_countersteer_speed() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.countersteer_speed;
		}
	}
	return countersteer_speed_;
}

void F194RustVehicle::set_max_steering_angle(double v) {
	max_steering_angle_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_max_steering_angle() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.max_steering_angle;
		}
	}
	return max_steering_angle_;
}

void F194RustVehicle::set_coefficient_of_drag(double v) {
	coefficient_of_drag_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_coefficient_of_drag() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.coefficient_of_drag;
		}
	}
	return coefficient_of_drag_;
}

void F194RustVehicle::set_frontal_area(double v) {
	frontal_area_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_frontal_area() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.frontal_area;
		}
	}
	return frontal_area_;
}

void F194RustVehicle::set_air_density(double v) {
	air_density_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_air_density() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.air_density;
		}
	}
	return air_density_;
}

void F194RustVehicle::set_automatic_transmission(bool p_val) {
	automatic_transmission_ = p_val;
	apply_runtime_config();
}

bool F194RustVehicle::get_automatic_transmission() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.automatic_transmission;
		}
	}
	return automatic_transmission_;
}

void F194RustVehicle::reset_vehicle(const Vector3 &p_pos, double p_yaw_rad) {
	if (sim_ptr_ && fn_reset_) {
		fn_reset_(sim_ptr_, p_pos.x, p_pos.y, p_pos.z, p_yaw_rad);
	}
	// Orchestrator facade: when F90Core drives this vehicle, reset the core's
	// entity (powertrain/suspension/aids/modules) to the same pose so a reset does
	// not leave stale internal state behind.
	if (core_driver_ != nullptr) {
		core_driver_->reset_core_at(p_pos.x, p_pos.y, p_pos.z, p_yaw_rad);
	}
	Transform3D t(Basis(Vector3(0.0, 1.0, 0.0), p_yaw_rad), p_pos);
	set_global_transform(t);
	set_linear_velocity(Vector3());
	set_angular_velocity(Vector3());
	lin_vel_ = Vector3();
	ang_vel_ = Vector3();
	speed_kmh_ = 0.0;
	speed_ms_ = 0.0;
	throttle_amount_ = 0.0;
	steering_input_ = 0.0;
	brake_amount_ = 0.0;
	handbrake_amount_ = 0.0;
	clutch_amount_ = 0.0;
}

void F194RustVehicle::_exit_tree() {
	unload_rust_dll();
}

// --- Differential (Salisbury LSD) tuning accessors ---

void F194RustVehicle::set_diff_preload(double v) {
	diff_preload_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_diff_preload() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.diff_preload;
		}
	}
	return diff_preload_;
}

void F194RustVehicle::set_diff_power_ramp_angle_deg(double v) {
	diff_power_ramp_angle_deg_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_diff_power_ramp_angle_deg() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.diff_power_ramp_angle_deg;
		}
	}
	return diff_power_ramp_angle_deg_;
}

void F194RustVehicle::set_diff_coast_ramp_angle_deg(double v) {
	diff_coast_ramp_angle_deg_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_diff_coast_ramp_angle_deg() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.diff_coast_ramp_angle_deg;
		}
	}
	return diff_coast_ramp_angle_deg_;
}

void F194RustVehicle::set_diff_clutches(double v) {
	diff_clutches_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_diff_clutches() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.diff_clutches;
		}
	}
	return diff_clutches_;
}

void F194RustVehicle::set_diff_clutch_friction_coeff(double v) {
	diff_clutch_friction_coeff_ = v;
	apply_runtime_config();
}

double F194RustVehicle::get_diff_clutch_friction_coeff() const {
	if (sim_ptr_ && fn_get_runtime_config_) {
		F90RuntimeConfig cfg = {};
		if (fn_get_runtime_config_(sim_ptr_, &cfg)) {
			return cfg.diff_clutch_friction_coeff;
		}
	}
	return diff_clutch_friction_coeff_;
}

void F194RustVehicle::set_rear_locking_differential_engage_torque(double v) {
	// GEVP-facing alias: map a single engage-torque threshold onto the Salisbury
	// model as a flat capacity ceiling (preload = engage_torque, mu = 0 => ramp_lock = 0).
	rear_locking_differential_engage_torque_ = v;
	if (v >= 0.0) {
		diff_preload_ = v;
		diff_clutch_friction_coeff_ = 0.0;
	}
	apply_runtime_config();
}

double F194RustVehicle::get_rear_locking_differential_engage_torque() const {
	return rear_locking_differential_engage_torque_;
}

void F194RustVehicle::set_aids_enabled_mask(uint32_t v) {
	aids_enabled_mask_ = v;
	tc_enabled_ = (v & (1U << 1)) != 0;
	apply_runtime_config();
}

uint32_t F194RustVehicle::get_aids_enabled_mask() const {
	return aids_enabled_mask_;
}

void F194RustVehicle::set_bridge_controlled(bool p_v) {
	bridge_controlled_ = p_v;
	// The bridge uses the core's force solver (solve_external), which excludes gravity;
	// Godot supplies gravity and integrates the rigid body, so keep gravity on.
	set_gravity_scale(1.0);
	// Keep the body awake while bridge-controlled so applied forces integrate even at rest.
	set_can_sleep(!p_v);
	if (p_v) {
		set_sleeping(false);
	}
}

void F194RustVehicle::collect_core_samples(CSimTriRaycastSample p_samples[4]) {
	for (int w = 0; w < 4; ++w) {
		RayCast3D *rays[3] = { raycasts_[w][0], raycasts_[w][1], raycasts_[w][2] };
		for (int k = 0; k < 3; ++k) {
			RayCast3D *ray = rays[k];
			CSimRaycastHit &hit = (k == 0) ? p_samples[w].inner : (k == 1) ? p_samples[w].center : p_samples[w].outer;
			hit.is_colliding = false;
			hit.distance = 0.65;
			hit.px = hit.py = hit.pz = 0.0;
			hit.nx = 0.0;
			hit.ny = 1.0;
			hit.nz = 0.0;
			hit.surface = 0;
			if (ray) {
				ray->force_raycast_update();
				if (ray->is_colliding()) {
					hit.is_colliding = true;
					Vector3 pt = ray->get_collision_point();
					Vector3 n = ray->get_collision_normal();
					hit.distance = (pt - ray->get_global_position()).length();
					hit.px = pt.x;
					hit.py = pt.y;
					hit.pz = pt.z;
					if (n.is_finite() && n.length_squared() > 1e-4) {
						n.normalize();
						hit.nx = n.x;
						hit.ny = n.y;
						hit.nz = n.z;
					}
					hit.surface = detect_surface_type(ray);
				}
			}
		}
	}
}

void F194RustVehicle::collect_underfloor_sample(F90UnderfloorSample *p_sample, PhysicsDirectBodyState3D *p_state) {
	if (!p_sample) {
		return;
	}
	*p_sample = {};
	for (int i = 0; i < 5; ++i) {
		RayCast3D *ray = underfloor_raycasts_[i];
		F90UnderfloorRayHit &hit = p_sample->rays[i];
		hit.clearance_m = 0.35;
		hit.normal_y = 1.0;
		if (!ray) {
			continue;
		}
		ray->force_raycast_update();
		if (!ray->is_colliding()) {
			continue;
		}
		const Vector3 point = ray->get_collision_point();
		Vector3 normal = ray->get_collision_normal();
		if (normal.is_finite() && normal.length_squared() > 1e-6) {
			normal.normalize();
		}
		hit.valid = 1.0;
		hit.clearance_m = (point - ray->get_global_position()).length();
		hit.point_x = point.x; hit.point_y = point.y; hit.point_z = point.z;
		hit.normal_x = normal.x; hit.normal_y = normal.y; hit.normal_z = normal.z;
		hit.surface_code = (double)detect_surface_type(ray);
	}

	if (!p_state) {
		return;
	}
	const Vector3 body_velocity = p_state->get_linear_velocity();
	double strongest_impulse = 0.0;
	for (int i = 0; i < p_state->get_contact_count(); ++i) {
		const Vector3 local_position = p_state->get_contact_local_position(i);
		const Vector3 local_normal = p_state->get_contact_local_normal(i);
		// Collision proxies bottom at roughly -0.16..-0.20 m. Reject side/nose hits.
		if (local_position.y > -0.12 || local_normal.y < 0.55) {
			continue;
		}
		const Vector3 impulse = p_state->get_contact_impulse(i);
		const double impulse_magnitude = impulse.length();
		if (p_sample->rigid_confirmed > 0.5 && impulse_magnitude <= strongest_impulse) {
			continue;
		}
		strongest_impulse = impulse_magnitude;
		const Vector3 collider_velocity = p_state->get_contact_collider_velocity_at_position(i);
		const Vector3 relative_velocity = body_velocity - collider_velocity;
		const Vector3 world_normal = p_state->get_transform().basis.xform(local_normal);
		const Vector3 tangent = relative_velocity - world_normal * relative_velocity.dot(world_normal);
		p_sample->rigid_confirmed = 1.0;
		p_sample->rigid_local_x = local_position.x;
		p_sample->rigid_local_y = local_position.y;
		p_sample->rigid_local_z = local_position.z;
		p_sample->rigid_normal_impulse_ns = impulse_magnitude;
		p_sample->rigid_tangential_speed_m_s = tangent.length();
	}
}

void F194RustVehicle::apply_core_motion(const Vector3 &p_lin_vel, const Vector3 &p_ang_vel) {
	set_linear_velocity(p_lin_vel);
	set_angular_velocity(p_ang_vel);
}

void F194RustVehicle::set_core_tire_telemetry(
	const double pressure_kpa[4],
	const double tread_inner_c[4],
	const double tread_center_c[4],
	const double tread_outer_c[4],
	const double carcass_c[4],
	const double gas_c[4]) {
	for (int i = 0; i < 4; ++i) {
		tire_pressure_kpa_[i] = pressure_kpa[i];
		tire_tread_inner_c_[i] = tread_inner_c[i];
		tire_tread_center_c_[i] = tread_center_c[i];
		tire_tread_outer_c_[i] = tread_outer_c[i];
		tire_carcass_c_[i] = carcass_c[i];
		tire_gas_c_[i] = gas_c[i];
	}
}

void F194RustVehicle::set_core_brake_telemetry(
	const double disc_c[4],
	const double rim_c[4],
	const double efficiency[4],
	const double duct_mass_flow_kg_s[4],
	const double duct_drag_n[4],
	double optimal_min_c,
	double optimal_max_c,
	double fade_start_c,
	double critical_c) {
	brake_optimal_min_c_ = optimal_min_c;
	brake_optimal_max_c_ = optimal_max_c;
	brake_fade_start_c_ = fade_start_c;
	brake_critical_c_ = critical_c;
	for (int i = 0; i < 4; ++i) {
		brake_disc_c_[i] = disc_c[i];
		brake_rim_c_[i] = rim_c[i];
		brake_efficiency_[i] = efficiency[i];
		duct_mass_flow_kg_s_[i] = duct_mass_flow_kg_s[i];
		duct_drag_n_[i] = duct_drag_n[i];
	}
}

Dictionary F194RustVehicle::get_tire_state_snapshot() const {
	static const char *WHEELS[4] = { "FL", "FR", "RL", "RR" };
	Dictionary out;
	out["schema_version"] = 1;
	for (int i = 0; i < 4; ++i) {
		Dictionary wheel;
		wheel["pressure_kpa"] = tire_pressure_kpa_[i];
		wheel["tread_inner_c"] = tire_tread_inner_c_[i];
		wheel["tread_center_c"] = tire_tread_center_c_[i];
		wheel["tread_outer_c"] = tire_tread_outer_c_[i];
		wheel["carcass_c"] = tire_carcass_c_[i];
		wheel["gas_c"] = tire_gas_c_[i];
		out[String(WHEELS[i])] = wheel;
	}
	return out;
}

Dictionary F194RustVehicle::get_brake_state_snapshot() const {
	static const char *WHEELS[4] = { "FL", "FR", "RL", "RR" };
	Dictionary out;
	out["schema_version"] = 1;
	for (int i = 0; i < 4; ++i) {
		Dictionary wheel;
		wheel["disc_c"] = brake_disc_c_[i];
		wheel["natural_cooling_w_k"] = brake_natural_cooling_w_k_[i];
		wheel["speed_cooling_w_k"] = brake_speed_cooling_w_k_[i];
		wheel["rim_c"] = brake_rim_c_[i];
		wheel["efficiency"] = brake_efficiency_[i];
		wheel["duct_mass_flow_kg_s"] = duct_mass_flow_kg_s_[i];
		wheel["duct_drag_n"] = duct_drag_n_[i];
		wheel["brake_torque_nm"] = brake_torque_nm_[i];
		wheel["spin_pre_rad_s"] = brake_spin_pre_rad_s_[i];
		wheel["spin_post_rad_s"] = brake_spin_post_rad_s_[i];
		wheel["brake_power_w"] = brake_power_w_[i];
		wheel["brake_energy_j"] = brake_energy_j_[i];
		wheel["optimal_min_c"] = brake_optimal_min_c_;
		wheel["optimal_max_c"] = brake_optimal_max_c_;
		wheel["fade_start_c"] = brake_fade_start_c_;
		wheel["critical_c"] = brake_critical_c_;
		out[String(WHEELS[i])] = wheel;
	}
	return out;
}

Dictionary F194RustVehicle::get_underfloor_state_snapshot() const {
	static const char *NAMES[5] = { "front_left", "front_right", "center", "diffuser_throat", "diffuser_exit" };
	Dictionary out;
	out["schema_version"] = 1;
	Dictionary clearances;
	for (int i = 0; i < 5; ++i) {
		clearances[String(NAMES[i])] = underfloor_clearance_m_[i];
	}
	out["clearance_m"] = clearances;
	out["valid_mask"] = (int64_t)underfloor_valid_mask_;
	out["scrape_phase"] = underfloor_scrape_phase_;
	out["minimum_clearance_m"] = underfloor_min_clearance_m_;
	out["rake_rad"] = underfloor_rake_rad_;
	out["roll_rad"] = underfloor_roll_rad_;
	out["contact_confidence"] = underfloor_contact_confidence_;
	out["scrape_intensity"] = underfloor_scrape_intensity_;
	out["audio_scrape_gain"] = audio_scrape_gain_;
	out["audio_scrape_pitch"] = audio_scrape_pitch_;
	out["audio_scrape_cursor"] = audio_scrape_cursor_;
	Dictionary compression, closing_speed, force, bottoming_phase;
	for (int i = 0; i < 5; ++i) {
		const String name(NAMES[i]);
		compression[name] = underfloor_compression_m_[i];
		closing_speed[name] = underfloor_closing_speed_m_s_[i];
		force[name] = underfloor_normal_force_n_[i];
		bottoming_phase[name] = underfloor_bottoming_phase_[i];
	}
	out["compression_m"] = compression;
	out["closing_speed_m_s"] = closing_speed;
	out["normal_force_n"] = force;
	out["bottoming_phase"] = bottoming_phase;
	out["active_probe_mask"] = (int64_t)underfloor_active_probe_mask_;
	out["total_normal_force_n"] = underfloor_total_normal_force_n_;
	out["max_probe_force_n"] = underfloor_max_probe_force_n_;
	out["force_center_local"] = Vector3(underfloor_force_center_local_[0], underfloor_force_center_local_[1], underfloor_force_center_local_[2]);
	out["bottoming_torque_nm"] = Vector3(underfloor_bottoming_torque_[0], underfloor_bottoming_torque_[1], underfloor_bottoming_torque_[2]);
	out["dissipated_energy_j"] = underfloor_dissipated_energy_j_;
	out["rigid_contact_blend"] = underfloor_rigid_contact_blend_;
	Dictionary aero;
	static const char *AERO_NAMES[17] = {
		"total_downforce_n", "raw_downforce_n", "front_downforce_n", "floor_downforce_n",
		"rear_downforce_n", "drag_n", "front_wing_angle_deg", "rear_wing_angle_deg",
		"front_wing_cl", "rear_wing_cl", "floor_height_factor", "floor_rake_factor",
		"floor_seal_factor", "diffuser_stall_factor",
		"global_limit_factor", "load_ratio", "balance_front"
	};
	for (int i = 0; i < 17; ++i) {
		aero[String(AERO_NAMES[i])] = aero_telemetry_[i];
	}
	out["aero"] = aero;
	return out;
}

void F194RustVehicle::set_core_underfloor_telemetry(const F90CoreFrameOut &p_frame) {
	for (int i = 0; i < 5; ++i) {
		underfloor_clearance_m_[i] = p_frame.underfloor_clearance_m[i];
	}
	underfloor_valid_mask_ = p_frame.underfloor_valid_mask;
	underfloor_scrape_phase_ = p_frame.underfloor_scrape_phase;
	underfloor_min_clearance_m_ = p_frame.underfloor_min_clearance_m;
	underfloor_rake_rad_ = p_frame.underfloor_rake_rad;
	underfloor_roll_rad_ = p_frame.underfloor_roll_rad;
	underfloor_contact_confidence_ = p_frame.underfloor_contact_confidence;
	underfloor_scrape_intensity_ = p_frame.underfloor_scrape_intensity;
	audio_scrape_gain_ = p_frame.audio_scrape_gain;
	audio_scrape_pitch_ = p_frame.audio_scrape_pitch;
	audio_scrape_cursor_ = p_frame.audio_scrape_cursor;
	for (int i = 0; i < 5; ++i) {
		underfloor_compression_m_[i] = p_frame.underfloor_compression_m[i];
		underfloor_closing_speed_m_s_[i] = p_frame.underfloor_closing_speed_m_s[i];
		underfloor_normal_force_n_[i] = p_frame.underfloor_normal_force_n[i];
		underfloor_bottoming_phase_[i] = p_frame.underfloor_bottoming_phase[i];
	}
	underfloor_active_probe_mask_ = p_frame.underfloor_active_probe_mask;
	underfloor_total_normal_force_n_ = p_frame.underfloor_total_normal_force_n;
	underfloor_max_probe_force_n_ = p_frame.underfloor_max_probe_force_n;
	for (int i = 0; i < 3; ++i) {
		underfloor_force_center_local_[i] = p_frame.underfloor_force_center_local[i];
		underfloor_bottoming_torque_[i] = p_frame.underfloor_bottoming_torque[i];
	}
	underfloor_dissipated_energy_j_ = p_frame.underfloor_dissipated_energy_j;
	underfloor_rigid_contact_blend_ = p_frame.underfloor_rigid_contact_blend;
	const double aero_values[17] = {
		p_frame.aero_total_downforce_n, p_frame.aero_raw_downforce_n,
		p_frame.aero_front_downforce_n, p_frame.aero_floor_downforce_n,
		p_frame.aero_rear_downforce_n, p_frame.aero_drag_n,
		p_frame.aero_front_wing_angle_deg, p_frame.aero_rear_wing_angle_deg,
		p_frame.aero_front_wing_cl, p_frame.aero_rear_wing_cl,
		p_frame.aero_floor_height_factor, p_frame.aero_floor_rake_factor,
		p_frame.aero_floor_seal_factor, p_frame.aero_diffuser_stall_factor,
		p_frame.aero_global_limit_factor, p_frame.aero_load_ratio,
		p_frame.aero_balance_front
	};
	for (int i = 0; i < 17; ++i) {
		aero_telemetry_[i] = aero_values[i];
	}
}

void F194RustVehicle::set_core_brake_energy_telemetry(
	const double torque_nm[4],
	const double spin_pre_rad_s[4],
	const double spin_post_rad_s[4],
	const double power_w[4],
	const double energy_j[4]) {
	for (int i = 0; i < 4; ++i) {
		brake_torque_nm_[i] = torque_nm[i];
		brake_spin_pre_rad_s_[i] = spin_pre_rad_s[i];
		brake_spin_post_rad_s_[i] = spin_post_rad_s[i];
		brake_power_w_[i] = power_w[i];
		brake_energy_j_[i] = energy_j[i];
	}
}

void F194RustVehicle::set_core_brake_cooling_telemetry(
	const double natural_cooling_w_k[4],
	const double speed_cooling_w_k[4]) {
	for (int i = 0; i < 4; ++i) {
		brake_natural_cooling_w_k_[i] = natural_cooling_w_k[i];
		brake_speed_cooling_w_k_[i] = speed_cooling_w_k[i];
	}
}

void F194RustVehicle::apply_core_telemetry(const CSimTelemetry &p_telemetry, double p_dt) {
	speed_kmh_ = p_telemetry.speed_kmh;
	speed_ms_ = p_telemetry.speed_kmh / 3.6;
	motor_rpm_ = p_telemetry.rpm;
	current_gear_ = (int)p_telemetry.gear;
	true_steering_amount_ = p_telemetry.steer;
	steer_angle_rad_ = p_telemetry.steer * max_steering_angle_;
	lat_g_ = p_telemetry.lat_g;
	long_g_ = p_telemetry.long_g;
	vert_g_ = p_telemetry.vert_g;

	wheel_compressions_[0] = p_telemetry.fl_comp_mm;
	wheel_compressions_[1] = p_telemetry.fr_comp_mm;
	wheel_compressions_[2] = p_telemetry.rl_comp_mm;
	wheel_compressions_[3] = p_telemetry.rr_comp_mm;

	wheel_slips_[0] = p_telemetry.front_slip;
	wheel_slips_[1] = p_telemetry.front_slip;
	wheel_slips_[2] = p_telemetry.rear_slip;
	wheel_slips_[3] = p_telemetry.rear_slip;

	// Rolling spin approximation (core telemetry lacks per-wheel spin): derive from speed.
	double roll = (speed_ms_ / 0.33) * p_dt;
	wheel_spins_[0] += roll;
	wheel_spins_[1] += roll;
	wheel_spins_[2] += roll;
	wheel_spins_[3] += roll;

	wheel_drive_torques_[2] = p_telemetry.drive_torque;
	wheel_drive_torques_[3] = p_telemetry.drive_torque;

	update_wheel_visuals(p_dt);
}

void F194RustVehicle::set_core_powertrain_telemetry(const F90CoreFrameOut &p_frame) {
	for (int i = 0; i < 4; ++i) {
		wheel_drive_torques_[i] = p_frame.wheel_drive_torque_nm[i];
	}
	for (int i = 0; i < 4; ++i) {
		wheel_drive_torques_pre_tc_[i] = p_frame.wheel_drive_torque_pre_tc_nm[i];
		tc_slip_ratio_[i] = p_frame.tc_slip_ratio[i];
	}
	tc_enabled_ = p_frame.tc_enabled > 0.5;
	tc_eligible_ = p_frame.tc_eligible > 0.5;
	tc_intervening_ = p_frame.tc_active > 0.5;
	tc_gear_authority_ = p_frame.tc_gear_authority;
	tc_slip_target_ = p_frame.tc_slip_target;
	tc_raw_cut_ratio_ = p_frame.tc_raw_cut_ratio;
	tc_cut_ratio_ = p_frame.tc_cut_ratio;
	pre_tc_drive_power_w_ = p_frame.pre_tc_drive_power_w;
	net_drive_power_w_ = p_frame.net_drive_power_w;
}

} // namespace godot
