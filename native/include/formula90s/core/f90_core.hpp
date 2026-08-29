#pragma once

#include "formula90s/core/f90_core.h"

#include <godot_cpp/classes/audio_server.hpp>
#include <godot_cpp/classes/audio_stream_generator.hpp>
#include <godot_cpp/classes/audio_stream_generator_playback.hpp>
#include <godot_cpp/classes/camera3d.hpp>
#include <godot_cpp/classes/engine.hpp>
#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/classes/physics_direct_body_state3d.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/classes/viewport.hpp>
#include <godot_cpp/variant/packed_float32_array.hpp>
#include <godot_cpp/variant/packed_float64_array.hpp>

#include <cstddef>
#include <string>
#include <vector>

namespace godot {

class F194RustVehicle;

// Static layout guards: the C struct MUST mirror the Rust `F90CoreFrameOut`
// (locked by `game/core/src/ffi.rs::layout_tests`). Keep BOTH in sync.
static_assert(offsetof(F90CoreFrameOut, force_x) == 0);
static_assert(offsetof(F90CoreFrameOut, force_z) == 16);
static_assert(offsetof(F90CoreFrameOut, torque_z) == 40);
static_assert(offsetof(F90CoreFrameOut, speed_kmh) == 48);
static_assert(offsetof(F90CoreFrameOut, rpm) == 56);
static_assert(offsetof(F90CoreFrameOut, gear) == 64);
static_assert(offsetof(F90CoreFrameOut, steer) == 72);
static_assert(offsetof(F90CoreFrameOut, throttle) == 80);
static_assert(offsetof(F90CoreFrameOut, lat_g) == 88);
static_assert(offsetof(F90CoreFrameOut, drive_torque) == 168);
static_assert(offsetof(F90CoreFrameOut, px) == 176);
static_assert(offsetof(F90CoreFrameOut, yaw) == 200);
static_assert(offsetof(F90CoreFrameOut, surface_code) == 256);
static_assert(offsetof(F90CoreFrameOut, active_bed_code) == 260);
static_assert(offsetof(F90CoreFrameOut, trigger_code) == 264);
static_assert(offsetof(F90CoreFrameOut, last_norm) == 268);
static_assert(offsetof(F90CoreFrameOut, last_rpm) == 272);
static_assert(offsetof(F90CoreFrameOut, last_throttle) == 280);
static_assert(offsetof(F90CoreFrameOut, last_speed_kph) == 288);
static_assert(offsetof(F90CoreFrameOut, last_slip) == 296);
static_assert(offsetof(F90CoreFrameOut, last_engine_gain) == 300);
static_assert(offsetof(F90CoreFrameOut, weights) == 304);
static_assert(offsetof(F90CoreFrameOut, pitches) == 324);
static_assert(offsetof(F90CoreFrameOut, tire_pressure_kpa) == 344);
static_assert(offsetof(F90CoreFrameOut, tire_tread_inner_c) == 376);
static_assert(offsetof(F90CoreFrameOut, tire_tread_center_c) == 408);
static_assert(offsetof(F90CoreFrameOut, tire_tread_outer_c) == 440);
static_assert(offsetof(F90CoreFrameOut, tire_carcass_c) == 472);
static_assert(offsetof(F90CoreFrameOut, tire_gas_c) == 504);
static_assert(offsetof(F90CoreFrameOut, brake_disc_c) == 536);
static_assert(offsetof(F90CoreFrameOut, brake_caliper_c) == 568);
static_assert(offsetof(F90CoreFrameOut, brake_hub_c) == 600);
static_assert(offsetof(F90CoreFrameOut, brake_rim_c) == 632);
static_assert(offsetof(F90CoreFrameOut, brake_efficiency) == 664);
static_assert(offsetof(F90CoreFrameOut, duct_mass_flow_kg_s) == 696);
static_assert(offsetof(F90CoreFrameOut, duct_drag_n) == 728);
static_assert(offsetof(F90CoreFrameOut, total_brake_duct_drag_n) == 760);
static_assert(offsetof(F90CoreFrameOut, brake_optimal_min_c) == 768);
static_assert(offsetof(F90CoreFrameOut, brake_optimal_max_c) == 776);
static_assert(offsetof(F90CoreFrameOut, brake_fade_start_c) == 784);
static_assert(offsetof(F90CoreFrameOut, brake_critical_c) == 792);
static_assert(offsetof(F90CoreFrameOut, brake_torque_nm) == 800);
static_assert(offsetof(F90CoreFrameOut, brake_spin_pre_rad_s) == 832);
static_assert(offsetof(F90CoreFrameOut, brake_spin_post_rad_s) == 864);
static_assert(offsetof(F90CoreFrameOut, brake_power_w) == 896);
static_assert(offsetof(F90CoreFrameOut, brake_energy_j) == 928);
static_assert(offsetof(F90CoreFrameOut, brake_disc_bulk_c) == 960);
static_assert(offsetof(F90CoreFrameOut, brake_surface_capacity_j_k) == 992);
static_assert(offsetof(F90CoreFrameOut, brake_bulk_capacity_j_k) == 1024);
static_assert(offsetof(F90CoreFrameOut, brake_surface_bulk_w_k) == 1056);
static_assert(offsetof(F90CoreFrameOut, brake_natural_cooling_w_k) == 1088);
static_assert(offsetof(F90CoreFrameOut, brake_speed_cooling_w_k) == 1120);
static_assert(offsetof(F90CoreFrameOut, brake_surface_to_bulk_heat_w) == 1152);
static_assert(offsetof(F90CoreFrameOut, underfloor_clearance_m) == 1184);
static_assert(offsetof(F90CoreFrameOut, underfloor_valid_mask) == 1224);
static_assert(offsetof(F90CoreFrameOut, underfloor_scrape_phase) == 1228);
static_assert(offsetof(F90CoreFrameOut, audio_scrape_cursor) == 1280);
static_assert(offsetof(F90CoreFrameOut, underfloor_compression_m) == 1288);
static_assert(offsetof(F90CoreFrameOut, underfloor_bottoming_phase) == 1408);
static_assert(offsetof(F90CoreFrameOut, underfloor_active_probe_mask) == 1428);
static_assert(offsetof(F90CoreFrameOut, underfloor_total_normal_force_n) == 1432);
static_assert(offsetof(F90CoreFrameOut, underfloor_bottoming_torque) == 1472);
static_assert(offsetof(F90CoreFrameOut, underfloor_rigid_contact_blend) == 1504);
static_assert(offsetof(F90CoreFrameOut, aero_total_downforce_n) == 1512);
static_assert(offsetof(F90CoreFrameOut, aero_balance_front) == 1648);
static_assert(offsetof(F90CoreFrameOut, wheel_drive_torque_nm) == 1656);
static_assert(offsetof(F90CoreFrameOut, tc_cut_ratio) == 1688);
static_assert(offsetof(F90CoreFrameOut, net_drive_power_w) == 1696);
static_assert(offsetof(F90CoreFrameOut, tc_enabled) == 1704);
static_assert(offsetof(F90CoreFrameOut, tc_eligible) == 1712);
static_assert(offsetof(F90CoreFrameOut, tc_gear_authority) == 1720);
static_assert(offsetof(F90CoreFrameOut, tc_slip_ratio) == 1744);
static_assert(offsetof(F90CoreFrameOut, wheel_drive_torque_pre_tc_nm) == 1776);
static_assert(offsetof(F90CoreFrameOut, pre_tc_drive_power_w) == 1808);
static_assert(sizeof(F90CoreFrameOut) == 1816);

/// The orchestrator node. Loads the SINGLE `formula90_core.dll` facade (one
/// handshake / one ABI version), owns the sim + audio, drives the vehicle inside
/// `_integrate_forces` via `f90_core_step`, and pumps the audio mixer with a single
/// batched `push_buffer` per call (no per-sample Variant dispatch).
///
/// Replaces the three independent loads (`F90SimBridge` + `VehicleAudioControllerNative`
/// + the legacy physics DLL path inside `F194RustVehicle`) with one controller.
class F90Core : public Node3D {
	GDCLASS(F90Core, Node3D)

public:
	static constexpr uint32_t EXPECTED_ABI_VERSION = 13;

