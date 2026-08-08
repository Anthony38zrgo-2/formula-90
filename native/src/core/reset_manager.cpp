#include "formula90s/core/reset_manager.hpp"
#include <godot_cpp/classes/input.hpp>

using namespace godot;

void ResetManager::_bind_methods() {
	ClassDB::bind_method(D_METHOD("reset_vehicle"), &ResetManager::reset_vehicle);
	ClassDB::bind_method(D_METHOD("set_car_path", "path"), &ResetManager::set_car_path);
	ClassDB::bind_method(D_METHOD("get_car_path"), &ResetManager::get_car_path);
	ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, "car_path"), "set_car_path", "get_car_path");
}

void ResetManager::_ready() {
	car_adapter = VehicleAdapter(get_node_or_null(car_path));
	if (car_adapter.is_valid())
		spawn = car_adapter.get_global_transform();
}

void ResetManager::_physics_process(double delta) {
	if (!car_adapter.is_valid()) return;
	Vector3 p = car_adapter.get_global_position();
	if (Input::get_singleton()->is_action_just_pressed("Reset Vehicle") || p.y < -5 || !p.is_finite())
		reset_vehicle();
}

void ResetManager::reset_vehicle() {
	if (!car_adapter.is_valid()) return;
	car_adapter.set_global_transform(spawn);
	car_adapter.reset_motion();
}
