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
#include <chrono>
#include <cmath>
#include <cstdint>
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
	ClassDB::bind_method(D_METHOD("set_audio_pump_mode", "v"), &F90Core::set_audio_pump_mode);
	ClassDB::bind_method(D_METHOD("get_audio_pump_mode"), &F90Core::get_audio_pump_mode);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "audio_pump_mode", PROPERTY_HINT_RANGE, "0,1,1"), "set_audio_pump_mode", "get_audio_pump_mode");
	ClassDB::bind_method(D_METHOD("set_audio_worker_enabled", "v"), &F90Core::set_audio_worker_enabled);
	ClassDB::bind_method(D_METHOD("get_audio_worker_enabled"), &F90Core::get_audio_worker_enabled);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "audio_worker_enabled"), "set_audio_worker_enabled", "get_audio_worker_enabled");
	ClassDB::bind_method(D_METHOD("set_audio_worker_core", "v"), &F90Core::set_audio_worker_core);
	ClassDB::bind_method(D_METHOD("get_audio_worker_core"), &F90Core::get_audio_worker_core);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "audio_worker_core", PROPERTY_HINT_RANGE, "-1,63,1"), "set_audio_worker_core", "get_audio_worker_core");
	ClassDB::bind_method(D_METHOD("set_audio_worker_priority", "v"), &F90Core::set_audio_worker_priority);
	ClassDB::bind_method(D_METHOD("get_audio_worker_priority"), &F90Core::get_audio_worker_priority);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "audio_worker_priority", PROPERTY_HINT_RANGE, "0,2,1"), "set_audio_worker_priority", "get_audio_worker_priority");
	ClassDB::bind_method(D_METHOD("set_audio_latency_ms", "v"), &F90Core::set_audio_latency_ms);
	ClassDB::bind_method(D_METHOD("get_audio_latency_ms"), &F90Core::get_audio_latency_ms);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "audio_latency_ms", PROPERTY_HINT_RANGE, "1,200,1"), "set_audio_latency_ms", "get_audio_latency_ms");
	ClassDB::bind_method(D_METHOD("is_audio_worker_active"), &F90Core::is_audio_worker_active);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "audio_worker_active"), "", "is_audio_worker_active");
	ClassDB::bind_method(D_METHOD("get_audio_worker_stats"), &F90Core::get_audio_worker_stats);
	ADD_PROPERTY(PropertyInfo(Variant::DICTIONARY, "audio_worker_stats"), "", "get_audio_worker_stats");
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

	// --- audio pump diagnostics -------------------------------------------------
	ClassDB::bind_method(D_METHOD("get_audio_pump_calls"), &F90Core::get_audio_pump_calls);
	ClassDB::bind_method(D_METHOD("get_audio_frames_pushed"), &F90Core::get_audio_frames_pushed);
	ClassDB::bind_method(D_METHOD("get_audio_last_available"), &F90Core::get_audio_last_available);
	ClassDB::bind_method(D_METHOD("get_audio_max_available"), &F90Core::get_audio_max_available);
	ClassDB::bind_method(D_METHOD("get_audio_render_usec_total"), &F90Core::get_audio_render_usec_total);
	ClassDB::bind_method(D_METHOD("get_audio_skips"), &F90Core::get_audio_skips);
	ClassDB::bind_method(D_METHOD("get_audio_mix_rate"), &F90Core::get_audio_mix_rate);
	ClassDB::bind_method(D_METHOD("get_audio_buffer_length"), &F90Core::get_audio_buffer_length);
	ClassDB::bind_method(D_METHOD("get_audio_output_rms"), &F90Core::get_audio_output_rms);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "audio_output_rms"), "", "get_audio_output_rms");
	ClassDB::bind_method(D_METHOD("get_audio_source_code"), &F90Core::get_audio_source_code);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "audio_source_code", PROPERTY_HINT_RANGE, "-1,2,1"), "", "get_audio_source_code");
	ClassDB::bind_method(D_METHOD("get_audio_gen_occupancy"), &F90Core::get_audio_gen_occupancy);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "audio_gen_occupancy"), "", "get_audio_gen_occupancy");
	ClassDB::bind_method(D_METHOD("get_audio_gen_capacity"), &F90Core::get_audio_gen_capacity);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "audio_gen_capacity"), "", "get_audio_gen_capacity");
	ClassDB::bind_method(D_METHOD("get_audio_push_rejections"), &F90Core::get_audio_push_rejections);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "audio_push_rejections"), "", "get_audio_push_rejections");
	ClassDB::bind_method(D_METHOD("reset_audio_stats"), &F90Core::reset_audio_stats);

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

