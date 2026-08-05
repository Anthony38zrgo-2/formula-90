#pragma once
#include <godot_cpp/classes/control.hpp>
namespace godot {
class ArcadeCarController;
class StaticMinimapController : public Control {
    GDCLASS(StaticMinimapController, Control)
    ArcadeCarController *car = nullptr;
    Vector2 world_to_map(const Vector3 &world_position) const;
protected:
    static void _bind_methods();
public:
    void _ready() override;
    void _process(double delta) override;
    void _draw() override;
};
}

