#include "formula90s/presentation/vehicle_audio_controller_native.hpp"
#include <godot_cpp/classes/file_access.hpp>
#include <algorithm>
#include <cmath>
#include <vector>

#ifndef FORMULA90_BUILD_SHA
#define FORMULA90_BUILD_SHA "unknown"
#endif

using namespace godot;

VehicleAudioControllerNative::VehicleAudioControllerNative() {
}

VehicleAudioControllerNative::~VehicleAudioControllerNative() {
	unload_dll();
}

void VehicleAudioControllerNative::_bind_methods() {
	ClassDB::bind_method(D_METHOD("set_vehicle", "path"), &VehicleAudioControllerNative::set_vehicle);
	ClassDB::bind_method(D_METHOD("get_vehicle"), &VehicleAudioControllerNative::get_vehicle);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "vehicle"), "set_vehicle", "get_vehicle");

	ClassDB::bind_method(D_METHOD("set_idle_rpm", "v"), &VehicleAudioControllerNative::set_idle_rpm);
	ClassDB::bind_method(D_METHOD("get_idle_rpm"), &VehicleAudioControllerNative::get_idle_rpm);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "idle_rpm"), "set_idle_rpm", "get_idle_rpm");

	ClassDB::bind_method(D_METHOD("set_max_rpm", "v"), &VehicleAudioControllerNative::set_max_rpm);
	ClassDB::bind_method(D_METHOD("get_max_rpm"), &VehicleAudioControllerNative::get_max_rpm);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "max_rpm"), "set_max_rpm", "get_max_rpm");

	ClassDB::bind_method(D_METHOD("trigger", "name"), &VehicleAudioControllerNative::trigger);

	// Telemetry (read-only, consumed by audio_telemetry.gd).
	ClassDB::bind_method(D_METHOD("get_last_norm"), &VehicleAudioControllerNative::get_last_norm);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_norm"), "", "get_last_norm");
	ClassDB::bind_method(D_METHOD("get_last_rpm"), &VehicleAudioControllerNative::get_last_rpm);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_rpm"), "", "get_last_rpm");
	ClassDB::bind_method(D_METHOD("get_last_throttle"), &VehicleAudioControllerNative::get_last_throttle);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_throttle"), "", "get_last_throttle");
	ClassDB::bind_method(D_METHOD("get_last_slip"), &VehicleAudioControllerNative::get_last_slip);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_slip"), "", "get_last_slip");
	ClassDB::bind_method(D_METHOD("get_last_speed_kph"), &VehicleAudioControllerNative::get_last_speed_kph);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_speed_kph"), "", "get_last_speed_kph");
	ClassDB::bind_method(D_METHOD("get_last_engine_gain"), &VehicleAudioControllerNative::get_last_engine_gain);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_engine_gain"), "", "get_last_engine_gain");
	ClassDB::bind_method(D_METHOD("get_last_weights"), &VehicleAudioControllerNative::get_last_weights);
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT32_ARRAY, "last_weights"), "", "get_last_weights");
	ClassDB::bind_method(D_METHOD("get_last_pitches"), &VehicleAudioControllerNative::get_last_pitches);
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT32_ARRAY, "last_pitches"), "", "get_last_pitches");
	ClassDB::bind_method(D_METHOD("get_last_trigger"), &VehicleAudioControllerNative::get_last_trigger);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "last_trigger"), "", "get_last_trigger");
	ClassDB::bind_method(D_METHOD("get_surface"), &VehicleAudioControllerNative::get_surface);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "surface"), "", "get_surface");
	ClassDB::bind_method(D_METHOD("get_active_bed"), &VehicleAudioControllerNative::get_active_bed);
	ADD_PROPERTY(PropertyInfo(Variant::STRING, "_active_bed"), "", "get_active_bed");
	ClassDB::bind_method(D_METHOD("get_engine_band_native_rpm"), &VehicleAudioControllerNative::get_engine_band_native_rpm);
	ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT64_ARRAY, "ENGINE_BAND_NATIVE_RPM"), "", "get_engine_band_native_rpm");
	ClassDB::bind_method(D_METHOD("is_engine_loaded"), &VehicleAudioControllerNative::is_engine_loaded);
	ClassDB::bind_method(D_METHOD("is_audio_active"), &VehicleAudioControllerNative::is_audio_active);

	ClassDB::bind_method(D_METHOD("get_last_normalized_engine_load"), &VehicleAudioControllerNative::get_last_normalized_engine_load);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_normalized_engine_load"), "", "get_last_normalized_engine_load");
	ClassDB::bind_method(D_METHOD("get_last_normalized_engine_torque"), &VehicleAudioControllerNative::get_last_normalized_engine_torque);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_normalized_engine_torque"), "", "get_last_normalized_engine_torque");
	ClassDB::bind_method(D_METHOD("get_last_torque_sign"), &VehicleAudioControllerNative::get_last_torque_sign);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "last_torque_sign"), "", "get_last_torque_sign");
	ClassDB::bind_method(D_METHOD("get_last_rpm_derivative"), &VehicleAudioControllerNative::get_last_rpm_derivative);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_rpm_derivative"), "", "get_last_rpm_derivative");
	ClassDB::bind_method(D_METHOD("get_last_throttle_derivative"), &VehicleAudioControllerNative::get_last_throttle_derivative);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_throttle_derivative"), "", "get_last_throttle_derivative");
	ClassDB::bind_method(D_METHOD("get_last_shift_phase"), &VehicleAudioControllerNative::get_last_shift_phase);
	ADD_PROPERTY(PropertyInfo(Variant::INT, "last_shift_phase"), "", "get_last_shift_phase");
	ClassDB::bind_method(D_METHOD("get_last_clutch_engagement"), &VehicleAudioControllerNative::get_last_clutch_engagement);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_clutch_engagement"), "", "get_last_clutch_engagement");
	ClassDB::bind_method(D_METHOD("get_last_tc_cut_ratio"), &VehicleAudioControllerNative::get_last_tc_cut_ratio);
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "last_tc_cut_ratio"), "", "get_last_tc_cut_ratio");
	ClassDB::bind_method(D_METHOD("get_last_rev_limiter_active"), &VehicleAudioControllerNative::get_last_rev_limiter_active);
	ADD_PROPERTY(PropertyInfo(Variant::BOOL, "last_rev_limiter_active"), "", "get_last_rev_limiter_active");
}

