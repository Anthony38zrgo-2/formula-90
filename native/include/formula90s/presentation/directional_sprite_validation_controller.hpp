#pragma once

#include <godot_cpp/classes/node3d.hpp>

namespace godot {
class DirectionalVehicleSprite;
class Label;

class DirectionalSpriteValidationController : public Node3D {
	GDCLASS(DirectionalSpriteValidationController, Node3D)

	Node3D *vehicle = nullptr;
	DirectionalVehicleSprite *sprite = nullptr;
	Label *readout = nullptr;
	bool auto_rotate = true;
	double degrees_per_second = 30.0;

protected:
	static void _bind_methods();

public:
	DirectionalSpriteValidationController();
	void _ready() override;
	void _physics_process(double delta) override;
	void _process(double delta) override;
	void set_auto_rotate(bool value) { auto_rotate = value; }
	bool get_auto_rotate() const { return auto_rotate; }
	void set_degrees_per_second(double value) { degrees_per_second = value; }
	double get_degrees_per_second() const { return degrees_per_second; }
};

} // namespace godot
