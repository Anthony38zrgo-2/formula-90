#include "formula90s/core/reset_manager.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/input.hpp>
using namespace godot;
void ResetManager::_bind_methods(){ClassDB::bind_method(D_METHOD("reset_vehicle"),&ResetManager::reset_vehicle);}
void ResetManager::_ready(){car=Object::cast_to<ArcadeCarController>(get_parent()->get_node_or_null("PlayerCar")); if(car)spawn=car->get_global_transform();}
void ResetManager::_physics_process(double delta){if(!car)return; Vector3 p=car->get_global_position(); if(Input::get_singleton()->is_action_just_pressed("reset_vehicle")||p.y < -5||!p.is_finite())reset_vehicle();}
void ResetManager::reset_vehicle(){if(!car)return;car->set_global_transform(spawn);car->clear_motion();}
