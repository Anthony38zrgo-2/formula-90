#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/input.hpp>
#include <godot_cpp/core/math.hpp>
#include "formula90s/vehicle/physics_math.hpp"
using namespace godot;
ArcadeCarController::ArcadeCarController() { transmission.instantiate(); set_floor_snap_length(0.4); }
void ArcadeCarController::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_config", "config"), &ArcadeCarController::set_config);
    ClassDB::bind_method(D_METHOD("get_config"), &ArcadeCarController::get_config);
    ClassDB::bind_method(D_METHOD("get_speed_kph"), &ArcadeCarController::get_speed_kph);
    ClassDB::bind_method(D_METHOD("get_rpm"), &ArcadeCarController::get_rpm);
    ClassDB::bind_method(D_METHOD("get_gear"), &ArcadeCarController::get_gear);
    ClassDB::bind_method(D_METHOD("get_gear_label"), &ArcadeCarController::get_gear_label);
    ClassDB::bind_method(D_METHOD("clear_motion"), &ArcadeCarController::clear_motion);
    ADD_PROPERTY(PropertyInfo(Variant::OBJECT, "config", PROPERTY_HINT_RESOURCE_TYPE, "CarPhysicsConfig"), "set_config", "get_config");
}
double ArcadeCarController::get_speed_kph() const { return get_velocity().length() * 3.6; }
double ArcadeCarController::get_rpm() const { return transmission->get_rpm(); }
int ArcadeCarController::get_gear() const { return transmission->get_gear(); }
String ArcadeCarController::get_gear_label() const { return transmission->get_gear_label(); }
void ArcadeCarController::clear_motion() { set_velocity(Vector3()); transmission->reset(); }
void ArcadeCarController::_physics_process(double delta) {
    if (config.is_null() || !config->is_valid()) { state = "INVALID CONFIG"; return; }
    Input *input = Input::get_singleton();
    throttle = input->get_action_strength("accelerate"); brake = input->get_action_strength("brake");
    double steer = input->get_axis("steer_left", "steer_right");
    Vector3 velocity = get_velocity(); Basis basis = get_global_basis(); Vector3 forward = -basis.get_column(2); Vector3 right = basis.get_column(0);
    double longitudinal = velocity.dot(forward); lateral_speed = velocity.dot(right);
    if (brake > 0.1 && Math::abs(longitudinal) < 0.8 && throttle < 0.1) transmission->set_reverse(true);
    else if (throttle > 0.1 && transmission->get_gear() < 0 && Math::abs(longitudinal) < 0.8) transmission->set_reverse(false);
    double drive = transmission->torque_factor() * config->get_engine_force();
    if (transmission->get_gear() < 0) longitudinal -= brake * drive * delta;
    else longitudinal += throttle * drive * delta;
    double braking = (input->is_action_pressed("strong_brake") ? config->get_strong_brake_force() : config->get_brake_force());
    if (transmission->get_gear() > 0 && brake > 0.0) longitudinal = Math::move_toward(longitudinal, 0.0, braking * brake * delta);
    longitudinal = formula90s::physics::apply_drag(longitudinal,config->get_drag(),config->get_rolling_resistance(),delta);
    double max_ms = (transmission->get_gear() < 0 ? config->get_reverse_max_speed_kph() : config->get_max_speed_kph()) / 3.6;
    longitudinal = formula90s::physics::clamp_speed(longitudinal,max_ms,max_ms);
    double speed_ratio = Math::clamp(Math::abs(longitudinal) / max_ms, 0.0, 1.0);
    double steer_rate = formula90s::physics::steering_rate(speed_ratio,config->get_low_speed_steering(),config->get_high_speed_steering());

    if (Math::abs(longitudinal) > 0.2) rotate_y(-steer * steer_rate * delta * (longitudinal >= 0 ? 1.0 : -1.0));
    double drift_input=input->get_action_strength("strong_brake");
    double effective_grip=config->get_lateral_grip()*Math::lerp(1.0,config->get_drift_factor(),drift_input);
    lateral_speed = Math::move_toward(lateral_speed, 0.0, (effective_grip+config->get_stability_recovery()*speed_ratio) * delta);
    if(Math::abs(lateral_speed)>0.1) rotate_y(-lateral_speed*0.002*config->get_drift_factor());
    velocity = forward * longitudinal + right * lateral_speed; if (!is_on_floor()) velocity.y -= 24.0 * delta; else velocity.y = -0.5;
    set_velocity(velocity); move_and_slide(); transmission->update(longitudinal, throttle, delta, config); state = is_on_floor() ? "DRIVING" : "AIRBORNE";
}
