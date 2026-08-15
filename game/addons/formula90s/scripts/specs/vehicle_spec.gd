class_name VehicleSpec
extends Resource

@export var id: StringName = &""
@export var display_name: String = ""

@export_group("Subsystems")
@export var chassis: ChassisSpec
@export var engine: EngineSpec
@export var gearbox: GearboxSpec
@export var suspension: SuspensionSpec
@export var steering_brakes: SteeringBrakesSpec
@export var tires: TiresSpec
@export var aero: AeroSpec

func apply_to(vehicle: Vehicle) -> void:
	if not vehicle:
		return

	if chassis:
		chassis.apply_to(vehicle)
	if engine:
		engine.apply_to(vehicle)
	if gearbox:
		gearbox.apply_to(vehicle)
	if suspension:
		suspension.apply_to(vehicle)
	if steering_brakes:
		steering_brakes.apply_to(vehicle)
	if tires:
		tires.apply_to(vehicle)
	if aero:
		aero.apply_to(vehicle)
