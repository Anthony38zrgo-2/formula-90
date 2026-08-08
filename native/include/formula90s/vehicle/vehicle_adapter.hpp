#pragma once

#include <godot_cpp/classes/node3d.hpp>
#include <godot_cpp/variant/variant.hpp>
#include <godot_cpp/core/math.hpp>

namespace godot {

class VehicleAdapter {
	Node3D *node_ = nullptr;

public:
	VehicleAdapter() = default;
	explicit VehicleAdapter(Node *node) { set_node(Object::cast_to<Node3D>(node)); }
	void set_node(Node3D *node) { node_ = node; }
	bool is_valid() const { return node_ != nullptr; }
	Node3D *get_node() const { return node_; }

	double get_speed() const { return double(node_->get("speed")); }
	double get_motor_rpm() const { return double(node_->get("motor_rpm")); }
	int get_current_gear() const { return int(node_->get("current_gear")); }
	double get_throttle_amount() const { return double(node_->get("throttle_amount")); }
	double get_true_steering_amount() const { return double(node_->get("true_steering_amount")); }
	double get_steering_input() const { return double(node_->get("steering_input")); }
	double get_brake_amount() const { return double(node_->get("brake_amount")); }
	bool get_automatic_transmission() const { return bool(node_->get("automatic_transmission")); }
	Vector3 get_linear_velocity() const { return Vector3(node_->get("linear_velocity")); }
	Vector3 get_angular_velocity() const { return Vector3(node_->get("angular_velocity")); }

	Transform3D get_global_transform() const { return node_->get_global_transform(); }
	Vector3 get_global_position() const { return node_->get_global_position(); }

	void set_global_transform(const Transform3D &t) { node_->set_global_transform(t); }
	void set_linear_velocity(const Vector3 &v) { node_->set("linear_velocity", v); }
	void set_angular_velocity(const Vector3 &v) { node_->set("angular_velocity", v); }

	void reset_motion() {
		set_linear_velocity(Vector3());
		set_angular_velocity(Vector3());
	}
};

} // namespace godot
