import argparse
import hashlib
import json
import math
import sys
from collections import Counter, defaultdict
from pathlib import Path

import bpy

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(Path(__file__).resolve().parent))
from common.output_policy import validate_output_path
from formula_one_2030_object_names import BODYWORK_COMPONENT_NAMES


BODYWORK_ASSEMBLY_NAME = "GEO_BODYWORK_NOSE_SIDEPODS_AND_ENGINE_COVER"


def file_digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def mesh_components(mesh):
    neighbors = [[] for vertex in mesh.vertices]
    for edge in mesh.edges:
        first_vertex, second_vertex = edge.vertices
        neighbors[first_vertex].append(second_vertex)
        neighbors[second_vertex].append(first_vertex)
    remaining_vertices = set(range(len(mesh.vertices)))
    components = []
    while remaining_vertices:
        pending_vertices = [remaining_vertices.pop()]
        component = []
        while pending_vertices:
            vertex_index = pending_vertices.pop()
            component.append(vertex_index)
            for neighbor_index in neighbors[vertex_index]:
                if neighbor_index in remaining_vertices:
                    remaining_vertices.remove(neighbor_index)
                    pending_vertices.append(neighbor_index)
        components.append(component)
    return sorted(components, key=len, reverse=True)


def component_for_polygon(mesh, polygon, vertex_components):
    connected_component = vertex_components[polygon.vertices[0]]
    center = polygon.center
    if connected_component == 5:
        return "BodyworkEngineCoverCentralFin"
    if connected_component in (6, 7):
        return "BodyworkEngineCover"
    if connected_component in (2, 3):
        return "BodyworkNose" if center.x < -1.118 else "BodyworkMonocoqueExterior"
    if connected_component == 4:
        return "BodyworkMonocoqueExterior"
    if center.x > 0.28 and (center.z > 0.17 or (center.x > 0.78 and abs(center.y) < 0.115)):
        return "BodyworkEngineCover"
    if polygon.material_index != 3 and center.x < 0.28 and (
        center.x < -0.42 or (abs(center.y) < 0.355 and center.z > 0.14)
    ):
        return "BodyworkMonocoqueExterior"
    return "BodyworkSidepodLeft" if center.y < 0 else "BodyworkSidepodRight"


def attribute_field(attribute):
    if not attribute.data:
        return None
    return next((field for field in ("value", "vector", "color") if hasattr(attribute.data[0], field)), None)


def attribute_value(entry, field):
    value = getattr(entry, field)
    return tuple(value) if hasattr(value, "__len__") else value


def mesh_fingerprint(mesh):
    document = {
        "vertices": [tuple(vertex.co) for vertex in mesh.vertices],
        "edges": [(tuple(edge.vertices), edge.use_edge_sharp) for edge in mesh.edges],
        "polygons": [(tuple(polygon.vertices), polygon.material_index, polygon.use_smooth) for polygon in mesh.polygons],
        "attributes": {},
        "materials": [material.name if material else None for material in mesh.materials],
        "normals": [tuple(normal.vector) for normal in mesh.corner_normals],
    }
    for attribute in mesh.attributes:
        field = attribute_field(attribute)
        if field is not None:
            document["attributes"][attribute.name] = [attribute_value(entry, field) for entry in attribute.data]
    return hashlib.sha256(json.dumps(document, sort_keys=True).encode()).hexdigest()


def protected_object_fingerprint(scene_object):
    return {
        "type": scene_object.type,
        "matrix": [list(row) for row in scene_object.matrix_world],
        "parent": scene_object.parent.name if scene_object.parent else None,
        "collections": sorted(collection.name for collection in scene_object.users_collection),
        "mesh": mesh_fingerprint(scene_object.data) if scene_object.type == "MESH" else None,
    }


