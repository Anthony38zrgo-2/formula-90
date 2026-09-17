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
#include <godot_cpp/variant/dictionary.hpp>
#include <godot_cpp/variant/packed_float32_array.hpp>
#include <godot_cpp/variant/packed_float64_array.hpp>

#include <atomic>
#include <cstddef>
#include <cstdint>
#include <string>
#include <thread>
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
static_assert(offsetof(F90CoreFrameOut, brake_rim_c) == 568);
static_assert(offsetof(F90CoreFrameOut, brake_efficiency) == 600);
static_assert(offsetof(F90CoreFrameOut, duct_mass_flow_kg_s) == 632);
static_assert(offsetof(F90CoreFrameOut, duct_drag_n) == 664);
static_assert(offsetof(F90CoreFrameOut, total_brake_duct_drag_n) == 696);
static_assert(offsetof(F90CoreFrameOut, brake_optimal_min_c) == 704);
static_assert(offsetof(F90CoreFrameOut, brake_optimal_max_c) == 712);
static_assert(offsetof(F90CoreFrameOut, brake_fade_start_c) == 720);
static_assert(offsetof(F90CoreFrameOut, brake_critical_c) == 728);
static_assert(offsetof(F90CoreFrameOut, brake_torque_nm) == 736);
static_assert(offsetof(F90CoreFrameOut, brake_spin_pre_rad_s) == 768);
static_assert(offsetof(F90CoreFrameOut, brake_spin_post_rad_s) == 800);
static_assert(offsetof(F90CoreFrameOut, brake_power_w) == 832);
static_assert(offsetof(F90CoreFrameOut, brake_energy_j) == 864);
static_assert(offsetof(F90CoreFrameOut, brake_natural_cooling_w_k) == 896);
static_assert(offsetof(F90CoreFrameOut, brake_speed_cooling_w_k) == 928);
static_assert(offsetof(F90CoreFrameOut, underfloor_clearance_m) == 960);
static_assert(offsetof(F90CoreFrameOut, underfloor_valid_mask) == 1000);
static_assert(offsetof(F90CoreFrameOut, underfloor_scrape_phase) == 1004);
static_assert(offsetof(F90CoreFrameOut, audio_scrape_cursor) == 1056);
static_assert(offsetof(F90CoreFrameOut, underfloor_compression_m) == 1064);
static_assert(offsetof(F90CoreFrameOut, underfloor_bottoming_phase) == 1184);
static_assert(offsetof(F90CoreFrameOut, underfloor_active_probe_mask) == 1204);
static_assert(offsetof(F90CoreFrameOut, underfloor_total_normal_force_n) == 1208);
static_assert(offsetof(F90CoreFrameOut, underfloor_bottoming_torque) == 1248);
static_assert(offsetof(F90CoreFrameOut, underfloor_rigid_contact_blend) == 1280);
static_assert(offsetof(F90CoreFrameOut, aero_total_downforce_n) == 1288);
static_assert(offsetof(F90CoreFrameOut, aero_balance_front) == 1416);
static_assert(offsetof(F90CoreFrameOut, wheel_drive_torque_nm) == 1424);
static_assert(offsetof(F90CoreFrameOut, tc_cut_ratio) == 1456);
static_assert(offsetof(F90CoreFrameOut, net_drive_power_w) == 1464);
static_assert(offsetof(F90CoreFrameOut, tc_enabled) == 1472);
static_assert(offsetof(F90CoreFrameOut, tc_eligible) == 1480);
static_assert(offsetof(F90CoreFrameOut, tc_gear_authority) == 1488);
static_assert(offsetof(F90CoreFrameOut, tc_slip_ratio) == 1512);
static_assert(offsetof(F90CoreFrameOut, wheel_drive_torque_pre_tc_nm) == 1544);
static_assert(offsetof(F90CoreFrameOut, pre_tc_drive_power_w) == 1576);
static_assert(offsetof(F90CoreFrameOut, underfloor_rigid_local_y) == 1584);
static_assert(offsetof(F90CoreFrameOut, underfloor_rigid_normal_impulse_ns) == 1592);
static_assert(sizeof(F90CoreFrameOut) == 1600);

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
	/// Audio output rate driven by the AudioStreamGenerator (Hz).
	static constexpr int kAudioMixRate = 44100;
	/// Legacy fixed per-pump sample cap. Kept only for the `audio_pump_mode == 0`
	/// diagnostic comparison; it underproduces below kAudioMixRate / cap FPS.
	static constexpr int kPumpBudgetFrames = 1024;
	/// Delta-budget scheduler (`audio_pump_mode == 1`): render the samples the
	/// mixer consumed during the last frame (ceil(rate * delta)) plus this
	/// catch-up allowance so a drained buffer can refill without ever falling
	/// below the steady-state consumption rate.
	static constexpr int kPumpCatchUpFrames = 256;
	/// Hard per-pump ceiling to bound worst-case `_process` cost after a hitch.
	static constexpr int kPumpMaxBatchFrames = 8192;
	/// Extra frames of generator pre-buffer on top of one frame's demand. Keeps
	/// the pipeline continuous without stacking a fixed backlog (latency).
	static constexpr int kGenHeadroomFrames = 128;
	/// Floor so delta mode never issues many tiny render batches.
	static constexpr int kPumpMinBatchFrames = 128;

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
	/// Runtime gate: disabling stops production immediately; re-enabling flushes
	/// any stale worker backlog so audio resumes at the live cursor.
	void set_enable_audio(bool v);
	bool get_enable_audio() const { return enable_audio_; }
	/// 0 = legacy fixed cap (kPumpBudgetFrames), 1 = elapsed-delta budget.
	void set_audio_pump_mode(int v) { audio_pump_mode_ = v; }
	int get_audio_pump_mode() const { return audio_pump_mode_; }
	/// Dedicated-core audio worker: the mixer runs on its own OS thread (pinned +
	/// above-normal priority + MMCSS "Pro Audio") instead of the render thread.
	/// Init-time only; falls back to the inline pump when unavailable.
	void set_audio_worker_enabled(bool v) { audio_worker_enabled_ = v; }
	bool get_audio_worker_enabled() const { return audio_worker_enabled_; }
	/// Logical processor for the worker (-1 = auto: best P-core, highest index).
	void set_audio_worker_core(int v) { audio_worker_core_ = v; }
	int get_audio_worker_core() const { return audio_worker_core_; }
	/// 0 = normal, 1 = above normal (default), 2 = highest thread priority.
	void set_audio_worker_priority(int v) { audio_worker_priority_ = v; }
	int get_audio_worker_priority() const { return audio_worker_priority_; }
	/// Per-stage audio pre-buffer target in milliseconds (runtime tunable).
	/// Total listener latency is ~2x this (worker ring + generator). Lower =
	/// tighter sync, higher = absorbs longer frame hitches without dropouts.
	void set_audio_latency_ms(int v) { audio_latency_ms_ = v < 1 ? 1 : (v > 200 ? 200 : v); }
	int get_audio_latency_ms() const { return audio_latency_ms_; }
	bool is_audio_worker_active() const { return audio_worker_active_; }
	/// Worker counters/affinity snapshot for diagnostics (schema in f90_core.h).
	Dictionary get_audio_worker_stats() const;
	// Audio pump diagnostics (cumulative; see reset_audio_stats()).
	int64_t get_audio_pump_calls() const { return audio_pump_calls_; }
	int64_t get_audio_frames_pushed() const { return audio_frames_pushed_; }
	int get_audio_last_available() const { return audio_last_available_; }
	int get_audio_max_available() const { return audio_max_available_; }
	int64_t get_audio_render_usec_total() const { return audio_render_usec_total_; }
	int64_t get_audio_skips() const { return audio_skips_; }
	/// RMS of the most recently pushed PCM block (0.0 == silence).
	float get_audio_output_rms() const { return audio_output_rms_; }
	/// Generator pre-buffer occupancy in frames (listener delay beyond the driver).
	int get_audio_gen_occupancy() const { return audio_gen_occupancy_; }
	int get_audio_gen_capacity() const { return audio_gen_capacity_; }
	/// push_buffer calls rejected by a full generator buffer since the last reset.
	int64_t get_audio_push_rejections() const { return audio_push_rejections_; }
	int get_audio_mix_rate() const { return kAudioMixRate; }
	double get_audio_buffer_length() const { return audio_buffer_length_; }
	void reset_audio_stats();
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
	/// Render the scheduled frames from the mixer and push them in ONE batched call.
	void pump_audio(double delta);
	/// Push one stereo block to the generator; records RMS + rejection counter.
	bool push_audio_batch(const float *left, const float *right, int frames);
	// --- dedicated audio worker (phase 2) ---------------------------------------
	void start_audio_worker();
	void stop_audio_worker();
	/// Pull the worker ring and push it in ONE batched call (no DSP here).
	/// `delta` sizes the ring/generator occupancy targets so buffering tracks
	/// the rendered frame time instead of adding a fixed backlog.
	void pump_audio_worker(double delta);
	/// Discard whatever the ring accumulated while audio was disabled.
	void flush_audio_worker_ring();
	/// Resolve the worker's logical processor (-1 auto -> best P-core).
	int resolve_audio_core() const;
	/// Affinity/priority/MMCSS setup that must run ON the worker thread.
	void configure_audio_worker_thread(int core_index);
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
	FnCoreAudioWorkerStart fn_audio_worker_start_ = nullptr;
	FnCoreAudioWorkerHandle fn_audio_worker_handle_ = nullptr;
	FnCoreAudioWorkerRun fn_audio_worker_run_ = nullptr;
	FnCoreAudioWorkerPull fn_audio_worker_pull_ = nullptr;
	FnCoreAudioWorkerStatsGet fn_audio_worker_stats_get_ = nullptr;
	FnCoreAudioWorkerSetTarget fn_audio_worker_set_target_ = nullptr;

	double fixed_dt_ = 1.0 / 120.0;
	String config_json_path_ = "res://data/vehicles/f1_2026_2008/f1_2026_2008_physics.json";
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
	// Audio pump scheduler + diagnostics.
	int audio_pump_mode_ = 1;
	int64_t audio_pump_calls_ = 0;
	int64_t audio_frames_pushed_ = 0;
	int audio_last_available_ = 0;
	int audio_max_available_ = 0;
	int64_t audio_render_usec_total_ = 0;
	int64_t audio_skips_ = 0;
	float audio_output_rms_ = 0.0f;
	int64_t audio_push_rejections_ = 0;
	double audio_buffer_length_ = 0.06;

	// Dedicated audio worker state. `audio_worker_stop_` is written by the main
	// thread and polled by the worker thread through the FFI run loop.
	bool audio_worker_enabled_ = true;
	int audio_worker_core_ = -1;
	int audio_worker_priority_ = 1;
	bool audio_worker_active_ = false;
	void *audio_worker_handle_ = nullptr;
	std::atomic<uint32_t> audio_worker_stop_{ 0 };
	std::thread audio_worker_thread_;
	std::atomic<uint32_t> audio_worker_core_observed_{ 0xFFFFFFFFu };
	std::atomic<int64_t> audio_worker_affinity_mask_{ 0 };
	std::atomic<int64_t> audio_worker_thread_id_{ 0 };
	bool audio_worker_timer_raised_ = false;
	int audio_latency_ms_ = 25;
	F90AudioWorkerStats audio_worker_stats_ = {};
	std::vector<float> worker_l_, worker_r_;
	// Measured generator ring: capacity (max free space seen when empty) and the
	// last observed occupancy. Used to hold the pre-buffer at the frame target.
	int audio_gen_capacity_ = 0;
	int audio_gen_occupancy_ = 0;
	// Pump accounting: distinguishes "the host asked for less than the mixer
	// consumed" (need too small) from "the worker ring could not supply" (unmet).
	int64_t audio_need_total_ = 0;
	int64_t audio_need_zero_calls_ = 0;
	int64_t audio_pull_unmet_frames_ = 0;
	int64_t audio_pump_entries_ = 0;
	int64_t audio_pump_no_room_ = 0;
	/// Frames the mixer is expected to consume before the next push (leaky
	/// integrator that keeps the generator's pre-buffer bounded).
	int64_t audio_push_deficit_ = 0;
	double collision_cooldown_ = 0.0;
	void process_collision_audio(F194RustVehicle *veh, PhysicsDirectBodyState3D *state, double dt);
	/// Smooth the camera-to-vehicle distance with a ~0.1 s one-pole in `_process`.
	void update_listener_distance(double delta);

	double telemetry_print_accum_ = 0.0;
};

} // namespace godot
