#include "formula90s/ui/debug_hud_controller.hpp"
#include <godot_cpp/classes/label.hpp>
#include <godot_cpp/variant/variant.hpp>
using namespace godot;
void DebugHudController::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_vehicle_path","path"),&DebugHudController::set_vehicle_path);
    ClassDB::bind_method(D_METHOD("get_vehicle_path"),&DebugHudController::get_vehicle_path);
    ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH,"vehicle_path"),"set_vehicle_path","get_vehicle_path");
    ClassDB::bind_method(D_METHOD("set_aids_path","path"),&DebugHudController::set_aids_path);
    ClassDB::bind_method(D_METHOD("get_aids_path"),&DebugHudController::get_aids_path);
    ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH,"aids_path"),"set_aids_path","get_aids_path");
}
void DebugHudController::_process(double delta) {
    Object *vehicle=vehicle_path.is_empty()?nullptr:get_node_or_null(vehicle_path);
    Object *aids=aids_path.is_empty()?nullptr:get_node_or_null(aids_path);
    Label *label=Object::cast_to<Label>(get_node_or_null("Panel/Readout"));
    if(!label)return;
    String text;
    if(vehicle){
        double speed_mps=vehicle->get("speed");double rpm=vehicle->get("motor_rpm");int gear=vehicle->get("current_gear");
        bool automatic=vehicle->get("automatic_transmission");String gear_label=gear<0?"R":(gear==0?"N":String::num_int64(gear));
        double speed_kph=Math::abs(speed_mps)*3.6;
        text="VELOCIDAD "+String::num_int64((int)speed_kph)+" km/h\nMARCHA "+gear_label+"\nMODO "+(automatic?String("AUTO"):String("MANUAL"))+"\nRPM "+String::num_int64((int)rpm)+"\n";
    } else text="VEHICULO NO DETECTADO\n";
    if(aids){
        text+="\nAYUDAS ACTIVAS\n";
        for(int i=0;i<5;i++){
            String aid_label=aids->call("get_aid_label",i);
            String aid_status=aids->call("get_aid_status",i);
            text+=String::num_int64(i+1)+" "+aid_label+"  "+aid_status+"\n";
        }
    }
    label->set_text(text);
}
