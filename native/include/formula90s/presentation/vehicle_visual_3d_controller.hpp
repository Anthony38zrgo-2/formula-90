#pragma once

#include "formula90s/presentation/vehicle_visual_3d_config.hpp"
#include <godot_cpp/classes/node3d.hpp>
#include <array>

namespace godot {

class ArcadeCarController;
class Node;

class VehicleVisual3DController : public Node3D {
    GDCLASS(VehicleVisual3DController, Node3D)

    Ref<VehicleVisual3DConfig> config;
    ArcadeCarController *car = nullptr;
    Node3D *model = nullptr;
    Node *surface_probes = nullptr;
    std::array<Node3D *, 4> wheels = { nullptr, nullptr, nullptr, nullptr };
    std::array<Basis, 4> wheel_base_bases;
    double smoothed_steering = 0.0;
    double smoothed_roll = 0.0;
    double smoothed_pitch = 0.0;
    double wheel_spin = 0.0;
    double vibration_phase = 0.0;
    uint64_t presentation_epoch = 0;
    bool initialized = false;
    bool model_ready = false;

    Node3D *resolve_optional_node(const NodePath &path) const;
    void resolve_nodes();
    void apply_model_configuration();
    double detect_surface_vibration() const;
    void apply_wheel_animation(double signed_speed, double delta);

protected:
    static void _bind_methods();

public:
    VehicleVisual3DController();
    void _ready() override;
    void _process(double delta) override;
    void set_config(const Ref<VehicleVisual3DConfig> &value);
    Ref<VehicleVisual3DConfig> get_config() const { return config; }
    bool has_model() const { return model_ready; }
    int get_resolved_wheel_count() const;
    double get_visual_roll_degrees() const { return smoothed_roll; }
    double get_visual_pitch_degrees() const { return smoothed_pitch; }
};

} // namespace godot