PackedFloat32Array VehicleAudioControllerNative::get_last_weights() const {
	PackedFloat32Array arr;
	arr.resize(5);
	for (int i = 0; i < 5; ++i) {
		arr.set(i, last_weights_[i]);
	}
	return arr;
}

PackedFloat32Array VehicleAudioControllerNative::get_last_pitches() const {
	PackedFloat32Array arr;
	arr.resize(5);
	for (int i = 0; i < 5; ++i) {
		arr.set(i, last_pitches_[i]);
	}
	return arr;
}

PackedFloat64Array VehicleAudioControllerNative::get_engine_band_native_rpm() const {
	PackedFloat64Array arr;
	arr.resize(5);
	for (int i = 0; i < 5; ++i) {
		arr.set(i, (double)ENGINE_BAND_NATIVE_RPM_[i]);
	}
	return arr;
}

int VehicleAudioControllerNative::trigger_code(const String &name) {
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

void VehicleAudioControllerNative::trigger(const String &name) {
	last_trigger_ = name;
	const int code = trigger_code(name);
	if (engine_ && fn_trigger_ && code >= 0) {
		fn_trigger_(engine_, code);
	}
}

bool VehicleAudioControllerNative::load_dll() {
	const String build_source_path = "res://BUILD_SOURCE";
	if (!FileAccess::file_exists(build_source_path)) {
		UtilityFunctions::printerr("[VehicleAudioControllerNative] FATAL: res://BUILD_SOURCE is missing");
		return false;
	}
	Ref<FileAccess> build_source_file = FileAccess::open(build_source_path, FileAccess::READ);
	const String recorded_source_sha = build_source_file.is_valid()
		? build_source_file->get_as_text().strip_edges()
		: String();
	if (recorded_source_sha.length() != 40 || recorded_source_sha != String(FORMULA90_BUILD_SHA)) {
		UtilityFunctions::printerr(String("[VehicleAudioControllerNative] FATAL: native BUILD does not match recorded source. Native=") +
			String(FORMULA90_BUILD_SHA) + " recorded=" + recorded_source_sha);
		return false;
	}

	if (dll_handle_ != nullptr) {
		return true;
	}
#ifdef _WIN32
	Array candidate_paths;
	candidate_paths.append("res://addons/formula90s/bin/vehicle_audio_engine.windows.template_release.x86_64.dll");
	candidate_paths.append("res://addons/formula90s/bin/vehicle_audio_engine.windows.template_debug.x86_64.dll");
	candidate_paths.append("res://addons/formula90s/bin/vehicle_audio_engine.dll");
	candidate_paths.append("game/addons/formula90s/bin/vehicle_audio_engine.windows.template_release.x86_64.dll");
	candidate_paths.append("game/addons/formula90s/bin/vehicle_audio_engine.windows.template_debug.x86_64.dll");
	candidate_paths.append("game/addons/formula90s/bin/vehicle_audio_engine.dll");
	candidate_paths.append("vehicle_audio_engine.windows.template_release.x86_64.dll");
	candidate_paths.append("vehicle_audio_engine.windows.template_debug.x86_64.dll");
	candidate_paths.append("vehicle_audio_engine.dll");

	HMODULE hDll = nullptr;
	ProjectSettings *ps = ProjectSettings::get_singleton();
	String loaded_path = "";

	for (int i = 0; i < candidate_paths.size(); ++i) {
		String p = candidate_paths[i];
		String global_p = ps ? ps->globalize_path(p) : p;
		hDll = LoadLibraryW((LPCWSTR)global_p.utf16().get_data());
		if (hDll) {
			loaded_path = global_p;
			break;
		}
	}

	if (!hDll) {
		UtilityFunctions::printerr("[VehicleAudioControllerNative] Failed to load vehicle_audio_engine.dll from all candidates!");
		return false;
	}

	dll_handle_ = (void *)hDll;

	fn_abi_version_ = (FnAudioAbiVersion)GetProcAddress(hDll, "vehicle_audio_abi_version");
	fn_build_sha_ = (FnAudioBuildSha)GetProcAddress(hDll, "vehicle_audio_build_sha");
	fn_create_ = (FnAudioCreate)GetProcAddress(hDll, "vehicle_audio_create");
	fn_destroy_ = (FnAudioDestroy)GetProcAddress(hDll, "vehicle_audio_destroy");
	fn_set_state_ = (FnAudioSetState)GetProcAddress(hDll, "vehicle_audio_set_state");
	fn_set_telemetry_ = (FnAudioSetTelemetry)GetProcAddress(hDll, "vehicle_audio_set_telemetry");
	fn_trigger_ = (FnAudioTrigger)GetProcAddress(hDll, "vehicle_audio_trigger");
	fn_render_ = (FnAudioRender)GetProcAddress(hDll, "vehicle_audio_render");

	const uint32_t abi_ver = fn_abi_version_ ? fn_abi_version_() : 0;
	const char *build_sha = fn_build_sha_ ? fn_build_sha_() : "unknown";
	UtilityFunctions::print(String("[VehicleAudioControllerNative]\nDLL=") + loaded_path +
		"\nABI=" + String::num_int64(abi_ver) + "\nBUILD=" + String(build_sha));

	if (abi_ver != EXPECTED_ABI_VERSION) {
		UtilityFunctions::printerr(String("[VehicleAudioControllerNative] FATAL: ABI mismatch! Expected ") +
			String::num_int64(EXPECTED_ABI_VERSION) + " but loaded DLL has " + String::num_int64(abi_ver));
		unload_dll();
		return false;
	}
	if (String(build_sha) != recorded_source_sha || String(build_sha) == "unknown") {
		UtilityFunctions::printerr(String("[VehicleAudioControllerNative] FATAL: BUILD mismatch! Expected ") +
			recorded_source_sha + " but loaded DLL has " + String(build_sha));
		unload_dll();
		return false;
	}
	if (!fn_create_ || (!fn_set_telemetry_ && !fn_set_state_) || !fn_render_ || !fn_destroy_) {
		UtilityFunctions::printerr("[VehicleAudioControllerNative] Missing required exported symbols in vehicle_audio_engine.dll!");
		unload_dll();
		return false;
	}

	// Create the engine with the real OS path to the v10_vehicle bank.
	const String bank_res = "res://sounds/banks/v10_vehicle";
	const String bank_global = ps ? ps->globalize_path(bank_res) : bank_res;
	CharString cs = bank_global.utf8();
	engine_ = fn_create_(cs.get_data());
	if (!engine_) {
		UtilityFunctions::printerr("[VehicleAudioControllerNative] Rust engine failed to load bank at " + bank_global);
		return false;
	}
	UtilityFunctions::print("[VehicleAudioControllerNative] Rust audio core ready (bank=" + bank_global + ")");
	return true;
#else
	UtilityFunctions::printerr("[VehicleAudioControllerNative] Only Windows runtime is supported in this build.");
	return false;
#endif
}

void VehicleAudioControllerNative::unload_dll() {
	if (engine_ && fn_destroy_) {
		fn_destroy_(engine_);
		engine_ = nullptr;
	}
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
	fn_set_state_ = nullptr;
	fn_set_telemetry_ = nullptr;
	fn_trigger_ = nullptr;
	fn_render_ = nullptr;
}

void VehicleAudioControllerNative::ensure_vehicle_bus() {
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}
	AudioServer *as = AudioServer::get_singleton();
	if (!as) {
		return;
	}
	// Create the "Vehicle" bus if missing. The clipping safety net is provided by
	// the Rust core's per-sample limiter (it clamps the final mix), so we do not
	// attempt to instantiate an AudioEffectLimiter here (that class is not exposed
	// by the godot-cpp bindings in this build). If a limiter is already present on
	// the bus (e.g. configured in the project's bus layout) it is preserved.
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

void VehicleAudioControllerNative::create_audio_nodes() {
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}
	if (OS::get_singleton()->has_feature("headless") || AudioServer::get_singleton()->get_driver_name() == "Dummy") {
		return;
	}
	generator.instantiate();
	generator->set_mix_rate_mode(AudioStreamGenerator::MIX_RATE_CUSTOM);
	generator->set_mix_rate(44100);
	generator->set_buffer_length(0.06f);
	// The engine is a non-positional source. The original GDScript used a non-positional
	// AudioStreamPlayer, which needs no AudioListener3D and is audible from any viewport
	// (including the runtime SubViewport). This godot-cpp build only wraps
	// AudioStreamPlayer3D, so we instantiate the non-3D player generically via ClassDB
	// and drive it through the Object/Variant API. AudioStreamPlayer3D would be silent
	// because the project has no AudioListener3D.
	Object *p = ClassDB::instantiate("AudioStreamPlayer");
	if (!p) {
		UtilityFunctions::printerr("[VehicleAudioControllerNative] Failed to instantiate AudioStreamPlayer.");
		return;
	}
	Node *pn = Object::cast_to<Node>(p);
	p->set("stream", generator);
	p->set("bus", String("Vehicle"));
	add_child(pn);
	p->call("play");
	audio_player_ = p;
	// get_stream_playback() is not guaranteed on the frame the player starts;
	// resolve it lazily in the pump instead of gating initialization on it.
	audio_playback_ = nullptr;
	audio_initialized_ = true;
}

