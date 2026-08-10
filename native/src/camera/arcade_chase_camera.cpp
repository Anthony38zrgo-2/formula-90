#include "formula90s/camera/arcade_chase_camera.hpp"
#include <godot_cpp/classes/camera3d.hpp>

using namespace godot;

ArcadeChaseCamera::ArcadeChaseCamera() { set_process_priority(-10); }

formula90s::camera::ChaseCameraConfig ArcadeChaseCamera::current_config() const {
	return {
		distance, height, follow_damping, horizontal_smoothing, look_ahead, look_height,
		horizontal_dead_zone, velocity_anticipation, inertia_strength, maximum_camera_offset,
		offset_smoothing, lateral_swing, turn_look_offset, base_fov, speed_fov_gain,
		heading_smoothing, maximum_follow_lag, turn_offset_smoothing, turn_activation_speed
	};
}

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
	car_state = VehicleStateReader(get_node_or_null(car_path));
	if (!car_state.is_valid()) return;

	const formula90s::camera::ChaseCameraOutput output = solver.reset(current_config(), car_state.get_global_transform());
	set_global_position(output.position);
	look_at(output.look_target, Vector3(0, 1, 0));
	locked_pitch = get_global_rotation().x;
	Vector3 rotation = get_global_rotation();
	rotation.z = 0.0;
	set_global_rotation(rotation);
}

void ArcadeChaseCamera::_process(double delta) {
	if (!car_state.is_valid() || delta <= 0.0) return;
	Camera3D *camera = Object::cast_to<Camera3D>(get_node_or_null("Camera3D"));
	if (!camera) return;

	formula90s::camera::ChaseCameraInput input;
	input.vehicle_pose = car_state.get_global_transform();
	input.linear_velocity = car_state.get_linear_velocity();
	input.speed_mps = car_state.get_speed();
	input.steering_amount = car_state.get_true_steering_amount();
	const formula90s::camera::ChaseCameraOutput output = solver.step(
		current_config(), input, get_global_position(), double(camera->get_fov()), delta);
	set_global_position(output.position);
	look_at(output.look_target, Vector3(0, 1, 0));
	Vector3 locked_rotation = get_global_rotation();
	locked_rotation.x = locked_pitch;
	locked_rotation.z = 0.0;
	set_global_rotation(locked_rotation);
	camera->set_fov(output.fov);
}
