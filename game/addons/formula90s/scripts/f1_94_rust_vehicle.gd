## F1-94 Rust Vehicle — Thin GDScript wrapper over the native GDExtension F194RustVehicle
## All 6-DOF physics, powertrain, tires, and suspension calculations run deterministically inside vehicle_physics_engine.dll.

class_name F194RustVehicleGD
extends F194RustVehicle

# All physics processing, raycasting, powertrain, suspension, tire model,
# and telemetry are executed directly by vehicle_physics_engine.dll via the
# native C++ GDExtension class F194RustVehicle.
