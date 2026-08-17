#include "formula90s/sim/f90_sim_bridge.hpp"

#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/classes/input.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <godot_cpp/variant/basis.hpp>
#include <godot_cpp/variant/vector3.hpp>
#include <godot_cpp/variant/string_name.hpp>
#include <godot_cpp/variant/array.hpp>
#include <godot_cpp/variant/string.hpp>
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

	// Read the engine's InputMap with the same action names the game uses. Godot is
	// the input source; the core never reads input directly.
	double throttle = 0.0, brake = 0.0, steer = 0.0, handbrake = 0.0;
	Input *in = Input::get_singleton();
	if (in != nullptr) {
		throttle = in->get_action_strength(StringName("Throttle"));
		brake = in->get_action_strength(StringName("Brakes"));
		steer = in->get_action_strength(StringName("Steer Right")) - in->get_action_strength(StringName("Steer Left"));
		handbrake = in->get_action_strength(StringName("Handbrake"));
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

void F90SimBridge::_exit_tree() { unload_dll(); }

} // namespace godot
