"""Transform-aware per-vertex predicate evidence for GLB geometry.

Pure Python, no bpy. Predicate names, axes, centres, thresholds, sections,
coordinate convention and tolerances live only in configuration; nothing
vehicle-specific (Jordan, FIA, Z-forward, millimetres, metres) is hard-coded.
"""
import math

from . import contracts
from .glb_adapter import load_inspector

PREDICATE_TYPES = (
    'above_axis',
    'below_axis',
    'ahead_of_axis',
    'behind_axis',
    'inside_band',
    'outside_band',
    'radial_distance',
)

FILTER_TYPES = PREDICATE_TYPES[:-1]


def validate_probe_config(config):
    """Validate the probe-geometry config schema; raise ConfigError."""
    sections = config.get('sections', {})
    if sections is None:
        sections = {}
    sections = contracts.require_dict(sections, 'sections')
    for name, section in sections.items():
        section = contracts.require_dict(section, 'sections.%s' % name)
        contracts.parse_axis(section.get('axis'), 'sections.%s.axis' % name)
        for bound in ('min', 'max'):
            if bound in section:
                contracts.require_finite_number(
                    section[bound], 'sections.%s.%s' % (name, bound)
                )
    if 'sample_size' in config:
        contracts.require_non_negative_number(config['sample_size'], 'sample_size')
    predicates = contracts.require_list(config.get('predicates', []), 'predicates')
    for index, predicate in enumerate(predicates):
        _validate_predicate(predicate, index, sections)
    return sections


def _validate_predicate(predicate, index, sections):
    label = 'predicates[%d]' % index
    predicate = contracts.require_dict(predicate, label)
    predicate_id = contracts.require_str(predicate.get('id'), label + '.id')
    ptype = contracts.require_str(predicate.get('type'), label + '.type')
    if ptype not in PREDICATE_TYPES:
        raise contracts.ConfigError(
            '%s.type must be one of: %s' % (label, ', '.join(PREDICATE_TYPES))
        )
    if ptype in FILTER_TYPES:
        contracts.parse_axis(predicate.get('axis'), label + '.axis')
        if ptype in ('above_axis', 'below_axis', 'ahead_of_axis', 'behind_axis'):
            contracts.require_finite_number(
                predicate.get('threshold'), label + '.threshold'
            )
        else:
            contracts.require_finite_number(predicate.get('centre'), label + '.centre')
            contracts.require_non_negative_number(
                predicate.get('width'), label + '.width'
            )
    else:  # radial_distance
        contracts.parse_axis(predicate.get('plane_axis'), label + '.plane_axis')
        centre = contracts.require_list(predicate.get('centre'), label + '.centre')
        if len(centre) != 3:
            raise contracts.ConfigError('%s.centre must have 3 values' % label)
        for value in centre:
            contracts.require_finite_number(value, label + '.centre')
        for key in ('min_radius', 'max_radius'):
            if key in predicate:
                contracts.require_non_negative_number(
                    predicate[key], label + '.' + key
                )
    section = predicate.get('section')
    if section is not None:
        if not isinstance(section, str) or section not in sections:
            raise contracts.ConfigError(
                '%s.section references unknown section: %r' % (label, section)
            )
    if 'sample_size' in predicate:
        contracts.require_non_negative_number(
            predicate['sample_size'], label + '.sample_size'
        )
    return predicate_id


