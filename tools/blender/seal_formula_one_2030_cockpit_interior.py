import argparse
import hashlib
import json
import math
import subprocess
import sys
from pathlib import Path

import bpy
import bmesh
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

PROJECT_DIRECTORY = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools'))
sys.path.insert(0, str(PROJECT_DIRECTORY / 'tools/blender'))
from common.output_policy import validate_output_path
from export_f1_2030_v10 import export_chassis
from fit_f1_2030_driver_seat import mesh_fingerprint
from fit_f1_2030_cockpit_clearance import driver_capture_provenance
from validate_formula_one_2030_cockpit_geometry import inspect_mesh_geometry, validate_cockpit_geometry


def surface_for_object(scene_object, conversion=Matrix.Identity(4)):
    return BVHTree.FromPolygons([conversion @ scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices], [list(polygon.vertices) for polygon in scene_object.data.polygons])


def cockpit_opening_contour():
    body = bpy.data.objects['GEO_CHASSIS_BODY']
    topology = bmesh.new()
    topology.from_mesh(body.data)
    bmesh.ops.transform(topology, matrix=body.matrix_world, verts=list(topology.verts))
    bmesh.ops.remove_doubles(topology, verts=list(topology.verts), dist=0.00001)
    remaining_edges = set(edge for edge in topology.edges if edge.is_boundary)
    contours = []
    while remaining_edges:
        first_edge = remaining_edges.pop()
        component_edges = {first_edge}
        pending_edges = [first_edge]
        while pending_edges:
            current_edge = pending_edges.pop()
            for vertex in current_edge.verts:
                for adjacent_edge in vertex.link_edges:
                    if adjacent_edge in remaining_edges:
                        remaining_edges.remove(adjacent_edge)
                        component_edges.add(adjacent_edge)
                        pending_edges.append(adjacent_edge)
        positions = [vertex.co for edge in component_edges for vertex in edge.verts]
        if min(position.x for position in positions) > -0.6 and max(position.x for position in positions) < 0.45 and min(position.z for position in positions) > 0.145 and max(position.z for position in positions) < 0.22 and len(component_edges) > 40:
            start_vertex = next(iter(component_edges)).verts[0]
            current_vertex = start_vertex
            previous_edge = None
            contour = []
            while True:
                contour.append(current_vertex.co.copy())
                choices = [edge for edge in current_vertex.link_edges if edge in component_edges and edge != previous_edge]
                if len(choices) != (2 if previous_edge is None else 1):
                    raise RuntimeError('Cockpit boundary is not a simple closed contour')
                current_edge = choices[0]
                current_vertex = current_edge.other_vert(current_vertex)
                previous_edge = current_edge
                if current_vertex == start_vertex:
                    break
            contours.append(contour)
    topology.free()
    if len(contours) != 1:
        raise RuntimeError('Could not identify exactly one cockpit opening')
    return contours[0]


def section_of_contour(contour,longitudinal_position,side_sign):
    hits=[]
    for first,second in zip(contour,contour[1:]+contour[:1]):
        if abs(second.x-first.x)<1e-7:
            if abs(longitudinal_position-first.x)<1e-6:
                hits.extend([first.copy(),second.copy()])
        else:
            fraction=(longitudinal_position-first.x)/(second.x-first.x)
            if min(first.x,second.x)-1e-6<=longitudinal_position<=max(first.x,second.x)+1e-6:
                hits.append(first.lerp(second,min(1,max(0,fraction))))
    if not hits:
        raise RuntimeError('Missing contour section')
    return max(hits,key=lambda position:position.y*side_sign)


def interior_texture_samples(mesh):
    return {layer.name:sorted(set((mesh.loops[index].vertex_index,float(coordinate.uv.x),float(coordinate.uv.y)) for index,coordinate in enumerate(layer.data))) for layer in mesh.uv_layers}


