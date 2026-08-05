#include "formula90s/vehicle/vehicle_definition.hpp"
using namespace godot;
void VehicleDefinition::_bind_methods(){
#define BIND_VDEF(type,name) ClassDB::bind_method(D_METHOD("set_" #name,"value"),&VehicleDefinition::set_##name);ClassDB::bind_method(D_METHOD("get_" #name),&VehicleDefinition::get_##name);ADD_PROPERTY(PropertyInfo(type,#name),"set_" #name,"get_" #name)
    BIND_VDEF(Variant::STRING,vehicle_id);BIND_VDEF(Variant::STRING,display_name);BIND_VDEF(Variant::STRING,sprite_metadata_path);BIND_VDEF(Variant::STRING,audio_bank_path);BIND_VDEF(Variant::STRING,audio_config_path);
    BIND_VDEF(Variant::FLOAT,visual_scale);BIND_VDEF(Variant::FLOAT,visual_offset_y);BIND_VDEF(Variant::FLOAT,maximum_speed_kph);BIND_VDEF(Variant::FLOAT,maximum_rpm);
#undef BIND_VDEF
    ClassDB::bind_method(D_METHOD("set_physics","value"),&VehicleDefinition::set_physics);ClassDB::bind_method(D_METHOD("get_physics"),&VehicleDefinition::get_physics);ADD_PROPERTY(PropertyInfo(Variant::OBJECT,"physics",PROPERTY_HINT_RESOURCE_TYPE,"CarPhysicsConfig"),"set_physics","get_physics");
    ClassDB::bind_method(D_METHOD("set_sprite_sheet","value"),&VehicleDefinition::set_sprite_sheet);ClassDB::bind_method(D_METHOD("get_sprite_sheet"),&VehicleDefinition::get_sprite_sheet);ADD_PROPERTY(PropertyInfo(Variant::OBJECT,"sprite_sheet",PROPERTY_HINT_RESOURCE_TYPE,"Texture2D"),"set_sprite_sheet","get_sprite_sheet");
}
