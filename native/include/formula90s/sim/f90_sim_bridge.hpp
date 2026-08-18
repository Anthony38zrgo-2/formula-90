#pragma once
#include "f90_sim_bridge.h"
#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/classes/physics_direct_body_state3d.hpp>
#include <godot_cpp/variant/transform3d.hpp>

namespace godot {

class F194RustVehicle;

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
	FnSimWorldSetPose fn_set_pose_ = nullptr;
	FnSimWorldSetPoseAndVelocity fn_set_pose_and_velocity_ = nullptr;
	FnSimWorldStepWithSamples fn_step_with_samples_ = nullptr;
	FnSimWorldSolveExternal fn_solve_external_ = nullptr;

	double fixed_dt_ = 1.0 / 120.0;
	String config_json_path_ = "res://data/vehicles/f1_94/f1_94_physics.json";
	bool use_canonical_config_ = false;
	NodePath target_vehicle_path_;
	F194RustVehicle *cached_veh_ = nullptr;
	double telemetry_print_accum_ = 0.0;
	double debug_throttle_ = 0.0; // 0 = use InputMap; >0 forces throttle (debug/demo)

	bool load_dll();
	void unload_dll();
	void spawn_vehicle();

	// Locate the first F194RustVehicle in the tree (used when target_vehicle_path is empty).
	F194RustVehicle *find_first_vehicle(Node *p_from);

	// Default demo: drive this node's own transform from the core.
	void drive_self(double delta);

protected:
	static void _bind_methods();

public:
	F90SimBridge();
	~F90SimBridge() override;

	void _ready() override;
	void _physics_process(double delta) override;
	void _exit_tree() override;

	// Snapshot-server wiring: called from F194RustVehicle::_integrate_forces. Samples the
	// vehicle's raycasts, steps the authoritative core, and applies the resulting body
	// velocity via the physics state. Runs in the integrate context so raycasts are fresh.
	void drive_integrate(class F194RustVehicle *veh, PhysicsDirectBodyState3D *state);

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
};

} // namespace godot
