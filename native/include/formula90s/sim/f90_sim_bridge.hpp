#pragma once
#include "f90_sim_bridge.h"
#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/variant/transform3d.hpp>

namespace godot {

// Thin Godot-side bridge for the authoritative Rust simulation core (`game_sim`).
// It loads `game_sim.dll`, owns a `World`, steps it every physics frame, and
// reflects the core's pose/telemetry onto THIS node's transform + prints them.
// The core is the single simulator; Godot is a mirror (snapshot-server model).
class F90SimBridge : public Node3D {
	GDCLASS(F90SimBridge, Node3D)

private:
	void *world_ = nullptr;
	void *dll_handle_ = nullptr;
	uint32_t entity_id_ = 0;

	FnSimWorldCreate fn_create_ = nullptr;
	FnSimWorldDestroy fn_destroy_ = nullptr;
	FnSimWorldSpawnFromJson fn_spawn_json_ = nullptr;
	FnSimWorldSpawnCanonical fn_spawn_canonical_ = nullptr;
	FnSimWorldSetInput fn_set_input_ = nullptr;
	FnSimWorldStep fn_step_ = nullptr;
	FnSimWorldPose fn_pose_ = nullptr;
	FnSimWorldTelemetry fn_telemetry_ = nullptr;
	FnSimWorldFlatSamples fn_flat_samples_ = nullptr;

	double fixed_dt_ = 1.0 / 120.0;
	String config_json_path_ = "res://data/vehicles/f1_94/f1_94_physics.json";
	bool use_canonical_config_ = false;
	double telemetry_print_accum_ = 0.0;

	bool load_dll();
	void unload_dll();
	void spawn_vehicle();

protected:
	static void _bind_methods();

public:
	F90SimBridge();
	~F90SimBridge() override;

	void _ready() override;
	void _physics_process(double delta) override;
	void _exit_tree() override;

	void set_fixed_dt(double v) { fixed_dt_ = v; }
	double get_fixed_dt() const { return fixed_dt_; }
	void set_config_json_path(const String &p) { config_json_path_ = p; }
	String get_config_json_path() const { return config_json_path_; }
	void set_use_canonical_config(bool v) { use_canonical_config_ = v; }
	bool get_use_canonical_config() const { return use_canonical_config_; }
};

} // namespace godot
