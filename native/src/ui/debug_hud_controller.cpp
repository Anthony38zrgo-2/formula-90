#include "formula90s/ui/debug_hud_controller.hpp"
#include "formula90s/presentation/directional_vehicle_sprite.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/label.hpp>
using namespace godot;
void DebugHudController::_bind_methods() {}
void DebugHudController::_process(double delta) { ArcadeCarController *car = Object::cast_to<ArcadeCarController>(get_node_or_null("../PlayerCar")); Label *label=Object::cast_to<Label>(get_node_or_null("Panel/Readout")); if(car&&label){DirectionalVehicleSprite *sprite=Object::cast_to<DirectionalVehicleSprite>(car->get_node_or_null("DirectionalVehicleSprite"));String sprite_status=sprite?"\nSPRITE ANG " + String::num(sprite->get_current_relative_angle(),1) + "\nFRAME " + String::num_int64(sprite->get_selected_frame()) + " CAND " + String::num_int64(sprite->get_candidate_frame()) + "\nHISTERESIS " + String(sprite->is_hysteresis_held()?"RETENIENDO":"LIBRE"):"";label->set_text("VELOCIDAD " + String::num_int64((int)car->get_speed_kph()) + " km/h\nMARCHA " + car->get_gear_label() + "\nCAMBIO AUTO " + String(car->is_automatic_transmission()?"ACTIVADO":"DESACTIVADO") + "\nRPM " + String::num_int64((int)car->get_rpm()) + "\n" + car->get_state()+sprite_status);} }
