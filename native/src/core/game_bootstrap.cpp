#include "formula90s/core/game_bootstrap.hpp"
#include <godot_cpp/variant/utility_functions.hpp>
#include <godot_cpp/classes/resource_loader.hpp>
#include <godot_cpp/classes/packed_scene.hpp>
#include <godot_cpp/classes/scene_tree.hpp>
#include <godot_cpp/classes/input.hpp>
using namespace godot;
void GameBootstrap::_bind_methods() { ClassDB::bind_method(D_METHOD("show_menu"),&GameBootstrap::show_menu);ClassDB::bind_method(D_METHOD("start_game"),&GameBootstrap::start_game);ClassDB::bind_method(D_METHOD("quit_game"),&GameBootstrap::quit_game);ClassDB::bind_method(D_METHOD("set_menu_scene_path","path"),&GameBootstrap::set_menu_scene_path);ClassDB::bind_method(D_METHOD("get_menu_scene_path"),&GameBootstrap::get_menu_scene_path);ADD_PROPERTY(PropertyInfo(Variant::STRING,"menu_scene_path",PROPERTY_HINT_FILE,"*.tscn"),"set_menu_scene_path","get_menu_scene_path");ClassDB::bind_method(D_METHOD("set_track_scene_path","path"),&GameBootstrap::set_track_scene_path);ClassDB::bind_method(D_METHOD("get_track_scene_path"),&GameBootstrap::get_track_scene_path);ADD_PROPERTY(PropertyInfo(Variant::STRING,"track_scene_path",PROPERTY_HINT_FILE,"*.tscn"),"set_track_scene_path","get_track_scene_path");ClassDB::bind_method(D_METHOD("set_world_compositor_scene_path","path"),&GameBootstrap::set_world_compositor_scene_path);ClassDB::bind_method(D_METHOD("get_world_compositor_scene_path"),&GameBootstrap::get_world_compositor_scene_path);ADD_PROPERTY(PropertyInfo(Variant::STRING,"world_compositor_scene_path",PROPERTY_HINT_FILE,"*.tscn"),"set_world_compositor_scene_path","get_world_compositor_scene_path"); }
void GameBootstrap::_ready() { UtilityFunctions::print("[formula90s] GDExtension bootstrap ready"); show_menu(); }
void GameBootstrap::replace_content(const String &path){ Node *old=get_node_or_null("Content");if(old){remove_child(old);old->queue_free();}Ref<PackedScene> scene=ResourceLoader::get_singleton()->load(path);if(scene.is_null()){UtilityFunctions::printerr("[formula90s] cannot load ",path);return;}Node *node=scene->instantiate();node->set_name("Content");add_child(node);}
void GameBootstrap::show_menu(){replace_content(menu_scene_path);Node *menu=get_node_or_null("Content");if(menu){menu->connect("start_requested",Callable(this,"start_game"));menu->connect("quit_requested",Callable(this,"quit_game"));}}
void GameBootstrap::start_game(){replace_content(world_compositor_scene_path);}
void GameBootstrap::quit_game(){get_tree()->quit();}
void GameBootstrap::_process(double delta){if(get_node_or_null("Content/PlayerCar")&&Input::get_singleton()->is_action_just_pressed("ui_back_to_menu"))show_menu();}
