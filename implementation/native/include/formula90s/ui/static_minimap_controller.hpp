#pragma once
#include <godot_cpp/classes/control.hpp>
namespace godot {
class Node3D;
class StaticMinimapController : public Control {
    GDCLASS(StaticMinimapController, Control)
    Node3D *target = nullptr; NodePath target_path;
    Vector2 world_to_map(const Vector3 &world_position) const;
protected:
    static void _bind_methods();
public:
    void _ready() override; void _process(double delta) override; void _draw() override;
    void set_target_path(const NodePath &p) { target_path = p; } NodePath get_target_path() const { return target_path; }
};
}