	F90Core();
	~F90Core() override;

	void _ready() override;
	void _physics_process(double delta) override;
	void _process(double delta) override;
	void _exit_tree() override;

	/// Called from `F194RustVehicle::_integrate_forces` (fresh raycast context):
	/// samples rays, steps the facade (physics + modules + audio inputs), applies the
	/// resolved force/torque to the body and mirrors telemetry onto the vehicle.
	void drive_integrate(F194RustVehicle *veh, PhysicsDirectBodyState3D *state);

	// --- configuration (bound as Godot properties) ------------------------------
	void set_fixed_dt(double v) { fixed_dt_ = v; }
	double get_fixed_dt() const { return fixed_dt_; }
	void set_config_json_path(const String &p) { config_json_path_ = p; }
	String get_config_json_path() const { return config_json_path_; }
	void set_use_canonical_config(bool v) { use_canonical_config_ = v; }
	bool get_use_canonical_config() const { return use_canonical_config_; }
	void set_target_vehicle_path(const String &p) { target_vehicle_path_ = p; }
	String get_target_vehicle_path() const { return target_vehicle_path_; }
	void set_debug_throttle(double v) { debug_throttle_ = v; }
	double get_debug_throttle() const { return debug_throttle_; }
	void set_enable_audio(bool v) { enable_audio_ = v; }
	bool get_enable_audio() const { return enable_audio_; }
	void set_bank_dir(const String &p) { bank_dir_res_ = p; }
	String get_bank_dir() const { return bank_dir_res_; }
	void set_modules(const String &p) { modules_ = p; }
	String get_modules() const { return modules_; }
	void set_idle_rpm(float v) { idle_rpm_ = v; }
	float get_idle_rpm() const { return idle_rpm_; }
	void set_max_rpm(float v) { max_rpm_ = v; }
	float get_max_rpm() const { return max_rpm_; }

