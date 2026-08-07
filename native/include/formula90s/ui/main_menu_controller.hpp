#pragma once
#include <godot_cpp/classes/control.hpp>
namespace godot { class MainMenuController : public Control { GDCLASS(MainMenuController, Control)
    NodePath start_button_path = "Center/Menu/StartButton";
    NodePath quit_button_path = "Center/Menu/QuitButton";
protected: static void _bind_methods();
public: void _ready() override; void on_start_pressed(); void on_quit_pressed();
    void set_start_button_path(const NodePath &p) { start_button_path = p; } NodePath get_start_button_path() const { return start_button_path; }
    void set_quit_button_path(const NodePath &p) { quit_button_path = p; } NodePath get_quit_button_path() const { return quit_button_path; }
}; }

