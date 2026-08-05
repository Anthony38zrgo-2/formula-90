#pragma once
#include <godot_cpp/classes/ref_counted.hpp>
#include "formula90s/vehicle/car_physics_config.hpp"
namespace godot {
class AutomaticTransmission : public RefCounted {
    GDCLASS(AutomaticTransmission, RefCounted)
    int gear = 1; double rpm = 1100.0; double shift_timer = 0.0; bool automatic_enabled = true;
protected: static void _bind_methods();
public:
    void update(double speed_mps, double throttle, double delta, const Ref<CarPhysicsConfig> &config);
    void reset(); void set_reverse(bool enabled); int get_gear() const { return gear; }
    void shift_up(); void shift_down(); void set_automatic_enabled(bool enabled); bool is_automatic_enabled() const { return automatic_enabled; }
    double get_rpm() const { return rpm; } String get_gear_label() const;
    double torque_factor() const;
};
}
