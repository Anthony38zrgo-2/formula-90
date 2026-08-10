#pragma once

#include <godot_cpp/variant/transform3d.hpp>
#include <godot_cpp/variant/vector3.hpp>

namespace formula90s::camera {

struct ChaseCameraConfig {
	double distance = 9.0;
	double height = 4.5;
	double follow_damping = 6.0;
	double horizontal_smoothing = 5.0;
	double look_ahead = 4.0;
	double look_height = 1.0;
	double horizontal_dead_zone = 0.12;
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
};

struct ChaseCameraInput {
	godot::Transform3D vehicle_pose;
	godot::Vector3 linear_velocity;
	double speed_mps = 0.0;
	double steering_amount = 0.0;
};

struct ChaseCameraOutput {
	godot::Vector3 position;
	godot::Vector3 look_target;
	double fov = 60.0;
};

// Stateful math only: it does not resolve or mutate Godot scene nodes.
class ChaseCameraSolver {
	double locked_world_y = 0.0;
	double locked_look_y = 0.0;
	double smoothed_turn_amount = 0.0;
	double smoothed_longitudinal_inertia = 0.0;
	double previous_longitudinal_source = 0.0;
	godot::Vector3 smoothed_velocity_lead;
	godot::Vector3 smoothed_look_target;
	godot::Vector3 filtered_acceleration;
	godot::Vector3 smoothed_forward;
	godot::Vector3 previous_world_velocity;
	godot::Vector3 last_vehicle_position;
	bool initialized = false;
	bool inertia_initialized = false;
	bool acceleration_initialized = false;
	uint64_t epoch = 0;

public:
	ChaseCameraOutput reset(const ChaseCameraConfig &config, const godot::Transform3D &vehicle_pose);
	ChaseCameraOutput step(const ChaseCameraConfig &config, const ChaseCameraInput &input,
		const godot::Vector3 &current_position, double current_fov, double delta);
};

} // namespace formula90s::camera