def create_upper_lining():
    interior=bpy.data.objects['GEO_CHASSIS_INTERIOR']
    if len(interior.data.vertices)!=4686:
        raise RuntimeError('Unexpected interior topology')
    original_positions=[vertex.co.copy() for vertex in interior.data.vertices]
    original_texture_samples=interior_texture_samples(interior.data)
    opening_contour=cockpit_opening_contour()
    for row in range(71):
        longitudinal_position=interior.data.vertices[row*33].co.x
        if longitudinal_position <= -1.249:
            for layer in (0,2343):
                for column in (0,32):
                    interior.data.vertices[layer+row*33+column].co.z=0.160
        if min(position.x for position in opening_contour)<=longitudinal_position<=max(position.x for position in opening_contour):
            for column,side_sign in ((0,-1),(32,1)):
                opening_position=section_of_contour(opening_contour,longitudinal_position,side_sign)
                attachment_vertex=interior.data.vertices[row*33+column]
                displacement=side_sign*max(abs(opening_position.y)+0.010-abs(attachment_vertex.co.y),0)
                attachment_vertex.co.y+=displacement
                interior.data.vertices[2343+row*33+column].co.y+=displacement
    interior.data.update()
    interior_topology=bmesh.new()
    interior_topology.from_mesh(interior.data)
    interior_topology.verts.ensure_lookup_table()
    for face in list(interior_topology.faces):
        indices=[vertex.index for vertex in face.verts]
        if len(indices)==4 and (all(index<2343 for index in indices) or all(index>=2343 for index in indices)):
            first_index=min(range(4),key=lambda index:indices[index]%2343)
            bmesh.ops.triangulate(interior_topology,faces=[face],quad_method='FIXED' if first_index%2==0 else 'ALTERNATE')
    bmesh.ops.triangulate(interior_topology,faces=[face for face in interior_topology.faces if len(face.verts)>3],quad_method='FIXED')
    bmesh.ops.recalc_face_normals(interior_topology,faces=list(interior_topology.faces))
    interior_topology.to_mesh(interior.data)
    interior_topology.free()
    left_wall=[interior.matrix_world@interior.data.vertices[row*33].co for row in range(71)]
    right_wall=[interior.matrix_world@interior.data.vertices[row*33+32].co for row in range(71)]
    lower_contour=left_wall+list(reversed(right_wall))
    opening_front=min(position.x for position in opening_contour)
    opening_rear=max(position.x for position in opening_contour)
    outer_front=min(position.x for position in lower_contour)
    outer_rear=max(position.x for position in lower_contour)
    vertices=[]
    faces=[]
    body_surface=surface_for_object(bpy.data.objects['GEO_CHASSIS_BODY'])
    for side_sign in (-1,1):
        stations=sorted(set([round(position.x,7) for position in opening_contour+lower_contour if opening_front-1e-6<=position.x<=opening_rear+1e-6]+[opening_front,opening_rear]))
        stations=[value for index,value in enumerate(stations) if index==0 or value-stations[index-1]>1e-6]
        stations[0]=opening_front
        stations[-1]=opening_rear
        start_index=len(vertices)
        for longitudinal_position in stations:
            opening_position=section_of_contour(opening_contour,longitudinal_position,side_sign)
            attachment=section_of_contour(lower_contour,longitudinal_position,side_sign)
            for level in range(3):
                vertices.append(opening_position.lerp(attachment,level/2))
        for row in range(len(stations)-1):
            for level in range(2):
                first_index=start_index+row*3+level
                second_index=first_index+3
                faces.append((first_index,second_index,second_index+1,first_index+1))
    for is_front in (True,False):
        boundary=opening_front if is_front else opening_rear
        outer_boundary=outer_front if is_front else outer_rear
        stations=sorted(set([round(position.x,7) for position in lower_contour if min(boundary,outer_boundary)-1e-6<=position.x<=max(boundary,outer_boundary)+1e-6]+[boundary,outer_boundary]))
        stations=[value for index,value in enumerate(stations) if index==0 or value-stations[index-1]>1e-6]
        stations[0]=min(boundary,outer_boundary)
        stations[-1]=max(boundary,outer_boundary)
        start_index=len(vertices)
        for longitudinal_position in stations:
            left_attachment=section_of_contour(lower_contour,longitudinal_position,-1)
            right_attachment=section_of_contour(lower_contour,longitudinal_position,1)
            if abs(longitudinal_position-boundary)<1e-6:
                upper_left=section_of_contour(opening_contour,boundary,-1)
                upper_right=section_of_contour(opening_contour,boundary,1)
            else:
                location,normal,polygon_index,distance=body_surface.ray_cast(Vector((longitudinal_position,0.1,0.02)),Vector((0,0,1)),0.33)
                center_height=location.z-0.006 if location is not None and normal.z>0.5 else max(left_attachment.z,right_attachment.z)+0.03
                outer_transition=min(abs(longitudinal_position-outer_boundary)/0.075,1)
                outer_transition=outer_transition*outer_transition*(3-2*outer_transition)
                center_height=(left_attachment.z+right_attachment.z)/2*(1-outer_transition)+center_height*outer_transition
                upper_left=Vector((longitudinal_position,left_attachment.y*0.70,center_height))
                upper_right=Vector((longitudinal_position,right_attachment.y*0.70,center_height))
                opening_transition=min(abs(longitudinal_position-boundary)/0.075,1)
                opening_transition=opening_transition*opening_transition*(3-2*opening_transition)
                boundary_left=section_of_contour(opening_contour,boundary,-1)
                boundary_right=section_of_contour(opening_contour,boundary,1)
                upper_left.y=boundary_left.y*(1-opening_transition)+upper_left.y*opening_transition
                upper_right.y=boundary_right.y*(1-opening_transition)+upper_right.y*opening_transition
                upper_left.z=boundary_left.z*(1-opening_transition)+upper_left.z*opening_transition
                upper_right.z=boundary_right.z*(1-opening_transition)+upper_right.z*opening_transition
            for position in (left_attachment,left_attachment.lerp(upper_left,0.5),upper_left,upper_right,upper_right.lerp(right_attachment,0.5),right_attachment):
                vertices.append(position)
        for row in range(len(stations)-1):
            for lateral_index in range(5):
                first_index=start_index+row*6+lateral_index
                second_index=first_index+6
                faces.append((first_index,second_index,second_index+1,first_index+1))
    previous_lining=bpy.data.objects.get('GEO_CHASSIS_CockpitUpperLining')
    collections=list(previous_lining.users_collection) if previous_lining else list(interior.users_collection)
    if previous_lining:
        bpy.data.objects.remove(previous_lining,do_unlink=True)
    lining_mesh=bpy.data.meshes.new('StationMatchedCockpitContourTransition')
    lining_mesh.from_pydata(vertices,[],faces)
    lining_mesh.materials.append(interior.data.materials[0])
    lining=bpy.data.objects.new('GEO_CHASSIS_CockpitUpperLining',lining_mesh)
    for collection in collections:
        collection.objects.link(lining)
    lining_topology=bmesh.new()
    lining_topology.from_mesh(lining_mesh)
    bmesh.ops.remove_doubles(lining_topology,verts=list(lining_topology.verts),dist=0.000002)
    bmesh.ops.recalc_face_normals(lining_topology,faces=list(lining_topology.faces))
    bmesh.ops.triangulate(lining_topology,faces=list(lining_topology.faces))
    lining_topology.to_mesh(lining_mesh)
    lining_topology.free()
    bpy.ops.object.select_all(action='DESELECT')
    lining.select_set(True)
    bpy.context.view_layer.objects.active=lining
    lining_surface_positions=[vertex.co.copy() for vertex in lining_mesh.vertices]
    original_faces=[tuple(polygon.vertices) for polygon in lining_mesh.polygons]
    boundary_topology=bmesh.new()
    boundary_topology.from_mesh(lining_mesh)
    boundary_topology.verts.ensure_lookup_table()
    boundary_edges=[tuple(vertex.index for vertex in edge.verts) for edge in boundary_topology.edges if edge.is_boundary]
    boundary_topology.free()
    vertex_count=len(lining_surface_positions)
    closed_positions=lining_surface_positions+[position+Vector((0,0,0.0015)) for position in lining_surface_positions]
    closed_faces=original_faces+[tuple(index+vertex_count for index in reversed(face)) for face in original_faces]
    closed_faces += [(first_index,second_index,second_index+vertex_count,first_index+vertex_count) for first_index,second_index in boundary_edges]
    lining_mesh.clear_geometry()
    lining_mesh.from_pydata(closed_positions,[],closed_faces)
    lining_topology=bmesh.new()
    lining_topology.from_mesh(lining.data)
    bmesh.ops.recalc_face_normals(lining_topology,faces=list(lining_topology.faces))
    if lining_topology.calc_volume(signed=True)<0:
        bmesh.ops.reverse_faces(lining_topology,faces=list(lining_topology.faces))
    lining_topology.to_mesh(lining.data)
    lining_topology.free()
    texture_coordinates=lining_mesh.uv_layers.new(name='CockpitLiningTextureCoordinates')
    for polygon in lining_mesh.polygons:
        for loop_index in polygon.loop_indices:
            position=lining_mesh.vertices[lining_mesh.loops[loop_index].vertex_index].co
            texture_coordinates.data[loop_index].uv=((position.x+1.4)/2,(position.y+0.4)/0.8)

    changed_vertices=[{'vertex':index,'before':list(position),'after':list(interior.data.vertices[index].co),'displacement_meters':(position-interior.data.vertices[index].co).length} for index,position in enumerate(original_positions) if (position-interior.data.vertices[index].co).length>1e-8]
    allowed_vertices={layer+row*33+column for layer in (0,2343) for row in range(71) for column in (0,32)}
    if any(sample['vertex'] not in allowed_vertices or abs(sample['before'][0]-sample['after'][0])>1e-8 or (sample['before'][0]>-1.249 and abs(sample['before'][2]-sample['after'][2])>1e-8) for sample in changed_vertices):
        raise RuntimeError('Interior changes extend beyond the lateral attachment rim')
    if original_texture_samples!=interior_texture_samples(interior.data):
        raise RuntimeError('Interior texture coordinates changed')
    lining['cockpit_interior_seal']=True
    return lining, {'construction':'Longitudinal sections with matched side, front and rear joins','vertices':len(lining.data.vertices),'polygons':len(lining.data.polygons),'vertical_shell_separation_meters':0.0015,'interior_rim_changes':changed_vertices,'interior_texture_coordinates_preserved':True}


