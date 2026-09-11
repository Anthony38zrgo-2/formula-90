#include "formula90s/core/f90_core.hpp"
#include "formula90s/vehicle/f1_94_rust_vehicle.hpp"

#include <godot_cpp/classes/scene_tree.hpp>
#include <godot_cpp/classes/file_access.hpp>
#include <godot_cpp/classes/os.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <godot_cpp/variant/packed_vector2_array.hpp>
#include <godot_cpp/variant/quaternion.hpp>
#include <godot_cpp/variant/string.hpp>
#include <godot_cpp/classes/input.hpp>
#include <cmath>
#include <utility>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

using namespace godot;

void F90Core::_bind_methods() {
	// --- configuration ---------------------------------------------------------
	ClassDB::bind_method(D_METHOD("set_fixed_dt", "v"), &F90Core::set_fixed_dt);
	ClassDB::bind_method(D_METHOD("get_fixed_dt"), &F90Core::get_fixed_dt);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "fixed_dt"), "set_fixed_dt", "get_fixed_dt");

	ClassDB::bind_method(D_METHOD("set_config_json_path", "p"), &F90Core::set_config_json_path);
	ClassDB::bind_method(D_METHOD("get_config_json_path"), &F90Core::get_config_json_path);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "config_json_path"), "set_config_json_path", "get_config_json_path");

	ClassDB::bind_method(D_METHOD("set_use_canonical_config", "v"), &F90Core::set_use_canonical_config);
	ClassDB::bind_method(D_METHOD("get_use_canonical_config"), &F90Core::get_use_canonical_config);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "use_canonical_config"), "set_use_canonical_config", "get_use_canonical_config");

	ClassDB::bind_method(D_METHOD("set_target_vehicle_path", "p"), &F90Core::set_target_vehicle_path);
	ClassDB::bind_method(D_METHOD("get_target_vehicle_path"), &F90Core::get_target_vehicle_path);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "target_vehicle_path"), "set_target_vehicle_path", "get_target_vehicle_path");

	ClassDB::bind_method(D_METHOD("set_debug_throttle", "v"), &F90Core::set_debug_throttle);
	ClassDB::bind_method(D_METHOD("get_debug_throttle"), &F90Core::get_debug_throttle);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "debug_throttle", PROPERTY_HINT_RANGE, "0,1,0.01"), "set_debug_throttle", "get_debug_throttle");

	ClassDB::bind_method(D_METHOD("set_enable_audio", "v"), &F90Core::set_enable_audio);
	ClassDB::bind_method(D_METHOD("get_enable_audio"), &F90Core::get_enable_audio);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "enable_audio"), "set_enable_audio", "get_enable_audio");

	ClassDB::bind_method(D_METHOD("set_bank_dir", "p"), &F90Core::set_bank_dir);
	ClassDB::bind_method(D_METHOD("get_bank_dir"), &F90Core::get_bank_dir);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "bank_dir"), "set_bank_dir", "get_bank_dir");

	ClassDB::bind_method(D_METHOD("set_modules", "p"), &F90Core::set_modules);
	ClassDB::bind_method(D_METHOD("get_modules"), &F90Core::get_modules);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "modules"), "set_modules", "get_modules");

	ClassDB::bind_method(D_METHOD("set_idle_rpm", "v"), &F90Core::set_idle_rpm);
	ClassDB::bind_method(D_METHOD("get_idle_rpm"), &F90Core::get_idle_rpm);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "idle_rpm"), "set_idle_rpm", "get_idle_rpm");

	ClassDB::bind_method(D_METHOD("set_max_rpm", "v"), &F90Core::set_max_rpm);
	ClassDB::bind_method(D_METHOD("get_max_rpm"), &F90Core::get_max_rpm);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "max_rpm"), "set_max_rpm", "get_max_rpm");

	// --- readouts (telemetry HUD, same names as legacy VehicleAudioControllerNative)
	ClassDB::bind_method(D_METHOD("get_last_norm"), &F90Core::get_last_norm);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_norm"), "", "get_last_norm");
	ClassDB::bind_method(D_METHOD("get_last_rpm"), &F90Core::get_last_rpm);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_rpm"), "", "get_last_rpm");
	ClassDB::bind_method(D_METHOD("get_last_throttle"), &F90Core::get_last_throttle);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_throttle"), "", "get_last_throttle");
	ClassDB::bind_method(D_METHOD("get_last_slip"), &F90Core::get_last_slip);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_slip"), "", "get_last_slip");
	ClassDB::bind_method(D_METHOD("get_last_speed_kph"), &F90Core::get_last_speed_kph);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_speed_kph"), "", "get_last_speed_kph");
	ClassDB::bind_method(D_METHOD("get_last_engine_gain"), &F90Core::get_last_engine_gain);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_engine_gain"), "", "get_last_engine_gain");
	ClassDB::bind_method(D_METHOD("get_last_weights"), &F90Core::get_last_weights);
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT32_ARRAY, "last_weights"), "", "get_last_weights");
	ClassDB::bind_method(D_METHOD("get_last_pitches"), &F90Core::get_last_pitches);
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT32_ARRAY, "last_pitches"), "", "get_last_pitches");
	ClassDB::bind_method(D_METHOD("get_last_trigger"), &F90Core::get_last_trigger);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "last_trigger"), "", "get_last_trigger");
	ClassDB::bind_method(D_METHOD("get_surface"), &F90Core::get_surface);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "surface"), "", "get_surface");
	ClassDB::bind_method(D_METHOD("get_active_bed"), &F90Core::get_active_bed);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "active_bed"), "", "get_active_bed");
	ClassDB::bind_method(D_METHOD("get_engine_band_native_rpm"), &F90Core::get_engine_band_native_rpm);
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "engine_band_native_rpm"), "", "get_engine_band_native_rpm");
	ClassDB::bind_method(D_METHOD("is_engine_loaded"), &F90Core::is_engine_loaded);
	ClassDB::bind_method(D_METHOD("is_audio_active"), &F90Core::is_audio_active);

	// --- actions ----------------------------------------------------------------
	ClassDB::bind_method(D_METHOD("trigger", "name"), &F90Core::trigger);
	ClassDB::bind_method(D_METHOD("reset_vehicle"), &F90Core::reset_vehicle);
}

