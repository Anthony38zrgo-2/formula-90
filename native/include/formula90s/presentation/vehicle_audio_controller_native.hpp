#pragma once

#include <godot_cpp/classes/node.hpp>
#include <godot_cpp/classes/audio_stream_generator.hpp>
#include <godot_cpp/classes/audio_stream_generator_playback.hpp>
#include <godot_cpp/classes/audio_server.hpp>
#include <godot_cpp/classes/project_settings.hpp>
#include <godot_cpp/classes/os.hpp>
#include <godot_cpp/classes/ray_cast3d.hpp>
#include <godot_cpp/classes/engine.hpp>
#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <godot_cpp/variant/packed_float32_array.hpp>
#include <godot_cpp/variant/packed_float64_array.hpp>

#include <string>
#include <vector>

#ifdef _WIN32
#include <windows.h>
#else
#include <dlfcn.h>
#endif

namespace godot {

#pragma pack(push, 8)
struct VehicleAudioTelemetryV3 {
	uint32_t schema_version;
	uint32_t struct_size;

	double rpm;
	double idle_rpm;
	double max_rpm;

	float throttle;
	float normalized_engine_load;
	float normalized_engine_torque;
	float rpm_derivative;
	float throttle_derivative;

	double speed_kph;
	float slip;

	int32_t gear;
	int32_t torque_sign;
	int32_t shift_phase;

	float clutch_engagement;
	float tc_cut_ratio;
	uint32_t rev_limiter_active;
};
#pragma pack(pop)

static_assert(sizeof(VehicleAudioTelemetryV3) == 96, "VehicleAudioTelemetryV3 size must be 96 bytes");

/// Audio controller that drives the pure-Rust `vehicle_audio_engine` cdylib.
///
/// Mirrors the physics GDExtension pattern (`f1_94_rust_vehicle.cpp`): the cdylib
/// is loaded at runtime and called via a stable C ABI. Each frame we push vehicle
/// telemetry into the Rust core, render samples from its sample-accurate mixer,
/// and feed them into an `AudioStreamGenerator` on the `Vehicle` bus (which has a
/// limiter as a safety net). All mixing rules live in Rust; this node is only
/// glue + telemetry gathering. Drop-in replacement for `VehicleAudioController`
/// (GDScript) for the F1-94 vehicle.
class VehicleAudioControllerNative : public Node {
	GDCLASS(VehicleAudioControllerNative, Node)

public:
	static constexpr uint32_t EXPECTED_ABI_VERSION = 3;

	// Typedefs for the Rust C ABI.
	typedef uint32_t (*FnAudioAbiVersion)();
	typedef const char *(*FnAudioBuildSha)();
	typedef void *(*FnAudioCreate)(const char *);
	typedef void (*FnAudioDestroy)(void *);
	typedef void (*FnAudioSetState)(void *, double, double, double, float, double, int, float, const char *);
	typedef bool (*FnAudioSetTelemetry)(void *, const VehicleAudioTelemetryV3 *, const char *);
	typedef void (*FnAudioTrigger)(void *, int);
	typedef uint32_t (*FnAudioRender)(void *, float *, float *, uint32_t);

	VehicleAudioControllerNative();
	~VehicleAudioControllerNative();

	void _ready() override;
	void _physics_process(double delta) override;
	void _exit_tree() override;

	// Telemetry / control surface (read by audio_telemetry.gd).
	void set_vehicle(const NodePath &p) { vehicle_path_ = p; }
	NodePath get_vehicle() const { return vehicle_path_; }
	void set_idle_rpm(float v) { idle_rpm_ = v; }
	float get_idle_rpm() const { return idle_rpm_; }
	void set_max_rpm(float v) { max_rpm_ = v; }
	float get_max_rpm() const { return max_rpm_; }

	float get_last_norm() const { return last_norm_; }
	double get_last_rpm() const { return last_rpm_; }
	float get_last_throttle() const { return last_throttle_; }
	float get_last_slip() const { return last_slip_; }
	double get_last_speed_kph() const { return last_speed_kph_; }
	float get_last_engine_gain() const { return last_engine_gain_; }
	float get_last_normalized_engine_load() const { return last_normalized_engine_load_; }
	float get_last_normalized_engine_torque() const { return last_normalized_engine_torque_; }
	int get_last_torque_sign() const { return last_torque_sign_; }
	float get_last_rpm_derivative() const { return last_rpm_derivative_; }
	float get_last_throttle_derivative() const { return last_throttle_derivative_; }
	int get_last_shift_phase() const { return last_shift_phase_; }
	float get_last_clutch_engagement() const { return last_clutch_engagement_; }
	float get_last_tc_cut_ratio() const { return last_tc_cut_ratio_; }
	bool get_last_rev_limiter_active() const { return last_rev_limiter_active_; }
	PackedFloat32Array get_last_weights() const;
	PackedFloat32Array get_last_pitches() const;
	String get_last_trigger() const { return last_trigger_; }
	String get_surface() const { return surface_; }
	String get_active_bed() const { return active_bed_; }
	PackedFloat64Array get_engine_band_native_rpm() const;
	bool is_engine_loaded() const { return engine_ != nullptr; }
	bool is_audio_active() const { return audio_initialized_; }

