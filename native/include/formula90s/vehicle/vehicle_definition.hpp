#pragma once
#include <godot_cpp/classes/resource.hpp>
#include <godot_cpp/classes/texture2d.hpp>
#include "formula90s/vehicle/car_physics_config.hpp"
namespace godot {
class VehicleDefinition : public Resource {
    GDCLASS(VehicleDefinition, Resource)
    String vehicle_id="v10", display_name="V10", sprite_metadata_path, audio_bank_path, audio_config_path;
    Ref<CarPhysicsConfig> physics; Ref<Texture2D> sprite_sheet;
    double visual_scale=0.015, visual_offset_y=0.08, maximum_speed_kph=285, maximum_rpm=9000;
protected: static void _bind_methods();
public:
#define VDEF_PROP(type,name) void set_##name(type v){name=v;} type get_##name()const{return name;}
    VDEF_PROP(String,vehicle_id) VDEF_PROP(String,display_name) VDEF_PROP(String,sprite_metadata_path) VDEF_PROP(String,audio_bank_path) VDEF_PROP(String,audio_config_path)
    VDEF_PROP(double,visual_scale) VDEF_PROP(double,visual_offset_y) VDEF_PROP(double,maximum_speed_kph) VDEF_PROP(double,maximum_rpm)
#undef VDEF_PROP
    void set_physics(const Ref<CarPhysicsConfig>&v){physics=v;} Ref<CarPhysicsConfig> get_physics()const{return physics;}
    void set_sprite_sheet(const Ref<Texture2D>&v){sprite_sheet=v;} Ref<Texture2D> get_sprite_sheet()const{return sprite_sheet;}
};
}