void VehicleAudioControllerNative::update_telemetry_snapshot(double rpm, double idle, double maxr, double throttle, int, double speed_kph, double slip, const String &surface) {
	const double norm = (maxr > idle) ? std::clamp((rpm - idle) / (maxr - idle), 0.0, 1.0) : 0.0;
	float w[5] = { 0.0f, 0.0f, 0.0f, 0.0f, 0.0f };
	float sum = 0.0f;
	for (int i = 0; i < 5; ++i) {
		w[i] = std::max(0.0f, 1.0f - std::abs((float)norm - BAND_CENTERS_[i]) / BAND_WIDTH_);
		sum += w[i];
	}
	if (sum <= 0.0f) {
		w[0] = 1.0f;
		for (int i = 1; i < 5; ++i) w[i] = 0.0f;
		sum = 1.0f;
	}
	for (int i = 0; i < 5; ++i) {
		last_weights_[i] = w[i] / sum;
		last_pitches_[i] = (float)std::clamp(rpm / (double)ENGINE_BAND_NATIVE_RPM_[i], 0.5, 3.5);
	}
	last_engine_gain_ = 0.45f + 0.55f * (float)throttle;
	last_norm_ = (float)norm;
	last_rpm_ = rpm;
	last_throttle_ = (float)throttle;
	last_speed_kph_ = speed_kph;
	last_slip_ = (float)slip;
	surface_ = surface;
	if (surface == "rumble") {
		active_bed_ = "surf_rumble";
	} else if (surface == "grass") {
		active_bed_ = "surf_grass";
	} else if (surface == "sand") {
		active_bed_ = "surf_sand";
	} else {
		active_bed_ = "";
	}
}