F90Core::F90Core() {}
F90Core::~F90Core() { unload_dll(); }

PackedFloat32Array F90Core::get_last_weights() const {
	PackedFloat32Array arr;
	arr.resize(5);
	for (int i = 0; i < 5; ++i) {
		arr.set(i, frame_.weights[i]);
	}
	return arr;
}

PackedFloat32Array F90Core::get_last_pitches() const {
	PackedFloat32Array arr;
	arr.resize(5);
	for (int i = 0; i < 5; ++i) {
		arr.set(i, frame_.pitches[i]);
	}
	return arr;
}

PackedFloat64Array F90Core::get_engine_band_native_rpm() const {
	PackedFloat64Array arr;
	arr.resize(5);
	constexpr double ENGINE_BAND_NATIVE_RPM[5] = { 3941.0, 7429.0, 8196.0, 5580.0, 7687.0 };
	for (int i = 0; i < 5; ++i) {
		arr.set(i, ENGINE_BAND_NATIVE_RPM[i]);
	}
	return arr;
}

const char *F90Core::trigger_name(int code) {
	switch (code) {
		case 0: return "shift_up";
		case 1: return "shift_down";
		case 2: return "engine_backfire";
		case 3: return "impact_hit_1";
		case 4: return "impact_hit_2";
		case 5: return "impact_hit_3";
		case 6: return "impact_hit_4";
		case 7: return "impact_barrier";
		case 8: return "impact_cone";
		case 9: return "engine_fire";
		case 10: return "scrape";
		default: return "";
	}
}

int F90Core::trigger_code(const String &name) {
	if (name == "shift_up") return 0;
	if (name == "shift_down") return 1;
	if (name == "engine_backfire") return 2;
	if (name == "impact_hit_1") return 3;
	if (name == "impact_hit_2") return 4;
	if (name == "impact_hit_3") return 5;
	if (name == "impact_hit_4") return 6;
	if (name == "impact_barrier") return 7;
	if (name == "impact_cone") return 8;
	if (name == "engine_fire") return 9;
	if (name == "scrape") return 10;
	return -1;
}

String F90Core::get_last_trigger() const {
	return String(trigger_name(frame_.trigger_code));
}

String F90Core::get_surface() const {
	switch (frame_.surface_code) {
		case 1: return "rumble";
		case 2: return "grass";
		case 3: return "sand";
		default: return "asphalt";
	}
}

