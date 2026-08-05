#pragma once
#include <godot_cpp/classes/character_body3d.hpp>
#include "formula90s/vehicle/automatic_transmission.hpp"
#include "formula90s/vehicle/vehicle_definition.hpp"
namespace godot {
class ArcadeCarController : public CharacterBody3D {
    GDCLASS(ArcadeCarController, CharacterBody3D)
    Ref<CarPhysicsConfig> config; Ref<VehicleDefinition> vehicle_definition; Ref<AutomaticTransmission> transmission;
    double throttle = 0, brake = 0, steering_input = 0, lateral_speed = 0, direction_stop_timer = 0; String state = "READY";
    Vector3 world_acceleration; Vector3 previous_world_velocity; bool acceleration_initialized = false;
protected: static void _bind_methods();
public:
    ArcadeCarController(); void _physics_process(double delta) override;
    void set_config(const Ref<CarPhysicsConfig> &v) { config = v; } Ref<CarPhysicsConfig> get_config() const { return config; }
    void set_vehicle_definition(const Ref<VehicleDefinition>&v){vehicle_definition=v;} Ref<VehicleDefinition> get_vehicle_definition()const{return vehicle_definition;}
    double get_speed_kph() const; double get_rpm() const; int get_gear() const; String get_gear_label() const;
    bool is_automatic_transmission() const { return transmission->is_automatic_enabled(); }
    double get_lateral_speed() const { return lateral_speed; } double get_throttle() const { return throttle; }
    double get_steering_input() const { return steering_input; }
    Vector3 get_world_acceleration() const { return world_acceleration; }
    double get_brake() const { return brake; } String get_state() const { return state; } void clear_motion();
};
}
