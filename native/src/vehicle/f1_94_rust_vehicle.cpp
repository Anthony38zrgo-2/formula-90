#include "formula90s/vehicle/f1_94_rust_vehicle.hpp"

#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/variant/utility_functions.hpp>

#include <cmath>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

namespace godot {

void F194RustVehicle::_bind_methods() {
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
	ClassDB::bind_method(D_METHOD("get_true_steering_amount"), &F194RustVehicle::get_true_steering_amount);
	ClassDB::bind_method(D_METHOD("get_steer_angle_rad"), &F194RustVehicle::get_steer_angle_rad);

	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "speed"), "", "get_speed");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "speed_kmh"), "", "get_speed_kmh");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "motor_rpm"), "", "get_motor_rpm");
	ADD_PROPERTY(PropertyInfo(Variant::INT, "current_gear"), "", "get_current_gear");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "engine_torque"), "", "get_engine_torque");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "clutch_engagement"), "", "get_clutch_engagement");
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

	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "wheel_compressions"), "", "get_wheel_compressions");
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "wheel_spins"), "", "get_wheel_spins");
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "wheel_slips"), "", "get_wheel_slips");

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

	ClassDB::bind_method(D_METHOD("reset_vehicle", "pos", "yaw_rad"), &F194RustVehicle::reset_vehicle);
	ClassDB::bind_method(D_METHOD("solve_forces_for_state", "state"), &F194RustVehicle::solve_forces_for_state);
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

	for (int i = 0; i < candidate_paths.size(); ++i) {
		String p = candidate_paths[i];
		String global_p = ps ? ps->globalize_path(p) : p;
		hDll = LoadLibraryW((LPCWSTR)global_p.utf16().get_data());
		if (hDll) {
			break;
		}
	}

	if (!hDll) {
		UtilityFunctions::printerr("[F194RustVehicle] Failed to load vehicle_physics_engine.dll from all candidates!");
		return false;
	}

	dll_handle_ = (void *)hDll;

	fn_create_default_ = (FnPhysicsCreateDefault)GetProcAddress(hDll, "f1_94_physics_create_default");
	fn_create_with_pos_ = (FnPhysicsCreateWithPos)GetProcAddress(hDll, "f1_94_physics_create_with_pos");
	fn_reset_ = (FnPhysicsReset)GetProcAddress(hDll, "f1_94_physics_reset");
	fn_solve_forces_ = (FnPhysicsSolveForces)GetProcAddress(hDll, "f1_94_physics_solve_forces");
	fn_step_ = (FnPhysicsStep)GetProcAddress(hDll, "f1_94_physics_step");
	fn_get_anchor_ = (FnPhysicsGetWheelAnchorLocal)GetProcAddress(hDll, "f1_94_physics_get_wheel_anchor_local");
	fn_get_tri_span_ = (FnPhysicsGetTriRaySpan)GetProcAddress(hDll, "f1_94_physics_get_tri_ray_span");
	fn_get_ray_length_ = (FnPhysicsGetRayLength)GetProcAddress(hDll, "f1_94_physics_get_ray_length");
	fn_get_vehicle_mass_ = (FnPhysicsGetVehicleMass)GetProcAddress(hDll, "f1_94_physics_get_vehicle_mass");
	fn_get_default_spawn_height_ = (FnPhysicsGetDefaultSpawnHeight)GetProcAddress(hDll, "f1_94_physics_get_default_spawn_height");
	fn_get_center_of_mass_local_ = (FnPhysicsGetCenterOfMassLocal)GetProcAddress(hDll, "f1_94_physics_get_center_of_mass_local");
	fn_destroy_ = (FnPhysicsDestroy)GetProcAddress(hDll, "f1_94_physics_destroy");

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
				Vector3(-0.79625, 0.0, -1.60636),
				Vector3(0.79625, 0.0, -1.60636),
				Vector3(-0.80000, 0.0, 1.31364),
				Vector3(0.80000, 0.0, 1.31364)
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

	if (fn_create_default_) {
		sim_ptr_ = fn_create_default_();
	} else if (fn_create_with_pos_) {
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

	// 2. Obtain mass from Rust
	if (fn_get_vehicle_mass_) {
		double mass = fn_get_vehicle_mass_(sim_ptr_);
		set_mass((float)mass);
	}

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

uint32_t F194RustVehicle::detect_surface_type(const RayCast3D *ray) {
	if (!ray || !ray->is_colliding()) {
		return 0; // Road
	}
	Object *collider = ray->get_collider();
	if (!collider) {
		return 0;
	}
	Node *node = Object::cast_to<Node>(collider);
	if (node) {
		if (node->is_in_group("Curb") || node->is_in_group("curb") || node->is_in_group("Kerb") || node->is_in_group("kerb")) return 1;
		if (node->is_in_group("Dirt") || node->is_in_group("dirt")) return 2;
		if (node->is_in_group("Grass") || node->is_in_group("grass") || node->is_in_group("Cesped") || node->is_in_group("cesped")) return 3;
		if (node->is_in_group("Gravel") || node->is_in_group("gravel") || node->is_in_group("Grava") || node->is_in_group("grava")) return 4;
		if (node->is_in_group("Sand") || node->is_in_group("sand") || node->is_in_group("Arena") || node->is_in_group("arena")) return 5;
		if (node->is_in_group("Wall") || node->is_in_group("wall") || node->is_in_group("Barrier") || node->is_in_group("barrier") || node->is_in_group("Guardrail") || node->is_in_group("guardrail")) return 6;
		if (node->is_in_group("Metal") || node->is_in_group("metal")) return 7;
		if (node->is_in_group("Road") || node->is_in_group("road") || node->is_in_group("Track") || node->is_in_group("track") || node->is_in_group("Asphalt") || node->is_in_group("asphalt")) return 0;

		// Fallback check by name
		String name = node->get_name().to_lower();
		if (name.contains("curb") || name.contains("kerb") || name.contains("piano")) return 1;
		if (name.contains("dirt")) return 2;
		if (name.contains("grass") || name.contains("cesped")) return 3;
		if (name.contains("gravel") || name.contains("grava")) return 4;
		if (name.contains("sand") || name.contains("arena")) return 5;
		if (name.contains("wall") || name.contains("guardrail") || name.contains("barrier")) return 6;
		if (name.contains("metal")) return 7;
	}
	return 0; // Default Road
}

void F194RustVehicle::_integrate_forces(PhysicsDirectBodyState3D *p_state) {
	solve_forces_for_state(p_state);
}

void F194RustVehicle::solve_forces_for_state(PhysicsDirectBodyState3D *p_state) {
	if (Engine::get_singleton()->is_editor_hint() || !sim_ptr_ || !fn_solve_forces_ || !p_state) {
		return;
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

	// 2. Gather Driver Inputs
	F90VehicleInput input = {};
	if (enable_player_input_) {
		Input *inp = Input::get_singleton();
		input.throttle = inp ? inp->get_action_strength("Throttle") : 0.0;
		input.steering = inp ? (inp->get_action_strength("Steer Right") - inp->get_action_strength("Steer Left")) : 0.0;
		input.brake = inp ? inp->get_action_strength("Brakes") : 0.0;
		input.handbrake = inp ? inp->get_action_strength("Handbrake") : 0.0;
		input.clutch = inp ? inp->get_action_strength("Clutch") : 0.0;
	} else {
		input.throttle = throttle_amount_;
		input.steering = steering_input_;
		input.brake = brake_amount_;
		input.handbrake = handbrake_amount_;
		input.clutch = clutch_amount_;
	}
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

		// Vertical suspension displacement (resting compression nominal ~100mm)
		double comp_m = std::isfinite(wheel_compressions_[w]) ? (wheel_compressions_[w] * 0.001) : 0.100;
		double rest_y = wheel_base_positions_[w].y;
		double visual_y = rest_y + (0.100 - comp_m);
		Vector3 pos = w_node->get_position();
		pos.y = visual_y;
		if (pos.is_finite()) {
			w_node->set_position(pos);
		}

		// Front steering and rolling rotations
		double steer = 0.0;
		if (w < 2) {
			steer = std::isfinite(steer_angle_rad_) ? steer_angle_rad_ : (true_steering_amount_ * 0.436332);
		}
		double angle = std::isfinite(wheel_angles_[w]) ? wheel_angles_[w] : 0.0;
		w_node->set_rotation(Vector3(angle, steer, 0.0));
	}
}

PackedFloat64Array F194RustVehicle::get_wheel_compressions() const {
	PackedFloat64Array arr;
	arr.resize(4);
	for (int i = 0; i < 4; ++i) arr[i] = wheel_compressions_[i];
	return arr;
}

PackedFloat64Array F194RustVehicle::get_wheel_spins() const {
	PackedFloat64Array arr;
	arr.resize(4);
	for (int i = 0; i < 4; ++i) arr[i] = wheel_spins_[i];
	return arr;
}

PackedFloat64Array F194RustVehicle::get_wheel_slips() const {
	PackedFloat64Array arr;
	arr.resize(4);
	for (int i = 0; i < 4; ++i) arr[i] = wheel_slips_[i];
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

void F194RustVehicle::reset_vehicle(const Vector3 &p_pos, double p_yaw_rad) {
	if (sim_ptr_ && fn_reset_) {
		fn_reset_(sim_ptr_, p_pos.x, p_pos.y, p_pos.z, p_yaw_rad);
	}
	Transform3D t(Basis(Vector3(0.0, 1.0, 0.0), p_yaw_rad), p_pos);
	set_global_transform(t);
	set_linear_velocity(Vector3());
	set_angular_velocity(Vector3());
	lin_vel_ = Vector3();
	ang_vel_ = Vector3();
}

void F194RustVehicle::_exit_tree() {
	unload_rust_dll();
}

} // namespace godot
