#include "formula90s/presentation/directional_vehicle_sprite.hpp"
using namespace godot;
DirectionalVehicleSprite::DirectionalVehicleSprite() { set_billboard_mode(BaseMaterial3D::BILLBOARD_ENABLED); set_centered(true); }
void DirectionalVehicleSprite::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_orientation_count","value"),&DirectionalVehicleSprite::set_orientation_count); ClassDB::bind_method(D_METHOD("get_orientation_count"),&DirectionalVehicleSprite::get_orientation_count);
    ClassDB::bind_method(D_METHOD("set_allow_mirroring","value"),&DirectionalVehicleSprite::set_allow_mirroring); ClassDB::bind_method(D_METHOD("get_allow_mirroring"),&DirectionalVehicleSprite::get_allow_mirroring);
    ClassDB::bind_method(D_METHOD("set_ground_offset","value"),&DirectionalVehicleSprite::set_ground_offset); ClassDB::bind_method(D_METHOD("get_ground_offset"),&DirectionalVehicleSprite::get_ground_offset);
    ADD_PROPERTY(PropertyInfo(Variant::INT,"orientation_count",PROPERTY_HINT_ENUM,"8,12,16"),"set_orientation_count","get_orientation_count"); ADD_PROPERTY(PropertyInfo(Variant::BOOL,"allow_mirroring"),"set_allow_mirroring","get_allow_mirroring"); ADD_PROPERTY(PropertyInfo(Variant::FLOAT,"ground_offset"),"set_ground_offset","get_ground_offset");
}
