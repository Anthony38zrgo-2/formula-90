#pragma once

#include <godot_cpp/classes/resource.hpp>
#include <godot_cpp/variant/node_path.hpp>
#include <godot_cpp/variant/vector3.hpp>

namespace godot {

class VehicleVisual3DConfig : public Resource {
    GDCLASS(VehicleVisual3DConfig, Resource)

    Vector3 model_scale = Vector3(4.0, 4.0, 4.0);
    Vector3 model_rotation_degrees;
    Vector3 model_offset;
    NodePath model_path = NodePath("V10Model");
    NodePath front_left_wheel_path;
    NodePath front_right_wheel_path;
    NodePath rear_left_wheel_path;
    NodePath rear_right_wheel_path;
    NodePath surface_probes_path = NodePath("../SurfaceProbes");
    double wheel_radius = 0.33;
    double wheel_spin_multiplier = 1.0;
    double maximum_steering_degrees = 22.0;
    double steering_smoothing = 12.0;
    double maximum_roll_degrees = 2.5;
    double roll_acceleration_gain = 0.04;
    double maximum_pitch_degrees = 1.5;
    double pitch_acceleration_gain = 0.025;
    double pose_smoothing = 8.0;
    double kerb_vibration_amplitude = 0.012;
    double gravel_vibration_amplitude = 0.02;
    double vibration_frequency = 24.0;
    String kerb_group = "kerb";
    String gravel_group = "gravel";

protected:
    static void _bind_methods();

public:
    void set_model_scale(const Vector3 &value);
    Vector3 get_model_scale() const { return model_scale; }
    void set_model_rotation_degrees(const Vector3 &value) { model_rotation_degrees = value; }
    Vector3 get_model_rotation_degrees() const { return model_rotation_degrees; }
    void set_model_offset(const Vector3 &value) { model_offset = value; }
    Vector3 get_model_offset() const { return model_offset; }

#define F90_VISUAL_PATH_PROPERTY(name) void set_##name(const NodePath &value) { name = value; } NodePath get_##name() const { return name; }
    F90_VISUAL_PATH_PROPERTY(model_path)
    F90_VISUAL_PATH_PROPERTY(front_left_wheel_path)
    F90_VISUAL_PATH_PROPERTY(front_right_wheel_path)
    F90_VISUAL_PATH_PROPERTY(rear_left_wheel_path)
    F90_VISUAL_PATH_PROPERTY(rear_right_wheel_path)
    F90_VISUAL_PATH_PROPERTY(surface_probes_path)
#undef F90_VISUAL_PATH_PROPERTY

#define F90_VISUAL_FLOAT_PROPERTY(name) void set_##name(double value); double get_##name() const { return name; }
    F90_VISUAL_FLOAT_PROPERTY(wheel_radius)
    F90_VISUAL_FLOAT_PROPERTY(wheel_spin_multiplier)
    F90_VISUAL_FLOAT_PROPERTY(maximum_steering_degrees)
    F90_VISUAL_FLOAT_PROPERTY(steering_smoothing)
    F90_VISUAL_FLOAT_PROPERTY(maximum_roll_degrees)
    F90_VISUAL_FLOAT_PROPERTY(roll_acceleration_gain)
    F90_VISUAL_FLOAT_PROPERTY(maximum_pitch_degrees)
    F90_VISUAL_FLOAT_PROPERTY(pitch_acceleration_gain)
    F90_VISUAL_FLOAT_PROPERTY(pose_smoothing)
    F90_VISUAL_FLOAT_PROPERTY(kerb_vibration_amplitude)
    F90_VISUAL_FLOAT_PROPERTY(gravel_vibration_amplitude)
    F90_VISUAL_FLOAT_PROPERTY(vibration_frequency)
#undef F90_VISUAL_FLOAT_PROPERTY

    void set_kerb_group(const String &value) { kerb_group = value; }
    String get_kerb_group() const { return kerb_group; }
    void set_gravel_group(const String &value) { gravel_group = value; }
    String get_gravel_group() const { return gravel_group; }
    bool is_valid() const;
};

} // namespace godot
