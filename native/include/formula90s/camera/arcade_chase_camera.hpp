#pragma once
#include <godot_cpp/classes/node3d.hpp>
namespace godot { class Camera3D; class ArcadeCarController;
class ArcadeChaseCamera : public Node3D { GDCLASS(ArcadeChaseCamera, Node3D)
    double distance=9.0, height=4.5, follow_damping=6.0, look_ahead=4.0, base_fov=62.0, speed_fov_gain=13.0;
protected: static void _bind_methods();
public:
    void _physics_process(double delta) override;
    void set_distance(double v){distance=v;} double get_distance()const{return distance;}
    void set_height(double v){height=v;} double get_height()const{return height;}
    void set_follow_damping(double v){follow_damping=v;} double get_follow_damping()const{return follow_damping;}
    void set_look_ahead(double v){look_ahead=v;} double get_look_ahead()const{return look_ahead;}
    void set_base_fov(double v){base_fov=v;} double get_base_fov()const{return base_fov;}
    void set_speed_fov_gain(double v){speed_fov_gain=v;} double get_speed_fov_gain()const{return speed_fov_gain;}
}; }
