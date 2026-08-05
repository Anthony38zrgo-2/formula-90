#pragma once
#include <godot_cpp/classes/control.hpp>
namespace godot { class DebugHudController : public Control { GDCLASS(DebugHudController, Control)
protected: static void _bind_methods();
public: void _process(double delta) override; }; }

