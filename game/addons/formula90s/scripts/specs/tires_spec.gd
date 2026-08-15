class_name TiresSpec
extends Resource

@export_group("Dimensions and Mass")
@export var front_tire_radius: float = 0.316954494
@export var front_tire_width: float = 300.298989
@export var front_wheel_mass: float = 12.0
@export var rear_tire_radius: float = 0.329008996
@export var rear_tire_width: float = 368.324995
@export var rear_wheel_mass: float = 16.0
@export var contact_patch: float = 0.21

@export_group("Surface Grip Tables")
@export var tire_stiffnesses: Dictionary = {
	"Curb": 7.0,
	"Dirt": 0.5,
	"Grass": 0.5,
	"Gravel": 0.5,
	"Road": 8.75
}
@export var coefficient_of_friction: Dictionary = {
	"Curb": 2.2,
	"Dirt": 1.4,
	"Grass": 0.9,
	"Gravel": 1.1,
	"Road": 2.9
}
@export var rolling_resistance: Dictionary = {
	"Curb": 1.5,
	"Dirt": 2.0,
	"Grass": 4.0,
	"Gravel": 2.0,
	"Road": 1.0
}
@export var lateral_grip_assist: Dictionary = {
	"Curb": 0.0,
	"Dirt": 0.0,
	"Grass": 0.0,
	"Gravel": 0.0,
	"Road": 0.02
}
@export var longitudinal_grip_ratio: Dictionary = {
	"Curb": 0.45,
	"Dirt": 0.56,
	"Grass": 0.45,
	"Gravel": 0.45,
	"Road": 0.52
}

func apply_to(vehicle: Vehicle) -> void:
	if not vehicle:
		return
	vehicle.front_tire_radius = front_tire_radius
	vehicle.front_tire_width = front_tire_width
	vehicle.front_wheel_mass = front_wheel_mass
	vehicle.rear_tire_radius = rear_tire_radius
	vehicle.rear_tire_width = rear_tire_width
	vehicle.rear_wheel_mass = rear_wheel_mass
	vehicle.contact_patch = contact_patch

	vehicle.tire_stiffnesses = tire_stiffnesses.duplicate()
	vehicle.coefficient_of_friction = coefficient_of_friction.duplicate()
	vehicle.rolling_resistance = rolling_resistance.duplicate()
	vehicle.lateral_grip_assist = lateral_grip_assist.duplicate()
	vehicle.longitudinal_grip_ratio = longitudinal_grip_ratio.duplicate()