	/// Fire a named one-shot (e.g. "shift_up", "impact_barrier", "engine_backfire").
	void trigger(const String &name);

protected:
	static void _bind_methods();

private:
	bool load_dll();
	void unload_dll();
	void ensure_vehicle_bus();
	void create_audio_nodes();
	void update_telemetry_snapshot(double rpm, double idle, double maxr, double throttle, int gear, double speed_kph, double slip, const String &surface);
	String detect_surface(Node *vehicle);
	String surface_token_from_wheel_type(const String &st);
	double aggregate_slip(Node *vehicle);
	static int trigger_code(const String &name);

	void *dll_handle_ = nullptr;
	void *engine_ = nullptr;
	FnAudioAbiVersion fn_abi_version_ = nullptr;
	FnAudioBuildSha fn_build_sha_ = nullptr;
	FnAudioCreate fn_create_ = nullptr;
	FnAudioDestroy fn_destroy_ = nullptr;
	FnAudioSetState fn_set_state_ = nullptr;
	FnAudioSetTelemetry fn_set_telemetry_ = nullptr;
	FnAudioTrigger fn_trigger_ = nullptr;
	FnAudioRender fn_render_ = nullptr;

	NodePath vehicle_path_ = NodePath("..");
	float idle_rpm_ = 1000.0f;
	float max_rpm_ = 15000.0f;

	Ref<AudioStreamGenerator> generator;
	// The non-positional AudioStreamPlayer is not wrapped by this godot-cpp build, so it
	// is instantiated generically via ClassDB and driven through the Object/Variant API.
	Object *audio_player_ = nullptr;
	Object *audio_playback_ = nullptr;

	int last_gear_ = 0;
	bool audio_initialized_ = false;

	// Temporal derivative tracking (EPIC E)
	double prev_rpm_ = 0.0;
	double prev_throttle_ = 0.0;
	float filtered_rpm_derivative_ = 0.0f;
	float filtered_throttle_derivative_ = 0.0f;
	bool derivatives_initialized_ = false;

	// Shift phase state machine (EPIC F)
	enum ShiftPhaseCode {
		SHIFT_PHASE_NONE = 0,
		SHIFT_PHASE_UPSHIFT_CUT = 1,
		SHIFT_PHASE_UPSHIFT_RECOVERY = 2,
		SHIFT_PHASE_DOWNSHIFT_CUT = 3,
		SHIFT_PHASE_DOWNSHIFT_BLIP = 4,
		SHIFT_PHASE_DOWNSHIFT_RECOVERY = 5
	};
	int current_shift_phase_ = SHIFT_PHASE_NONE;
	double shift_recovery_timer_ = 0.0;

	// Reused sample buffers (resized only when the frame count grows) so the
	// per-tick render path does no allocation — keeps the DSP steady under load.
	std::vector<float> mix_l_, mix_r_;

	// Telemetry snapshot (mirrors Rust mix; kept alive even if the DLL fails).
	float last_norm_ = 0.0f;
	double last_rpm_ = 0.0;
	float last_throttle_ = 0.0f;
	float last_slip_ = 0.0f;
	double last_speed_kph_ = 0.0;
	float last_engine_gain_ = 0.0f;
	float last_normalized_engine_load_ = 0.0f;
	float last_normalized_engine_torque_ = 0.0f;
	int last_torque_sign_ = 0;
	float last_rpm_derivative_ = 0.0f;
	float last_throttle_derivative_ = 0.0f;
	int last_shift_phase_ = 0;
	float last_clutch_engagement_ = 0.0f;
	float last_tc_cut_ratio_ = 0.0f;
	bool last_rev_limiter_active_ = false;
	float last_weights_[5] = { 0.0f, 0.0f, 0.0f, 0.0f, 0.0f };
	float last_pitches_[5] = { 1.0f, 1.0f, 1.0f, 1.0f, 1.0f };
	String last_trigger_ = "";
	String surface_ = "asphalt";
	String active_bed_ = "";

	static constexpr float BAND_CENTERS_[5] = { 0.0f, 0.25f, 0.5f, 0.75f, 1.0f };
	static constexpr float BAND_WIDTH_ = 0.25f;
	static constexpr float ENGINE_BAND_NATIVE_RPM_[5] = { 3941.0f, 7429.0f, 8196.0f, 5580.0f, 7687.0f };
};

} // namespace godot
