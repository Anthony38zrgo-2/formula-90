#include "formula90s/ui/main_menu_controller.hpp"
#include <godot_cpp/classes/button.hpp>
using namespace godot;
void MainMenuController::_bind_methods() { ClassDB::bind_method(D_METHOD("on_start_pressed"), &MainMenuController::on_start_pressed); ClassDB::bind_method(D_METHOD("on_quit_pressed"), &MainMenuController::on_quit_pressed); ADD_SIGNAL(MethodInfo("start_requested")); ADD_SIGNAL(MethodInfo("quit_requested")); }
void MainMenuController::_ready() { Button *start = Object::cast_to<Button>(get_node_or_null("Center/Menu/StartButton")); Button *quit = Object::cast_to<Button>(get_node_or_null("Center/Menu/QuitButton")); if (start && quit) { start->connect("pressed", Callable(this, "on_start_pressed")); quit->connect("pressed", Callable(this, "on_quit_pressed")); start->grab_focus(); } }
void MainMenuController::on_start_pressed() { emit_signal("start_requested"); }
void MainMenuController::on_quit_pressed() { emit_signal("quit_requested"); }
