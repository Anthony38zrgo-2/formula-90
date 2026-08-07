#pragma once
#include <godot_cpp/classes/character_body3d.hpp>
#include "formula90s/vehicle/automatic_transmission.hpp"
#include "formula90s/vehicle/vehicle_definition.hpp"
namespace godot {
class ArcadeCarController : public CharacterBody3D {
    GDCLASS(ArcadeCarController, CharacterBody3D)
    Ref<CarPhysicsConfig> config;
    Ref<VehicleDefinition> vehicle_definition;
    Ref<AutomaticTransmission> transmission;
    double throttle = 0;
    double brake = 0;
    double steering_input = 0;
    double lateral_speed = 0;
    double direction_stop_timer = 0;
    String state = "READY";
    Vector3 world_acceleration;
    Vector3 previous_world_velocity;
    bool acceleration_initialized = false;
    Transform3D previous_physics_transform;
    Transform3D current_physics_transform;
    bool presentation_initialized = false;
    uint64_t presentation_epoch = 0;
    bool input_ok = false;
    void reset_presentation_pose(); void cache_input_actions();
    void poll_input(); bool handle_direction_change(double delta);
    void apply_longitudinal_forces(double delta);
    void apply_lateral_forces(double delta);
    void finalize_physics(double delta);
protected: static void _bind_methods();
public:
    ArcadeCarController(); void _ready() override; void _physics_process(double delta) override;
    void set_config(const Ref<CarPhysicsConfig> &v) { config = v; } Ref<CarPhysicsConfig> get_config() const { return config; }
    void set_vehicle_definition(const Ref<VehicleDefinition>&v){vehicle_definition=v;} Ref<VehicleDefinition> get_vehicle_definition()const{return vehicle_definition;}
    double get_speed_kph() const; double get_rpm() const; int get_gear() const; String get_gear_label() const;
    bool is_automatic_transmission() const { return transmission->is_automatic_enabled(); }
    double get_lateral_speed() const { return lateral_speed; } double get_throttle() const { return throttle; }
    double get_steering_input() const { return steering_input; }
    Vector3 get_world_acceleration() const { return world_acceleration; }
    Transform3D get_visual_transform() const;
    uint64_t get_presentation_epoch() const { return presentation_epoch; }
    double get_brake() const { return brake; } String get_state() const { return state; } void clear_motion();
};
}
