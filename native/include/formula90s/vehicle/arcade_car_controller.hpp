#pragma once
#include <godot_cpp/classes/character_body3d.hpp>
#include "formula90s/vehicle/automatic_transmission.hpp"
namespace godot {
class ArcadeCarController : public CharacterBody3D {
    GDCLASS(ArcadeCarController, CharacterBody3D)
    Ref<CarPhysicsConfig> config; Ref<AutomaticTransmission> transmission;
    double throttle = 0, brake = 0, lateral_speed = 0; String state = "READY";
protected: static void _bind_methods();
public:
    ArcadeCarController(); void _physics_process(double delta) override;
    void set_config(const Ref<CarPhysicsConfig> &v) { config = v; } Ref<CarPhysicsConfig> get_config() const { return config; }
    double get_speed_kph() const; double get_rpm() const; int get_gear() const; String get_gear_label() const;
    double get_lateral_speed() const { return lateral_speed; } double get_throttle() const { return throttle; }
    double get_brake() const { return brake; } String get_state() const { return state; } void clear_motion();
};
}

