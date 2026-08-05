#pragma once
#include <godot_cpp/classes/resource.hpp>
#include <godot_cpp/variant/packed_float32_array.hpp>
namespace godot {
class CarPhysicsConfig : public Resource {
    GDCLASS(CarPhysicsConfig, Resource)
    double mass = 850.0, engine_force = 34.0, brake_force = 48.0, strong_brake_force = 70.0;
    double max_speed_kph = 285.0, reverse_max_speed_kph = 35.0, drag = 0.006, rolling_resistance = 1.4;
    double low_speed_steering = 1.9, high_speed_steering = 0.55, lateral_grip = 7.5;
    double drift_factor = 0.82, stability_recovery = 3.0;
    double idle_rpm = 1100.0, max_rpm = 9000.0, upshift_rpm = 8200.0, downshift_rpm = 4300.0;
    double minimum_shift_time = 0.32, reverse_ratio = 3.2, final_drive = 3.7;
    PackedFloat32Array gear_ratios;
protected: static void _bind_methods();
public:
    CarPhysicsConfig();
#define F90_PROP(type, name) void set_##name(type v) { name = v; } type get_##name() const { return name; }
    F90_PROP(double, mass) F90_PROP(double, engine_force) F90_PROP(double, brake_force)
    F90_PROP(double, strong_brake_force) F90_PROP(double, max_speed_kph) F90_PROP(double, reverse_max_speed_kph)
    F90_PROP(double, drag) F90_PROP(double, rolling_resistance) F90_PROP(double, low_speed_steering)
    F90_PROP(double, high_speed_steering) F90_PROP(double, lateral_grip) F90_PROP(double, drift_factor)
    F90_PROP(double, stability_recovery) F90_PROP(double, idle_rpm) F90_PROP(double, max_rpm)
    F90_PROP(double, upshift_rpm) F90_PROP(double, downshift_rpm) F90_PROP(double, minimum_shift_time)
    F90_PROP(double, reverse_ratio) F90_PROP(double, final_drive)
#undef F90_PROP
    void set_gear_ratios(const PackedFloat32Array &v) { gear_ratios = v; }
    PackedFloat32Array get_gear_ratios() const { return gear_ratios; }
    bool is_valid() const;
};
}

