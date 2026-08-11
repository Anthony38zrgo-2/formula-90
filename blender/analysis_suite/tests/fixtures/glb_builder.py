"""Tiny in-memory GLB builder used exclusively by the test suite.

Tests generate every fixture in temporary directories and leave no files
behind. Mirrors the fixture style of blender/vehicle_pipeline/tests but
supports arbitrary node hierarchies, transforms and float payloads.
"""
import json
import struct


def _pad4(data, fill=b' '):
    return data + fill * ((4 - len(data) % 4) % 4)


def build_glb(nodes, meshes, asset_generator='analysis-suite test'):
    """Build a GLB byte string from simple JSON structures.

    nodes: list of dicts with optional keys 'name', 'mesh' (index into
        ``meshes``), 'translation', 'scale', 'rotation', 'children'.
    meshes: list of dicts {'name', 'primitives': [{'points': [[x, y, z], ...],
        'nan_point': [x, y, z] or None, 'indices': [i, j, k, ...] or None}]}.
    """
    binary = bytearray()
    buffer_views = []
    accessors = []

    def add_accessor(payload, component_type, accessor_type, count, **extra):
        while len(binary) % 4:
            binary.append(0)
        offset = len(binary)
        binary.extend(payload)
        buffer_views.append({
            'buffer': 0,
            'byteOffset': offset,
            'byteLength': len(payload),
        })
        accessor = {
            'bufferView': len(buffer_views) - 1,
            'componentType': component_type,
            'count': count,
            'type': accessor_type,
        }
        accessor.update(extra)
        accessors.append(accessor)
        return len(accessors) - 1

    built_meshes = []
    for mesh in meshes:
        primitives = []
        for primitive in mesh.get('primitives', []):
            points = list(primitive.get('points', []))
            nan_point = primitive.get('nan_point')
            if nan_point is not None:
                points.append(nan_point)
            payload = b''.join(
                struct.pack('<3f', *point) for point in points
            )
            minimum = [min(p[i] for p in points) for i in range(3)]
            maximum = [max(p[i] for p in points) for i in range(3)]
            position_accessor = add_accessor(
                payload, 5126, 'VEC3', len(points),
                min=minimum, max=maximum,
            )
            attributes = {'POSITION': position_accessor}
            extra = {'attributes': attributes}
            indices = primitive.get('indices')
            if indices:
                index_payload = struct.pack('<%dH' % len(indices), *indices)
                index_accessor = add_accessor(
                    index_payload, 5123, 'SCALAR', len(indices)
                )
                extra['indices'] = index_accessor
            primitives.append(extra)
        built_meshes.append({'name': mesh.get('name'), 'primitives': primitives})

    gltf = {
        'asset': {'version': '2.0', 'generator': asset_generator},
        'scene': 0,
        'scenes': [{'nodes': list(range(len(nodes)))}],
        'nodes': nodes,
        'meshes': built_meshes,
        'buffers': [{'byteLength': len(binary)}],
        'bufferViews': buffer_views,
        'accessors': accessors,
    }
    json_chunk = _pad4(
        json.dumps(gltf, separators=(',', ':')).encode('utf-8'), b' '
    )
    bin_chunk = _pad4(bytes(binary), b'\x00')
    total_length = 12 + 8 + len(json_chunk) + 8 + len(bin_chunk)
    return b''.join([
        struct.pack('<4sII', b'glTF', 2, total_length),
        struct.pack('<II', len(json_chunk), 0x4E4F534A),
        json_chunk,
        struct.pack('<II', len(bin_chunk), 0x004E4942),
        bin_chunk,
    ])
