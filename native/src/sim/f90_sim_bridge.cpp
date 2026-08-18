#include "formula90s/sim/f90_sim_bridge.hpp"
#include "formula90s/vehicle/f1_94_rust_vehicle.hpp"

#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/classes/input.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <godot_cpp/variant/basis.hpp>
#include <godot_cpp/variant/transform3d.hpp>
#include <godot_cpp/variant/vector3.hpp>
#include <godot_cpp/variant/string_name.hpp>
#include <godot_cpp/variant/array.hpp>
#include <godot_cpp/variant/string.hpp>
#include <godot_cpp/classes/scene_tree.hpp>
#include <cmath>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

namespace godot {

void F90SimBridge::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_fixed_dt", "v"), &F90SimBridge::set_fixed_dt);
	ClassDB::bind_method(D_METHOD("get_fixed_dt"), &F90SimBridge::get_fixed_dt);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "fixed_dt"), "set_fixed_dt", "get_fixed_dt");

	ClassDB::bind_method(D_METHOD("set_config_json_path", "p"), &F90SimBridge::set_config_json_path);
	ClassDB::bind_method(D_METHOD("get_config_json_path"), &F90SimBridge::get_config_json_path);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "config_json_path"), "set_config_json_path", "get_config_json_path");

	ClassDB::bind_method(D_METHOD("set_use_canonical_config", "v"), &F90SimBridge::set_use_canonical_config);
	ClassDB::bind_method(D_METHOD("get_use_canonical_config"), &F90SimBridge::get_use_canonical_config);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "use_canonical_config"), "set_use_canonical_config", "get_use_canonical_config");

	ClassDB::bind_method(D_METHOD("set_target_vehicle_path", "p"), &F90SimBridge::set_target_vehicle_path);
	ClassDB::bind_method(D_METHOD("get_target_vehicle_path"), &F90SimBridge::get_target_vehicle_path);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "target_vehicle_path"), "set_target_vehicle_path", "get_target_vehicle_path");

	ClassDB::bind_method(D_METHOD("set_debug_throttle", "v"), &F90SimBridge::set_debug_throttle);
	ClassDB::bind_method(D_METHOD("get_debug_throttle"), &F90SimBridge::get_debug_throttle);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "debug_throttle", PROPERTY_HINT_RANGE, "0,1,0.01"), "set_debug_throttle", "get_debug_throttle");
}

F90SimBridge::F90SimBridge() {}
F90SimBridge::~F90SimBridge() { unload_dll(); }

bool F90SimBridge::load_dll() {
	if (dll_handle_ != nullptr) {
		return true;
	}
#ifdef _WIN32
	Array candidates;
	candidates.append("res://addons/formula90s/bin/game_sim.windows.template_release.x86_64.dll");
	candidates.append("res://addons/formula90s/bin/game_sim.windows.template_debug.x86_64.dll");
	candidates.append("res://addons/formula90s/bin/game_sim.dll");
	candidates.append("game/addons/formula90s/bin/game_sim.windows.template_release.x86_64.dll");
	candidates.append("game/addons/formula90s/bin/game_sim.windows.template_debug.x86_64.dll");
	candidates.append("game/addons/formula90s/bin/game_sim.dll");
	candidates.append("game_sim.dll");

	HMODULE hDll = nullptr;
	ProjectSettings *ps = ProjectSettings::get_singleton();
	String loaded;
	for (int i = 0; i < candidates.size(); ++i) {
		String p = candidates[i];
		String gp = ps ? ps->globalize_path(p) : p;
		hDll = LoadLibraryW((LPCWSTR)gp.utf16().get_data());
		if (hDll) {
			loaded = gp;
			break;
		}
	}
	if (!hDll) {
		UtilityFunctions::printerr("[F90SimBridge] Failed to load game_sim.dll from all candidates!");
		return false;
	}
	dll_handle_ = (void *)hDll;

	fn_create_ = (FnSimWorldCreate)GetProcAddress(hDll, "sim_world_create");
	fn_destroy_ = (FnSimWorldDestroy)GetProcAddress(hDll, "sim_world_destroy");
	fn_spawn_json_ = (FnSimWorldSpawnFromJson)GetProcAddress(hDll, "sim_world_spawn_from_json");
	fn_spawn_canonical_ = (FnSimWorldSpawnCanonical)GetProcAddress(hDll, "sim_world_spawn_canonical");
	fn_set_input_ = (FnSimWorldSetInput)GetProcAddress(hDll, "sim_world_set_input");
	fn_step_ = (FnSimWorldStep)GetProcAddress(hDll, "sim_world_step");
	fn_pose_ = (FnSimWorldPose)GetProcAddress(hDll, "sim_world_pose");
	fn_telemetry_ = (FnSimWorldTelemetry)GetProcAddress(hDll, "sim_world_telemetry");
	fn_flat_samples_ = (FnSimWorldFlatSamples)GetProcAddress(hDll, "sim_world_flat_samples");
	fn_set_pose_ = (FnSimWorldSetPose)GetProcAddress(hDll, "sim_world_set_pose");
	fn_set_pose_and_velocity_ = (FnSimWorldSetPoseAndVelocity)GetProcAddress(hDll, "sim_world_set_pose_and_velocity");
	fn_step_with_samples_ = (FnSimWorldStepWithSamples)GetProcAddress(hDll, "sim_world_step_with_samples");
	fn_solve_external_ = (FnSimWorldSolveExternal)GetProcAddress(hDll, "sim_world_solve_external");

	if (!fn_create_ || !fn_destroy_ || !fn_step_ || !fn_pose_ || !fn_telemetry_ || !fn_set_input_) {
		UtilityFunctions::printerr("[F90SimBridge] Missing required symbols in game_sim.dll!");
		unload_dll();
		return false;
	}
	UtilityFunctions::print("[F90SimBridge] loaded game_sim.dll from " + loaded);
	return true;
#else
	return false;
#endif
}

