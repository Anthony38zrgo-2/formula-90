#include "formula90s/ui/main_menu_controller.hpp"
#include <godot_cpp/classes/button.hpp>
using namespace godot;
void MainMenuController::_bind_methods() {
    ClassDB::bind_method(D_METHOD("on_start_pressed"), &MainMenuController::on_start_pressed);
    ClassDB::bind_method(D_METHOD("on_quit_pressed"), &MainMenuController::on_quit_pressed);
    ADD_SIGNAL(MethodInfo("start_requested")); ADD_SIGNAL(MethodInfo("quit_requested"));
    ClassDB::bind_method(D_METHOD("set_start_button_path","path"),&MainMenuController::set_start_button_path);
    ClassDB::bind_method(D_METHOD("get_start_button_path"),&MainMenuController::get_start_button_path);
    ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH,"start_button_path"),"set_start_button_path","get_start_button_path");
    ClassDB::bind_method(D_METHOD("set_quit_button_path","path"),&MainMenuController::set_quit_button_path);
    ClassDB::bind_method(D_METHOD("get_quit_button_path"),&MainMenuController::get_quit_button_path);
    ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH,"quit_button_path"),"set_quit_button_path","get_quit_button_path");
}
void MainMenuController::_ready() {
    Button *start = Object::cast_to<Button>(get_node_or_null(start_button_path));
    Button *quit = Object::cast_to<Button>(get_node_or_null(quit_button_path));
    if (start && quit) { start->connect("pressed", Callable(this, "on_start_pressed")); quit->connect("pressed", Callable(this, "on_quit_pressed")); start->grab_focus(); }
}
void MainMenuController::on_start_pressed() { emit_signal("start_requested"); }
void MainMenuController::on_quit_pressed() { emit_signal("quit_requested"); }
