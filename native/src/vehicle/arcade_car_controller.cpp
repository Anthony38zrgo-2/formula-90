#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/engine.hpp>
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
    ClassDB::bind_method(D_METHOD("get_steering_input"), &ArcadeCarController::get_steering_input);
    ClassDB::bind_method(D_METHOD("get_world_acceleration"), &ArcadeCarController::get_world_acceleration);
    ClassDB::bind_method(D_METHOD("get_visual_transform"), &ArcadeCarController::get_visual_transform);
    ClassDB::bind_method(D_METHOD("get_presentation_epoch"), &ArcadeCarController::get_presentation_epoch);
    ClassDB::bind_method(D_METHOD("is_automatic_transmission"), &ArcadeCarController::is_automatic_transmission);
    ClassDB::bind_method(D_METHOD("clear_motion"), &ArcadeCarController::clear_motion);
    ADD_PROPERTY(PropertyInfo(Variant::OBJECT, "config", PROPERTY_HINT_RESOURCE_TYPE, "CarPhysicsConfig"), "set_config", "get_config");
    ClassDB::bind_method(D_METHOD("set_vehicle_definition","definition"),&ArcadeCarController::set_vehicle_definition);
    ClassDB::bind_method(D_METHOD("get_vehicle_definition"),&ArcadeCarController::get_vehicle_definition);
    ADD_PROPERTY(PropertyInfo(Variant::OBJECT,"vehicle_definition",PROPERTY_HINT_RESOURCE_TYPE,"VehicleDefinition"),"set_vehicle_definition","get_vehicle_definition");
}
void ArcadeCarController::_ready() { reset_presentation_pose(); }
void ArcadeCarController::reset_presentation_pose() {
    const Transform3D pose=get_global_transform();
    previous_physics_transform=pose;current_physics_transform=pose;presentation_initialized=true;++presentation_epoch;
}
Transform3D ArcadeCarController::get_visual_transform() const {
    if(!presentation_initialized)return get_global_transform();
    Engine *engine=Engine::get_singleton();
    const double fraction=engine?Math::clamp(engine->get_physics_interpolation_fraction(),0.0,1.0):1.0;
    return previous_physics_transform.interpolate_with(current_physics_transform,fraction);
}
double ArcadeCarController::get_speed_kph() const { return get_velocity().length() * 3.6; }
double ArcadeCarController::get_rpm() const { return transmission->get_rpm(); }
int ArcadeCarController::get_gear() const { return transmission->get_gear(); }
String ArcadeCarController::get_gear_label() const { return transmission->get_gear_label(); }
void ArcadeCarController::clear_motion() { set_velocity(Vector3()); transmission->reset(); steering_input=0; world_acceleration=Vector3(); previous_world_velocity=Vector3(); acceleration_initialized=false; direction_stop_timer=0; reset_presentation_pose(); }
void ArcadeCarController::_physics_process(double delta) {
    if (config.is_null() || !config->is_valid()) { state = "INVALID CONFIG"; return; }
    const Transform3D physics_start=get_global_transform();
    if(!presentation_initialized||(physics_start.origin-current_physics_transform.origin).length_squared()>9.0)reset_presentation_pose();
    previous_physics_transform=current_physics_transform;
    Input *input = Input::get_singleton();
    throttle = input->get_action_strength("accelerate"); brake = input->get_action_strength("brake");
    if(input->is_action_just_pressed("toggle_automatic")) transmission->set_automatic_enabled(!transmission->is_automatic_enabled());
    if(input->is_action_just_pressed("shift_up") && transmission->get_gear()>0) transmission->shift_up();
    if(input->is_action_just_pressed("shift_down") && transmission->get_gear()>0) transmission->shift_down();
    steering_input = input->get_axis("steer_left", "steer_right");
    Vector3 velocity = get_velocity(); Basis basis = get_global_basis(); Vector3 forward = -basis.get_column(2); Vector3 right = basis.get_column(0);
    double longitudinal = velocity.dot(forward); lateral_speed = velocity.dot(right);
    double horizontal_speed = Vector2(velocity.x, velocity.z).length();
    bool requests_reverse = transmission->get_gear()>0 && brake>0.1 && throttle<0.1;
    bool requests_forward = transmission->get_gear()<0 && throttle>0.1;
    bool requests_direction_change = requests_reverse || requests_forward;
    if(requests_direction_change && formula90s::physics::is_stopped(horizontal_speed,config->get_stopped_speed_threshold())) {
        longitudinal=0; lateral_speed=0; direction_stop_timer+=delta;
    } else direction_stop_timer=0;
    bool can_change_direction=formula90s::physics::can_change_direction(horizontal_speed,direction_stop_timer,config->get_stopped_speed_threshold(),config->get_direction_change_delay());
    bool changed_direction=false;
    if(requests_reverse && can_change_direction){transmission->set_reverse(true);changed_direction=true;direction_stop_timer=0;}
    else if(requests_forward && can_change_direction){transmission->set_reverse(false);changed_direction=true;direction_stop_timer=0;}
    double drive = transmission->torque_factor() * config->get_engine_force();
    double braking = (input->is_action_pressed("strong_brake") ? config->get_strong_brake_force() : config->get_brake_force());
    if(!changed_direction) {
        if(transmission->get_gear()<0) {
            if(throttle>0.0) longitudinal=Math::move_toward(longitudinal,0.0,braking*throttle*delta);
            else longitudinal-=brake*drive*delta;
        } else {
            if(brake>0.0) longitudinal=Math::move_toward(longitudinal,0.0,braking*brake*delta);
            else longitudinal+=throttle*drive*delta;
        }
    }
    longitudinal = formula90s::physics::apply_drag(longitudinal,config->get_drag(),config->get_rolling_resistance(),delta);
    double max_ms = (transmission->get_gear() < 0 ? config->get_reverse_max_speed_kph() : config->get_max_speed_kph()) / 3.6;
    longitudinal = formula90s::physics::clamp_speed(longitudinal,max_ms,max_ms);
    double speed_ratio = Math::clamp(Math::abs(longitudinal) / max_ms, 0.0, 1.0);
    double steer_rate = formula90s::physics::steering_rate(speed_ratio,config->get_low_speed_steering(),config->get_high_speed_steering());

    if (Math::abs(longitudinal) > 0.2) rotate_y(-steering_input * steer_rate * delta * (longitudinal >= 0 ? 1.0 : -1.0));
    double drift_input=input->get_action_strength("strong_brake");
    double effective_grip=config->get_lateral_grip()*Math::lerp(1.0,config->get_drift_factor(),drift_input);
    lateral_speed = formula90s::physics::apply_lateral_grip(lateral_speed,effective_grip+config->get_stability_recovery()*speed_ratio,delta);
    if(Math::abs(lateral_speed)>0.1) rotate_y(-lateral_speed*0.002*config->get_drift_factor());
    basis = get_global_basis(); forward = -basis.get_column(2); right = basis.get_column(0);
    velocity = forward * longitudinal + right * lateral_speed; if (!is_on_floor()) velocity.y -= 24.0 * delta; else velocity.y = -0.5;
    set_velocity(velocity); move_and_slide();
    current_physics_transform=get_global_transform();
    const Vector3 current_world_velocity=get_velocity();
    if(acceleration_initialized&&delta>0.000001)world_acceleration=(current_world_velocity-previous_world_velocity)/delta;else world_acceleration=Vector3();
    previous_world_velocity=current_world_velocity;acceleration_initialized=true;
    transmission->update(longitudinal, throttle, delta, config);
    if(!is_on_floor()) state="AIRBORNE"; else if(requests_reverse&&!can_change_direction) state="BRAKING TO REVERSE"; else if(requests_forward&&!can_change_direction) state="BRAKING TO DRIVE"; else state=transmission->get_gear()<0?"REVERSE":"DRIVING";
}