String F90Core::get_active_bed() const {
	switch (frame_.active_bed_code) {
		case 1: return "surf_rumble";
		case 2: return "surf_grass";
		case 3: return "surf_sand";
		default: return "";
	}
}

void F90Core::trigger(const String &name) {
	const int code = trigger_code(name);
	if (code < 0) {
		return;
	}
	if (core_ && fn_audio_trigger_) {
		fn_audio_trigger_(core_, code);
		frame_.trigger_code = code;
	}
}

void F90Core::reset_vehicle() {
	if (!core_ || !fn_reset_) {
		return;
	}
	F194RustVehicle *veh = Object::cast_to<F194RustVehicle>(get_node_or_null(target_vehicle_path_));
	if (!veh) {
		veh = find_first_vehicle(get_node<Node>(NodePath("/root")));
	}
	if (!veh) {
		return;
	}
	const Transform3D t = veh->get_global_transform();
	Vector3 fwd = t.basis.xform(Vector3(0, 0, -1));
	const double yaw = std::atan2(fwd.x, -fwd.z);
	fn_reset_(core_, t.origin.x, t.origin.y, t.origin.z, yaw);
}

bool F90Core::load_dll() {
	const String build_source_path = "res://BUILD_SOURCE";
	if (!FileAccess::file_exists(build_source_path)) {
		UtilityFunctions::printerr("[F90Core] FATAL: res://BUILD_SOURCE is missing");
		return false;
	}
	Ref<FileAccess> build_source_file = FileAccess::open(build_source_path, FileAccess::READ);
	const String recorded_source_sha = build_source_file.is_valid()
		? build_source_file->get_as_text().strip_edges()
		: String();
	if (recorded_source_sha.length() != 40 || recorded_source_sha != String(FORMULA90_BUILD_SHA)) {
		UtilityFunctions::printerr(String("[F90Core] FATAL: native BUILD does not match recorded source. Native=") +
			String(FORMULA90_BUILD_SHA) + " recorded=" + recorded_source_sha);
		return false;
	}
	if (dll_handle_ != nullptr) {
		return true;
	}
#ifdef _WIN32
	Array candidates;
	candidates.append("res://addons/formula90s/bin/formula90_core.windows.template_release.x86_64.dll");
	candidates.append("res://addons/formula90s/bin/formula90_core.windows.template_debug.x86_64.dll");
	candidates.append("res://addons/formula90s/bin/formula90_core.dll");
	candidates.append("game/addons/formula90s/bin/formula90_core.windows.template_release.x86_64.dll");
	candidates.append("game/addons/formula90s/bin/formula90_core.windows.template_debug.x86_64.dll");
	candidates.append("game/addons/formula90s/bin/formula90_core.dll");
	candidates.append("formula90_core.dll");

	HMODULE hDll = nullptr;
	ProjectSettings *ps = ProjectSettings::get_singleton();
	String loaded_path = "";
	for (int i = 0; i < candidates.size(); ++i) {
		String p = candidates[i];
		String global_p = ps ? ps->globalize_path(p) : p;
		hDll = LoadLibraryW((LPCWSTR)global_p.utf16().get_data());
		if (hDll) {
			loaded_path = std::move(global_p);
			break;
		}
	}
	if (!hDll) {
		UtilityFunctions::printerr("[F90Core] Failed to load formula90_core.dll from all candidates!");
		return false;
	}
	dll_handle_ = (void *)hDll;

	fn_abi_version_ = (FnCoreAbiVersion)GetProcAddress(hDll, "f90_core_abi_version");
	fn_build_sha_ = (FnCoreBuildSha)GetProcAddress(hDll, "f90_core_build_sha");
	fn_create_ = (FnCoreCreate)GetProcAddress(hDll, "f90_core_create");
	fn_destroy_ = (FnCoreDestroy)GetProcAddress(hDll, "f90_core_destroy");
	fn_spawn_ = (FnCoreSpawn)GetProcAddress(hDll, "f90_core_spawn");
	fn_reset_ = (FnCoreReset)GetProcAddress(hDll, "f90_core_reset");
	fn_apply_runtime_config_ = (FnCoreApplyRuntimeConfig)GetProcAddress(hDll, "f90_core_apply_runtime_config");
	fn_step_ = (FnCoreStep)GetProcAddress(hDll, "f90_core_step");
	fn_audio_render_ = (FnCoreAudioRender)GetProcAddress(hDll, "f90_core_audio_render");
	fn_audio_trigger_ = (FnCoreAudioTrigger)GetProcAddress(hDll, "f90_core_audio_trigger");
	fn_audio_readouts_ = (FnCoreAudioReadouts)GetProcAddress(hDll, "f90_core_audio_readouts");
	fn_audio_set_ambient_ = (FnCoreAudioSetAmbient)GetProcAddress(hDll, "f90_core_audio_set_ambient");

	const uint32_t abi_ver = fn_abi_version_ ? fn_abi_version_() : 0;
	const String core_build_sha = fn_build_sha_ ? String(fn_build_sha_()) : String("unknown");
	UtilityFunctions::print(String("[F90Core]\nDLL=") + loaded_path + "\nABI=" + String::num_int64(abi_ver) +
		"\nEXPECTED=" + String::num_int64(EXPECTED_ABI_VERSION) + "\nBUILD=" + core_build_sha);

	if (abi_ver != EXPECTED_ABI_VERSION) {
		UtilityFunctions::printerr(String("[F90Core] FATAL: ABI mismatch! Expected ") +
			String::num(EXPECTED_ABI_VERSION) + " but loaded DLL has " + String::num(abi_ver));
		unload_dll();
		return false;
	}
	if (core_build_sha != recorded_source_sha || core_build_sha == "unknown") {
		UtilityFunctions::printerr(String("[F90Core] FATAL: BUILD mismatch. Expected ") +
			recorded_source_sha + " but loaded DLL has " + core_build_sha);
		unload_dll();
		return false;
	}
	if (!fn_create_ || !fn_destroy_ || !fn_spawn_ || !fn_step_) {
		UtilityFunctions::printerr("[F90Core] Missing required symbols in formula90_core.dll!");
		unload_dll();
		return false;
	}
	return true;
#else
	UtilityFunctions::printerr("[F90Core] Only Windows runtime is supported in this build.");
	return false;
#endif
}

