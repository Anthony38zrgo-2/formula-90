#include "formula90s/presentation/vehicle_visual_3d_controller.hpp"

#include "formula90s/presentation/vehicle_visual_3d_math.hpp"
#include "formula90s/vehicle/arcade_car_controller.hpp"
#include <godot_cpp/classes/ray_cast3d.hpp>
#include <godot_cpp/core/math.hpp>
#include <godot_cpp/variant/utility_functions.hpp>
#include <numbers>

using namespace godot;

namespace {
constexpr int FRONT_LEFT = 0;
constexpr int FRONT_RIGHT = 1;
constexpr int REAR_LEFT = 2;
constexpr int REAR_RIGHT = 3;
constexpr double TWO_PI = 2.0 * std::numbers::pi_v<double>;
constexpr double VIBRATION_PHASE_MULT = 0.73;
constexpr double VIBRATION_AMPLITUDE_FACTOR = 0.35;

bool surface_matches(Object *collider, const String &group, const String &surface_name) {
    Node *node = Object::cast_to<Node>(collider);
    if (node != nullptr && !group.is_empty() && node->is_in_group(StringName(group))) {
        return true;
    }
    if (collider != nullptr && collider->has_meta("surface_type")) {
        return String(collider->get_meta("surface_type")).to_lower() == surface_name;
    }
    return false;
}
}

VehicleVisual3DController::VehicleVisual3DController() { set_process_priority(5); }

void VehicleVisual3DController::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_config", "config"), &VehicleVisual3DController::set_config);
    ClassDB::bind_method(D_METHOD("get_config"), &VehicleVisual3DController::get_config);
    ClassDB::bind_method(D_METHOD("has_model"), &VehicleVisual3DController::has_model);
    ClassDB::bind_method(D_METHOD("get_resolved_wheel_count"), &VehicleVisual3DController::get_resolved_wheel_count);
    ClassDB::bind_method(D_METHOD("get_visual_roll_degrees"), &VehicleVisual3DController::get_visual_roll_degrees);
    ClassDB::bind_method(D_METHOD("get_visual_pitch_degrees"), &VehicleVisual3DController::get_visual_pitch_degrees);
    ADD_PROPERTY(PropertyInfo(Variant::OBJECT, "config", PROPERTY_HINT_RESOURCE_TYPE, "VehicleVisual3DConfig"), "set_config", "get_config");
    ClassDB::bind_method(D_METHOD("set_car_path","path"),&VehicleVisual3DController::set_car_path);
    ClassDB::bind_method(D_METHOD("get_car_path"),&VehicleVisual3DController::get_car_path);
    ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH,"car_path"),"set_car_path","get_car_path");
}

void VehicleVisual3DController::set_config(const Ref<VehicleVisual3DConfig> &value) {
    config = value;
    if (is_inside_tree()) {
        resolve_nodes();
    }
}

Node3D *VehicleVisual3DController::resolve_optional_node(const NodePath &path) const {
    if (path.is_empty()) {
        return nullptr;
    }
    return Object::cast_to<Node3D>(get_node_or_null(path));
}

void VehicleVisual3DController::resolve_nodes() {
    model = nullptr;
    surface_probes = nullptr;
    wheels.fill(nullptr);
    model_ready = false;
    if (config.is_null() || !config->is_valid()) {
        UtilityFunctions::push_error("[formula90s] VehicleVisual3DController requires a valid VehicleVisual3DConfig");
        return;
    }
    model = resolve_optional_node(config->get_model_path());
    if (model == nullptr) {
        UtilityFunctions::push_error("[formula90s] V10 visual model node was not found at ", String(config->get_model_path()));
        return;
    }
    wheels[FRONT_LEFT] = resolve_optional_node(config->get_front_left_wheel_path());
    wheels[FRONT_RIGHT] = resolve_optional_node(config->get_front_right_wheel_path());
    wheels[REAR_LEFT] = resolve_optional_node(config->get_rear_left_wheel_path());
    wheels[REAR_RIGHT] = resolve_optional_node(config->get_rear_right_wheel_path());
    for (int index = 0; index < int(wheels.size()); ++index) {
        if (wheels[index] != nullptr) {
            wheel_base_bases[index] = wheels[index]->get_basis();
        }
    }
    if (!config->get_surface_probes_path().is_empty()) {
        surface_probes = get_node_or_null(config->get_surface_probes_path());
    }
    apply_model_configuration();
    model_ready = true;
    UtilityFunctions::print("[formula90s] V10 3D visual ready; optional wheel nodes: ", get_resolved_wheel_count());
}

void VehicleVisual3DController::apply_model_configuration() {
    if (model == nullptr || config.is_null()) {
        return;
    }
    const Vector3 degrees = config->get_model_rotation_degrees();
    const Basis rotation = Basis(Vector3(0.0, 1.0, 0.0), Math::deg_to_rad(degrees.y)) *
        Basis(Vector3(1.0, 0.0, 0.0), Math::deg_to_rad(degrees.x)) *
        Basis(Vector3(0.0, 0.0, 1.0), Math::deg_to_rad(degrees.z));
    model->set_transform(Transform3D(rotation.scaled(config->get_model_scale()), config->get_model_offset()));
}

