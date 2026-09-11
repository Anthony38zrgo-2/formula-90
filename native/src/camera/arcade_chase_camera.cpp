#include "formula90s/camera/arcade_chase_camera.hpp"
#include "formula90s/camera/camera_math.hpp"
#include "formula90s/vehicle/vehicle_adapter.hpp"
#include <godot_cpp/classes/camera3d.hpp>
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

ArcadeChaseCamera::ArcadeChaseCamera() { set_process_priority(-10); }

void ArcadeChaseCamera::_bind_methods() {
#define CAMERA_PROP(name) \
	ClassDB::bind_method(D_METHOD("set_" #name, "value"), &ArcadeChaseCamera::set_##name); \
	ClassDB::bind_method(D_METHOD("get_" #name), &ArcadeChaseCamera::get_##name); \
	ADD_PROPERTY(PropertyInfo(Variant::FLOAT, #name), "set_" #name, "get_" #name)
	CAMERA_PROP(distance); CAMERA_PROP(height); CAMERA_PROP(follow_damping); CAMERA_PROP(horizontal_smoothing);
	CAMERA_PROP(vertical_smoothing); CAMERA_PROP(look_ahead); CAMERA_PROP(look_height);
	CAMERA_PROP(horizontal_dead_zone); CAMERA_PROP(vertical_dead_zone); CAMERA_PROP(velocity_anticipation);
	CAMERA_PROP(inertia_strength); CAMERA_PROP(maximum_camera_offset); CAMERA_PROP(offset_smoothing);
	CAMERA_PROP(lateral_swing); CAMERA_PROP(turn_look_offset); CAMERA_PROP(base_fov); CAMERA_PROP(speed_fov_gain);
	CAMERA_PROP(heading_smoothing); CAMERA_PROP(maximum_follow_lag);
	CAMERA_PROP(turn_offset_smoothing); CAMERA_PROP(turn_activation_speed);
#undef CAMERA_PROP
	ClassDB::bind_method(D_METHOD("set_car_path", "path"), &ArcadeChaseCamera::set_car_path);
	ClassDB::bind_method(D_METHOD("get_car_path"), &ArcadeChaseCamera::get_car_path);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "car_path"), "set_car_path", "get_car_path");
}

void ArcadeChaseCamera::_ready() {
	car_adapter = VehicleAdapter(get_node_or_null(car_path));
	if (!car_adapter.is_valid()) return;

	const Transform3D pose = car_adapter.get_global_transform();
	Vector3 forward = -pose.basis.get_column(2);
	forward.y = 0;
	forward.normalize();
	smoothed_forward = forward;
	locked_world_y = pose.origin.y + height;
	locked_look_y = pose.origin.y + look_height;
	Vector3 initial_position = pose.origin - forward * distance;
	initial_position.y = locked_world_y;
	set_global_position(initial_position);
	smoothed_look_target = pose.origin + forward * look_ahead;
	smoothed_look_target.y = locked_look_y;
	look_at(smoothed_look_target, Vector3(0, 1, 0));
	locked_pitch = get_global_rotation().x;
	Vector3 rotation = get_global_rotation();
	rotation.z = 0.0;
	set_global_rotation(rotation);
	epoch = 0;
	last_car_position = pose.origin;
	acceleration_initialized = false;
	initialized = true;
}

void ArcadeChaseCamera::_process(double delta) {
	if (!car_adapter.is_valid() || delta <= 0.0) return;
	Camera3D *camera = Object::cast_to<Camera3D>(get_node_or_null("Camera3D"));
	if (!camera) return;

	const Transform3D visual_pose = car_adapter.get_global_transform();
	Vector3 physical_forward = -visual_pose.basis.get_column(2);
	physical_forward.y = 0;
	physical_forward.normalize();

	const bool discontinuity = !initialized || !acceleration_initialized ||
		(visual_pose.origin - last_car_position).length_squared() > 9.0;
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
	last_car_position = visual_pose.origin;

	Vector3 heading_blend = smoothed_forward.lerp(physical_forward, smoothing_alpha(heading_smoothing, delta));
	smoothed_forward = heading_blend.length_squared() > 0.000001 ? heading_blend.normalized() : physical_forward;
	Vector3 right(-smoothed_forward.z, 0, smoothed_forward.x);

	const Vector3 car_position = visual_pose.origin;
	Vector3 horizontal_velocity = car_adapter.get_linear_velocity();
	horizontal_velocity.y = 0;

	const Vector3 current_world_velocity = horizontal_velocity;
	Vector3 world_acceleration;
	if (acceleration_initialized && delta > 0.000001)
		world_acceleration = (current_world_velocity - previous_world_velocity) / delta;
	previous_world_velocity = current_world_velocity;
	acceleration_initialized = true;

	Vector3 horizontal_acceleration = world_acceleration;
	horizontal_acceleration.y = 0;

	const double dynamic_limit = Math::max(maximum_camera_offset, 0.0);
	const double signed_forward_speed = horizontal_velocity.dot(physical_forward);
	const double lead_distance = Math::clamp(
		signed_forward_speed * Math::max(velocity_anticipation, 0.0), -dynamic_limit * 0.35, dynamic_limit * 0.35);
	const Vector3 lead_target = smoothed_forward * lead_distance;
	smoothed_velocity_lead = smoothed_velocity_lead.lerp(lead_target, smoothing_alpha(offset_smoothing, delta));

	filtered_acceleration = filtered_acceleration.lerp(horizontal_acceleration, smoothing_alpha(10.0, delta));
	const double inertia_source = Math::clamp(
		-filtered_acceleration.dot(smoothed_forward) * Math::max(inertia_strength, 0.0), -dynamic_limit, dynamic_limit);
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
		smoothed_longitudinal_inertia, 0.0, smoothing_alpha(offset_smoothing, delta));

	const double turn_target = formula90s::camera::travel_corrected_turn(
		car_adapter.get_true_steering_amount(), signed_forward_speed, turn_activation_speed);
	smoothed_turn_amount = Math::lerp(smoothed_turn_amount, turn_target, smoothing_alpha(turn_offset_smoothing, delta));
	const Vector3 longitudinal_offset = smoothed_forward * smoothed_longitudinal_inertia;
	const double lateral_position = formula90s::camera::lateral_camera_offset(smoothed_turn_amount, lateral_swing);
	const Vector3 turn_offset = right * lateral_position;

	Vector3 desired = car_position - smoothed_forward * distance + longitudinal_offset + turn_offset;
	desired.y = car_position.y + height;
	if (discontinuity) set_global_position(desired);
	const Vector3 error = desired - get_global_position();
	const Vector3 horizontal_error =
		right * outside_dead_zone(error.dot(right), horizontal_dead_zone) +
		smoothed_forward * outside_dead_zone(error.dot(smoothed_forward), horizontal_dead_zone);

	Vector3 next = get_global_position() + horizontal_error * smoothing_alpha(horizontal_smoothing, delta);
	next.y += outside_dead_zone(error.y, vertical_dead_zone) * smoothing_alpha(vertical_smoothing, delta);
	Vector3 lag = next - desired;
	lag.y = 0.0;
	const double lag_limit = Math::max(maximum_follow_lag, 0.0);
	if (lag_limit > 0.0) lag = clamp_length(lag, lag_limit);
	else lag = Vector3();
	next = desired + lag;
	set_global_position(next);

	const double lateral_look = formula90s::camera::lateral_look_offset(smoothed_turn_amount, lateral_swing, turn_look_offset);
	Vector3 look_desired = car_position + smoothed_forward * look_ahead + smoothed_velocity_lead + right * lateral_look;
	look_desired.y = car_position.y + look_height;
	if (discontinuity) smoothed_look_target = look_desired;
	else smoothed_look_target = smoothed_look_target.lerp(look_desired, smoothing_alpha(follow_damping, delta));
	look_at(smoothed_look_target, Vector3(0, 1, 0));
	Vector3 locked_rotation = get_global_rotation();
	locked_rotation.x = locked_pitch;
	locked_rotation.z = 0.0;
	set_global_rotation(locked_rotation);

	const double fov_target = base_fov + (speed_fov_gain * Math::clamp(car_adapter.get_speed() * 3.6 / 285.0, 0.0, 1.0));
	camera->set_fov(Math::lerp(double(camera->get_fov()), fov_target, smoothing_alpha(follow_damping, delta)));
}