def extract_component(source_mesh, polygon_indices, component_name):
    source_polygons = [source_mesh.polygons[index] for index in polygon_indices]
    vertex_indices = sorted({index for polygon in source_polygons for index in polygon.vertices})
    vertex_mapping = {source_index: destination_index for destination_index, source_index in enumerate(vertex_indices)}
    source_edge_lookup = {tuple(sorted(edge.vertices)): edge.index for edge in source_mesh.edges}
    component_mesh = bpy.data.meshes.new(component_name + "Mesh")
    component_mesh.from_pydata(
        [source_mesh.vertices[index].co[:] for index in vertex_indices],
        [],
        [[vertex_mapping[index] for index in polygon.vertices] for polygon in source_polygons],
    )
    for material in source_mesh.materials:
        component_mesh.materials.append(material)
    corner_indices = [index for polygon in source_polygons for index in polygon.loop_indices]
    edge_indices = [source_edge_lookup[tuple(sorted(vertex_indices[index] for index in edge.vertices))] for edge in component_mesh.edges]
    domain_indices = {"POINT": vertex_indices, "EDGE": edge_indices, "FACE": polygon_indices, "CORNER": corner_indices}
    topology_attributes = {"position", ".edge_verts", ".corner_vert", ".corner_edge", "custom_normal"}
    for source_attribute in source_mesh.attributes:
        if source_attribute.name in topology_attributes:
            continue
        field = attribute_field(source_attribute)
        if field is None:
            raise ValueError("Unsupported mesh attribute: " + source_attribute.name)
        destination_attribute = component_mesh.attributes.get(source_attribute.name)
        if destination_attribute is None:
            destination_attribute = component_mesh.attributes.new(source_attribute.name, source_attribute.data_type, source_attribute.domain)
        for destination_entry, source_index in zip(destination_attribute.data, domain_indices[source_attribute.domain]):
            setattr(destination_entry, field, attribute_value(source_attribute.data[source_index], field))
    for destination_polygon, source_polygon in zip(component_mesh.polygons, source_polygons):
        destination_polygon.material_index = source_polygon.material_index
        destination_polygon.use_smooth = source_polygon.use_smooth
    component_mesh.update()
    component_mesh.normals_split_custom_set([source_mesh.corner_normals[index].vector[:] for index in corner_indices])
    preserved_normals = component_mesh.attributes.new("bodywork_preserved_corner_normal", "FLOAT_VECTOR", "CORNER")
    for entry, source_index in zip(preserved_normals.data, corner_indices):
        entry.vector = source_mesh.corner_normals[source_index].vector
    for source_coordinates in source_mesh.uv_layers:
        destination_coordinates = component_mesh.uv_layers.get(source_coordinates.name)
        if destination_coordinates is None:
            raise ValueError("Missing texture coordinate layer: " + source_coordinates.name)
        destination_coordinates.active_render = source_coordinates.active_render
    component_mesh.uv_layers.active_index = source_mesh.uv_layers.active_index
    vertex_provenance = component_mesh.attributes.new("bodywork_source_vertex_index", "INT", "POINT")
    for entry, source_index in zip(vertex_provenance.data, vertex_indices):
        entry.value = source_index
    polygon_provenance = component_mesh.attributes.new("bodywork_source_polygon_index", "INT", "FACE")
    for entry, source_index in zip(polygon_provenance.data, polygon_indices):
        entry.value = source_index
    return component_mesh, domain_indices


def validate_component(source_mesh, component_mesh, domain_indices):
    maximum_normal_difference = 0.0
    for polygon, source_index in zip(component_mesh.polygons, domain_indices["FACE"]):
        source_polygon = source_mesh.polygons[source_index]
        restored_vertices = tuple(domain_indices["POINT"][index] for index in polygon.vertices)
        assert restored_vertices == tuple(source_polygon.vertices)
        assert polygon.material_index == source_polygon.material_index
        assert polygon.use_smooth == source_polygon.use_smooth
    for vertex, source_index in zip(component_mesh.vertices, domain_indices["POINT"]):
        assert tuple(vertex.co) == tuple(source_mesh.vertices[source_index].co)
    for source_attribute in source_mesh.attributes:
        if source_attribute.name in {"position", ".edge_verts", ".corner_vert", ".corner_edge", "custom_normal"}:
            continue
        destination_attribute = component_mesh.attributes[source_attribute.name]
        field = attribute_field(source_attribute)
        for destination_entry, source_index in zip(destination_attribute.data, domain_indices[source_attribute.domain]):
            assert attribute_value(destination_entry, field) == attribute_value(source_attribute.data[source_index], field), source_attribute.name
    for destination_normal, source_index in zip(component_mesh.corner_normals, domain_indices["CORNER"]):
        maximum_normal_difference = max(maximum_normal_difference, (destination_normal.vector - source_mesh.corner_normals[source_index].vector).length)
    maximum_normal_angle_degrees = math.degrees(2 * math.asin(min(1, maximum_normal_difference / 2)))
    assert maximum_normal_angle_degrees < 0.25, maximum_normal_angle_degrees
    component_mesh.calc_loop_triangles()
    source_mesh.calc_loop_triangles()
    selected_polygon_indices = set(domain_indices["FACE"])
    expected_triangles = Counter(
        tuple(triangle.vertices) for triangle in source_mesh.loop_triangles if triangle.polygon_index in selected_polygon_indices
    )
    actual_triangles = Counter(tuple(domain_indices["POINT"][index] for index in triangle.vertices) for triangle in component_mesh.loop_triangles)
    assert actual_triangles == expected_triangles, "Triangulation changed"
    return maximum_normal_angle_degrees


