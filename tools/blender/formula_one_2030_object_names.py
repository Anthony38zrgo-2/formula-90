import json
from pathlib import Path


CHASSIS_OBJECT_NAMES = {
    "GEO_CHASSIS_ACTIVEFRONT": "GEO_FRONT_WING_INTERMEDIATE_ELEMENT",
    "GEO_CHASSIS_AERO2": "GEO_FRONT_WING_UPPER_ELEMENT",
    "GEO_CHASSIS_AEROPART1": "GEO_SIDEPOD_FRONT_FLOW_DEFLECTORS",
    "GEO_CHASSIS_AEROPART2": "GEO_REAR_WING_UPPER_ELEMENT",
    "GEO_CHASSIS_AEROPART3": "GEO_REAR_WING_LOWER_BEAM_ELEMENT",
    "GEO_CHASSIS_ACTIVEREAR": "GEO_REAR_WING_MAIN_ELEMENT",
    "GEO_CHASSIS_BODY": "GEO_BODYWORK_NOSE_SIDEPODS_AND_ENGINE_COVER",
    "GEO_CHASSIS_CARBON3": "GEO_ENGINE_AIR_INTAKE_INLET_SURFACE",
    "GEO_CHASSIS_CARBON4": "GEO_COCKPIT_MIRROR_REFLECTIVE_SURFACES",
    "GEO_CHASSIS_CARBON5": "GEO_ENGINE_COVER_REAR_CLOSURE",
    "GEO_CHASSIS_CYLINDER_001": "GEO_STEERING_WHEEL_CENTER_FASTENER",
    "GEO_CHASSIS_CockpitHeadrestPadding": "GEO_COCKPIT_HeadrestPadding",
    "GEO_CHASSIS_CockpitUpperLining": "GEO_COCKPIT_UpperInteriorLining",
    "GEO_CHASSIS_ENDPLATE": "GEO_FRONT_WING_ENDPLATES",
    "GEO_CHASSIS_EXHAUST_TUBE_LEFT": "GEO_EXHAUST_OUTLET_TUBE_LEFT",
    "GEO_CHASSIS_EXHAUST_TUBE_RIGHT": "GEO_EXHAUST_OUTLET_TUBE_RIGHT",
    "GEO_CHASSIS_FLOOR": "GEO_UNDERBODY_FLOOR_AND_DIFFUSER",
    "GEO_CHASSIS_FRONTSUPPORT": "GEO_FRONT_WING_MOUNTING_PYLONS",
    "GEO_CHASSIS_FRONTWING": "GEO_FRONT_WING_MAIN_ELEMENT",
    "GEO_CHASSIS_FRONT_SUSPENSION": "GEO_SUSPENSION_FRONT_LINK_ASSEMBLY",
    "GEO_CHASSIS_INTERIOR": "GEO_COCKPIT_INTERIOR_SHELL",
    "GEO_CHASSIS_LATERALCAM": "GEO_ENGINE_COVER_SIDE_CAMERA_HOUSINGS",
    "GEO_CHASSIS_MIRRORS": "GEO_COCKPIT_MIRROR_HOUSINGS_AND_SUPPORTS",
    "GEO_CHASSIS_ONBOARDCAM": "GEO_ENGINE_COVER_TOP_CAMERA_HOUSING",
    "GEO_CHASSIS_REARLIGHT": "GEO_REAR_SAFETY_LIGHT",
    "GEO_CHASSIS_REARSUPPORT": "GEO_REAR_WING_CENTRAL_SUPPORT",
    "GEO_CHASSIS_REAR_SUSPENSION": "GEO_SUSPENSION_REAR_LINK_ASSEMBLY",
    "GEO_CHASSIS_REAR_WING": "GEO_REAR_WING_ENDPLATE_ASSEMBLY",
    "GEO_CHASSIS_REAR_WING_MID": "GEO_REAR_WING_INTERMEDIATE_ELEMENT",
    "GEO_CHASSIS_SCREEN": "GEO_STEERING_WHEEL_DISPLAY",
    "GEO_CHASSIS_SEAT": "GEO_COCKPIT_DRIVER_SEAT",
    "GEO_CHASSIS_SHIFTERBRAK": "GEO_STEERING_WHEEL_PADDLE_LEFT",
    "GEO_CHASSIS_SHIFTERTHRO": "GEO_STEERING_WHEEL_PADDLE_RIGHT",
    "GEO_CHASSIS_STEER": "GEO_STEERING_WHEEL_BODY",
    "GEO_CHASSIS_STEERCOLUM": "GEO_STEERING_COLUMN_SHAFT",
}

WHEEL_POSITIONS = {
    "FRONT_L": "FRONT_LEFT",
    "FRONT_R": "FRONT_RIGHT",
    "REAR_L": "REAR_LEFT",
    "REAR_R": "REAR_RIGHT",
}

WHEEL_COMPONENTS = {
    "NUT_02": ("WHEEL", "CENTER_LOCK_NUT"),
    "NUT_05": ("BRAKE", "CALIPER"),
    "RIM_01": ("WHEEL", "HUB"),
    "RIM_03": ("BRAKE", "DISC"),
    "RIM_04": ("WHEEL", "RIM_AND_SPOKES"),
    "RIM_06": ("BRAKE", "INBOARD_DUCT_COVER"),
    "TIRE": ("WHEEL", "TIRE"),
}

