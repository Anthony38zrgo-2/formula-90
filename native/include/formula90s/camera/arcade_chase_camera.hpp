#pragma once

#include "formula90s/vehicle/vehicle_adapter.hpp"
#include <godot_cpp/classes/node3d.hpp>

namespace godot {
class Camera3D;

class ArcadeChaseCamera : public Node3D {
	GDCLASS(ArcadeChaseCamera, Node3D)

	double distance = 9.0;
	double height = 4.5;
	double follow_damping = 6.0;
	double horizontal_smoothing = 5.0;
	double vertical_smoothing = 7.5;
	double look_ahead = 4.0;
	double look_height = 1.0;
	double horizontal_dead_zone = 0.12;
	double vertical_dead_zone = 0.08;
	double velocity_anticipation = 0.0;
	double inertia_strength = 0.003;
	double maximum_camera_offset = 0.05;
	double offset_smoothing = 4.5;
	double lateral_swing = 2.1;
	double turn_look_offset = 1.5;
	double base_fov = 62.0;
	double speed_fov_gain = 4.0;
	double heading_smoothing = 5.0;
	double maximum_follow_lag = 1.25;
	double turn_offset_smoothing = 4.5;
	double turn_activation_speed = 0.75;
	double locked_world_y = 0.0;
	double locked_look_y = 0.0;
	double locked_pitch = 0.0;
	double smoothed_turn_amount = 0.0;
	double smoothed_longitudinal_inertia = 0.0;
	double previous_longitudinal_source = 0.0;
	Vector3 smoothed_velocity_lead;
	Vector3 smoothed_look_target;
	Vector3 filtered_acceleration;
	Vector3 smoothed_forward;
	bool initialized = false;
	bool inertia_initialized = false;

	VehicleAdapter car_adapter;
	Vector3 previous_world_velocity;
	Vector3 last_car_position;
	bool acceleration_initialized = false;
	uint64_t epoch = 0;

	NodePath car_path = "..";

protected:
	static void _bind_methods();

public:
	ArcadeChaseCamera();
	void _ready() override;
	void _process(double delta) override;
	void set_car_path(const NodePath &p) { car_path = p; }
	NodePath get_car_path() const { return car_path; }

#define CAMERA_ACCESSOR(name) \
	void set_##name(double v) { name = v; } \
	double get_##name() const { return name; }
	CAMERA_ACCESSOR(distance) CAMERA_ACCESSOR(height) CAMERA_ACCESSOR(follow_damping)
	CAMERA_ACCESSOR(horizontal_smoothing) CAMERA_ACCESSOR(vertical_smoothing) CAMERA_ACCESSOR(look_ahead) CAMERA_ACCESSOR(look_height)
	CAMERA_ACCESSOR(horizontal_dead_zone) CAMERA_ACCESSOR(vertical_dead_zone) CAMERA_ACCESSOR(velocity_anticipation)
	CAMERA_ACCESSOR(inertia_strength) CAMERA_ACCESSOR(maximum_camera_offset) CAMERA_ACCESSOR(offset_smoothing)
	CAMERA_ACCESSOR(lateral_swing) CAMERA_ACCESSOR(turn_look_offset) CAMERA_ACCESSOR(base_fov) CAMERA_ACCESSOR(speed_fov_gain)
	CAMERA_ACCESSOR(heading_smoothing) CAMERA_ACCESSOR(maximum_follow_lag)
	CAMERA_ACCESSOR(turn_offset_smoothing) CAMERA_ACCESSOR(turn_activation_speed)
#undef CAMERA_ACCESSOR
};

} // namespace godot
