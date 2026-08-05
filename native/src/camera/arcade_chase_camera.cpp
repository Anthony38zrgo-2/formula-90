#include "formula90s/camera/arcade_chase_camera.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/camera3d.hpp>
#include <godot_cpp/core/math.hpp>
using namespace godot;
void ArcadeChaseCamera::_bind_methods() {
#define CAMERA_PROP(name) ClassDB::bind_method(D_METHOD("set_" #name,"value"),&ArcadeChaseCamera::set_##name); ClassDB::bind_method(D_METHOD("get_" #name),&ArcadeChaseCamera::get_##name); ADD_PROPERTY(PropertyInfo(Variant::FLOAT,#name),"set_" #name,"get_" #name)
    CAMERA_PROP(distance); CAMERA_PROP(height); CAMERA_PROP(follow_damping); CAMERA_PROP(look_ahead); CAMERA_PROP(base_fov); CAMERA_PROP(speed_fov_gain);
#undef CAMERA_PROP
}
void ArcadeChaseCamera::_physics_process(double delta) {
    ArcadeCarController *car = Object::cast_to<ArcadeCarController>(get_parent()); Camera3D *camera = Object::cast_to<Camera3D>(get_node_or_null("Camera3D")); if (!car || !camera) return;
    Vector3 forward = -car->get_global_basis().get_column(2); Vector3 target = car->get_global_position();
    Vector3 desired = target - forward * distance + Vector3(0,height,0); double blend = 1.0 - Math::exp(-follow_damping * delta);
    set_global_position(get_global_position().lerp(desired, blend)); look_at(target + forward * look_ahead + Vector3(0,1,0), Vector3(0,1,0));
    camera->set_fov(Math::lerp((double)camera->get_fov(), base_fov + speed_fov_gain * Math::clamp(car->get_speed_kph()/285.0,0.0,1.0), blend));
}
