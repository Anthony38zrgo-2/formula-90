#pragma once
#include <godot_cpp/classes/control.hpp>
namespace godot { class MainMenuController : public Control { GDCLASS(MainMenuController, Control)
protected: static void _bind_methods();
public: void _ready() override; void on_start_pressed(); void on_quit_pressed(); }; }