def main():
    arguments = sys.argv[sys.argv.index("--") + 1:]
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-file", type=Path, required=True)
    parser.add_argument("--output-directory", type=Path, required=True)
    options = parser.parse_args(arguments)
    repository = Path(__file__).resolve().parents[2]
    validate_output_path(repository, options.output_directory, "preview")
    options.output_directory.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file))
    source_object = bpy.data.objects[BODYWORK_ASSEMBLY_NAME]
    if source_object.type != "MESH":
        raise ValueError("Bodywork has already been separated")
    source_mesh = source_object.data
    source_mesh_name = source_mesh.name
    assert not source_mesh.shape_keys and not source_object.modifiers and not source_object.children
    protected_objects = {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object != source_object}
    components = mesh_components(source_mesh)
    assert [len(component) for component in components] == [5329, 5329, 1773, 449, 84, 76, 4, 4], "Unexpected source topology"
    vertex_components = {vertex_index: component_index for component_index, component in enumerate(components) for vertex_index in component}
    grouped_polygons = defaultdict(list)
    for polygon in source_mesh.polygons:
        grouped_polygons[component_for_polygon(source_mesh, polygon, vertex_components)].append(polygon.index)
    assert set(grouped_polygons) == set(BODYWORK_COMPONENT_NAMES)
    source_matrix = source_object.matrix_world.copy()
    source_parent = source_object.parent
    source_collections = list(source_object.users_collection)
    source_properties = {name: value for name, value in source_object.items()}
    source_object.name = "BodyworkSeparationOriginalTemporary"
    assembly = bpy.data.objects.new(BODYWORK_ASSEMBLY_NAME, None)
    assembly.parent = source_parent
    assembly.matrix_world = source_matrix
    assembly.empty_display_size = 0.15
    for name, value in source_properties.items():
        assembly[name] = value
    assembly["bodywork_component_names"] = json.dumps(BODYWORK_COMPONENT_NAMES)
    assembly["bodywork_source_digest"] = file_digest(options.source_file)
    for collection in source_collections:
        collection.objects.link(assembly)
    records = []
    for component_name in BODYWORK_COMPONENT_NAMES:
        component_mesh, domain_indices = extract_component(source_mesh, grouped_polygons[component_name], component_name)
        maximum_normal_angle_degrees = validate_component(source_mesh, component_mesh, domain_indices)
        component_object = bpy.data.objects.new(component_name, component_mesh)
        component_object.parent = assembly
        component_object.matrix_world = source_matrix
        for collection in source_collections:
            collection.objects.link(component_object)
        component_object["bodywork_source_assembly"] = BODYWORK_ASSEMBLY_NAME
        records.append({"name": component_name, "vertices": len(component_mesh.vertices), "polygons": len(component_mesh.polygons), "triangles": len(component_mesh.loop_triangles), "maximum_corner_normal_angle_degrees": maximum_normal_angle_degrees, "source_polygon_indices": domain_indices["FACE"]})
    assert sum(record["polygons"] for record in records) == len(source_mesh.polygons)
    source_mesh.calc_loop_triangles()
    original_triangle_count = len(source_mesh.loop_triangles)
    assert sum(record["triangles"] for record in records) == original_triangle_count
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected_objects} == protected_objects
    bpy.data.objects.remove(source_object, do_unlink=True)
    bpy.data.meshes.remove(source_mesh)
    bpy.ops.object.select_all(action="DESELECT")
    for component_name in BODYWORK_COMPONENT_NAMES:
        bpy.data.objects[component_name].select_set(True)
    bpy.context.view_layer.objects.active = bpy.data.objects["BodyworkMonocoqueExterior"]
    output_source = options.output_directory / "f1_2030.blend"
    validate_output_path(repository, output_source, "preview")
    bpy.ops.wm.save_as_mainfile(filepath=str(output_source))
    report = {
        "source_file": str(options.source_file),
        "source_digest": file_digest(options.source_file),
        "source_mesh_name": source_mesh_name,
        "output_file": str(output_source),
        "output_digest": file_digest(output_source),
        "original_assembly": BODYWORK_ASSEMBLY_NAME,
        "original_polygons": sum(record["polygons"] for record in records),
        "original_triangles": original_triangle_count,
        "positions_materials_texture_coordinates_and_triangulation_preserved": True,
        "protected_objects_preserved": True,
        "protected_object_fingerprints": protected_objects,
        "boundary_policy": "Partition existing polygons; duplicate shared boundary vertices at identical positions; no added caps, holes in the assembled surface, or new faces.",
        "left_side_policy": "Negative source lateral Y is driver left; source forward is negative X.",
        "components": records,
    }
    report_path = options.output_directory / "bodywork_separation_report.json"
    validate_output_path(repository, report_path, "preview")
    report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    bpy.ops.wm.open_mainfile(filepath=str(output_source))
    assert {scene_object.name: protected_object_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.name in protected_objects} == protected_objects
    assert bpy.data.objects[BODYWORK_ASSEMBLY_NAME].type == "EMPTY"
    assert all(bpy.data.objects[name].parent.name == BODYWORK_ASSEMBLY_NAME for name in BODYWORK_COMPONENT_NAMES)
    assert all(tuple(layer.name for layer in bpy.data.objects[name].data.uv_layers) == tuple(layer.name for layer in bpy.data.objects[BODYWORK_COMPONENT_NAMES[0]].data.uv_layers) for name in BODYWORK_COMPONENT_NAMES)
    with bpy.data.libraries.load(str(options.source_file), link=False) as (available_data, imported_data):
        assert source_mesh_name in available_data.meshes
        imported_data.meshes = [source_mesh_name]
    reopened_baseline_mesh = imported_data.meshes[0]
    baseline_edges = {tuple(sorted(edge.vertices)): edge.index for edge in reopened_baseline_mesh.edges}
    for component_name in BODYWORK_COMPONENT_NAMES:
        reopened_component_mesh = bpy.data.objects[component_name].data
        vertex_indices = [entry.value for entry in reopened_component_mesh.attributes['bodywork_source_vertex_index'].data]
        polygon_indices = [entry.value for entry in reopened_component_mesh.attributes['bodywork_source_polygon_index'].data]
        corner_indices = [index for polygon_index in polygon_indices for index in reopened_baseline_mesh.polygons[polygon_index].loop_indices]
        edge_indices = [baseline_edges[tuple(sorted(vertex_indices[index] for index in edge.vertices))] for edge in reopened_component_mesh.edges]
        validate_component(reopened_baseline_mesh, reopened_component_mesh, {'POINT': vertex_indices, 'FACE': polygon_indices, 'CORNER': corner_indices, 'EDGE': edge_indices})
        for entry, source_index in zip(reopened_component_mesh.attributes['bodywork_preserved_corner_normal'].data, corner_indices):
            assert tuple(entry.vector) == tuple(reopened_baseline_mesh.corner_normals[source_index].vector)
    report['saved_source_reopened_and_geometry_revalidated'] = True
    validate_output_path(repository, report_path, 'preview')
    report_path.write_text(json.dumps(report, indent=2), encoding='utf-8')
    print(json.dumps({"saved_source": str(output_source), "polygons": report["original_polygons"], "triangles": original_triangle_count, "components": [{name: value for name, value in record.items() if name != "source_polygon_indices"} for record in records]}, indent=2))


if __name__ == "__main__":
    main()
