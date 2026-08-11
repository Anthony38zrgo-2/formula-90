#!/usr/bin/env python3
"""Thin bpy implementation for the probe-blend command (ANL-001).

Runs inside Blender in background mode:
  blender.exe --background --python scripts/blender_scene_probe.py -- \
      --root <repo-root> --config <config-path>

Opens the requested .blend without saving it, reports per-object mesh and
transform facts, clears evaluated meshes and emits exactly one JSON object
prefixed by a sentinel line. It never renders, imports external assets,
modifies the scene or saves the file.
"""
import argparse
import json
import math
import os
import sys

import mathutils

SENTINEL = '__FORMULA90_ANALYSIS_JSON__'


def emit(payload):
    print(SENTINEL + json.dumps(
        payload,
        sort_keys=True,
        separators=(',', ':'),
        ensure_ascii=False,
        allow_nan=False,
    ))


def determinant3(matrix):
    return (
        matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
    )


def matrix_rows(matrix):
    return [
        [matrix[0][0], matrix[0][1], matrix[0][2], matrix[0][3]],
        [matrix[1][0], matrix[1][1], matrix[1][2], matrix[1][3]],
        [matrix[2][0], matrix[2][1], matrix[2][2], matrix[2][3]],
        [matrix[3][0], matrix[3][1], matrix[3][2], matrix[3][3]],
    ]


def object_facts(name, obj, depsgraph):
    """Deterministic per-object facts; exceptions become an object-level error."""
    try:
        if obj.type == 'MESH':
            mesh_facts = mesh_facts_for(obj, depsgraph)
        else:
            mesh_facts = None
    except Exception as exc:  # noqa: BLE001 - keep the probe going per object
        return {'name': name, 'exists': True, 'type': obj.type,
                'error': str(exc)}
    return {
        'name': name,
        'exists': True,
        'type': obj.type,
        'world_transform': matrix_rows(obj.matrix_world),
        'world_determinant': determinant3(
            [row[:3] for row in matrix_rows(obj.matrix_world)]
        ),
        'mesh_facts': mesh_facts,
    }


def mesh_facts_for(obj, depsgraph):
    """Evaluate the object's mesh, collect facts, then clear the evaluated mesh."""
    evaluated = obj.evaluated_get(depsgraph)
    mesh = evaluated.to_mesh()
    try:
        local_min = [math.inf, math.inf, math.inf]
        local_max = [-math.inf, -math.inf, -math.inf]
        finite = True
        for vertex in mesh.vertices:
            co = vertex.co
            if not all(math.isfinite(v) for v in co):
                finite = False
                continue
            for axis in range(3):
                local_min[axis] = min(local_min[axis], co[axis])
                local_max[axis] = max(local_max[axis], co[axis])
        world_min = [math.inf, math.inf, math.inf]
        world_max = [-math.inf, -math.inf, -math.inf]
        for corner in (
            (local_min[0], local_min[1], local_min[2]),
            (local_min[0], local_min[1], local_max[2]),
            (local_min[0], local_max[1], local_min[2]),
            (local_min[0], local_max[1], local_max[2]),
            (local_max[0], local_min[1], local_min[2]),
            (local_max[0], local_min[1], local_max[2]),
            (local_max[0], local_max[1], local_min[2]),
            (local_max[0], local_max[1], local_max[2]),
        ):
            world = obj.matrix_world @ mathutils.Vector(corner)
            for axis in range(3):
                world_min[axis] = min(world_min[axis], world[axis])
                world_max[axis] = max(world_max[axis], world[axis])
        if math.isinf(local_min[0]):
            local_bounds = world_bounds = None
        else:
            local_bounds = {'min': local_min, 'max': local_max}
            world_bounds = {'min': world_min, 'max': world_max}
        mesh.calc_loop_triangles()
        triangle_count = len(mesh.loop_triangles)
        return {
            'vertex_count': len(mesh.vertices),
            'triangle_count': triangle_count,
            'local_bounds': local_bounds,
            'world_bounds': world_bounds,
            'material_slots': [slot.name for slot in obj.material_slots],
            'finite_vertices': finite,
        }
    finally:
        evaluated.to_mesh_clear()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', required=True)
    parser.add_argument('--config', required=True)
    raw_argv = sys.argv
    if '--' in raw_argv:
        raw_argv = raw_argv[raw_argv.index('--') + 1:]
    args = parser.parse_args(raw_argv)

    try:
        with open(args.config, encoding='utf-8') as stream:
            config = json.load(stream)
    except (ValueError, OSError) as exc:
        emit({'ok': False, 'class': 'precondition',
              'error': 'invalid probe-blend config: %s' % exc})
        return 2
    if not isinstance(config, dict):
        emit({'ok': False, 'class': 'precondition',
              'error': 'probe-blend config root must be an object'})
        return 2

    blend_rel = config.get('blend_file')
    if not isinstance(blend_rel, str) or not blend_rel.strip():
        emit({'ok': False, 'class': 'precondition',
              'error': 'config missing blend_file'})
        return 2
    blend_path = os.path.normpath(os.path.join(args.root, blend_rel))
    if not os.path.isfile(blend_path):
        emit({'ok': False, 'class': 'precondition',
              'error': 'blend file not found: %s' % blend_rel})
        return 2
    if not blend_path.lower().endswith('.blend'):
        emit({'ok': False, 'class': 'precondition',
              'error': 'unsupported extension: %s' % blend_rel})
        return 2

    try:
        import bpy  # noqa: PLC0415 - available only inside Blender

        bpy.ops.wm.open_mainfile(filepath=blend_path, load_ui=False)
    except Exception as exc:  # noqa: BLE001 - report, never traceback
        emit({'ok': False, 'class': 'corrupt',
              'error': 'unable to open blend file: %s' % exc})
        return 3

    requested = config.get('objects')
    if requested is None:
        requested = []
    if not isinstance(requested, list) or not all(
        isinstance(name, str) for name in requested
    ):
        emit({'ok': False, 'class': 'precondition',
              'error': 'config objects must be an array of names'})
        return 2

    depsgraph = bpy.context.evaluated_depsgraph_get()
    if requested:
        facts = []
        for name in sorted(set(requested)):
            obj = bpy.data.objects.get(name)
            if obj is None:
                facts.append({'name': name, 'exists': False})
            else:
                fact = object_facts(name, obj, depsgraph)
                if fact is not None:
                    facts.append(fact)
    else:
        facts = []
        for obj in sorted(bpy.data.objects, key=lambda o: o.name):
            if obj.type == 'MESH':
                facts.append(object_facts(obj.name, obj, depsgraph))

    emit({'ok': True, 'blend_file': blend_rel, 'objects': facts,
          'object_count': len(facts)})
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