	// --- readouts for HUD / audio_telemetry.gd (same getter names as the legacy
	// VehicleAudioControllerNative so existing GDScript keeps working) -----------
	float get_last_norm() const { return frame_.last_norm; }
	double get_last_rpm() const { return frame_.last_rpm; }
	float get_last_throttle() const { return frame_.last_throttle; }
	float get_last_slip() const { return frame_.last_slip; }
	double get_last_speed_kph() const { return frame_.last_speed_kph; }
	float get_last_engine_gain() const { return frame_.last_engine_gain; }
	PackedFloat32Array get_last_weights() const;
	PackedFloat32Array get_last_pitches() const;
	String get_last_trigger() const;
	String get_surface() const;
	String get_active_bed() const;
	PackedFloat64Array get_engine_band_native_rpm() const;
	bool is_engine_loaded() const { return core_ != nullptr && entity_id_ != 0; }
	bool is_audio_active() const { return audio_initialized_; }

	/// Fire a one-shot by name (e.g. "shift_up", "impact_barrier", "engine_backfire").
	void trigger(const String &name);
	/// Set the ambient/listener downlink (camera-to-vehicle distance, TC cut ratio,
	/// limiter enabled flag) that the audio facade applies on the mixer.
	void set_audio_ambient(float tc_cut_ratio, bool limiter_active);
	/// Camera-to-vehicle listener distance (metres), smoothed in `_process`.
	float get_listener_distance_m() const { return static_cast<float>(listener_distance_smoothed_); }
	/// Reset the core to the vehicle's current pose/yaw (also resets modules).
	void reset_vehicle();
	/// Reset the facade's entity + modules to an explicit pose/yaw. Called from
	/// `F194RustVehicle::reset_vehicle` when this orchestrator drives the vehicle.
	void reset_core_at(double x, double y, double z, double yaw);
	/// Apply a runtime config (F90RuntimeConfig) to the facade entity — the GDScript
	/// tuning panel / vehicle setters keep working in bridge-controlled mode.
	void apply_runtime_config(const F90RuntimeConfig &cfg);

protected:
	static void _bind_methods();

private:
	bool load_dll();
	void unload_dll();
	F194RustVehicle *find_first_vehicle(Node *from);
	void ensure_vehicle_bus();
	void create_audio_nodes();
	/// Render available frames from the mixer and push them in ONE batched call.
	void pump_audio();
	static int trigger_code(const String &name);
	static const char *trigger_name(int code);
	F90CoreFrameOut frame_ = {};

	void *dll_handle_ = nullptr;
	void *core_ = nullptr;
	uint32_t entity_id_ = 0;

	// Single-handshake function pointers (see f90_core.h).
	FnCoreAbiVersion fn_abi_version_ = nullptr;
	FnCoreBuildSha fn_build_sha_ = nullptr;
	FnCoreCreate fn_create_ = nullptr;
	FnCoreDestroy fn_destroy_ = nullptr;
	FnCoreSpawn fn_spawn_ = nullptr;
	FnCoreReset fn_reset_ = nullptr;
	FnCoreApplyRuntimeConfig fn_apply_runtime_config_ = nullptr;
	FnCoreStep fn_step_ = nullptr;
	FnCoreAudioRender fn_audio_render_ = nullptr;
	FnCoreAudioTrigger fn_audio_trigger_ = nullptr;
	FnCoreAudioReadouts fn_audio_readouts_ = nullptr;
	FnCoreAudioSetAmbient fn_audio_set_ambient_ = nullptr;

	double fixed_dt_ = 1.0 / 120.0;
	String config_json_path_ = "res://data/vehicles/f1_94/f1_94_physics.json";
	bool use_canonical_config_ = false;
	NodePath target_vehicle_path_;
	F194RustVehicle *cached_veh_ = nullptr;
	double debug_throttle_ = 0.0;
	bool enable_audio_ = true;
	String bank_dir_res_ = "res://sounds/banks/v10_vehicle";
	String modules_ = ""; // comma-separated module names (e.g. "weather,ai")
	float idle_rpm_ = 1000.0f;
	float max_rpm_ = 15000.0f;

	// Listener/ambient downlink: smoothed camera-to-vehicle distance (m) plus the
	// TC/limiter telemetry forwarded to the mixer. Default limiter on so the
	// existing behaviour is preserved when nothing calls `set_audio_ambient`.
	double listener_distance_smoothed_ = 0.0;
	float tc_cut_ratio_ = 0.0f;
	bool limiter_active_ = true;

	// Audio plumbing (mirrors the legacy native controller, but batched).
	Ref<AudioStreamGenerator> generator_;
	Object *audio_player_ = nullptr;
	Object *audio_playback_ = nullptr;
	bool audio_initialized_ = false;
	std::vector<float> mix_l_, mix_r_;
	double collision_cooldown_ = 0.0;
	void process_collision_audio(F194RustVehicle *veh, PhysicsDirectBodyState3D *state, double dt);
	/// Smooth the camera-to-vehicle distance with a ~0.1 s one-pole in `_process`.
	void update_listener_distance(double delta);

	double telemetry_print_accum_ = 0.0;
};

} // namespace godot