void F90Core::unload_dll() {
	if (core_ && fn_destroy_) {
		fn_destroy_(core_);
		core_ = nullptr;
	}
	entity_id_ = 0;
#ifdef _WIN32
	if (dll_handle_) {
		FreeLibrary((HMODULE)dll_handle_);
		dll_handle_ = nullptr;
	}
#endif
	fn_abi_version_ = nullptr;
	fn_build_sha_ = nullptr;
	fn_create_ = nullptr;
	fn_destroy_ = nullptr;
	fn_spawn_ = nullptr;
	fn_reset_ = nullptr;
	fn_apply_runtime_config_ = nullptr;
	fn_step_ = nullptr;
	fn_audio_render_ = nullptr;
	fn_audio_trigger_ = nullptr;
	fn_audio_readouts_ = nullptr;
	fn_audio_set_ambient_ = nullptr;
}

static String json_escape(const String &s) {
	return s.replace("\\", "\\\\").replace("\"", "\\\"");
}

void F90Core::reset_core_at(double x, double y, double z, double yaw) {
	if (core_ && fn_reset_) {
		fn_reset_(core_, x, y, z, yaw);
	}
}

void F90Core::apply_runtime_config(const F90RuntimeConfig &cfg) {
	if (core_ && entity_id_ != 0 && fn_apply_runtime_config_) {
		fn_apply_runtime_config_(core_, entity_id_, &cfg);
	}
}