int F90Core::get_audio_source_code() const {
	if (fn_audio_source_code_ == nullptr || core_ == nullptr) {
		return -1;
	}
	return fn_audio_source_code_(core_);
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
	if (name == "shift_up") {
		return 0;
	}
	if (name == "shift_down") {
		return 1;
	}
	if (name == "engine_backfire") {
		return 2;
	}
	if (name == "impact_hit_1") {
		return 3;
	}
	if (name == "impact_hit_2") {
		return 4;
	}
	if (name == "impact_hit_3") {
		return 5;
	}
	if (name == "impact_hit_4") {
		return 6;
	}
	if (name == "impact_barrier") {
		return 7;
	}
	if (name == "impact_cone") {
		return 8;
	}
	if (name == "engine_fire") {
		return 9;
	}
	if (name == "scrape") {
		return 10;
	}
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
	Array rejected;
	for (int i = 0; i < candidates.size(); ++i) {
		const String p = candidates[i];
		const String global_p = ps ? ps->globalize_path(p) : p;
		HMODULE candidate = LoadLibraryW((LPCWSTR)global_p.utf16().get_data());
		if (candidate == nullptr) {
			continue;
		}
		// Validate ABI + BUILD BEFORE accepting. A stale sibling configuration
		// (e.g. a leftover template_release next to the current template_debug)
		// is skipped and reported; only "no candidate matches" is fatal. Fataling
		// on the first mismatched sibling made the whole runtime (including audio)
		// silently dead when a stale DLL shadowed a valid one.
		FnCoreAbiVersion candidate_abi = reinterpret_cast<FnCoreAbiVersion>(GetProcAddress(candidate, "f90_core_abi_version"));
		FnCoreBuildSha candidate_sha = reinterpret_cast<FnCoreBuildSha>(GetProcAddress(candidate, "f90_core_build_sha"));
		const uint32_t candidate_abi_ver = candidate_abi ? candidate_abi() : 0;
		const String candidate_build = candidate_sha ? String(candidate_sha()) : String("unknown");
		if (candidate_abi_ver != EXPECTED_ABI_VERSION || candidate_build != recorded_source_sha || candidate_build == "unknown") {
			rejected.append(global_p + String(" (ABI=") + String::num_int64(candidate_abi_ver) +
					String(" BUILD=") + candidate_build + String(")"));
			FreeLibrary(candidate);
			continue;
		}
		hDll = candidate;
		loaded_path = global_p;
		break;
	}
	if (!hDll) {
		String rejected_text = "";
		for (int i = 0; i < rejected.size(); ++i) {
			rejected_text += (i == 0 ? String("") : String(", ")) + String(rejected[i]);
		}
		UtilityFunctions::printerr(String("[F90Core] FATAL: no formula90_core.dll candidate matches BUILD ") +
			recorded_source_sha + String(". Rejected: ") + rejected_text);
		return false;
	}
	for (int i = 0; i < rejected.size(); ++i) {
		UtilityFunctions::print(String("[F90Core] skipped stale candidate: ") + String(rejected[i]));
	}
	dll_handle_ = reinterpret_cast<void *>(hDll);

	fn_abi_version_ = reinterpret_cast<FnCoreAbiVersion>(GetProcAddress(hDll, "f90_core_abi_version"));
	fn_build_sha_ = reinterpret_cast<FnCoreBuildSha>(GetProcAddress(hDll, "f90_core_build_sha"));
	fn_create_ = reinterpret_cast<FnCoreCreate>(GetProcAddress(hDll, "f90_core_create"));
	fn_destroy_ = reinterpret_cast<FnCoreDestroy>(GetProcAddress(hDll, "f90_core_destroy"));
	fn_spawn_ = reinterpret_cast<FnCoreSpawn>(GetProcAddress(hDll, "f90_core_spawn"));
	fn_reset_ = reinterpret_cast<FnCoreReset>(GetProcAddress(hDll, "f90_core_reset"));
	fn_apply_runtime_config_ = reinterpret_cast<FnCoreApplyRuntimeConfig>(GetProcAddress(hDll, "f90_core_apply_runtime_config"));
	fn_step_ = reinterpret_cast<FnCoreStep>(GetProcAddress(hDll, "f90_core_step"));
	fn_audio_render_ = reinterpret_cast<FnCoreAudioRender>(GetProcAddress(hDll, "f90_core_audio_render"));
	fn_audio_trigger_ = reinterpret_cast<FnCoreAudioTrigger>(GetProcAddress(hDll, "f90_core_audio_trigger"));
	fn_audio_readouts_ = reinterpret_cast<FnCoreAudioReadouts>(GetProcAddress(hDll, "f90_core_audio_readouts"));
	fn_audio_source_code_ = reinterpret_cast<FnCoreAudioSourceCode>(GetProcAddress(hDll, "f90_core_audio_source_code"));
	fn_audio_set_ambient_ = reinterpret_cast<FnCoreAudioSetAmbient>(GetProcAddress(hDll, "f90_core_audio_set_ambient"));
	fn_audio_worker_start_ = reinterpret_cast<FnCoreAudioWorkerStart>(GetProcAddress(hDll, "f90_core_audio_worker_start"));
	fn_audio_worker_handle_ = reinterpret_cast<FnCoreAudioWorkerHandle>(GetProcAddress(hDll, "f90_core_audio_worker_handle"));
	fn_audio_worker_run_ = reinterpret_cast<FnCoreAudioWorkerRun>(GetProcAddress(hDll, "f90_core_audio_worker_run"));
	fn_audio_worker_pull_ = reinterpret_cast<FnCoreAudioWorkerPull>(GetProcAddress(hDll, "f90_core_audio_worker_pull"));
	fn_audio_worker_stats_get_ = reinterpret_cast<FnCoreAudioWorkerStatsGet>(GetProcAddress(hDll, "f90_core_audio_worker_stats"));
	fn_audio_worker_set_target_ = reinterpret_cast<FnCoreAudioWorkerSetTarget>(GetProcAddress(hDll, "f90_core_audio_worker_set_target"));

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
	// The worker calls into the façade DLL; it must be joined before the core is
	// destroyed and the module unloaded.
	stop_audio_worker();
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
	fn_audio_source_code_ = nullptr;
	fn_audio_set_ambient_ = nullptr;
	fn_audio_worker_start_ = nullptr;
	fn_audio_worker_handle_ = nullptr;
	fn_audio_worker_run_ = nullptr;
	fn_audio_worker_pull_ = nullptr;
	fn_audio_worker_stats_get_ = nullptr;
	fn_audio_worker_set_target_ = nullptr;
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
		UtilityFunctions::printerr(String("[F90Core] f90_core_create failed: ") + String(reinterpret_cast<const char *>(err_buf)));
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
	// Phase 2: move the mixer DSP off the render thread when the loaded facade
	// exposes the worker ABI. Falls back to the inline pump automatically.
	start_audio_worker();
	UtilityFunctions::print(String("[F90Core] facade ready (entity=") + String::num(entity_id_) +
		", audio_worker=" + String(audio_worker_active_ ? "on" : "off") +
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
			if (impulse.length_squared() > 0.0F) {
				normal_impact = std::max(normal_impact, (float)impulse.length() * 0.1F);
			}
			float tang_speed = (rel_vel - normal * rel_vel.dot(normal)).length();

			Object *col_obj = state->get_contact_collider_object(i);
			Node *col_node = Object::cast_to<Node>(col_obj);

			bool is_barrier = col_node && (col_node->is_in_group("Barrier") || col_node->is_in_group("Wall") ||
				col_node->is_in_group("Armco") || col_node->is_in_group("TireBarrier") || col_node->is_in_group("Guardrail"));
			bool is_cone = col_node && (col_node->is_in_group("Cone") || col_node->is_in_group("Prop") ||
				col_node->is_in_group("DynamicObstacle") || col_node->is_in_group("Obstacle"));

			if (is_barrier) {
				if (normal_impact > 3.0F) {
					trigger("impact_barrier");
					collision_cooldown_ = 0.20;
					break;
				} else if (tang_speed > 4.0F) {
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
				if (normal_impact > 3.0F) {
					if (normal_impact < 8.0F) {
						trigger("impact_hit_1");
					} else if (normal_impact < 15.0F) {
						trigger("impact_hit_2");
					} else if (normal_impact < 25.0F) {
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
	if (audio_worker_active_) {
		pump_audio_worker(delta);
	} else {
		pump_audio(delta);
	}
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
	const double dist = std::sqrt((dx * dx) + (dy * dy) + (dz * dz));
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
	stop_audio_worker();
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
	generator_->set_mix_rate(kAudioMixRate);
	// A slightly deeper ring absorbs render stalls without the pump having to
	// spike a huge batch on recovery.
	generator_->set_buffer_length((float)audio_buffer_length_);

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

bool F90Core::push_audio_batch(const float *left, const float *right, int frames) {
	if (frames <= 0 || left == nullptr || right == nullptr) {
		return false;
	}
	// ONE batched Variant call per pump instead of one per sample. The Rust mixer
	// is sample-accurate; this only removes the per-sample dispatch overhead.
	PackedVector2Array batch;
	batch.resize(frames);
	double sum_squares = 0.0;
	for (int i = 0; i < frames; ++i) {
		const float l = left[i];
		const float r = right[i];
		sum_squares += (double)l * (double)l + (double)r * (double)r;
		batch.set(i, Vector2(l, r));
	}
	audio_output_rms_ = (float)std::sqrt(sum_squares / (double)(frames * 2));
	if (audio_playback_ == nullptr) {
		return false;
	}
	const bool accepted = (bool)audio_playback_->call("push_buffer", batch);
	if (!accepted) {
		audio_push_rejections_ += 1;
	}
	return accepted;
}

void F90Core::pump_audio(double delta) {
	// enable_audio_ is honored at call time (not just at _ready): setting it
	// false after initialization stops the per-frame pump immediately, the same
	// runtime-disable contract the visual suspension controller follows.
	if (!enable_audio_ || !audio_initialized_ || audio_player_ == nullptr || core_ == nullptr || fn_audio_render_ == nullptr) {
		return;
	}
	// Lazy playback: the player may not have yielded its stream playback yet.
	if (audio_playback_ == nullptr) {
		audio_playback_ = audio_player_->call("get_stream_playback");
		if (audio_playback_ == nullptr) {
			return;
		}
	}
	const int available = (int)audio_playback_->call("get_frames_available");
	audio_last_available_ = available;
	if (available > audio_max_available_) {
		audio_max_available_ = available;
	}
	if (available <= 0) {
		return;
	}

	int frames;
	if (audio_pump_mode_ == 0) {
		// Legacy fixed cap: bounds the per-call DSP but underproduces whenever
		// the rendered frame rate drops below kAudioMixRate / kPumpBudgetFrames
		// (~43.07 FPS), which drains the ring and inserts underrun silence.
		frames = available;
		if (frames > kPumpBudgetFrames) {
			frames = kPumpBudgetFrames;
		}
	} else {
		// Delta budget: replace exactly what the mixer consumed during the last
		// rendered frame (kAudioMixRate * delta) plus a bounded catch-up, so
		// production never falls below the steady-state consumption rate while a
		// single pump stays bounded by kPumpMaxBatchFrames. `available` still
		// caps the batch to the free space in the generator ring.
		int demand = (int)std::ceil((double)kAudioMixRate * delta) + kPumpCatchUpFrames;
		if (demand < kPumpMinBatchFrames) {
			demand = kPumpMinBatchFrames;
		}
		if (demand > kPumpMaxBatchFrames) {
			demand = kPumpMaxBatchFrames;
		}
		frames = available < demand ? available : demand;
	}
	if (frames <= 0) {
		return;
	}

	if ((int)mix_l_.size() < frames) {
		mix_l_.resize(frames);
		mix_r_.resize(frames);
	}
	const auto render_start = std::chrono::steady_clock::now();
	fn_audio_render_(core_, mix_l_.data(), mix_r_.data(), (uint32_t)frames);
	audio_render_usec_total_ += (int64_t)std::chrono::duration_cast<std::chrono::microseconds>(
			std::chrono::steady_clock::now() - render_start)
										.count();

	push_audio_batch(mix_l_.data(), mix_r_.data(), frames);
	audio_pump_calls_ += 1;
	audio_frames_pushed_ += frames;
	if (audio_playback_->has_method("get_skips")) {
		audio_skips_ = (int64_t)(int)audio_playback_->call("get_skips");
	}

	// Refresh presentation readouts (band weights ride the render).
	if (fn_audio_readouts_) {
		fn_audio_readouts_(core_, &frame_);
	}
}

void F90Core::reset_audio_stats() {
	audio_pump_calls_ = 0;
	audio_frames_pushed_ = 0;
	audio_last_available_ = 0;
	audio_max_available_ = 0;
	audio_render_usec_total_ = 0;
	audio_skips_ = 0;
	audio_output_rms_ = 0.0f;
	audio_push_rejections_ = 0;
	audio_gen_occupancy_ = 0;
	audio_need_total_ = 0;
	audio_need_zero_calls_ = 0;
	audio_pull_unmet_frames_ = 0;
	audio_push_deficit_ = 0;
}

// -------- dedicated audio worker (phase 2) ----------------------------------------

void F90Core::set_enable_audio(bool v) {
	if (v == enable_audio_) {
		return;
	}
	enable_audio_ = v;
	if (v && audio_worker_active_) {
		// The ring holds pre-disable audio while the gate is off; drop it so
		// re-enabling resumes at the live cursor instead of a stale burst.
		flush_audio_worker_ring();
	}
}

void F90Core::flush_audio_worker_ring() {
	if (!audio_worker_active_ || core_ == nullptr || fn_audio_worker_pull_ == nullptr) {
		return;
	}
	float discard_l[1024];
	float discard_r[1024];
	for (int i = 0; i < 64; ++i) {
		if (fn_audio_worker_pull_(core_, discard_l, discard_r, 1024) == 0) {
			break;
		}
	}
}

int F90Core::resolve_audio_core() const {
	if (audio_worker_core_ >= 0) {
		return audio_worker_core_;
	}
#ifdef _WIN32
	// Prefer a P-core (highest efficiency class) and its highest logical index so
	// the audio thread avoids sharing an SMT sibling with the render thread.
	DWORD length = 0;
	GetLogicalProcessorInformationEx(RelationProcessorCore, nullptr, &length);
	if (length > 0) {
		std::vector<uint8_t> buffer(length, 0);
		if (GetLogicalProcessorInformationEx(RelationProcessorCore,
					reinterpret_cast<PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(buffer.data()), &length)) {
			int best_class = -1;
			int best_lp = -1;
			const uint8_t *cursor = buffer.data();
			const uint8_t *end = buffer.data() + length;
			while (cursor < end) {
				const auto *info = reinterpret_cast<const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX *>(cursor);
				if (info->Relationship == RelationProcessorCore) {
					const int cls = (int)info->Processor.EfficiencyClass;
					for (WORD group_index = 0; group_index < info->Processor.GroupCount; ++group_index) {
						const KAFFINITY mask = info->Processor.GroupMask[group_index].Mask;
						if (mask == 0) {
							continue;
						}
						int bit = (int)(sizeof(KAFFINITY) * 8) - 1;
						while (bit >= 0 && (mask & ((KAFFINITY)1 << bit)) == 0) {
							--bit;
						}
						if (bit < 0) {
							continue;
						}
						const int lp = (int)info->Processor.GroupMask[group_index].Group * 64 + bit;
						if (cls > best_class || (cls == best_class && lp > best_lp)) {
							best_class = cls;
							best_lp = lp;
						}
					}
				}
				cursor += info->Size;
			}
			if (best_lp >= 0) {
				return best_lp;
			}
		}
	}
	const DWORD total = GetActiveProcessorCount(ALL_PROCESSOR_GROUPS);
	return total > 0 ? (int)(total - 1) : 0;
#else
	return 0;
#endif
}

void F90Core::configure_audio_worker_thread(int core_index) {
#ifdef _WIN32
	audio_worker_thread_id_.store((int64_t)GetCurrentThreadId(), std::memory_order_relaxed);
	int priority = THREAD_PRIORITY_ABOVE_NORMAL;
	if (audio_worker_priority_ <= 0) {
		priority = THREAD_PRIORITY_NORMAL;
	} else if (audio_worker_priority_ >= 2) {
		priority = THREAD_PRIORITY_HIGHEST;
	}
	SetThreadPriority(GetCurrentThread(), priority);
	// Raise the system timer resolution: without it `sleep(5ms)` rounds up to the
	// ~15.6 ms scheduler tick and the ring cannot be refilled once per rendered
	// frame, which starves the generator (measured: produced/s 31k at 108 FPS).
	HMODULE winmm = LoadLibraryW(L"winmm.dll");
	if (winmm != nullptr) {
		typedef UINT(WINAPI * TimeBeginPeriod_t)(UINT);
		TimeBeginPeriod_t time_begin = reinterpret_cast<TimeBeginPeriod_t>(GetProcAddress(winmm, "timeBeginPeriod"));
		if (time_begin != nullptr) {
			time_begin(1);
			audio_worker_timer_raised_ = true;
		}
	}
	// MMCSS "Pro Audio" keeps the mixer in the real-time audio scheduling class;
	// avrt.dll ships with Windows (dynamic load, no link-time dependency).
	HMODULE avrt = LoadLibraryW(L"avrt.dll");
	if (avrt != nullptr) {
		typedef HANDLE(WINAPI * AvSetMmThreadCharacteristicsW_t)(LPCWSTR, LPDWORD);
		AvSetMmThreadCharacteristicsW_t av_set =
				reinterpret_cast<AvSetMmThreadCharacteristicsW_t>(GetProcAddress(avrt, "AvSetMmThreadCharacteristicsW"));
		if (av_set != nullptr) {
			DWORD task_index = 0;
			av_set(L"Pro Audio", &task_index);
		}
	}
	if (core_index >= 0 && core_index < 64) {
		const DWORD_PTR mask = ((DWORD_PTR)1) << core_index;
		if (SetThreadAffinityMask(GetCurrentThread(), mask) != 0) {
			audio_worker_affinity_mask_.store((int64_t)mask, std::memory_order_relaxed);
			audio_worker_core_observed_.store((uint32_t)core_index, std::memory_order_relaxed);
		}
	}
	if (audio_worker_affinity_mask_.load(std::memory_order_relaxed) == 0) {
		audio_worker_core_observed_.store((uint32_t)GetCurrentProcessorNumber(), std::memory_order_relaxed);
	}
#else
	(void)core_index;
#endif
}

void F90Core::start_audio_worker() {
	if (!audio_worker_enabled_ || audio_worker_active_ || !audio_initialized_) {
		return;
	}
	if (core_ == nullptr || fn_audio_worker_start_ == nullptr || fn_audio_worker_handle_ == nullptr ||
			fn_audio_worker_run_ == nullptr) {
		return;
	}
	if (!fn_audio_worker_start_(core_)) {
		return;
	}
	audio_worker_handle_ = const_cast<void *>(fn_audio_worker_handle_(core_));
	if (audio_worker_handle_ == nullptr) {
		return;
	}
	audio_worker_stop_.store(0, std::memory_order_release);
	const int core_index = resolve_audio_core();
	audio_worker_thread_ = std::thread([this, core_index]() {
		configure_audio_worker_thread(core_index);
		fn_audio_worker_run_(audio_worker_handle_,
				reinterpret_cast<const volatile uint32_t *>(&audio_worker_stop_));
	});
	audio_worker_active_ = true;
	// Affinity/thread id settle on the worker thread; read them from
	// `audio_worker_stats` once the worker has started up.
	UtilityFunctions::print(String("[F90Core] audio worker started (requested_core=") + String::num_int64(core_index) + ")");
}

void F90Core::stop_audio_worker() {
	if (audio_worker_active_) {
		audio_worker_stop_.store(1, std::memory_order_release);
	}
	if (audio_worker_thread_.joinable()) {
		audio_worker_thread_.join();
	}
#ifdef _WIN32
	if (audio_worker_timer_raised_) {
		HMODULE winmm = LoadLibraryW(L"winmm.dll");
		if (winmm != nullptr) {
			typedef UINT(WINAPI * TimeEndPeriod_t)(UINT);
			TimeEndPeriod_t time_end = reinterpret_cast<TimeEndPeriod_t>(GetProcAddress(winmm, "timeEndPeriod"));
			if (time_end != nullptr) {
				time_end(1);
			}
		}
		audio_worker_timer_raised_ = false;
	}
#endif
	audio_worker_active_ = false;
	audio_worker_handle_ = nullptr;
}

void F90Core::pump_audio_worker(double delta) {
	if (!enable_audio_ || core_ == nullptr || audio_player_ == nullptr ||
			fn_audio_worker_pull_ == nullptr || fn_audio_worker_stats_get_ == nullptr) {
		return;
	}
	// Lazy playback: the player may not have yielded its stream playback yet.
	if (audio_playback_ == nullptr) {
		audio_playback_ = audio_player_->call("get_stream_playback");
		if (audio_playback_ == nullptr) {
			return;
		}
	}
	audio_pump_entries_ += 1;
	int available = (int)audio_playback_->call("get_frames_available");
	audio_last_available_ = available;
	if (available > audio_max_available_) {
		audio_max_available_ = available;
	}
	if (available <= 0) {
		audio_pump_no_room_ += 1;
		return;
	}
	const int configured = (int)std::ceil(audio_buffer_length_ * (double)kAudioMixRate);
	if (audio_gen_capacity_ < configured) {
		audio_gen_capacity_ = configured;
	}
	if (available > audio_gen_capacity_) {
		audio_gen_capacity_ = available;
	}
	audio_gen_occupancy_ = audio_gen_capacity_ - available;
	if (audio_gen_occupancy_ < 0) {
		audio_gen_occupancy_ = 0;
	}

	// Low-latency transfer law: accumulate exactly what the mixer will consume
	// this frame, spend it on the next push, and cap the backlog at ~two frames.
	// This never stacks the fixed 0.1 s pre-buffer that made the listener ~0.2 s
	// behind the image, and needs no capacity/occupancy calibration.
	const int frame_demand = (int)std::ceil((double)kAudioMixRate * delta);
	audio_need_total_ += frame_demand;
	audio_push_deficit_ += frame_demand;
	// One frame + margin is all the generator needs to bridge a transfer; more
	// is pure added latency.
	// Per-stage target: at least one frame + margin (so the transfer is always
	// servable) and at least the user-tuned `audio_latency_ms`. The same value
	// caps the generator backlog, so total listener latency is ~2x the stage.
	const int64_t latency_frames = (int64_t)audio_latency_ms_ * kAudioMixRate / 1000;
	int64_t stage = (int64_t)frame_demand + kGenHeadroomFrames;
	if (stage < latency_frames) {
		stage = latency_frames;
	}
	if (stage > kPumpMaxBatchFrames) {
		stage = kPumpMaxBatchFrames;
	}
	if (stage < kPumpMinBatchFrames) {
		stage = kPumpMinBatchFrames;
	}
	if (audio_push_deficit_ > stage) {
		audio_push_deficit_ = stage;
	}
	int need = (int)(audio_push_deficit_ < (int64_t)available ? audio_push_deficit_ : (int64_t)available);
	// Feedback: if the generator drifted above the stage target, stop feeding
	// until it drains back. Without this the occupancy random-walks upward and
	// the measured latency stops tracking `audio_latency_ms`.
	if (audio_gen_occupancy_ > (int)stage) {
		need -= audio_gen_occupancy_ - (int)stage;
	}
	if (need <= 0) {
		audio_need_zero_calls_ += 1;
		return;
	}
	// The worker ring mirrors the generator stage so the transfer is servable.
	if (fn_audio_worker_set_target_ != nullptr) {
		fn_audio_worker_set_target_(core_, (uint32_t)stage);
	}
	if ((int)worker_l_.size() < need) {
		worker_l_.resize(need);
		worker_r_.resize(need);
	}
	const uint32_t pulled = fn_audio_worker_pull_(core_, worker_l_.data(), worker_r_.data(), (uint32_t)need);
	if (pulled == 0) {
		return;
	}
	audio_push_deficit_ -= (int64_t)pulled;
	if (audio_push_deficit_ < 0) {
		audio_push_deficit_ = 0;
	}
	if ((int)pulled < need) {
		audio_pull_unmet_frames_ += (int64_t)(need - (int)pulled);
	}
	push_audio_batch(worker_l_.data(), worker_r_.data(), (int)pulled);
	audio_pump_calls_ += 1;
	audio_frames_pushed_ += (int64_t)pulled;
	if (audio_playback_->has_method("get_skips")) {
		audio_skips_ = (int64_t)(int)audio_playback_->call("get_skips");
	}
	if (fn_audio_readouts_) {
		fn_audio_readouts_(core_, &frame_);
	}
	fn_audio_worker_stats_get_(core_, &audio_worker_stats_);
}

Dictionary F90Core::get_audio_worker_stats() const {
	Dictionary d;
	d["active"] = audio_worker_active_;
	d["enabled"] = audio_worker_enabled_;
	d["requested_core"] = audio_worker_core_;
	d["core"] = (int64_t)audio_worker_core_observed_.load(std::memory_order_relaxed);
	d["thread_id"] = audio_worker_thread_id_.load(std::memory_order_relaxed);
	d["affinity_mask"] = audio_worker_affinity_mask_.load(std::memory_order_relaxed);
	d["priority"] = audio_worker_priority_;
	d["produced_frames"] = (int64_t)audio_worker_stats_.produced_frames;
	d["consumed_frames"] = (int64_t)audio_worker_stats_.consumed_frames;
	d["starved_iterations"] = (int64_t)audio_worker_stats_.starved_iterations;
	d["packets_applied"] = (int64_t)audio_worker_stats_.packets_applied;
	d["packets_dropped"] = (int64_t)audio_worker_stats_.packets_dropped;
	d["commands_applied"] = (int64_t)audio_worker_stats_.commands_applied;
	d["render_usec_total"] = (int64_t)audio_worker_stats_.render_usec_total;
	d["iterations"] = (int64_t)audio_worker_stats_.iterations;
	d["ring_frames"] = (int64_t)audio_worker_stats_.ring_frames;
	d["ring_capacity_frames"] = (int64_t)audio_worker_stats_.ring_capacity_frames;
	d["high_water_frames"] = (int64_t)audio_worker_stats_.high_water_frames;
	d["healthy"] = audio_worker_stats_.healthy != 0;
	d["output_rms"] = audio_output_rms_;
	d["push_rejections"] = audio_push_rejections_;
	d["demand_frames"] = audio_need_total_;
	d["need_zero_calls"] = audio_need_zero_calls_;
	d["pull_unmet_frames"] = audio_pull_unmet_frames_;
	d["pump_entries"] = audio_pump_entries_;
	d["pump_no_room"] = audio_pump_no_room_;
	return d;
}