void F90SimBridge::unload_dll() {
	if (world_ && fn_destroy_) {
		fn_destroy_(world_);
		world_ = nullptr;
	}
	entity_id_ = 0;
#ifdef _WIN32
	if (dll_handle_) {
		FreeLibrary((HMODULE)dll_handle_);
		dll_handle_ = nullptr;
	}
#endif
}

void F90SimBridge::spawn_vehicle() {
	if (use_canonical_config_) {
		entity_id_ = fn_spawn_canonical_(world_);
		return;
	}
	String gp = config_json_path_;
	ProjectSettings *ps = ProjectSettings::get_singleton();
	if (ps && gp.begins_with("res://")) {
		gp = ps->globalize_path(gp);
	}
	entity_id_ = fn_spawn_json_(world_, gp.utf8().get_data());
	if (entity_id_ == 0) {
		UtilityFunctions::printerr("[F90SimBridge] spawn failed for " + gp);
	}
}

void F90SimBridge::_ready() {
	if (!load_dll()) {
		return;
	}
	world_ = fn_create_(fixed_dt_);
	if (!world_) {
		UtilityFunctions::printerr("[F90SimBridge] sim_world_create failed");
		return;
	}
	spawn_vehicle();
}

void F90SimBridge::_physics_process(double delta) {
	if (!world_ || entity_id_ == 0) {
		return;
	}

	// Resolve the target vehicle: explicit path if set, otherwise auto-discover the
	// first F194RustVehicle in the tree (robust to dynamic/instanced scenes).
	F194RustVehicle *veh = nullptr;
	if (!target_vehicle_path_.is_empty()) {
		Node *node = get_node_or_null(target_vehicle_path_);
		veh = Object::cast_to<F194RustVehicle>(node);
	} else {
		if (cached_veh_ == nullptr) {
			Node *root = get_node<Node>(NodePath("/root"));
			cached_veh_ = find_first_vehicle(root);
		}
		veh = cached_veh_;
	}

	if (veh != nullptr && world_ != nullptr && fn_solve_external_ != nullptr &&
		fn_telemetry_ != nullptr) {
		veh->set_bridge_controlled(true);
		veh->set_sim_bridge(this);
		return;
	}

	// Otherwise: default demo — this node reflects the core's pose itself.
	drive_self(delta);
}

F194RustVehicle *F90SimBridge::find_first_vehicle(Node *p_from) {
	if (p_from == nullptr) {
		return nullptr;
	}
	F194RustVehicle *v = Object::cast_to<F194RustVehicle>(p_from);
	if (v != nullptr) {
		return v;
	}
	for (int i = 0; i < p_from->get_child_count(); ++i) {
		F194RustVehicle *r = find_first_vehicle(p_from->get_child(i));
		if (r != nullptr) {
			return r;
		}
	}
	return nullptr;
}

