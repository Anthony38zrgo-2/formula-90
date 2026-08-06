#include "formula90s/presentation/vehicle_visual_3d_config.hpp"

#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/core/math.hpp>
#include <algorithm>

using namespace godot;

namespace {
double non_negative(double value) { return Math::max(value, 0.0); }
}

void VehicleVisual3DConfig::set_model_scale(const Vector3 &value) {
    model_scale = Vector3(
        std::max<double>(Math::abs(value.x), 0.001),
        std::max<double>(Math::abs(value.y), 0.001),
        std::max<double>(Math::abs(value.z), 0.001));
}

#define DEFINE_NON_NEGATIVE_SETTER(name) void VehicleVisual3DConfig::set_##name(double value) { name = non_negative(value); }
DEFINE_NON_NEGATIVE_SETTER(wheel_radius)
DEFINE_NON_NEGATIVE_SETTER(wheel_spin_multiplier)
DEFINE_NON_NEGATIVE_SETTER(maximum_steering_degrees)
DEFINE_NON_NEGATIVE_SETTER(steering_smoothing)
DEFINE_NON_NEGATIVE_SETTER(maximum_roll_degrees)
DEFINE_NON_NEGATIVE_SETTER(roll_acceleration_gain)
DEFINE_NON_NEGATIVE_SETTER(maximum_pitch_degrees)
DEFINE_NON_NEGATIVE_SETTER(pitch_acceleration_gain)
DEFINE_NON_NEGATIVE_SETTER(pose_smoothing)
DEFINE_NON_NEGATIVE_SETTER(kerb_vibration_amplitude)
DEFINE_NON_NEGATIVE_SETTER(gravel_vibration_amplitude)
DEFINE_NON_NEGATIVE_SETTER(vibration_frequency)
#undef DEFINE_NON_NEGATIVE_SETTER

void VehicleVisual3DConfig::_bind_methods() {
#define BIND_VECTOR_PROPERTY(name) ClassDB::bind_method(D_METHOD("set_" #name, "value"), &VehicleVisual3DConfig::set_##name); ClassDB::bind_method(D_METHOD("get_" #name), &VehicleVisual3DConfig::get_##name); ADD_PROPERTY(PropertyInfo(Variant::VECTOR3, #name), "set_" #name, "get_" #name)
    BIND_VECTOR_PROPERTY(model_scale);
    BIND_VECTOR_PROPERTY(model_rotation_degrees);
    BIND_VECTOR_PROPERTY(model_offset);
#undef BIND_VECTOR_PROPERTY
#define BIND_PATH_PROPERTY(name) ClassDB::bind_method(D_METHOD("set_" #name, "value"), &VehicleVisual3DConfig::set_##name); ClassDB::bind_method(D_METHOD("get_" #name), &VehicleVisual3DConfig::get_##name); ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH, #name), "set_" #name, "get_" #name)
    BIND_PATH_PROPERTY(model_path);
    BIND_PATH_PROPERTY(front_left_wheel_path);
    BIND_PATH_PROPERTY(front_right_wheel_path);
    BIND_PATH_PROPERTY(rear_left_wheel_path);
    BIND_PATH_PROPERTY(rear_right_wheel_path);
    BIND_PATH_PROPERTY(surface_probes_path);
#undef BIND_PATH_PROPERTY
#define BIND_FLOAT_PROPERTY(name, hint) ClassDB::bind_method(D_METHOD("set_" #name, "value"), &VehicleVisual3DConfig::set_##name); ClassDB::bind_method(D_METHOD("get_" #name), &VehicleVisual3DConfig::get_##name); ADD_PROPERTY(PropertyInfo(Variant::FLOAT, #name, PROPERTY_HINT_RANGE, hint), "set_" #name, "get_" #name)
    BIND_FLOAT_PROPERTY(wheel_radius, "0.05,1.5,0.01");
    BIND_FLOAT_PROPERTY(wheel_spin_multiplier, "0,3,0.01");
    BIND_FLOAT_PROPERTY(maximum_steering_degrees, "0,45,0.1");
    BIND_FLOAT_PROPERTY(steering_smoothing, "0.1,30,0.1");
    BIND_FLOAT_PROPERTY(maximum_roll_degrees, "0,8,0.1");
    BIND_FLOAT_PROPERTY(roll_acceleration_gain, "0,0.25,0.001");
    BIND_FLOAT_PROPERTY(maximum_pitch_degrees, "0,6,0.1");
    BIND_FLOAT_PROPERTY(pitch_acceleration_gain, "0,0.2,0.001");
    BIND_FLOAT_PROPERTY(pose_smoothing, "0.1,30,0.1");
    BIND_FLOAT_PROPERTY(kerb_vibration_amplitude, "0,0.08,0.001");
    BIND_FLOAT_PROPERTY(gravel_vibration_amplitude, "0,0.12,0.001");
    BIND_FLOAT_PROPERTY(vibration_frequency, "0,60,0.1");
#undef BIND_FLOAT_PROPERTY
    ClassDB::bind_method(D_METHOD("set_kerb_group", "value"), &VehicleVisual3DConfig::set_kerb_group);
    ClassDB::bind_method(D_METHOD("get_kerb_group"), &VehicleVisual3DConfig::get_kerb_group);
    ClassDB::bind_method(D_METHOD("set_gravel_group", "value"), &VehicleVisual3DConfig::set_gravel_group);
    ClassDB::bind_method(D_METHOD("get_gravel_group"), &VehicleVisual3DConfig::get_gravel_group);
    ADD_PROPERTY(PropertyInfo(Variant::STRING, "kerb_group"), "set_kerb_group", "get_kerb_group");
    ADD_PROPERTY(PropertyInfo(Variant::STRING, "gravel_group"), "set_gravel_group", "get_gravel_group");
    ClassDB::bind_method(D_METHOD("is_valid"), &VehicleVisual3DConfig::is_valid);
}

bool VehicleVisual3DConfig::is_valid() const {
    return model_scale.x > 0.0 && model_scale.y > 0.0 && model_scale.z > 0.0 && !model_path.is_empty() &&
        wheel_radius > 0.001 && steering_smoothing > 0.0 && pose_smoothing > 0.0 && vibration_frequency >= 0.0;
}