void F90Core::_ready() {
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}
	set_process(true);
	set_physics_process(true);
	if (!load_dll()) {
		return;
	}

	// --- build the single create() options for the facade -----------------------
	ProjectSettings *ps = ProjectSettings::get_singleton();
	String bank_res = enable_audio_ ? bank_dir_res_ : "";
	String bank_global = (ps && bank_res.begins_with("res://")) ? ps->globalize_path(bank_res) : bank_res;
	String cfg_global = (ps && config_json_path_.begins_with("res://")) ? ps->globalize_path(config_json_path_) : config_json_path_;

	String modules_json = "[";
	{
		PackedStringArray parts = modules_.split(",");
		for (int i = 0; i < parts.size(); ++i) {
			String m = parts[i].strip_edges();
			if (m.is_empty()) {
				continue;
			}
			if (modules_json.length() > 1) {
				modules_json += ", ";
			}
			modules_json += "\"" + json_escape(m) + "\"";
		}
	}
	modules_json += "]";

	const bool use_canonical = use_canonical_config_ || cfg_global.is_empty();
	UtilityFunctions::print(String("use_canonical=") + String(use_canonical ? "true" : "false") +
		" json=" + cfg_global + " bank=" + bank_global);
	String opts = String("{\"bank_dir\":\"") + json_escape(bank_global) + "\",\"config_json_path\":\"" +
		json_escape(cfg_global) + "\",\"use_canonical\":" + String(use_canonical ? "true" : "false") +
		",\"fixed_dt\":" + String::num(fixed_dt_, 10) + ",\"enable_audio\":" + String(enable_audio_ ? "true" : "false") +
		",\"idle_rpm\":" + String::num(idle_rpm_, 1) + ",\"max_rpm\":" + String::num(max_rpm_, 1) +
		",\"modules\":" + modules_json +
		",\"underfloor_contact\":{\"enabled\":true,\"approach_clearance_m\":0.020,\"activation_clearance_m\":0.008,\"release_clearance_m\":0.016,\"linear_rate_n_m\":450000.0,\"progressive_rate_n_m2\":40000000.0,\"damping_n_s_m\":12000.0,\"max_force_per_probe_n\":12000.0,\"rigid_contact_spring_scale\":0.25,\"normal_min_y\":0.55}}";

	CharString cs = opts.utf8();
	uint8_t err_buf[256] = { 0 };
	core_ = fn_create_(cs.get_data(), err_buf, sizeof(err_buf));
	if (!core_) {
		UtilityFunctions::printerr(String("[F90Core] f90_core_create failed: ") + String((const char *)err_buf));
		return;
	}
	entity_id_ = fn_spawn_(core_);
	if (entity_id_ == 0) {
		UtilityFunctions::printerr("[F90Core] f90_core_spawn failed");
		return;
	}

	// --- audio plumbing: bus + generator + player (batched push_buffer). ---------
	// This was previously omitted: audio nodes must exist for the pump in _process
	// to have a playback to fill.
	audio_initialized_ = false;
	if (enable_audio_) {
		ensure_vehicle_bus();
		create_audio_nodes();
	}
	UtilityFunctions::print(String("[F90Core] facade ready (entity=") + String::num(entity_id_) +
		", audio=" + String(enable_audio_ ? "on" : "off") +
		", audio_initialized=" + String(audio_initialized_ ? "true" : "false") +
		", modules=" + modules_json + ")");
}

void F90Core::_physics_process(double delta) {
	if (!core_ || entity_id_ == 0 || Engine::get_singleton()->is_editor_hint()) {
		return;
	}
	F194RustVehicle *veh = nullptr;
	if (!target_vehicle_path_.is_empty()) {
		veh = Object::cast_to<F194RustVehicle>(get_node_or_null(target_vehicle_path_));
	} else {
		if (cached_veh_ == nullptr) {
			cached_veh_ = find_first_vehicle(get_node<Node>(NodePath("/root")));
		}
		veh = cached_veh_;
	}
	if (veh == nullptr) {
		return;
	}
	veh->set_bridge_controlled(true);
	veh->set_core_driver(this);
}

F194RustVehicle *F90Core::find_first_vehicle(Node *from) {
	if (from == nullptr) {
		return nullptr;
	}
	F194RustVehicle *v = Object::cast_to<F194RustVehicle>(from);
	if (v != nullptr) {
		return v;
	}
	for (int i = 0; i < from->get_child_count(); ++i) {
		F194RustVehicle *r = find_first_vehicle(from->get_child(i));
		if (r != nullptr) {
			return r;
		}
	}
	return nullptr;
}