OBJECT_NAME_MIGRATION = dict(CHASSIS_OBJECT_NAMES)
for previous_position, current_position in WHEEL_POSITIONS.items():
    OBJECT_NAME_MIGRATION[f"WHEEL_{previous_position}_CONTROL"] = f"WHEEL_{current_position}_CONTROL"
    for previous_component, (assembly, current_component) in WHEEL_COMPONENTS.items():
        OBJECT_NAME_MIGRATION[f"GEO_WHEEL_{previous_position}_{previous_component}"] = f"GEO_{assembly}_{current_position}_{current_component}"

OBJECT_NAME_MIGRATION.update({
    "StudioGround": "StudioGroundPlane",
    "StudioOverheadStrip": "StudioOverheadStripLight",
    "StudioFrontThreeQuarter": "StudioFrontThreeQuarterCamera",
    "StudioRearThreeQuarter": "StudioRearThreeQuarterCamera",
    "StudioSide": "StudioSideCamera",
    "StudioTop": "StudioTopCamera",
})


def canonical_object_name(name):
    if name in OBJECT_NAME_MIGRATION:
        return OBJECT_NAME_MIGRATION[name]
    for previous_name in sorted(OBJECT_NAME_MIGRATION, key=len, reverse=True):
        if name.startswith(previous_name + "_") or name.startswith(previous_name + "."):
            return OBJECT_NAME_MIGRATION[previous_name] + name[len(previous_name):]
    if name.startswith("GEO_CHASSIS_DECAL_"):
        return name.replace("GEO_CHASSIS_DECAL_", "GEO_BODYWORK_DECAL_", 1)
    for previous_position, current_position in WHEEL_POSITIONS.items():
        for previous_component, (assembly, current_component) in WHEEL_COMPONENTS.items():
            current_prefix = f"GEO_WHEEL_{current_position}_{previous_component}"
            if name == current_prefix or name.startswith(current_prefix + "_"):
                return f"GEO_{assembly}_{current_position}_{current_component}" + name[len(current_prefix):]
        previous_prefix = f"GEO_WHEEL_{previous_position}_DECAL_"
        if name.startswith(previous_prefix):
            return name.replace(previous_prefix, f"GEO_WHEEL_{current_position}_DECAL_", 1)
    return name


def normalize_object_name_document(value):
    if isinstance(value, str):
        return canonical_object_name(value)
    if isinstance(value, list):
        return [normalize_object_name_document(item) for item in value]
    if isinstance(value, dict):
        return {canonical_object_name(key): normalize_object_name_document(item) for key, item in value.items()}
    return value


def wheel_source_object_names(position):
    current_position = WHEEL_POSITIONS.get(position, position)
    return tuple(f"GEO_{assembly}_{current_position}_{component}" for assembly, component in WHEEL_COMPONENTS.values())


def is_chassis_object(name):
    return canonical_object_name(name) in CHASSIS_OBJECT_NAMES.values() or name.startswith(("GEO_CHASSIS_", "GEO_BODYWORK_DECAL_"))


def is_wheel_object(name, position):
    current_position = WHEEL_POSITIONS.get(position, position)
    current_name = canonical_object_name(name)
    return current_name in wheel_source_object_names(current_position) or current_name.startswith(f"GEO_WHEEL_{current_position}_DECAL_")


def normalize_loaded_vehicle_names(unused_scene=None):
    import bpy
    for scene_object in bpy.data.objects:
        current_name = canonical_object_name(scene_object.name)
        if current_name != scene_object.name:
            collision = bpy.data.objects.get(current_name)
            if collision is not None and collision != scene_object:
                raise ValueError("Object name migration collision: " + current_name)
            scene_object.name = current_name
        normalize_object_reference_properties(scene_object)
    for mesh in bpy.data.meshes:
        mesh.name = canonical_object_name(mesh.name)


def enable_legacy_source_names():
    import bpy
    from bpy.app.handlers import persistent
    normalize_loaded_vehicle_names()
    if not any(handler.__name__ == "normalize_loaded_vehicle_names" for handler in bpy.app.handlers.load_post):
        bpy.app.handlers.load_post.append(persistent(normalize_loaded_vehicle_names))


def read_object_name_document(path):
    return normalize_object_name_document(json.loads(Path(path).read_text(encoding="utf-8")))


def library_object_name(available_names, current_name):
    for available_name in available_names:
        if canonical_object_name(available_name) == current_name:
            return available_name
    raise KeyError('Library object is missing: ' + current_name)


def normalize_imported_object_names(imported_objects):
    for scene_object in imported_objects:
        scene_object.name = canonical_object_name(scene_object.name)


def normalize_object_reference_properties(scene_object):
    for property_name, value in list(scene_object.items()):
        if isinstance(value, str):
            try:
                document = json.loads(value)
            except json.JSONDecodeError:
                scene_object[property_name] = canonical_object_name(value)
            else:
                normalized_document = normalize_object_name_document(document)
                if normalized_document != document:
                    scene_object[property_name] = json.dumps(normalized_document)
