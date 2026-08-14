#pragma once
#include <godot_cpp/classes/node.hpp>
namespace godot {
class GameBootstrap : public Node {
    GDCLASS(GameBootstrap, Node)
    String menu_scene_path = "res://scenes/ui/main_menu.tscn";
    String track_scene_path = "res://scenes/tracks/test_field/formula90s_test_track.tscn";
    String world_compositor_scene_path = "res://scenes/runtime/world_hud_compositor.tscn";
protected:
    static void _bind_methods();
public:
    void _ready() override;
    void _process(double delta) override;
    void show_menu(); void start_game(); void quit_game();
    void set_menu_scene_path(const String &p) { menu_scene_path = p; } String get_menu_scene_path() const { return menu_scene_path; }
    void set_track_scene_path(const String &p) { track_scene_path = p; } String get_track_scene_path() const { return track_scene_path; }
    void set_world_compositor_scene_path(const String &p) { world_compositor_scene_path = p; } String get_world_compositor_scene_path() const { return world_compositor_scene_path; }
private: void replace_content(const godot::String &path);
};
}