void F90Core::drive_integrate(F194RustVehicle *veh, PhysicsDirectBodyState3D *state) {
	if (core_ == nullptr || entity_id_ == 0 || fn_step_ == nullptr) {
		return;
	}

	// 1. Inputs are produced each frame by F194RustInputController (GDScript) on the
	//    vehicle — the same proven path the legacy bridge used. The FULL aids mask
	//    flows through (bit0=ABS, 1=TC, 2=stability, 3=slip assist, 4=countersteer,
	//    5=auto-clutch, 6=launch, 7=brake-assist); the facade applies it every step,
	//    so there is no frozen default and no spurious first-frame TC toggle.
	const double throttle = debug_throttle_ > 0.0 ? debug_throttle_ : veh->get_throttle_amount();
	const double brake = veh->get_brake_amount();
	const double steer = veh->get_steering_input();
	const double handbrake = veh->get_handbrake_amount();
	const double clutch = veh->get_clutch_amount();
	const int gear_req = veh->get_gear_request();
	const uint32_t aids_mask = (uint32_t)veh->get_aids_enabled_mask();

	// 2. Real raycasts in the integrate context (guaranteed fresh).
	CSimTriRaycastSample samples[4];
	veh->collect_core_samples(samples);
	F90UnderfloorSample underfloor = {};
	veh->collect_underfloor_sample(&underfloor, state);

	// 3. Body kinematics from the collision-resolved Godot body.
	const Transform3D gt = state->get_transform();
	const Quaternion q = gt.basis.get_rotation_quaternion();
	const Vector3 old_pos = gt.origin;
	const Vector3 old_lin = state->get_linear_velocity();
	const Vector3 old_ang = state->get_angular_velocity();
	const double dt = (double)state->get_step();

	// 4. One orchestrated tick: physics solve + module ticks + audio inputs, with
	//    the whole frame returned (no second telemetry call, no Godot round-trip).
	frame_ = {};
	fn_step_(core_, entity_id_,
		old_pos.x, old_pos.y, old_pos.z,
		q.x, q.y, q.z, q.w,
		old_lin.x, old_lin.y, old_lin.z,
		old_ang.x, old_ang.y, old_ang.z,
		throttle, brake, steer, handbrake, clutch, (int8_t)gear_req, aids_mask, dt,
		samples, &underfloor, &frame_);

	const Vector3 force(frame_.force_x, frame_.force_y, frame_.force_z);
	const Vector3 torque(frame_.torque_x, frame_.torque_y, frame_.torque_z);
	if (force.is_finite()) {
		state->apply_central_force(force);
	}
	if (torque.is_finite()) {
		state->apply_torque(torque);
	}

	// 5. Mirror telemetry onto the vehicle (same fields/wiring as the legacy bridge).
	CSimTelemetry tel;
	tel.speed_kmh = frame_.speed_kmh;
	tel.rpm = frame_.rpm;
	tel.gear = (double)frame_.gear;
	tel.steer = frame_.steer;
	tel.lat_g = frame_.lat_g;
	tel.long_g = frame_.long_g;
	tel.vert_g = frame_.vert_g;
	tel.fl_comp_mm = frame_.fl_comp_mm;
	tel.fr_comp_mm = frame_.fr_comp_mm;
	tel.rl_comp_mm = frame_.rl_comp_mm;
	tel.rr_comp_mm = frame_.rr_comp_mm;
	tel.front_slip = frame_.front_slip;
	tel.rear_slip = frame_.rear_slip;
	tel.tc_active = frame_.tc_active;
	tel.drive_torque = frame_.drive_torque;
	veh->apply_core_telemetry(tel, dt);
	veh->set_core_powertrain_telemetry(frame_);

	// Tire pressure + thermal telemetry rides the same frame (WheelIndex order
	// FL/FR/RL/RR); the vehicle mirrors it into the HUD snapshot Dictionary.
	veh->set_core_tire_telemetry(
		frame_.tire_pressure_kpa,
		frame_.tire_tread_inner_c,
		frame_.tire_tread_center_c,
		frame_.tire_tread_outer_c,
		frame_.tire_carcass_c,
		frame_.tire_gas_c);
	veh->set_core_brake_telemetry(
		frame_.brake_disc_c,
		frame_.brake_rim_c,
		frame_.brake_efficiency,
		frame_.duct_mass_flow_kg_s,
		frame_.duct_drag_n,
		frame_.brake_optimal_min_c,
		frame_.brake_optimal_max_c,
		frame_.brake_fade_start_c,
		frame_.brake_critical_c);
	veh->set_core_brake_energy_telemetry(
		frame_.brake_torque_nm,
		frame_.brake_spin_pre_rad_s,
		frame_.brake_spin_post_rad_s,
		frame_.brake_power_w,
		frame_.brake_energy_j);
	veh->set_core_brake_cooling_telemetry(
		frame_.brake_natural_cooling_w_k,
		frame_.brake_speed_cooling_w_k);
	veh->set_core_underfloor_telemetry(frame_);

	process_collision_audio(veh, state, dt);

	telemetry_print_accum_ += dt;
	if (telemetry_print_accum_ >= 0.5) {
		telemetry_print_accum_ = 0.0;
		UtilityFunctions::print(String("[F90Core->Vehicle] v=") + String::num(tel.speed_kmh, 1) +
			" km/h rpm=" + String::num(tel.rpm, 0) + " gear=" + String::num(tel.gear, 0) +
			" fz=" + String::num(force.z, 1) + " surf=" + String::num(frame_.surface_code, 0) +
			" posZ=" + String::num(old_pos.z, 2));
	}
}