String VehicleAudioControllerNative::surface_token_from_wheel_type(const String &st) {
	if (st == " Grass" || st == "grass") {
		return "grass";
	}
	if (st == "Curb" || st == "curb" || st == "rumble") {
		return "rumble";
	}
	if (st == "Gravel" || st == "Dirt" || st == "Sand" || st == "gravel" || st == "dirt" || st == "sand") {
		return "sand";
	}
	return "asphalt";
}

String VehicleAudioControllerNative::detect_surface(Node *vehicle) {
	if (!vehicle) {
		return "asphalt";
	}
	int cnt_grass = 0, cnt_sand = 0, cnt_rumble = 0;

	// Rust F194RustVehicle path: it exposes per-wheel surface codes via
	// get_wheel_surface_types() (0=Road, 1=Curb, 2=Dirt, 3=Grass, 4=Gravel,
	// 5=Sand, 6=Wall, 7=Metal). This is the authoritative source for the Rust
	// vehicle; the GEVP `axle`/`Wheel*` RayCast fallbacks below do not apply to it.
	if (vehicle->has_method("get_wheel_surface_types")) {
		Variant v = vehicle->call("get_wheel_surface_types");
		if (v.get_type() == Variant::PACKED_INT64_ARRAY) {
			PackedInt64Array codes = v;
			for (int i = 0; i < codes.size(); ++i) {
				switch ((int)codes[i]) {
					case 1: cnt_rumble++; break;                              // Curb
					case 2: case 4: case 5: cnt_sand++; break;                // Dirt / Gravel / Sand
					case 3: cnt_grass++; break;                                // Grass
					default: break;                                            // Road/asphalt and others
				}
			}
		}
		// Fall through to the shared token-resolution tail below.
	}

	// Primary source: GEVP axle wheels expose `surface_type`.
	Variant axle = vehicle->get("axle");
	if (axle.get_type() != Variant::NIL) {
		Variant wheels = axle.get("wheels");
		if (wheels.get_type() == Variant::ARRAY) {
			Array arr = wheels;
			for (int i = 0; i < arr.size(); ++i) {
				Object *w = Object::cast_to<Object>(arr[i]);
				if (!w) {
					continue;
				}
				String st = w->get("surface_type");
				String tok = surface_token_from_wheel_type(st);
				if (tok == "rumble") {
					cnt_rumble++;
				} else if (tok == "grass") {
					cnt_grass++;
				} else if (tok == "sand") {
					cnt_sand++;
				}
			}
		}
	}

	// Fallback: collision groups of wheel RayCasts (by fixed names).
	const char *wheel_names[4] = { "WheelFrontLeft", "WheelFrontRight", "WheelRearLeft", "WheelRearRight" };
	for (int i = 0; i < 4; ++i) {
		RayCast3D *ray = Object::cast_to<RayCast3D>(vehicle->get_node_or_null(wheel_names[i]));
		if (ray && ray->is_colliding()) {
			Object *col = ray->get_collider();
			Node *cn = Object::cast_to<Node>(col);
			if (cn) {
				if (cn->is_in_group("Grass")) {
					cnt_grass++;
				} else if (cn->is_in_group("Curb")) {
					cnt_rumble++;
				} else if (cn->is_in_group("Gravel") || cn->is_in_group("Dirt") || cn->is_in_group("Sand")) {
					cnt_sand++;
				}
			}
		}
	}

	if (cnt_rumble > 0) {
		return "rumble";
	}
	if (cnt_grass > 0) {
		return "grass";
	}
	if (cnt_sand > 0) {
		return "sand";
	}
	return "asphalt";
}