double VehicleVisual3DController::detect_surface_vibration() const {
    if (surface_probes == nullptr || config.is_null()) {
        return 0.0;
    }
    bool kerb = false;
    const int child_count = surface_probes->get_child_count();
    for (int index = 0; index < child_count; ++index) {
        RayCast3D *probe = Object::cast_to<RayCast3D>(surface_probes->get_child(index));
        if (probe == nullptr || !probe->is_colliding()) {
            continue;
        }
        Object *collider = probe->get_collider();
        if (surface_matches(collider, config->get_gravel_group(), "gravel")) {
            return config->get_gravel_vibration_amplitude();
        }
        kerb = kerb || surface_matches(collider, config->get_kerb_group(), "kerb") || surface_matches(collider, "piano", "piano");
    }
    return kerb ? config->get_kerb_vibration_amplitude() : 0.0;
}

void VehicleVisual3DController::apply_wheel_animation(double signed_speed, double delta) {
    if (config.is_null()) {
        return;
    }
    wheel_spin = Math::fmod(wheel_spin - formula90s::presentation::wheel_rotation_delta(
        signed_speed * config->get_wheel_spin_multiplier(), config->get_wheel_radius(), delta), TWO_PI);
    const Basis steering_basis(Vector3(0.0, 1.0, 0.0), smoothed_steering);
    const Basis spin_basis(Vector3(1.0, 0.0, 0.0), wheel_spin);
    for (int index = 0; index < int(wheels.size()); ++index) {
        if (wheels[index] == nullptr) {
            continue;
        }
        const bool front = index == FRONT_LEFT || index == FRONT_RIGHT;
        wheels[index]->set_basis(wheel_base_bases[index] * (front ? steering_basis : Basis()) * spin_basis);
    }
}

void VehicleVisual3DController::_ready() {
    car = Object::cast_to<ArcadeCarController>(get_node_or_null(car_path));
    if (car == nullptr) {
        UtilityFunctions::push_error("[formula90s] VehicleVisual3DController: car not found at ", car_path);
        return;
    }
    set_as_top_level(true);
    resolve_nodes();
    presentation_epoch = car->get_presentation_epoch();
    set_global_transform(car->get_visual_transform());
    initialized = true;
}

void VehicleVisual3DController::_process(double delta) {
    if (car == nullptr || config.is_null() || !config->is_valid() || delta <= 0.0) {
        return;
    }
    const bool discontinuity = !initialized || presentation_epoch != car->get_presentation_epoch();
    presentation_epoch = car->get_presentation_epoch();
    const Transform3D visual_pose = car->get_visual_transform();
    Vector3 right = visual_pose.basis.get_column(0);
    Vector3 forward = -visual_pose.basis.get_column(2);
    right.y = 0.0;
    forward.y = 0.0;
    right = right.normalized();
    forward = forward.normalized();
    const Vector3 acceleration = car->get_world_acceleration();
    const double roll_target = formula90s::presentation::clamped_visual_response(
        acceleration.dot(right), config->get_roll_acceleration_gain(), config->get_maximum_roll_degrees());
    const double pitch_target = formula90s::presentation::clamped_visual_response(
        acceleration.dot(forward), config->get_pitch_acceleration_gain(), config->get_maximum_pitch_degrees());
    const double steering_target = Math::deg_to_rad(config->get_maximum_steering_degrees()) * car->get_steering_input();
    if (discontinuity) {
        smoothed_roll = 0.0;
        smoothed_pitch = 0.0;
        smoothed_steering = steering_target;
        wheel_spin = 0.0;
        vibration_phase = 0.0;
        initialized = true;
    } else {
        const double pose_alpha = formula90s::presentation::smoothing_alpha(config->get_pose_smoothing(), delta);
        smoothed_roll = Math::lerp(smoothed_roll, roll_target, pose_alpha);
        smoothed_pitch = Math::lerp(smoothed_pitch, pitch_target, pose_alpha);
        smoothed_steering = Math::lerp(smoothed_steering, steering_target,
            formula90s::presentation::smoothing_alpha(config->get_steering_smoothing(), delta));
    }
    const double vibration_amplitude = detect_surface_vibration();
    vibration_phase += delta * config->get_vibration_frequency() * TWO_PI;
    const Vector3 vibration_local(
        Math::sin(vibration_phase * VIBRATION_PHASE_MULT) * vibration_amplitude * VIBRATION_AMPLITUDE_FACTOR,
        Math::sin(vibration_phase) * vibration_amplitude,
        0.0);
    Transform3D presentation = visual_pose;
    const Basis dynamic_rotation = Basis(Vector3(0.0, 0.0, 1.0), Math::deg_to_rad(smoothed_roll)) *
        Basis(Vector3(1.0, 0.0, 0.0), Math::deg_to_rad(smoothed_pitch));
    presentation.basis = visual_pose.basis * dynamic_rotation;
    presentation.origin += visual_pose.basis.xform(vibration_local);
    set_global_transform(presentation);
    const double signed_speed = car->get_velocity().dot(forward);
    apply_wheel_animation(signed_speed, delta);
}

int VehicleVisual3DController::get_resolved_wheel_count() const {
    int count = 0;
    for (Node3D *wheel : wheels) {
        count += wheel != nullptr ? 1 : 0;
    }
    return count;
}