void F90Core::process_collision_audio(F194RustVehicle *veh, PhysicsDirectBodyState3D *state, double dt) {
	if (!enable_audio_ || !state) {
		return;
	}
	if (collision_cooldown_ > 0.0) {
		collision_cooldown_ -= dt;
	}

	const int contact_count = state->get_contact_count();
	if (contact_count > 0 && collision_cooldown_ <= 0.0) {
		const Vector3 body_lin_vel = state->get_linear_velocity();

		for (int i = 0; i < contact_count; ++i) {
			Vector3 normal = state->get_contact_local_normal(i);
			Vector3 collider_vel = state->get_contact_collider_velocity_at_position(i);
			Vector3 rel_vel = body_lin_vel - collider_vel;
			Vector3 impulse = state->get_contact_impulse(i);

			float normal_impact = (float)std::abs(rel_vel.dot(normal));
			if (impulse.length_squared() > 0.0f) {
				normal_impact = std::max(normal_impact, (float)impulse.length() * 0.1f);
			}
			float tang_speed = (rel_vel - normal * rel_vel.dot(normal)).length();

			Object *col_obj = state->get_contact_collider_object(i);
			Node *col_node = Object::cast_to<Node>(col_obj);

			bool is_barrier = col_node && (col_node->is_in_group("Barrier") || col_node->is_in_group("Wall") ||
				col_node->is_in_group("Armco") || col_node->is_in_group("TireBarrier") || col_node->is_in_group("Guardrail"));
			bool is_cone = col_node && (col_node->is_in_group("Cone") || col_node->is_in_group("Prop") ||
				col_node->is_in_group("DynamicObstacle") || col_node->is_in_group("Obstacle"));

			if (is_barrier) {
				if (normal_impact > 3.0f) {
					trigger("impact_barrier");
					collision_cooldown_ = 0.20;
					break;
				} else if (tang_speed > 4.0f) {
					trigger("scrape");
					collision_cooldown_ = 0.15;
					break;
				}
			} else if (is_cone) {
				trigger("impact_cone");
				collision_cooldown_ = 0.15;
				break;
			} else {
				// General collision (body hit or obstacle)
				if (normal_impact > 3.0f) {
					if (normal_impact < 8.0f) {
						trigger("impact_hit_1");
					} else if (normal_impact < 15.0f) {
						trigger("impact_hit_2");
					} else if (normal_impact < 25.0f) {
						trigger("impact_hit_3");
					} else {
						trigger("impact_hit_4");
					}
					collision_cooldown_ = 0.20;
					break;
				}
			}
		}
	}
}

void F90Core::_process(double delta) {
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}
	update_listener_distance(delta);
	set_audio_ambient(tc_cut_ratio_, limiter_active_);
	pump_audio();
}

void F90Core::update_listener_distance(double delta) {
	if (!cached_veh_) {
		return;
	}
	Viewport *vp = get_viewport();
	if (!vp) {
		return;
	}
	Camera3D *cam = vp->get_camera_3d();
	if (!cam) {
		return;
	}
	const double dx = cam->get_global_transform().get_origin().x - cached_veh_->get_global_transform().get_origin().x;
	const double dy = cam->get_global_transform().get_origin().y - cached_veh_->get_global_transform().get_origin().y;
	const double dz = cam->get_global_transform().get_origin().z - cached_veh_->get_global_transform().get_origin().z;
	const double dist = std::sqrt(dx * dx + dy * dy + dz * dz);
	// One-pole toward the target with ~0.1 s time constant (frame-rate independent).
	const double alpha = delta > 0.0 ? 1.0 - std::exp(-delta / 0.1) : 0.0;
	listener_distance_smoothed_ += (dist - listener_distance_smoothed_) * alpha;
}

