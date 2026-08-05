#pragma once
#include <godot_cpp/classes/node.hpp>
namespace godot { class ArcadeCarController; class ResetManager : public Node { GDCLASS(ResetManager, Node)
    Transform3D spawn; ArcadeCarController *car=nullptr;
protected: static void _bind_methods();
public: void _ready() override; void _physics_process(double delta) override; void reset_vehicle(); }; }

