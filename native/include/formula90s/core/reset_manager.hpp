#pragma once
#include <godot_cpp/classes/node.hpp>
namespace godot { class ArcadeCarController; class ResetManager : public Node { GDCLASS(ResetManager, Node)
    Transform3D spawn; ArcadeCarController *car=nullptr; NodePath car_path = "../PlayerCar";
protected: static void _bind_methods();
public: void _ready() override; void _physics_process(double delta) override; void reset_vehicle();
    void set_car_path(const NodePath &p) { car_path = p; } NodePath get_car_path() const { return car_path; }
}; }