def load_world_vertices(path):
    """Return (vertices, non_finite_count) in world space.

    Each vertex is a dict with provenance (node/mesh/primitive indices and
    names), its world-space point and a finiteness flag.
    """
    inspector = load_inspector()
    gltf, bin_chunk = inspector.read_glb(str(path))
    nodes = gltf.get('nodes', [])
    meshes = gltf.get('meshes', [])

    parents = {}
    for node_index, node in enumerate(nodes):
        for child in node.get('children', []):
            if child in parents:
                raise ValueError('node %d has multiple parents' % child)
            parents[child] = node_index

    cache = {}
    visiting = set()

    def world(node_index):
        if node_index in cache:
            return cache[node_index]
        if node_index in visiting:
            raise ValueError('node hierarchy contains a cycle at node %d' % node_index)
        visiting.add(node_index)
        try:
            matrix = inspector.mat_from_trs(nodes[node_index])
            parent = parents.get(node_index)
            if parent is not None:
                matrix = inspector.mat_mul(world(parent), matrix)
        finally:
            visiting.remove(node_index)
        cache[node_index] = matrix
        return matrix

    vertices = []
    global_index = 0
    non_finite = 0
    for node_index, node in enumerate(nodes):
        mesh_index = node.get('mesh')
        if mesh_index is None:
            continue
        mesh = meshes[mesh_index]
        world_matrix = world(node_index)
        primitives = mesh.get('primitives', [])
        for primitive_index, primitive in enumerate(primitives):
            position_accessor = primitive.get('attributes', {}).get('POSITION')
            if position_accessor is None:
                continue
            pdata, ncomp, accessor = inspector.read_accessor(
                gltf, bin_chunk, position_accessor
            )
            count = accessor['count']
            for vertex_index in range(count):
                base = vertex_index * 3
                point = inspector.transform_point(
                    world_matrix,
                    [pdata[base], pdata[base + 1], pdata[base + 2]],
                )
                finite = all(math.isfinite(value) for value in point)
                vertices.append({
                    'vertex_index': global_index,
                    'primitive_vertex_index': vertex_index,
                    'node': node_index,
                    'node_name': node.get('name'),
                    'mesh': mesh_index,
                    'mesh_name': mesh.get('name'),
                    'primitive': primitive_index,
                    'point': point,
                    'finite': finite,
                })
                if not finite:
                    non_finite += 1
                global_index += 1
    return vertices, non_finite


def world_bounds(vertices):
    finite_points = [v['point'] for v in vertices if v['finite']]
    if not finite_points:
        return None
    minimum = [min(p[i] for p in finite_points) for i in range(3)]
    maximum = [max(p[i] for p in finite_points) for i in range(3)]
    return {
        'min': minimum,
        'max': maximum,
        'dimensions': [maximum[i] - minimum[i] for i in range(3)],
    }


def counts(vertices):
    nodes = {}
    meshes = {}
    primitives = {}
    for vertex in vertices:
        node_key = (vertex['node'], vertex['node_name'])
        mesh_key = (vertex['mesh'], vertex['mesh_name'])
        primitive_key = (vertex['node'], vertex['primitive'])
        nodes[node_key] = nodes.get(node_key, 0) + 1
        meshes[mesh_key] = meshes.get(mesh_key, 0) + 1
        primitives[primitive_key] = primitives.get(primitive_key, 0) + 1
    return {
        'node_counts': [
            {'node': key[0], 'name': key[1], 'vertices': count}
            for key, count in sorted(nodes.items())
        ],
        'mesh_counts': [
            {'mesh': key[0], 'name': key[1], 'vertices': count}
            for key, count in sorted(meshes.items())
        ],
        'primitive_counts': [
            {'node': key[0], 'primitive': key[1], 'vertices': count}
            for key, count in sorted(primitives.items())
        ],
    }


def section_filter(vertices, section):
    """Filter vertices by a named section's axis bounds (signed axis)."""
    axis_index, sign = contracts.parse_axis(section['axis'], 'section.axis')
    minimum = section.get('min')
    maximum = section.get('max')
    result = []
    for vertex in vertices:
        if not vertex['finite']:
            continue
        signed = vertex['point'][axis_index] * sign
        if minimum is not None and signed < minimum:
            continue
        if maximum is not None and signed > maximum:
            continue
        result.append(vertex)
    return result


