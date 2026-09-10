#include "formula90s/ui/static_minimap_controller.hpp"
#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/variant/packed_vector2_array.hpp>
using namespace godot;
namespace {
constexpr double WORLD_LEFT = -90.0;
constexpr double WORLD_TOP = -280.0;
constexpr double WORLD_WIDTH = 180.0;
constexpr double WORLD_DEPTH = 360.0;
const Rect2 MAP_RECT(30.0, 22.0, 100.0, 180.0);
const Color BG_COLOR(0.015, 0.025, 0.045, 0.9);
const Color MAP_BG(0.11, 0.13, 0.16, 1.0);
const Color MAP_BORDER(0.42, 0.48, 0.58, 1.0);
const Color CENTER_LINE(0.72, 0.75, 0.78, 0.35);
const Color MARKER_COLOR(0.95, 0.18, 0.24, 1.0);
const Color SPAWN_COLOR(0.2, 0.95, 0.55, 1.0);
const Color ARROW_FILL(1.0, 0.82, 0.12, 1.0);
const Color ARROW_OUTLINE(0.05, 0.05, 0.04, 1.0);
}
void StaticMinimapController::_bind_methods() {
    ClassDB::bind_method(D_METHOD("set_target_path","path"),&StaticMinimapController::set_target_path);
    ClassDB::bind_method(D_METHOD("get_target_path"),&StaticMinimapController::get_target_path);
    ADD_PROPERTY(PropertyInfo(Variant::NODE_PATH,"target_path"),"set_target_path","get_target_path");
}
void StaticMinimapController::_ready() {
    if(!target_path.is_empty()) target=Object::cast_to<Node3D>(get_node_or_null(target_path));
    queue_redraw();
}
void StaticMinimapController::_process(double delta) {
    if(!target && !target_path.is_empty()) target=Object::cast_to<Node3D>(get_node_or_null(target_path));
    queue_redraw();
}
Vector2 StaticMinimapController::world_to_map(const Vector3 &position) const {
    double normalized_x = Math::clamp((position.x - WORLD_LEFT) / WORLD_WIDTH, 0.0, 1.0);
    double normalized_z = Math::clamp((position.z - WORLD_TOP) / WORLD_DEPTH, 0.0, 1.0);
    return MAP_RECT.position + Vector2(normalized_x * MAP_RECT.size.x, normalized_z * MAP_RECT.size.y);
}
void StaticMinimapController::_draw() {
    draw_rect(Rect2(Vector2(), get_size()), BG_COLOR, true);
    draw_rect(MAP_RECT, MAP_BG, true);
    draw_rect(MAP_RECT, MAP_BORDER, false, 2.0);
    draw_line(Vector2(MAP_RECT.get_center().x, MAP_RECT.position.y), Vector2(MAP_RECT.get_center().x, MAP_RECT.get_end().y), CENTER_LINE, 1.0);
    const Vector3 markers[] = { Vector3(-10,0,10), Vector3(10,0,-10), Vector3(-10,0,-40), Vector3(10,0,-80) };
    for (const Vector3 &marker : markers) draw_circle(world_to_map(marker), 2.5, MARKER_COLOR);
    Vector2 spawn = world_to_map(Vector3(0,0,30));
    draw_rect(Rect2(spawn-Vector2(3,3), Vector2(6,6)), SPAWN_COLOR, true);
    if (!target) return;
    Vector2 center = world_to_map(target->get_global_position());
    Vector3 world_forward = -target->get_global_basis().get_column(2);
    Vector2 heading(world_forward.x, world_forward.z);
    if (heading.length_squared() < 0.0001) heading = Vector2(0,-1); else heading = heading.normalized();
    Vector2 side(-heading.y, heading.x);
    PackedVector2Array arrow;
    arrow.push_back(center + heading * 9.0);
    arrow.push_back(center - heading * 6.0 + side * 5.0);
    arrow.push_back(center - heading * 3.0);
    arrow.push_back(center - heading * 6.0 - side * 5.0);
    draw_colored_polygon(arrow, ARROW_FILL);
    draw_polyline(arrow, ARROW_OUTLINE, 1.5);
}
