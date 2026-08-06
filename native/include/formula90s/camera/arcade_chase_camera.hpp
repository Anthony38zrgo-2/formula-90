#pragma once
#include <godot_cpp/classes/node3d.hpp>
namespace godot { class Camera3D; class ArcadeCarController;
class ArcadeChaseCamera : public Node3D { GDCLASS(ArcadeChaseCamera, Node3D)
    double distance=9.0, height=4.5, follow_damping=6.0, horizontal_smoothing=5.0, vertical_smoothing=7.5;
    double look_ahead=4.0, horizontal_dead_zone=0.12, vertical_dead_zone=0.08;
    double velocity_anticipation=0.0, inertia_strength=0.003, maximum_camera_offset=0.05, offset_smoothing=4.5;
    double lateral_swing=2.1, turn_look_offset=1.5, base_fov=62.0, speed_fov_gain=4.0, heading_smoothing=5.0, maximum_follow_lag=1.25;
    double turn_offset_smoothing=4.5, turn_activation_speed=0.75, locked_world_y=0.0, locked_look_y=0.0, locked_pitch=0.0, smoothed_turn_amount=0.0;
    double smoothed_longitudinal_inertia=0.0, previous_longitudinal_source=0.0;
    Vector3 smoothed_velocity_lead, smoothed_look_target, filtered_acceleration, smoothed_forward; bool initialized=false, inertia_initialized=false;
    uint64_t presentation_epoch=0;
protected: static void _bind_methods();
public:
    ArcadeChaseCamera(); void _ready() override; void _process(double delta) override;
#define CAMERA_ACCESSOR(name) void set_##name(double v){name=v;} double get_##name()const{return name;}
    CAMERA_ACCESSOR(distance) CAMERA_ACCESSOR(height) CAMERA_ACCESSOR(follow_damping)
    CAMERA_ACCESSOR(horizontal_smoothing) CAMERA_ACCESSOR(vertical_smoothing) CAMERA_ACCESSOR(look_ahead)
    CAMERA_ACCESSOR(horizontal_dead_zone) CAMERA_ACCESSOR(vertical_dead_zone) CAMERA_ACCESSOR(velocity_anticipation)
    CAMERA_ACCESSOR(inertia_strength) CAMERA_ACCESSOR(maximum_camera_offset) CAMERA_ACCESSOR(offset_smoothing)
    CAMERA_ACCESSOR(lateral_swing) CAMERA_ACCESSOR(turn_look_offset) CAMERA_ACCESSOR(base_fov) CAMERA_ACCESSOR(speed_fov_gain)
    CAMERA_ACCESSOR(heading_smoothing) CAMERA_ACCESSOR(maximum_follow_lag)
    CAMERA_ACCESSOR(turn_offset_smoothing) CAMERA_ACCESSOR(turn_activation_speed)
#undef CAMERA_ACCESSOR
}; }