double VehicleAudioControllerNative::aggregate_slip(Node *vehicle) {
	double slip = 0.0;
	if (vehicle && vehicle->has_method("get_wheel_slips")) {
		Variant v = vehicle->call("get_wheel_slips");
		if (v.get_type() == Variant::PACKED_FLOAT64_ARRAY) {
			PackedFloat64Array arr = v;
			for (int i = 0; i < arr.size(); ++i) {
				slip = std::max(slip, std::abs((double)arr[i]));
			}
		}
	}
	return slip;
}

void VehicleAudioControllerNative::_ready() {
	add_to_group("vehicle_audio");
	set_physics_process(true);
	if (Engine::get_singleton()->is_editor_hint()) {
		return;
	}
	const bool dll_ok = load_dll();
	ensure_vehicle_bus();
	create_audio_nodes();
	if (!dll_ok) {
		UtilityFunctions::push_warning("[VehicleAudioControllerNative] Audio core unavailable; telemetry only.");
	}
}

void VehicleAudioControllerNative::_physics_process(double delta) {
	Node *vehicle = get_node_or_null(vehicle_path_);
	if (!vehicle) {
		return;
	}

	const double rpm = (double)vehicle->get("motor_rpm");
	const double throttle = (double)vehicle->get("throttle_amount");
	const int gear = (int)vehicle->get("current_gear");
	const double speed_kph = (double)vehicle->get("speed") * 3.6;
	const Variant idle_v = vehicle->get("idle_rpm");
	const Variant max_v = vehicle->get("max_rpm");
	const double idle = (idle_v.get_type() != Variant::NIL) ? (double)idle_v : (double)idle_rpm_;
	const double maxr = (max_v.get_type() != Variant::NIL) ? (double)max_v : (double)max_rpm_;

	const Variant et_v = vehicle->get("engine_torque");
	const double engine_torque = (et_v.get_type() != Variant::NIL) ? (double)et_v : 0.0;

	const Variant ct_v = vehicle->get("clutch_torque");
	const double clutch_torque = (ct_v.get_type() != Variant::NIL) ? (double)ct_v : 0.0;

	const Variant ce_v = vehicle->get("clutch_engagement");
	const double clutch_engagement = (ce_v.get_type() != Variant::NIL) ? (double)ce_v : 1.0;

	const Variant mt_v = vehicle->get("max_torque");
	const double max_torque = (mt_v.get_type() != Variant::NIL) ? (double)mt_v : 455.0;

	const Variant tc_v = vehicle->get("tc_cut_ratio");
	const double tc_cut = (tc_v.get_type() != Variant::NIL) ? (double)tc_v : 0.0;

	const Variant rl_v = vehicle->get("is_rev_limited");
	const bool rev_limited = (rl_v.get_type() != Variant::NIL) ? (bool)rl_v : (rpm >= maxr - 50.0);

	const String surface = detect_surface(vehicle);
	const double slip = aggregate_slip(vehicle);

	update_telemetry_snapshot(rpm, idle, maxr, throttle, gear, speed_kph, slip, surface);

	// Temporal derivatives (EPIC E)
	if (delta <= 0.0 || delta > 0.2 || !std::isfinite(delta) || !derivatives_initialized_) {
		prev_rpm_ = rpm;
		prev_throttle_ = throttle;
		filtered_rpm_derivative_ = 0.0f;
		filtered_throttle_derivative_ = 0.0f;
		derivatives_initialized_ = true;
	} else {
		const double raw_rpm_dot = (rpm - prev_rpm_) / delta;
		const double raw_throttle_dot = (throttle - prev_throttle_) / delta;
		const float clamped_rpm_dot = (float)std::clamp(raw_rpm_dot, -100000.0, 100000.0);
		const float clamped_throttle_dot = (float)std::clamp(raw_throttle_dot, -50.0, 50.0);

		const float tau_rpm = 0.030f;
		const float alpha_rpm = (float)(delta / (tau_rpm + delta));
		filtered_rpm_derivative_ += alpha_rpm * (clamped_rpm_dot - filtered_rpm_derivative_);

		const float tau_throttle = 0.020f;
		const float alpha_throttle = (float)(delta / (tau_throttle + delta));
		filtered_throttle_derivative_ += alpha_throttle * (clamped_throttle_dot - filtered_throttle_derivative_);

		prev_rpm_ = rpm;
		prev_throttle_ = throttle;
	}

	// Normalized torque and sign (EPIC D-01, D-02)
	const float denom_torque = (float)(max_torque > 1e-4 ? max_torque : 455.0);
	const float normalized_engine_torque = std::clamp((float)(engine_torque / denom_torque), -1.0f, 1.0f);

	int torque_sign = 0;
	if (normalized_engine_torque > 0.025f) {
		torque_sign = 1;
	} else if (normalized_engine_torque < -0.025f) {
		torque_sign = -1;
	}

	// Coupled engine load (EPIC D-03, D-04)
	const float max_clutch_torque_ratio = 1.6f;
	const float clutch_capacity = denom_torque * max_clutch_torque_ratio;
	const float coupled_load = std::clamp((float)(std::abs(clutch_torque) / std::max(clutch_capacity, 1e-4f)), 0.0f, 1.0f);
	const float tc_delivery = 1.0f - std::clamp((float)tc_cut, 0.0f, 1.0f);
	float normalized_engine_load = std::clamp(coupled_load * tc_delivery, 0.0f, 1.0f);

	if (!std::isfinite(normalized_engine_load) || (ct_v.get_type() == Variant::NIL && et_v.get_type() != Variant::NIL)) {
		normalized_engine_load = std::abs(normalized_engine_torque);
	}

	// Shift phase state machine (EPIC F-01..F-05)
	if (gear != last_gear_) {
		if (last_gear_ != 0) {
			if (gear > last_gear_) {
				current_shift_phase_ = SHIFT_PHASE_UPSHIFT_RECOVERY;
				shift_recovery_timer_ = 0.065;
				last_trigger_ = "shift_up";
			} else {
				current_shift_phase_ = SHIFT_PHASE_DOWNSHIFT_RECOVERY;
				shift_recovery_timer_ = 0.080;
				last_trigger_ = "shift_down";
			}
		}
		last_gear_ = gear;
	} else if (shift_recovery_timer_ > 0.0) {
		shift_recovery_timer_ -= delta;
		if (shift_recovery_timer_ <= 0.0) {
			shift_recovery_timer_ = 0.0;
			current_shift_phase_ = SHIFT_PHASE_NONE;
		} else if (current_shift_phase_ == SHIFT_PHASE_DOWNSHIFT_RECOVERY) {
			if (filtered_throttle_derivative_ > 2.0f || filtered_rpm_derivative_ > 3000.0f) {
				current_shift_phase_ = SHIFT_PHASE_DOWNSHIFT_BLIP;
			}
		}
	} else {
		current_shift_phase_ = SHIFT_PHASE_NONE;
	}

	last_normalized_engine_load_ = normalized_engine_load;
	last_normalized_engine_torque_ = normalized_engine_torque;
	last_torque_sign_ = torque_sign;
	last_rpm_derivative_ = filtered_rpm_derivative_;
	last_throttle_derivative_ = filtered_throttle_derivative_;
	last_shift_phase_ = current_shift_phase_;
	last_clutch_engagement_ = (float)clutch_engagement;
	last_tc_cut_ratio_ = (float)tc_cut;
	last_rev_limiter_active_ = rev_limited;

	if (!engine_) {
		return;
	}

	// Push telemetry into the Rust core.
	CharString surf = surface.utf8();
	if (fn_set_telemetry_) {
		VehicleAudioTelemetryV3 packet = {};
		packet.schema_version = EXPECTED_ABI_VERSION;
		packet.struct_size = sizeof(VehicleAudioTelemetryV3);
		packet.rpm = rpm;
		packet.idle_rpm = idle;
		packet.max_rpm = maxr;
		packet.throttle = (float)throttle;
		packet.normalized_engine_load = normalized_engine_load;
		packet.normalized_engine_torque = normalized_engine_torque;
		packet.rpm_derivative = filtered_rpm_derivative_;
		packet.throttle_derivative = filtered_throttle_derivative_;
		packet.speed_kph = speed_kph;
		packet.slip = (float)slip;
		packet.gear = gear;
		packet.torque_sign = torque_sign;
		packet.shift_phase = current_shift_phase_;
		packet.clutch_engagement = (float)clutch_engagement;
		packet.tc_cut_ratio = (float)tc_cut;
		packet.rev_limiter_active = rev_limited ? 1u : 0u;

		fn_set_telemetry_(engine_, &packet, surf.get_data());
	} else if (fn_set_state_) {
		fn_set_state_(engine_, rpm, idle, maxr, (float)throttle, speed_kph, gear, (float)slip, surf.get_data());
	}

	// Render into the AudioStreamGenerator buffer.
	if (audio_initialized_) {
		// Playback may not be available yet; resolve it lazily and retry each tick.
		if (audio_playback_ == nullptr && audio_player_ != nullptr) {
			audio_playback_ = audio_player_->call("get_stream_playback");
		}
		if (audio_playback_ == nullptr) {
			return;
		}
		const int frames = (int)audio_playback_->call("get_frames_available");
		if (frames > 0) {
			// Reuse member buffers; only grow when the frame count increases.
			if ((int)mix_l_.size() < frames) {
				mix_l_.resize(frames);
				mix_r_.resize(frames);
			}
			fn_render_(engine_, mix_l_.data(), mix_r_.data(), (uint32_t)frames);
			for (int i = 0; i < frames; ++i) {
				audio_playback_->call("push_frame", Vector2(mix_l_[i], mix_r_[i]));
			}
		}
	}
}

void VehicleAudioControllerNative::_exit_tree() {
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
	generator.unref();
	unload_dll();
}
