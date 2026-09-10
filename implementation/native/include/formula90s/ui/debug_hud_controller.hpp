#pragma once
#include <godot_cpp/classes/control.hpp>
namespace godot {
class DebugHudController : public Control {
    GDCLASS(DebugHudController, Control)
    NodePath vehicle_path, aids_path;
protected: static void _bind_methods();
public:
    void _process(double delta) override;
    void set_vehicle_path(const NodePath &p) { vehicle_path = p; } NodePath get_vehicle_path() const { return vehicle_path; }
    void set_aids_path(const NodePath &p) { aids_path = p; } NodePath get_aids_path() const { return aids_path; }
};
}
