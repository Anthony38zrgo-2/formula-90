#pragma once

#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/variant/variant.hpp>

namespace godot {

// Read-only boundary between GEVP vehicle state and native presentation.
// Mutating a vehicle requires a separately justified command boundary.
class VehicleStateReader {
	Node3D *node_ = nullptr;

public:
	VehicleStateReader() = default;
	explicit VehicleStateReader(Node *node) { set_node(Object::cast_to<Node3D>(node)); }

	void set_node(Node3D *node) { node_ = node; }
	bool is_valid() const { return node_ != nullptr; }

	double get_speed() const { return double(node_->get("speed")); }
	double get_motor_rpm() const { return double(node_->get("motor_rpm")); }
	int get_current_gear() const { return int(node_->get("current_gear")); }
	double get_throttle_amount() const { return double(node_->get("throttle_amount")); }
	double get_true_steering_amount() const { return double(node_->get("true_steering_amount")); }
	double get_steering_input() const { return double(node_->get("steering_input")); }

	Vector3 get_linear_velocity() const { return Vector3(node_->get("linear_velocity")); }
	Transform3D get_global_transform() const { return node_->get_global_transform(); }
	Vector3 get_global_position() const { return node_->get_global_position(); }
};

} // namespace godot
