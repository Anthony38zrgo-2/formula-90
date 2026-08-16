#include "formula90s/vehicle/f1_94_rust_vehicle.hpp"

#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/variant/utility_functions.hpp>

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

	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "speed"), "", "get_speed");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "speed_kmh"), "", "get_speed_kmh");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "motor_rpm"), "", "get_motor_rpm");
	ADD_PROPERTY(PropertyInfo(Variant::INT, "current_gear"), "", "get_current_gear");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "engine_torque"), "", "get_engine_torque");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "clutch_engagement"), "", "get_clutch_engagement");
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "true_steering_amount"), "", "get_true_steering_amount");

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
}

F194RustVehicle::F194RustVehicle() {
	set_gravity_scale(0.0);
	set_mass(505.0);
	set_freeze_enabled(true);
	set_freeze_mode(FREEZE_MODE_KINEMATIC);
	set_collision_layer(2);
	set_collision_mask(0);

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
	if (dll_handle_ != nullptr && sim_ptr_ != nullptr) {
		return true;
	}

#ifdef _WIN32
	// List of candidate paths for the compiled Rust DLL
	Array candidate_paths;
	candidate_paths.append("res://addons/formula90s/bin/vehicle_physics_engine.windows.template_release.x86_64.dll");
	candidate_paths.append("res://addons/formula90s/bin/vehicle_physics_engine.windows.template_debug.x86_64.dll");
	candidate_paths.append("res://addons/formula90s/bin/vehicle_physics_engine.dll");
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

	fn_create_with_pos_ = (FnPhysicsCreateWithPos)GetProcAddress(hDll, "f1_94_physics_create_with_pos");
	fn_reset_ = (FnPhysicsReset)GetProcAddress(hDll, "f1_94_physics_reset");
	fn_step_ = (FnPhysicsStep)GetProcAddress(hDll, "f1_94_physics_step");
	fn_get_anchor_ = (FnPhysicsGetWheelAnchorLocal)GetProcAddress(hDll, "f1_94_physics_get_wheel_anchor_local");
	fn_get_tri_span_ = (FnPhysicsGetTriRaySpan)GetProcAddress(hDll, "f1_94_physics_get_tri_ray_span");
	fn_destroy_ = (FnPhysicsDestroy)GetProcAddress(hDll, "f1_94_physics_destroy");

	if (!fn_create_with_pos_ || !fn_step_ || !fn_destroy_) {
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
	// Base hub local positions for F1-94 at axle height (Y=0.0 matching Rust vehicle_config)
	wheel_base_positions_[0] = Vector3(-0.79625, 0.0, -1.60636); // FL
	wheel_base_positions_[1] = Vector3(0.79625, 0.0, -1.60636);  // FR
	wheel_base_positions_[2] = Vector3(-0.80000, 0.0, 1.31364);  // RL
	wheel_base_positions_[3] = Vector3(0.80000, 0.0, 1.31364);   // RR

	const char *w_names[4] = { "FL", "FR", "RL", "RR" };
	const char *r_names[3] = { "In", "Mid", "Out" };

	double front_span = 0.305 * 0.40; // ~0.122m
	double rear_span = 0.380 * 0.40;  // ~0.152m

	for (int w = 0; w < 4; ++w) {
		double span = (w < 2) ? front_span : rear_span;
		// Inner, Center, Outer offsets
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
			ray->set_target_position(Vector3(0.0, -0.80, 0.0)); // 800mm reach
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

void F194RustVehicle::_ready() {
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}

	if (!load_rust_dll()) {
		UtilityFunctions::printerr("[F194RustVehicle] GDExtension failed to initialize Rust physics DLL!");
		return;
	}

	Vector3 pos = get_global_position();
	Vector3 rot = get_global_rotation();
	double yaw = rot.y;

	sim_ptr_ = fn_create_with_pos_(pos.x, pos.y, pos.z, yaw);
	if (!sim_ptr_) {
		UtilityFunctions::printerr("[F194RustVehicle] Failed to create Rust VehicleSimulator instance!");
		return;
	}

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

void F194RustVehicle::_physics_process(double delta) {
	if (Engine::get_singleton()->is_editor_hint() || !sim_ptr_ || !fn_step_) {
		return;
	}

	// 1. Gather Driver Inputs
	FfiVehicleInput input = {};
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

	// 2. Sample 12 RayCast3Ds in Godot
	FfiTriRaycastSample samples[4] = {};
	for (int w = 0; w < 4; ++w) {
		RayCast3D *r_in = raycasts_[w][0];
		RayCast3D *r_mid = raycasts_[w][1];
		RayCast3D *r_out = raycasts_[w][2];

		// Inner
		samples[w].inner.normal_y = 1.0;
		samples[w].inner.distance = 0.65;
		if (r_in) {
			r_in->force_raycast_update();
			if (r_in->is_colliding()) {
				samples[w].inner.is_colliding = true;
				Vector3 pt = r_in->get_collision_point();
				Vector3 n = r_in->get_collision_normal();
				samples[w].inner.distance = (pt - r_in->get_global_position()).length();
				samples[w].inner.point_x = pt.x; samples[w].inner.point_y = pt.y; samples[w].inner.point_z = pt.z;
				if (n.is_finite() && n.length_squared() > 1e-4) {
					n.normalize();
					samples[w].inner.normal_x = n.x; samples[w].inner.normal_y = n.y; samples[w].inner.normal_z = n.z;
				}
				samples[w].inner.surface_type = detect_surface_type(r_in);
			}
		}

		// Center
		samples[w].center.normal_y = 1.0;
		samples[w].center.distance = 0.65;
		if (r_mid) {
			r_mid->force_raycast_update();
			if (r_mid->is_colliding()) {
				samples[w].center.is_colliding = true;
				Vector3 pt = r_mid->get_collision_point();
				Vector3 n = r_mid->get_collision_normal();
				samples[w].center.distance = (pt - r_mid->get_global_position()).length();
				samples[w].center.point_x = pt.x; samples[w].center.point_y = pt.y; samples[w].center.point_z = pt.z;
				if (n.is_finite() && n.length_squared() > 1e-4) {
					n.normalize();
					samples[w].center.normal_x = n.x; samples[w].center.normal_y = n.y; samples[w].center.normal_z = n.z;
				}
				samples[w].center.surface_type = detect_surface_type(r_mid);
			}
		}

		// Outer
		samples[w].outer.normal_y = 1.0;
		samples[w].outer.distance = 0.65;
		if (r_out) {
			r_out->force_raycast_update();
			if (r_out->is_colliding()) {
				samples[w].outer.is_colliding = true;
				Vector3 pt = r_out->get_collision_point();
				Vector3 n = r_out->get_collision_normal();
				samples[w].outer.distance = (pt - r_out->get_global_position()).length();
				samples[w].outer.point_x = pt.x; samples[w].outer.point_y = pt.y; samples[w].outer.point_z = pt.z;
				if (n.is_finite() && n.length_squared() > 1e-4) {
					n.normalize();
					samples[w].outer.normal_x = n.x; samples[w].outer.normal_y = n.y; samples[w].outer.normal_z = n.z;
				}
				samples[w].outer.surface_type = detect_surface_type(r_out);
			}
		}
	}

	// 3. Step Deterministic Rust Physics Core
	FfiTelemetryOutput telem;
	fn_step_(sim_ptr_, &input, samples, delta, &telem);

	// 4. Update Godot 6-DOF Transform & Velocities
	Vector3 new_pos(telem.pos_x, telem.pos_y, telem.pos_z);
	Quaternion new_rot(telem.rot_quat_x, telem.rot_quat_y, telem.rot_quat_z, telem.rot_quat_w);

	if (new_rot.is_finite() && new_rot.length_squared() > 1e-4) {
		new_rot.normalize();
		Transform3D t(Basis(new_rot), new_pos);
		if (t.is_finite()) {
			set_global_transform(t);
		}
	} else if (new_pos.is_finite()) {
		set_global_position(new_pos);
	}

	lin_vel_ = Vector3(telem.lin_vel_x, telem.lin_vel_y, telem.lin_vel_z);
	ang_vel_ = Vector3(telem.ang_vel_x, telem.ang_vel_y, telem.ang_vel_z);

	// 5. Store Telemetry Variables
	speed_kmh_ = telem.speed_kmh;
	speed_ms_ = telem.speed_kmh / 3.6;
	motor_rpm_ = telem.rpm;
	current_gear_ = telem.gear;
	engine_torque_ = telem.engine_torque;
	clutch_engagement_ = telem.clutch_engagement;
	true_steering_amount_ = telem.steer;

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

	// 6. Visual Animation of Wheel Meshes
	update_wheel_visuals(delta);
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

		// Vertical suspension displacement
		double comp_m = std::isfinite(wheel_compressions_[w]) ? (wheel_compressions_[w] * 0.001) : 0.100;
		double rest_y = wheel_base_positions_[w].y;
		double visual_y = rest_y + (0.100 - comp_m); // spring resting compression 100mm
		Vector3 pos = w_node->get_position();
		pos.y = visual_y;
		if (pos.is_finite()) {
			w_node->set_position(pos);
		}

		// Front steering and rolling rotations
		double steer = 0.0;
		if (w < 2 && std::isfinite(true_steering_amount_)) {
			steer = true_steering_amount_ * 0.436332;
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

void F194RustVehicle::reset_vehicle(const Vector3 &p_pos, double p_yaw_rad) {
	if (sim_ptr_ && fn_reset_) {
		fn_reset_(sim_ptr_, p_pos.x, p_pos.y, p_pos.z, p_yaw_rad);
	}
	set_global_position(p_pos);
	set_global_rotation(Vector3(0.0, p_yaw_rad, 0.0));
	lin_vel_ = Vector3();
	ang_vel_ = Vector3();
}

void F194RustVehicle::_exit_tree() {
	unload_rust_dll();
}

} // namespace godot
