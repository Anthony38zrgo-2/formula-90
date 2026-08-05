#pragma once
#include <godot_cpp/classes/node.hpp>
namespace godot {
class GameBootstrap : public Node {
    GDCLASS(GameBootstrap, Node)
protected:
    static void _bind_methods();
public:
    void _ready() override;
    void _process(double delta) override;
    void show_menu(); void start_game(); void quit_game();
private: void replace_content(const godot::String &path);
};
}