def validate_side_sealing(lining):
    surface = surface_for_object(lining)
    samples = []
    for longitudinal_position in (-0.9, -0.6, -0.3):
        for side_sign in (-1, 1):
            origin = Vector((longitudinal_position, 0, 0.15))
            direction = Vector((0, side_sign, 0))
            location, normal, polygon_index, distance = surface.ray_cast(origin, direction, 0.35)
            samples.append({'origin':list(origin), 'side':side_sign, 'sealed':location is not None and normal.dot(direction) < 0, 'distance_meters':distance, 'normal_dot_direction':normal.dot(direction) if normal else None})
    return samples


def validate_steering_sweep(lining_surface, conversion):
    steering_wheel = bpy.data.objects['GEO_CHASSIS_STEER']
    steering_column = bpy.data.objects['GEO_CHASSIS_STEERCOLUM']
    wheel_positions = [conversion @ steering_wheel.matrix_world @ vertex.co for vertex in steering_wheel.data.vertices]
    column_positions = [conversion @ steering_column.matrix_world @ vertex.co for vertex in steering_column.data.vertices]
    pivot = Vector(((min(position.x for position in column_positions) + max(position.x for position in column_positions)) / 2, (min(position.y for position in wheel_positions) + max(position.y for position in wheel_positions)) / 2, (min(position.z for position in column_positions) + max(position.z for position in column_positions)) / 2))
    failures = []
    for object_name in ('GEO_CHASSIS_STEER', 'GEO_CHASSIS_SCREEN', 'GEO_CHASSIS_SHIFTERBRAK', 'GEO_CHASSIS_SHIFTERTHRO', 'GEO_CHASSIS_STEERCOLUM', 'GEO_CHASSIS_CYLINDER_001'):
        scene_object = bpy.data.objects[object_name]
        original_positions = [conversion @ scene_object.matrix_world @ vertex.co for vertex in scene_object.data.vertices]
        polygons = [list(polygon.vertices) for polygon in scene_object.data.polygons]
        rotates = object_name not in ('GEO_CHASSIS_STEERCOLUM', 'GEO_CHASSIS_CYLINDER_001')
        for steering_degrees in range(-180, 181, 3) if rotates else (0,):
            rotation = Matrix.Rotation(math.radians(-steering_degrees), 4, 'Y')
            positions = [pivot + rotation @ (position - pivot) for position in original_positions] if rotates else original_positions
            surface = BVHTree.FromPolygons(positions, polygons)
            intersections = lining_surface.overlap(surface)
            if intersections:
                failures.append({'object':object_name, 'steering_degrees':steering_degrees, 'pairs':len(intersections)})
    return failures


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-file', type=Path, required=True)
    parser.add_argument('--driver-surfaces', type=Path, required=True)
    parser.add_argument('--output-directory', required=True)
    options = parser.parse_args(sys.argv[sys.argv.index('--') + 1:] if '--' in sys.argv else [])
    destination = validate_output_path(PROJECT_DIRECTORY, options.output_directory, 'preview').path
    destination.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.open_mainfile(filepath=str(options.source_file.resolve()))
    for source_image in bpy.data.images:
        if source_image.source == 'FILE' and source_image.filepath and not source_image.packed_file:
            if not Path(bpy.path.abspath(source_image.filepath)).is_file():
                raise RuntimeError('Source texture is missing; open the Blender source from its authored directory: ' + source_image.filepath)
    protected_meshes = {scene_object.name:mesh_fingerprint(scene_object) for scene_object in bpy.data.objects if scene_object.type == 'MESH' and scene_object.name not in ('GEO_CHASSIS_INTERIOR', 'GEO_CHASSIS_CockpitUpperLining')}
    report = {'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=PROJECT_DIRECTORY,text=True).strip(),'source_before_sha256':hashlib.sha256(Path(bpy.data.filepath).read_bytes()).hexdigest()}
    report['source_branch'] = subprocess.check_output(['git','branch','--show-current'],cwd=PROJECT_DIRECTORY,text=True).strip()
    report['source_status'] = subprocess.check_output(['git','status','--short'],cwd=PROJECT_DIRECTORY,text=True).splitlines()
    report['protected_mesh_fingerprints'] = protected_meshes
    report['interior_before_fingerprint'] = mesh_fingerprint(bpy.data.objects['GEO_CHASSIS_INTERIOR'])
    report['interior_geometry_before'] = inspect_mesh_geometry(bpy.data.objects['GEO_CHASSIS_INTERIOR'])
    lining, measurements = create_upper_lining()
    report['lining'] = measurements
    report['interior_after_fingerprint'] = mesh_fingerprint(bpy.data.objects['GEO_CHASSIS_INTERIOR'])
    report['full_geometry_validation'] = validate_cockpit_geometry()
    if not report['full_geometry_validation']['passed']:
        raise RuntimeError('Reconstructed cockpit geometry or full perimeter failed validation')
    report['protected_meshes_unchanged'] = all(mesh_fingerprint(bpy.data.objects[name]) == fingerprint for name,fingerprint in protected_meshes.items())
    if not report['protected_meshes_unchanged']:
        raise RuntimeError('An existing mesh changed')
    manifest = json.loads((PROJECT_DIRECTORY / 'game/assets/models/vehicles/f1-2030/manifest.json').read_text())
    vertical_offset = manifest['authored_alignment']['chassis_vertical_offset']
    conversion = Matrix.Translation((0,0,vertical_offset)) @ Matrix.Rotation(-math.pi / 2,4,'Z')
    lining_surface = surface_for_object(lining, conversion)
    interior_surface = surface_for_object(bpy.data.objects['GEO_CHASSIS_INTERIOR'], conversion)
    report['seat_intersection_pairs'] = len(lining_surface.overlap(surface_for_object(bpy.data.objects['GEO_CHASSIS_SEAT'], conversion)))
    report['interior_seat_intersection_pairs'] = len(interior_surface.overlap(surface_for_object(bpy.data.objects['GEO_CHASSIS_SEAT'], conversion)))
    if options.driver_surfaces:
        driver_path = options.driver_surfaces.resolve()
        provenance = json.loads(driver_path.with_suffix('.provenance.json').read_text())
        fresh = all(provenance.get(name) == value for name,value in driver_capture_provenance().items())
        report['driver_capture_matches_current_sources'] = fresh
        if not fresh:
            raise RuntimeError('Driver capture does not match current source files')
        if hashlib.sha256(driver_path.read_bytes()).hexdigest() != provenance['surfaces_sha256']:
            raise RuntimeError('Driver capture content does not match its recorded hash')
        if provenance['seated_position'] != [0,-0.011,-0.24]:
            raise RuntimeError('Driver capture seated position changed')
        for relative_path, expected_digest in provenance['camera_dependencies_sha256'].items():
            if hashlib.sha256((PROJECT_DIRECTORY / 'game' / relative_path).read_bytes()).hexdigest() != expected_digest:
                raise RuntimeError('Driver capture camera dependencies changed')
        captured = json.loads(driver_path.read_text())
        if len(captured['samples']) < 1000 or min(sample['steering_degrees'] for sample in captured['samples']) != -180 or max(sample['steering_degrees'] for sample in captured['samples']) != 180:
            raise RuntimeError('Driver capture does not cover the complete steering validation sequence')
        failures = []
        for index,sample in enumerate(captured['samples']):
            surface = BVHTree.FromPolygons([Vector(position) for position in sample['vertices']],sample['triangles'],all_triangles=True)
            intersections = lining_surface.overlap(surface)
            if intersections:
                failures.append({'object':lining.name,'sample':index,'steering_degrees':sample['steering_degrees'],'pairs':len(intersections)})
            interior_intersections = interior_surface.overlap(surface)
            if interior_intersections:
                failures.append({'object':'GEO_CHASSIS_INTERIOR','sample':index,'steering_degrees':sample['steering_degrees'],'pairs':len(interior_intersections)})
        report['driver_samples'] = len(captured['samples'])
        report['driver_intersections'] = failures
        report['driver_surfaces_sha256'] = hashlib.sha256(driver_path.read_bytes()).hexdigest()
        report['driver_capture_provenance'] = provenance
        print('DRIVER',report['driver_samples'],len(failures),failures[:8],flush=True)
    report['steering_intersections'] = validate_steering_sweep(lining_surface, conversion)
    report['interior_steering_intersections'] = validate_steering_sweep(interior_surface, conversion)
    report['sealed_side_samples'] = validate_side_sealing(lining)
    report['passed'] = report['full_geometry_validation']['passed'] and not report['seat_intersection_pairs'] and not report['interior_seat_intersection_pairs'] and not report['driver_intersections'] and not report['steering_intersections'] and not report['interior_steering_intersections'] and all(sample['sealed'] for sample in report['sealed_side_samples'])
    report_path = validate_output_path(PROJECT_DIRECTORY,destination / 'cockpit_sealing_report.json','preview').path
    report_path.write_text(json.dumps(report,indent=2)+'\n')
    if not report['passed']:
        raise RuntimeError('Cockpit sealing validation failed; inspect the report')
    bpy.context.preferences.filepaths.save_version = 0
    candidate = validate_output_path(PROJECT_DIRECTORY,destination / 'f1_2030_cockpit_sealed.blend','preview').path
    bpy.ops.wm.save_as_mainfile(filepath=str(candidate))
    chassis_path = validate_output_path(PROJECT_DIRECTORY,destination / 'f1_2030_v10_chassis.glb','preview').path
    report['chassis_mesh_statistics'] = export_chassis(chassis_path, vertical_offset)
    report['candidate_sha256'] = hashlib.sha256(candidate.read_bytes()).hexdigest()
    report['chassis_sha256'] = hashlib.sha256(chassis_path.read_bytes()).hexdigest()
    report_path.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'passed':report['passed'],'lining_vertices':len(lining.data.vertices),'protected_meshes':len(protected_meshes),'interior_rim_vertices_adjusted':len(measurements['interior_rim_changes']),'driver_samples':report['driver_samples'],'driver_intersection_samples':len(report['driver_intersections']),'opening_perimeter_samples':report['full_geometry_validation']['opening_perimeter']['samples'],'interior_perimeter_samples':report['full_geometry_validation']['interior_perimeter']['samples']}),flush=True)


if __name__ == '__main__':
    main()
