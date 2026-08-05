#pragma once
#include <godot_cpp/classes/sprite3d.hpp>
namespace godot { class DirectionalVehicleSprite : public Sprite3D { GDCLASS(DirectionalVehicleSprite, Sprite3D)
    int orientation_count=8; bool allow_mirroring=false; double ground_offset=0.05;
protected: static void _bind_methods();
public: DirectionalVehicleSprite(); int get_orientation_count() const{return orientation_count;} void set_orientation_count(int v){orientation_count=v;} bool get_allow_mirroring() const{return allow_mirroring;} void set_allow_mirroring(bool v){allow_mirroring=v;} double get_ground_offset()const{return ground_offset;} void set_ground_offset(double v){ground_offset=v;}
}; }