void F90Core::set_audio_ambient(float tc_cut_ratio, bool limiter_active) {
	if (core_ && fn_audio_set_ambient_) {
		fn_audio_set_ambient_(
			core_, static_cast<float>(listener_distance_smoothed_), tc_cut_ratio, limiter_active ? 1 : 0);
	}
}

void F90Core::_exit_tree() {
	if (audio_player_) {
		audio_player_->call("stop");
		Node *pn = Object::cast_to<Node>(audio_player_);
		if (pn && pn->get_parent() == this) {
			remove_child(pn);
		}
		memdelete(pn);
		audio_player_ = nullptr;
		audio_playback_ = nullptr;
	}
	generator_.unref();
	unload_dll();
}

// -------- audio plumbing (batched) ------------------------------------------------

void F90Core::ensure_vehicle_bus() {
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}
	AudioServer *as = AudioServer::get_singleton();
	if (!as) {
		return;
	}
	int idx = -1;
	for (int i = 0; i < as->get_bus_count(); ++i) {
		if (as->get_bus_name(i) == "Vehicle") {
			idx = i;
			break;
		}
	}
	if (idx == -1) {
		idx = as->get_bus_count();
		as->add_bus(idx);
		as->set_bus_name(idx, "Vehicle");
		as->set_bus_send(idx, "Master");
	}
}

void F90Core::create_audio_nodes() {
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}
	if (OS::get_singleton()->has_feature("headless") || AudioServer::get_singleton()->get_driver_name() == "Dummy") {
		return;
	}
	generator_.instantiate();
	generator_->set_mix_rate_mode(AudioStreamGenerator::MIX_RATE_CUSTOM);
	generator_->set_mix_rate(44100);
	generator_->set_buffer_length(0.06f);

	// Non-positional player (no AudioListener3D in the project): instantiate
	// AudioStreamPlayer generically (godot-cpp lacks a wrapper here) like the legacy
	// controller, and drive it through the Object/Variant API.
	Object *p = ClassDB::instantiate("AudioStreamPlayer");
	if (!p) {
		UtilityFunctions::printerr("[F90Core] Failed to instantiate AudioStreamPlayer.");
		return;
	}
	Node *pn = Object::cast_to<Node>(p);
	p->set("stream", generator_);
	p->set("bus", String("Vehicle"));
	add_child(pn);
	p->call("play");
	audio_player_ = p;
	// `get_stream_playback()` is NOT guaranteed on the same frame the player starts:
	// Godot hands out the playback object a frame later. Gating initialization on the
	// immediate result made the pump permanently dead (silent audio), so instead
	// mark the player as started and resolve the playback lazily in pump_audio().
	audio_playback_ = nullptr;
	audio_initialized_ = true;
}

void F90Core::pump_audio() {
	if (!audio_initialized_ || audio_player_ == nullptr || core_ == nullptr || fn_audio_render_ == nullptr) {
		return;
	}
	// Lazy playback: the player may not have yielded its stream playback yet.
	if (audio_playback_ == nullptr) {
		audio_playback_ = audio_player_->call("get_stream_playback");
		if (audio_playback_ == nullptr) {
			return;
		}
	}
	const int frames = (int)audio_playback_->call("get_frames_available");
	if (frames <= 0) {
		return;
	}
	if ((int)mix_l_.size() < frames) {
		mix_l_.resize(frames);
		mix_r_.resize(frames);
	}
	fn_audio_render_(core_, mix_l_.data(), mix_r_.data(), (uint32_t)frames);

	// ONE batched Variant call per pump instead of one per sample. The Rust mixer
	// is sample-accurate; this only removes the per-sample dispatch overhead.
	PackedVector2Array batch;
	batch.resize(frames);
	for (int i = 0; i < frames; ++i) {
		batch.set(i, Vector2(mix_l_[i], mix_r_[i]));
	}
	audio_playback_->call("push_buffer", batch);

	// Refresh presentation readouts (band weights ride the render).
	if (fn_audio_readouts_) {
		fn_audio_readouts_(core_, &frame_);
	}
}
