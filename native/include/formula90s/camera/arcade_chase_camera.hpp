#pragma once
#include <godot_cpp/classes/node3d.hpp>
namespace godot { class Camera3D; class ArcadeCarController;
class ArcadeChaseCamera : public Node3D { GDCLASS(ArcadeChaseCamera, Node3D)
    double distance=9.0, height=4.5, follow_damping=6.0, horizontal_smoothing=5.0, vertical_smoothing=7.5;
    double look_ahead=4.0, horizontal_dead_zone=0.12, vertical_dead_zone=0.08;
    double velocity_anticipation=0.025, inertia_strength=0.018, maximum_camera_offset=0.55, offset_smoothing=3.2;
    double lateral_swing=0.55, turn_look_offset=0.35, base_fov=62.0, speed_fov_gain=4.0;
    Vector3 smoothed_velocity_lead, smoothed_inertia, smoothed_look_target, filtered_acceleration, previous_inertia_source; bool initialized=false, inertia_initialized=false;
protected: static void _bind_methods();
public:
    ArcadeChaseCamera(); void _ready() override; void _process(double delta) override;
#define CAMERA_ACCESSOR(name) void set_##name(double v){name=v;} double get_##name()const{return name;}
    CAMERA_ACCESSOR(distance) CAMERA_ACCESSOR(height) CAMERA_ACCESSOR(follow_damping)
    CAMERA_ACCESSOR(horizontal_smoothing) CAMERA_ACCESSOR(vertical_smoothing) CAMERA_ACCESSOR(look_ahead)
    CAMERA_ACCESSOR(horizontal_dead_zone) CAMERA_ACCESSOR(vertical_dead_zone) CAMERA_ACCESSOR(velocity_anticipation)
    CAMERA_ACCESSOR(inertia_strength) CAMERA_ACCESSOR(maximum_camera_offset) CAMERA_ACCESSOR(offset_smoothing)
    CAMERA_ACCESSOR(lateral_swing) CAMERA_ACCESSOR(turn_look_offset) CAMERA_ACCESSOR(base_fov) CAMERA_ACCESSOR(speed_fov_gain)
#undef CAMERA_ACCESSOR
}; }
