#include "formula90s/ui/debug_hud_controller.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/label.hpp>
using namespace godot;
void DebugHudController::_bind_methods() {}
void DebugHudController::_process(double delta) { ArcadeCarController *car = Object::cast_to<ArcadeCarController>(get_node_or_null("../PlayerCar")); Label *label=Object::cast_to<Label>(get_node_or_null("Panel/Readout")); if(car&&label) label->set_text("VELOCIDAD " + String::num_int64((int)car->get_speed_kph()) + " km/h\nMARCHA " + car->get_gear_label() + "\nRPM " + String::num_int64((int)car->get_rpm()) + "\n" + car->get_state()); }