void F90SimBridge::drive_self(double delta) {
	double throttle = 0.0, brake = 0.0, steer = 0.0, handbrake = 0.0;
	Input *in = Input::get_singleton();
	if (in != nullptr) {
		throttle = in->get_action_strength(StringName("Throttle"));
		brake = in->get_action_strength(StringName("Brakes"));
		steer = in->get_action_strength(StringName("Steer Right")) - in->get_action_strength(StringName("Steer Left"));
		handbrake = in->get_action_strength(StringName("Handbrake"));
	}
	if (debug_throttle_ > 0.0) {
		throttle = debug_throttle_;
	}

	fn_set_input_(world_, entity_id_, throttle, brake, steer, handbrake, 0.0, 0, false);
	fn_step_(world_);

	CSimPose pose;
	fn_pose_(world_, entity_id_, &pose);
	Basis basis = Basis::from_euler(Vector3(0.0, pose.yaw, 0.0));
	Transform3D t(basis, Vector3(pose.x, pose.y, pose.z));
	set_global_transform(t);

	telemetry_print_accum_ += delta;
	if (telemetry_print_accum_ >= 0.5) {
		telemetry_print_accum_ = 0.0;
		CSimTelemetry tel;
		fn_telemetry_(world_, entity_id_, &tel);
		UtilityFunctions::print(String("[F90SimBridge] v=") + String::num(tel.speed_kmh, 1) +
			" km/h rpm=" + String::num(tel.rpm, 0) + " gear=" + String::num(tel.gear, 0) +
			" RSlip=" + String::num(tel.rear_slip, 3) + " TC=" + String::num(tel.tc_active, 0));
	}
}

void F90SimBridge::drive_integrate(F194RustVehicle *veh, PhysicsDirectBodyState3D *state) {
	if (world_ == nullptr || entity_id_ == 0) {
		return;
	}
	if (fn_solve_external_ == nullptr || fn_telemetry_ == nullptr) {
		return;
	}

	// 1. Input from the vehicle's control fields. These are populated every
	//    _physics_process by F194RustInputController (GDScript) straight from the
	//    engine's InputMap, using the correct left-right steering sign. Reading them
	//    here reuses the proven input path instead of querying action strength from
	//    inside the physics integration callback.
	double throttle = veh->get_throttle_amount();
	double brake = veh->get_brake_amount();
	double steer = veh->get_steering_input();
	double handbrake = veh->get_handbrake_amount();
	if (debug_throttle_ > 0.0) {
		throttle = debug_throttle_;
	}

	// 2. Sample the vehicle's real raycasts (12 RayCast3D children). In this integrate
	//    context force_raycast_update() is guaranteed to reflect the current physics state.
	CSimTriRaycastSample samples[4];
	veh->collect_core_samples(samples);

	// 3. Body kinematics (collision-resolved transform + velocity), passed to the core so
	//    it solves forces in the body's exact frame. Godot then integrates (gravity +
	//    collisions) — the same stable path the legacy vehicle_physics_engine DLL uses.
	Transform3D gt = state->get_transform();
	Vector3 old_pos = gt.origin;
	Vector3 fwd = gt.basis.xform(Vector3(0.0, 0.0, -1.0));
	double yaw = std::atan2(fwd.x, -fwd.z);
	Vector3 old_lin = state->get_linear_velocity();
	Vector3 old_ang = state->get_angular_velocity();
	double dt = (double)state->get_step();

	// 4. Solve the core's forces (suspension/tire/drivetrain/aero) for this exact body
	//    state and real samples. The core excludes gravity (left to Godot). Output is
	//    world-space force + torque; Godot applies it and integrates the rigid body.
	double force_out[3] = {0.0, 0.0, 0.0};
	double torque_out[3] = {0.0, 0.0, 0.0};
	fn_solve_external_(world_, entity_id_, old_pos.x, old_pos.y, old_pos.z, yaw,
		old_lin.x, old_lin.y, old_lin.z, old_ang.x, old_ang.y, old_ang.z,
		throttle, brake, steer, handbrake, 0.0, 0, false, dt, samples, force_out, torque_out);

	Vector3 force(force_out[0], force_out[1], force_out[2]);
	Vector3 torque(torque_out[0], torque_out[1], torque_out[2]);
	if (force.is_finite()) {
		state->apply_central_force(force);
	}
	if (torque.is_finite()) {
		state->apply_torque(torque);
	}

	// 5. Telemetry + wheel visuals.
	CSimTelemetry tel;
	fn_telemetry_(world_, entity_id_, &tel);
	veh->apply_core_telemetry(tel, dt);

	telemetry_print_accum_ += dt;
	if (telemetry_print_accum_ >= 0.5) {
		telemetry_print_accum_ = 0.0;
		UtilityFunctions::print(String("[F90SimBridge->Vehicle] v=") + String::num(tel.speed_kmh, 1) +
			" km/h rpm=" + String::num(tel.rpm, 0) + " gear=" + String::num(tel.gear, 0) +
			" RSlip=" + String::num(tel.rear_slip, 3) + " TC=" + String::num(tel.tc_active, 0) +
			" posY=" + String::num(old_pos.y, 3));
	}
}

void F90SimBridge::_exit_tree() { unload_dll(); }

} // namespace godot