def bounded_sample(vertices, size):
    """Deterministic bounded sample: lexicographically ordered (index, values)."""
    ordered = sorted(
        vertices, key=lambda v: (v['vertex_index'], tuple(v['point']))
    )
    entries = []
    for vertex in ordered[:size]:
        entries.append({
            'vertex_index': vertex['vertex_index'],
            'node': vertex['node'],
            'node_name': vertex['node_name'],
            'primitive': vertex['primitive'],
            'position': vertex['point'],
        })
    return entries


def _apply_filter(vertices, predicate):
    axis_index, sign = contracts.parse_axis(predicate['axis'], 'predicate.axis')
    ptype = predicate['type']
    if ptype in ('above_axis', 'ahead_of_axis'):
        def matches(value):
            return value * sign > predicate['threshold']
    elif ptype in ('below_axis', 'behind_axis'):
        def matches(value):
            return value * sign < predicate['threshold']
    else:
        centre = predicate['centre']
        width = predicate['width']

        def matches(value):
            if ptype == 'inside_band':
                return abs(value * sign - centre) <= width
            return abs(value * sign - centre) > width

    return [v for v in vertices if v['finite'] and matches(v['point'][axis_index])]


def _apply_radial(vertices, predicate):
    plane_index, _ = contracts.parse_axis(
        predicate['plane_axis'], 'predicate.plane_axis'
    )
    centre = predicate['centre']
    other = [i for i in range(3) if i != plane_index]
    distances = []
    for vertex in vertices:
        if not vertex['finite']:
            continue
        radius = math.sqrt(
            sum((vertex['point'][i] - centre[i]) ** 2 for i in other)
        )
        distances.append((vertex, radius))
    if distances:
        radius_min = min(distance for _, distance in distances)
        radius_max = max(distance for _, distance in distances)
        radius_mean = sum(distance for _, distance in distances) / len(distances)
    else:
        radius_min = radius_max = radius_mean = None
    entry = {
        'id': predicate['id'],
        'type': 'radial_distance',
        'plane_axis': predicate['plane_axis'],
        'centre': centre,
        'section': predicate.get('section'),
        'vertex_count': len(distances),
        'radius_min': radius_min,
        'radius_max': radius_max,
        'radius_mean': radius_mean,
    }
    for key in ('min_radius', 'max_radius'):
        if key in predicate:
            limit = predicate[key]
            within = sum(1 for _, radius in distances if radius <= limit)
            entry['within_%s_count' % key] = within
    return entry


def probe(path, config, sections):
    """Compute predicate evidence over a GLB's world-space vertices."""
    vertices, non_finite = load_world_vertices(path)
    default_sample_size = config.get('sample_size', 10)
    result = {
        'vertex_count': len(vertices),
        'finite_vertex_count': len(vertices) - non_finite,
        'non_finite_vertex_count': non_finite,
        'bounds': world_bounds(vertices),
        'counts': counts(vertices),
        'predicates': [],
    }
    for predicate in config.get('predicates', []):
        scope = vertices
        section_name = predicate.get('section')
        if section_name is not None:
            scope = section_filter(vertices, sections[section_name])
        if predicate['type'] == 'radial_distance':
            result['predicates'].append(_apply_radial(scope, predicate))
            continue
        matches = _apply_filter(scope, predicate)
        sample_size = int(predicate.get('sample_size', default_sample_size))
        entry = {
            'id': predicate['id'],
            'type': predicate['type'],
            'axis': predicate['axis'],
            'section': section_name,
            'total_count': len(matches),
            'sample_size': sample_size,
            'sample_count': min(sample_size, len(matches)),
            'sample': bounded_sample(matches, sample_size),
        }
        if predicate['type'] in ('above_axis', 'below_axis', 'ahead_of_axis', 'behind_axis'):
            entry['threshold'] = predicate['threshold']
        else:
            entry['centre'] = predicate['centre']
            entry['width'] = predicate['width']
        result['predicates'].append(entry)
    return result
