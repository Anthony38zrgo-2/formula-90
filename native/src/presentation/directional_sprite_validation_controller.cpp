#include "formula90s/presentation/directional_sprite_validation_controller.hpp"
#include "formula90s/presentation/directional_vehicle_sprite.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/input.hpp>
#include <godot_cpp/classes/label.hpp>
#include <godot_cpp/core/math.hpp>

using namespace godot;

DirectionalSpriteValidationController::DirectionalSpriteValidationController() {
    set_physics_process_priority(-20);
}

void DirectionalSpriteValidationController::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_auto_rotate", "value"), &DirectionalSpriteValidationController::set_auto_rotate);
    ClassDB::bind_method(D_METHOD("get_auto_rotate"), &DirectionalSpriteValidationController::get_auto_rotate);
    ClassDB::bind_method(D_METHOD("set_degrees_per_second", "value"), &DirectionalSpriteValidationController::set_degrees_per_second);
    ClassDB::bind_method(D_METHOD("get_degrees_per_second"), &DirectionalSpriteValidationController::get_degrees_per_second);
    ADD_PROPERTY(PropertyInfo(Variant::BOOL, "auto_rotate"), "set_auto_rotate", "get_auto_rotate");
    ADD_PROPERTY(PropertyInfo(Variant::FLOAT, "degrees_per_second", PROPERTY_HINT_RANGE, "1,180,1"), "set_degrees_per_second", "get_degrees_per_second");
}

void DirectionalSpriteValidationController::_ready() {
    vehicle = Object::cast_to<ArcadeCarController>(get_node_or_null("Vehicle"));
    sprite = Object::cast_to<DirectionalVehicleSprite>(get_node_or_null("Vehicle/DirectionalVehicleSprite"));
    readout = Object::cast_to<Label>(get_node_or_null("Readout"));
}

void DirectionalSpriteValidationController::_physics_process(double delta) {
    if (!vehicle || delta <= 0.0) return;
    const double direction = auto_rotate ? 1.0 : Input::get_singleton()->get_axis("steer_left", "steer_right");
    vehicle->rotate_y(Math::deg_to_rad(degrees_per_second * direction * delta));
}

void DirectionalSpriteValidationController::_process(double) {
    if (!vehicle || !sprite || !readout) return;
    const double yaw = Math::fmod(Math::rad_to_deg(vehicle->get_global_rotation().y) + 360.0, 360.0);
    readout->set_text(
        "VALIDACION SPRITE 360\n"
        "CHASIS " + String::num(yaw, 1) + " deg\n"
        "RELATIVO " + String::num(sprite->get_current_relative_angle(), 1) + " deg\n"
        "FRAME " + String::num_int64(sprite->get_selected_frame()) + " CAND " + String::num_int64(sprite->get_candidate_frame()) + "\n"
        "HISTERESIS " + String(sprite->is_hysteresis_held() ? "RETENIENDO" : "LIBRE") + "\n"
        "Auto: giro continuo | Manual: flechas izquierda/derecha");
}
