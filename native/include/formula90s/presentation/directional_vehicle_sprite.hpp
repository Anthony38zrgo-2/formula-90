#pragma once

#include "formula90s/vehicle/vehicle_state_reader.hpp"
#include <godot_cpp/classes/sprite3d.hpp>

namespace godot {

class DirectionalVehicleSprite : public Sprite3D {
	GDCLASS(DirectionalVehicleSprite, Sprite3D)

	int orientation_count = 8;
	int selected_frame = 0;
	int candidate_frame = 0;
	bool allow_mirroring = true;
	double ground_offset = 0.08;
	double angular_offset = 0;
	double angular_hysteresis = 3.0;
	double minimum_visual_speed = 0.5;
	double velocity_direction_influence = 0.0;
	String metadata_path;
	PackedFloat32Array frame_angles;
	Vector3 visual_local_offset;
	bool metadata_loaded = false;
	bool hysteresis_held = false;
	uint64_t epoch = 0;
	uint64_t camera_instance_id = 0;
	double current_relative_angle = 0.0;

	VehicleStateReader car_state;
	NodePath car_path;

	bool load_metadata();
	int find_nearest_frame(double angle_degrees) const;

protected:
	static void _bind_methods();

public:
	DirectionalVehicleSprite();
	void _ready() override;
	void _process(double delta) override;

	int get_orientation_count() const { return orientation_count; }
	void set_orientation_count(int v) { orientation_count = v; }
	bool get_allow_mirroring() const { return allow_mirroring; }
	void set_allow_mirroring(bool v) { allow_mirroring = v; }
	double get_ground_offset() const { return ground_offset; }
	void set_ground_offset(double v) { ground_offset = v; }
	void set_metadata_path(const String &v) { metadata_path = v; }
	String get_metadata_path() const { return metadata_path; }
	void set_angular_offset(double v) { angular_offset = v; }
	double get_angular_offset() const { return angular_offset; }
	void set_angular_hysteresis(double v) { angular_hysteresis = v; }
	double get_angular_hysteresis() const { return angular_hysteresis; }
	void set_minimum_visual_speed(double v) { minimum_visual_speed = v; }
	double get_minimum_visual_speed() const { return minimum_visual_speed; }
	void set_velocity_direction_influence(double v) { velocity_direction_influence = v; }
	double get_velocity_direction_influence() const { return velocity_direction_influence; }
	void set_car_path(const NodePath &p) { car_path = p; }
	NodePath get_car_path() const { return car_path; }
	int get_selected_frame() const { return selected_frame; }
	int get_candidate_frame() const { return candidate_frame; }
	double get_current_relative_angle() const { return current_relative_angle; }
	bool is_hysteresis_held() const { return hysteresis_held; }
};

} // namespace godot
