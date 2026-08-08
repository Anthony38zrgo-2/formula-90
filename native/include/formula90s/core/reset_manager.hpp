#pragma once

#include "formula90s/vehicle/vehicle_adapter.hpp"
#include <godot_cpp/classes/node.hpp>

namespace godot {

class ResetManager : public Node {
	GDCLASS(ResetManager, Node)

	Transform3D spawn;
	VehicleAdapter car_adapter;
	NodePath car_path = "../PlayerCar";

protected:
	static void _bind_methods();

public:
	void _ready() override;
	void _physics_process(double delta) override;
	void reset_vehicle();
	void set_car_path(const NodePath &p) { car_path = p; }
	NodePath get_car_path() const { return car_path; }
};

} // namespace godot
