#include "formula90s/camera/chase_camera_solver.hpp"
#include "formula90s/camera/camera_math.hpp"
#include <godot_cpp/core/math.hpp>

using namespace godot;
using formula90s::camera::smoothing_alpha;

namespace {
double outside_dead_zone(double value, double zone) {
	const double magnitude = Math::abs(value);
	return magnitude <= zone ? 0.0 : Math::sign(value) * (magnitude - zone);
}

Vector3 clamp_length(const Vector3 &value, double maximum) {
	const double length = value.length();
	return length > maximum && length > 0.000001 ? value * (maximum / length) : value;
}
} // namespace

namespace formula90s::camera {

ChaseCameraOutput ChaseCameraSolver::reset(const ChaseCameraConfig &config, const Transform3D &vehicle_pose) {
	Vector3 forward = -vehicle_pose.basis.get_column(2);
	forward.y = 0;
	forward.normalize();
	smoothed_forward = forward;
	locked_world_y = vehicle_pose.origin.y + config.height;
	locked_look_y = vehicle_pose.origin.y + config.look_height;
	Vector3 initial_position = vehicle_pose.origin - forward * config.distance;
	initial_position.y = locked_world_y;
	smoothed_look_target = vehicle_pose.origin + forward * config.look_ahead;
	smoothed_look_target.y = locked_look_y;
	epoch = 0;
	last_vehicle_position = vehicle_pose.origin;
	acceleration_initialized = false;
	initialized = true;
	return { initial_position, smoothed_look_target, config.base_fov };
}

ChaseCameraOutput ChaseCameraSolver::step(
	const ChaseCameraConfig &config,
	const ChaseCameraInput &input,
	const Vector3 &current_position,
	double current_fov,
	double delta) {
	const Transform3D &visual_pose = input.vehicle_pose;
	Vector3 physical_forward = -visual_pose.basis.get_column(2);
	physical_forward.y = 0;
	physical_forward.normalize();

	const bool discontinuity = !initialized || !acceleration_initialized ||
		(visual_pose.origin - last_vehicle_position).length_squared() > 9.0;
	if (discontinuity) {
		++epoch;
		smoothed_forward = physical_forward;
		smoothed_velocity_lead = Vector3();
		smoothed_longitudinal_inertia = 0.0;
		previous_longitudinal_source = 0.0;
		smoothed_turn_amount = 0.0;
		filtered_acceleration = Vector3();
		inertia_initialized = false;
		initialized = true;
	}
	last_vehicle_position = visual_pose.origin;

	Vector3 heading_blend = smoothed_forward.lerp(physical_forward, smoothing_alpha(config.heading_smoothing, delta));
	smoothed_forward = heading_blend.length_squared() > 0.000001 ? heading_blend.normalized() : physical_forward;
	Vector3 right(-smoothed_forward.z, 0, smoothed_forward.x);

	const Vector3 car_position = visual_pose.origin;
	Vector3 horizontal_velocity = input.linear_velocity;
	horizontal_velocity.y = 0;

	const Vector3 current_world_velocity = horizontal_velocity;
	Vector3 world_acceleration;
	if (acceleration_initialized && delta > 0.000001)
		world_acceleration = (current_world_velocity - previous_world_velocity) / delta;
	previous_world_velocity = current_world_velocity;
	acceleration_initialized = true;

	Vector3 horizontal_acceleration = world_acceleration;
	horizontal_acceleration.y = 0;

	const double dynamic_limit = Math::max(config.maximum_camera_offset, 0.0);
	const double signed_forward_speed = horizontal_velocity.dot(physical_forward);
	const double lead_distance = Math::clamp(
		signed_forward_speed * Math::max(config.velocity_anticipation, 0.0), -dynamic_limit * 0.35, dynamic_limit * 0.35);
	const Vector3 lead_target = smoothed_forward * lead_distance;
	smoothed_velocity_lead = smoothed_velocity_lead.lerp(lead_target, smoothing_alpha(config.offset_smoothing, delta));

	filtered_acceleration = filtered_acceleration.lerp(horizontal_acceleration, smoothing_alpha(10.0, delta));
	const double inertia_source = Math::clamp(
		-filtered_acceleration.dot(smoothed_forward) * Math::max(config.inertia_strength, 0.0), -dynamic_limit, dynamic_limit);
	if (inertia_initialized) {
		const double impulse_delta = inertia_source - previous_longitudinal_source;
		if (Math::abs(impulse_delta) > 0.005)
			smoothed_longitudinal_inertia = Math::clamp(
				smoothed_longitudinal_inertia + impulse_delta, -dynamic_limit, dynamic_limit);
	} else {
		inertia_initialized = true;
	}
	previous_longitudinal_source = inertia_source;
	smoothed_longitudinal_inertia = Math::lerp(
		smoothed_longitudinal_inertia, 0.0, smoothing_alpha(config.offset_smoothing, delta));

	const double turn_target = travel_corrected_turn(
		input.steering_amount, signed_forward_speed, config.turn_activation_speed);
	smoothed_turn_amount = Math::lerp(smoothed_turn_amount, turn_target, smoothing_alpha(config.turn_offset_smoothing, delta));
	const Vector3 longitudinal_offset = smoothed_forward * smoothed_longitudinal_inertia;
	const double lateral_position = lateral_camera_offset(smoothed_turn_amount, config.lateral_swing);
	const Vector3 turn_offset = right * lateral_position;

	Vector3 desired = car_position - smoothed_forward * config.distance + longitudinal_offset + turn_offset;
	desired.y = locked_world_y;
	const Vector3 position_before_smoothing = discontinuity ? desired : current_position;
	const Vector3 error = desired - position_before_smoothing;
	const Vector3 horizontal_error =
		right * outside_dead_zone(error.dot(right), config.horizontal_dead_zone) +
		smoothed_forward * outside_dead_zone(error.dot(smoothed_forward), config.horizontal_dead_zone);

	Vector3 next = position_before_smoothing + horizontal_error * smoothing_alpha(config.horizontal_smoothing, delta);
	Vector3 lag = next - desired;
	lag.y = 0.0;
	const double lag_limit = Math::max(config.maximum_follow_lag, 0.0);
	if (lag_limit > 0.0) lag = clamp_length(lag, lag_limit);
	else lag = Vector3();
	next = desired + lag;
	next.y = locked_world_y;

	const double lateral_look = lateral_look_offset(smoothed_turn_amount, config.lateral_swing, config.turn_look_offset);
	Vector3 look_desired = car_position + smoothed_forward * config.look_ahead + smoothed_velocity_lead + right * lateral_look;
	look_desired.y = locked_look_y;
	if (discontinuity) smoothed_look_target = look_desired;
	else smoothed_look_target = smoothed_look_target.lerp(look_desired, smoothing_alpha(config.follow_damping, delta));
	smoothed_look_target.y = locked_look_y;

	const double fov_target = config.base_fov + config.speed_fov_gain * Math::clamp(input.speed_mps * 3.6 / 285.0, 0.0, 1.0);
	return { next, smoothed_look_target, Math::lerp(current_fov, fov_target, smoothing_alpha(config.follow_damping, delta)) };
}

} // namespace formula90s::camera
