#include "formula90s/vehicle/car_physics_config.hpp"
using namespace godot;
CarPhysicsConfig::CarPhysicsConfig() { gear_ratios = PackedFloat32Array({3.1f, 2.2f, 1.65f, 1.3f, 1.05f, 0.86f}); }
void CarPhysicsConfig::_bind_methods() {
#define BIND_PROP(name) ClassDB::bind_method(D_METHOD("set_" #name, "value"), &CarPhysicsConfig::set_##name); ClassDB::bind_method(D_METHOD("get_" #name), &CarPhysicsConfig::get_##name); ADD_PROPERTY(PropertyInfo(Variant::FLOAT, #name), "set_" #name, "get_" #name)
    BIND_PROP(mass); BIND_PROP(engine_force); BIND_PROP(brake_force); BIND_PROP(strong_brake_force);
    BIND_PROP(max_speed_kph); BIND_PROP(reverse_max_speed_kph); BIND_PROP(drag); BIND_PROP(rolling_resistance);
    BIND_PROP(low_speed_steering); BIND_PROP(high_speed_steering); BIND_PROP(lateral_grip); BIND_PROP(drift_factor);
    BIND_PROP(stability_recovery); BIND_PROP(idle_rpm); BIND_PROP(max_rpm); BIND_PROP(upshift_rpm);
    BIND_PROP(downshift_rpm); BIND_PROP(minimum_shift_time); BIND_PROP(reverse_ratio); BIND_PROP(final_drive);
    BIND_PROP(stopped_speed_threshold); BIND_PROP(direction_change_delay);
#undef BIND_PROP
    ClassDB::bind_method(D_METHOD("set_gear_ratios", "value"), &CarPhysicsConfig::set_gear_ratios);
    ClassDB::bind_method(D_METHOD("get_gear_ratios"), &CarPhysicsConfig::get_gear_ratios);
    ADD_PROPERTY(PropertyInfo(Variant::PACKED_FLOAT32_ARRAY, "gear_ratios"), "set_gear_ratios", "get_gear_ratios");
    ClassDB::bind_method(D_METHOD("is_valid"), &CarPhysicsConfig::is_valid);
}
bool CarPhysicsConfig::is_valid() const {
    return mass > 0 && engine_force > 0 && max_speed_kph > 0 && idle_rpm > 0 && max_rpm > idle_rpm &&
        downshift_rpm < upshift_rpm && upshift_rpm <= max_rpm && minimum_shift_time >= 0 &&
        gear_ratios.size() == 6 && reverse_ratio > 0 && final_drive > 0 && stopped_speed_threshold >= 0 &&
        stopped_speed_threshold <= 0.1 && direction_change_delay >= 0;
}
