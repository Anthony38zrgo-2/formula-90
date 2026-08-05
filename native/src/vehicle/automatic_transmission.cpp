#include "formula90s/vehicle/automatic_transmission.hpp"
#include <godot_cpp/core/math.hpp>
#include "formula90s/vehicle/physics_math.hpp"
using namespace godot;
void AutomaticTransmission::_bind_methods() {
    ClassDB::bind_method(D_METHOD("reset"), &AutomaticTransmission::reset);
    ClassDB::bind_method(D_METHOD("set_reverse", "enabled"), &AutomaticTransmission::set_reverse);
    ClassDB::bind_method(D_METHOD("get_gear"), &AutomaticTransmission::get_gear);
    ClassDB::bind_method(D_METHOD("get_rpm"), &AutomaticTransmission::get_rpm);
    ClassDB::bind_method(D_METHOD("get_gear_label"), &AutomaticTransmission::get_gear_label);
    ADD_SIGNAL(MethodInfo("gear_changed", PropertyInfo(Variant::INT, "gear")));
}
void AutomaticTransmission::reset() { gear = 1; rpm = 1100.0; shift_timer = 0.0; }
void AutomaticTransmission::set_reverse(bool enabled) { int next = enabled ? -1 : 1; if (gear != next) { gear = next; emit_signal("gear_changed", gear); } }
String AutomaticTransmission::get_gear_label() const { return gear < 0 ? "R" : (gear == 0 ? "N" : String::num_int64(gear)); }
double AutomaticTransmission::torque_factor() const { double x = rpm / 9000.0; return Math::clamp(0.55 + 1.15 * x - 0.7 * x * x, 0.35, 1.0); }
void AutomaticTransmission::update(double speed_mps, double throttle, double delta, const Ref<CarPhysicsConfig> &c) {
    if (c.is_null() || !c->is_valid()) return;
    shift_timer += delta;
    double ratio = gear < 0 ? c->get_reverse_ratio() : c->get_gear_ratios()[Math::clamp(gear - 1, 0, 5)];
    rpm = formula90s::physics::engine_rpm(speed_mps, ratio, c->get_final_drive(), c->get_idle_rpm(), c->get_max_rpm());
    int next = formula90s::physics::choose_automatic_gear(gear,rpm,throttle,c->get_upshift_rpm(),c->get_downshift_rpm(),shift_timer>=c->get_minimum_shift_time());
    if (next != gear) { gear = next; shift_timer = 0; emit_signal("gear_changed", gear); }
}
