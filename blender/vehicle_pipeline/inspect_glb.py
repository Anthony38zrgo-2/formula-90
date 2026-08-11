"""Deterministic, stdlib-only JSON audit of a glTF 2.0 binary (GLB).

Usage:
  python inspect_glb.py <file.glb> [--full] [--pretty]
      [--normal-tolerance FLOAT]

Exit code 0 means the input was inspected successfully. Exit code 2 means an
input or tool precondition failed; an error JSON object is still emitted.
"""
import array
import hashlib
import json
import math
import os
import struct
import sys

COMPONENT_TYPE = {
    5120: ('b', 1), 5121: ('B', 1), 5122: ('h', 2),
    5123: ('H', 2), 5125: ('I', 4), 5126: ('f', 4),
}
TYPE_COUNT = {
    'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16,
}
AUDITED_ATTRIBUTES = ('POSITION', 'NORMAL', 'TEXCOORD_0')
DEFAULT_NORMAL_TOLERANCE = 0.01
JSON_CHUNK = 0x4E4F534A
BIN_CHUNK = 0x004E4942


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest().upper()


def read_glb(path):
    with open(path, 'rb') as f:
        data = f.read()
    if len(data) < 12 or data[0:4] != b'glTF':
        raise ValueError('not a GLB (bad magic)')
    version, length = struct.unpack_from('<II', data, 4)
    if version != 2:
        raise ValueError('unsupported glTF version %d' % version)
    if length < 12:
        raise ValueError('GLB length field is smaller than the header')
    if length != len(data):
        raise ValueError('GLB length field mismatch')
    offset = 12
    gltf = None
    bin_chunk = b''
    bin_seen = False
    while offset < len(data):
        if len(data) - offset < 8:
            raise ValueError('truncated GLB chunk header')
        chunk_len, chunk_type = struct.unpack_from('<II', data, offset)
        offset += 8
        chunk_end = offset + chunk_len
        if chunk_end > len(data):
            raise ValueError('GLB chunk exceeds file length')
        chunk = data[offset:chunk_end]
        offset = chunk_end
        if chunk_type == JSON_CHUNK:
            if gltf is not None:
                raise ValueError('GLB contains multiple JSON chunks')
            try:
                json_text = chunk.rstrip(b' \t\r\n').decode('utf-8')
                gltf = json.loads(json_text, parse_constant=_reject_json_constant)
            except (UnicodeDecodeError, ValueError, json.JSONDecodeError) as exc:
                raise ValueError('invalid GLB JSON chunk: %s' % exc) from exc
            if not isinstance(gltf, dict):
                raise ValueError('GLB JSON root must be an object')
        elif chunk_type == BIN_CHUNK:  # BIN
            if bin_seen:
                raise ValueError('GLB contains multiple BIN chunks')
            bin_chunk = chunk
            bin_seen = True
    if gltf is None:
        raise ValueError('no JSON chunk')
    return gltf, bin_chunk


def _reject_json_constant(value):
    raise ValueError('non-finite JSON constant %s is not allowed' % value)


def _is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _finite_number(value):
    if not _is_number(value):
        return False
    try:
        return math.isfinite(value)
    except (OverflowError, TypeError):
        return False


def _vector(value, length, label):
    if not isinstance(value, list) or len(value) != length:
        raise ValueError('%s must contain %d values' % (label, length))
    if not all(_finite_number(item) for item in value):
        raise ValueError('%s contains a non-finite or non-numeric value' % label)
    return value


def _accessor_info(gltf, index):
    accessors = gltf.get('accessors')
    if not isinstance(accessors, list):
        raise ValueError('GLB JSON has no valid accessors array')
    if not isinstance(index, int) or isinstance(index, bool):
        raise ValueError('accessor index is not an integer')
    if index < 0 or index >= len(accessors):
        raise ValueError('accessor index %d is out of range' % index)
    acc = accessors[index]
    if not isinstance(acc, dict):
        raise ValueError('accessor %d is not an object' % index)
    try:
        fmt, csize = COMPONENT_TYPE[acc['componentType']]
        ncomp = TYPE_COUNT[acc['type']]
    except (KeyError, TypeError) as exc:
        raise ValueError('accessor %d has an unsupported format' % index) from exc
    count = acc.get('count')
    if not isinstance(count, int) or isinstance(count, bool) or count < 0:
        raise ValueError('accessor %d has an invalid count' % index)
    buffer_view_index = acc.get('bufferView')
    buffer_views = gltf.get('bufferViews')
    if not isinstance(buffer_views, list):
        raise ValueError('GLB JSON has no valid bufferViews array')
    if (not isinstance(buffer_view_index, int)
            or isinstance(buffer_view_index, bool)
            or buffer_view_index < 0
            or buffer_view_index >= len(buffer_views)):
        raise ValueError('accessor %d has an invalid bufferView' % index)
    buffer_view = buffer_views[buffer_view_index]
    if not isinstance(buffer_view, dict):
        raise ValueError('bufferView %d is not an object' % buffer_view_index)
    view_offset = buffer_view.get('byteOffset', 0)
    accessor_offset = acc.get('byteOffset', 0)
    stride = buffer_view.get('byteStride', csize * ncomp)
    if (not isinstance(view_offset, int) or isinstance(view_offset, bool)
            or view_offset < 0):
        raise ValueError('bufferView %d has an invalid byteOffset' % buffer_view_index)
    if (not isinstance(accessor_offset, int) or isinstance(accessor_offset, bool)
            or accessor_offset < 0):
        raise ValueError('accessor %d has an invalid byteOffset' % index)
    if (not isinstance(stride, int) or isinstance(stride, bool)
            or stride < csize * ncomp):
        raise ValueError('accessor %d has an invalid byteStride' % index)
    return acc, fmt, csize, ncomp, view_offset + accessor_offset, stride


def read_accessor(gltf, bin_chunk, index):
    """Return (array.array flat, component_count, gltf_accessor)."""
    acc, fmt, csize, ncomp, start, stride = _accessor_info(gltf, index)
    count = acc['count']
    element_size = csize * ncomp
    if count:
        end = start + (count - 1) * stride + element_size
    else:
        end = start
    if start > len(bin_chunk) or end > len(bin_chunk):
        raise ValueError('accessor %d exceeds the BIN chunk' % index)
    out = array.array(fmt)
    if stride == element_size:
        out.frombytes(bin_chunk[start:end])
    else:
        for i in range(count):
            s = start + i * stride
            out.frombytes(bin_chunk[s:s + element_size])
    if sys.byteorder != 'little' and csize > 1:
        out.byteswap()
    return out, ncomp, acc


def mat_identity():
    return [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]]


def mat_mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)]
            for i in range(4)]


def mat_from_trs(node):
    if 'matrix' in node:
        m = _vector(node['matrix'], 16, 'node matrix')  # column-major
        return [[m[0], m[4], m[8], m[12]],
                [m[1], m[5], m[9], m[13]],
                [m[2], m[6], m[10], m[14]],
                [m[3], m[7], m[11], m[15]]]
    t = _vector(node.get('translation', [0, 0, 0]), 3, 'node translation')
    r = _vector(node.get('rotation', [0, 0, 0, 1]), 4, 'node rotation')
    s = _vector(node.get('scale', [1, 1, 1]), 3, 'node scale')
    x, y, z, w = r
    rm = [
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ]
    m = mat_identity()
    for i in range(3):
        for j in range(3):
            m[i][j] = rm[i][j] * s[j]
        m[i][3] = t[i]
    return m


def transform_point(m, p):
    return [m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2] + m[i][3]
            for i in range(3)]


def determinant3(m):
    return (
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    )


def _finite_values(values):
    return all(_finite_number(value) for value in values)


def _position_bounds(accessor, pdata, ncomp):
    if ncomp != 3:
        raise ValueError('POSITION accessor must be VEC3')
    if accessor['count'] == 0:
        return None
    amin = accessor.get('min')
    amax = accessor.get('max')
    if (isinstance(amin, list) and isinstance(amax, list)
            and len(amin) == 3 and len(amax) == 3
            and _finite_values(amin) and _finite_values(amax)):
        return amin, amax
    finite_points = []
    for i in range(accessor['count']):
        point = pdata[i * 3:i * 3 + 3]
        if _finite_values(point):
            finite_points.append(point)
    if not finite_points:
        return None
    return (
        [min(point[i] for point in finite_points) for i in range(3)],
        [max(point[i] for point in finite_points) for i in range(3)],
    )


def _node_ref(index, node, scale=None):
    entry = {'node': index, 'name': node.get('name')}
    if scale is not None:
        entry['scale'] = list(scale)
    return entry


def inspect(path, full=False, normal_tolerance=DEFAULT_NORMAL_TOLERANCE):
    if not _finite_number(normal_tolerance) or normal_tolerance < 0:
        raise ValueError('normal tolerance must be a finite non-negative number')
    result = {'file': path, 'ok': True}
    result['sha256'] = sha256_file(path)
    result['size_bytes'] = os.path.getsize(path)
    gltf, bin_chunk = read_glb(path)
    result['generator'] = gltf.get('asset', {}).get('generator')
    result['gltf_version'] = gltf.get('asset', {}).get('version')

    nodes = gltf.get('nodes', [])
    meshes = gltf.get('meshes', [])
    materials = gltf.get('materials', [])
    images = gltf.get('images', [])
    textures = gltf.get('textures', [])

    result['counts'] = {
        'nodes': len(nodes),
        'meshes': len(meshes),
        'materials': len(materials),
        'images': len(images),
        'textures': len(textures),
        'accessors': len(gltf.get('accessors', [])),
    }

    # World matrices per node (default scene graph). A malformed cycle is a
    # precondition error rather than an opportunity for unbounded recursion.
    parents = {}
    for i, n in enumerate(nodes):
        for c in n.get('children', []):
            if (not isinstance(c, int) or isinstance(c, bool)
                    or c < 0 or c >= len(nodes)):
                raise ValueError('node %d has an invalid child index' % i)
            if c in parents and parents[c] != i:
                raise ValueError('node %d has multiple parents' % c)
            parents[c] = i

    world_cache = {}
    world_visiting = set()

    def world_matrix(i):
        if i in world_cache:
            return world_cache[i]
        if i in world_visiting:
            raise ValueError('node hierarchy contains a cycle at node %d' % i)
        world_visiting.add(i)
        m = mat_from_trs(nodes[i])
        p = parents.get(i)
        try:
            if p is not None:
                m = mat_mul(world_matrix(p), m)
        finally:
            world_visiting.remove(i)
        world_cache[i] = m
        return m

    negative_scale_nodes = []
    negative_world_nodes = []
    for ni, node in enumerate(nodes):
        scale = node.get('scale')
        if (isinstance(scale, list) and len(scale) == 3
                and any(_finite_number(value) and value < 0 for value in scale)):
            negative_scale_nodes.append(_node_ref(ni, node, scale))
        determinant = determinant3(world_matrix(ni))
        if determinant < 0:
            entry = _node_ref(ni, node)
            entry['world_determinant'] = determinant
            negative_world_nodes.append(entry)

    total_positions = 0
    total_indices = 0
    finite = True
    world_min = [math.inf] * 3
    world_max = [-math.inf] * 3
    attr_coverage = {}
    attribute_summary = {
        attribute: {
            'primitive_count': 0,
            'missing_primitive_count': 0,
            'total_count': 0,
        }
        for attribute in AUDITED_ATTRIBUTES
    }
    primitive_attribute_coverage = []
    node_details = []
    normal_stats = {
        'near_unit_tolerance': normal_tolerance,
        'normal_count': 0,
        'finite_normal_count': 0,
        'non_finite_value_count': 0,
        'non_finite_normal_count': 0,
        'finite_length_count': 0,
        'length_minimum': None,
        'length_maximum': None,
        'length_mean': None,
        'outside_near_unit_tolerance_count': 0,
    }
    normal_length_sum = 0.0
    normal_length_min = math.inf
    normal_length_max = -math.inf
    normal_seen = False

    for ni, node in enumerate(nodes):
        if 'mesh' not in node:
            if full:
                node_details.append({'node': ni, 'name': node.get('name'),
                                     'mesh': None})
            continue
        mesh_index = node['mesh']
        if (not isinstance(mesh_index, int) or isinstance(mesh_index, bool)
                or mesh_index < 0 or mesh_index >= len(meshes)):
            raise ValueError('node %d has an invalid mesh index' % ni)
        mesh = meshes[mesh_index]
        if not isinstance(mesh, dict):
            raise ValueError('mesh %d is not an object' % mesh_index)
        wm = world_matrix(ni)
        node_min = [math.inf] * 3
        node_max = [-math.inf] * 3
        prims_out = []
        primitives = mesh.get('primitives', [])
        if not isinstance(primitives, list):
            raise ValueError('mesh %d has an invalid primitives array' % mesh_index)
        for pi, prim in enumerate(primitives):
            if not isinstance(prim, dict):
                raise ValueError('mesh %d primitive %d is not an object' % (mesh_index, pi))
            attrs = prim.get('attributes', {})
            if not isinstance(attrs, dict):
                raise ValueError('mesh %d primitive %d has invalid attributes' % (mesh_index, pi))
            for a in attrs:
                attr_coverage[a] = attr_coverage.get(a, 0) + 1
            primitive_entry = {
                'node': ni,
                'node_name': node.get('name'),
                'mesh': mesh_index,
                'mesh_name': mesh.get('name'),
                'primitive': pi,
                'attributes': {},
            }
            for attribute in AUDITED_ATTRIBUTES:
                accessor_index = attrs.get(attribute)
                if accessor_index is None:
                    primitive_entry['attributes'][attribute] = {
                        'present': False,
                        'count': 0,
                    }
                    attribute_summary[attribute]['missing_primitive_count'] += 1
                    continue
                accessor, _, _, _, _, _ = _accessor_info(gltf, accessor_index)
                count = accessor['count']
                primitive_entry['attributes'][attribute] = {
                    'present': True,
                    'count': count,
                }
                attribute_summary[attribute]['primitive_count'] += 1
                attribute_summary[attribute]['total_count'] += count
            primitive_attribute_coverage.append(primitive_entry)
            p_acc = attrs.get('POSITION')
            entry = {
                'attributes': sorted(attrs.keys()),
                'mode': prim.get('mode', 4),
                'material': prim.get('material'),
            }
            if p_acc is not None:
                pdata, ncomp, acc = read_accessor(gltf, bin_chunk, p_acc)
                total_positions += acc['count']
                if not _finite_values(pdata):
                    finite = False
                entry['position_count'] = acc['count']
                position_bounds = _position_bounds(acc, pdata, ncomp)
                if position_bounds is not None:
                    amin, amax = position_bounds
                    entry['position_min'] = amin
                    entry['position_max'] = amax
                    # transform AABB corners to world
                    for cx in (amin[0], amax[0]):
                        for cy in (amin[1], amax[1]):
                            for cz in (amin[2], amax[2]):
                                wp = transform_point(wm, [cx, cy, cz])
                                for k in range(3):
                                    node_min[k] = min(node_min[k], wp[k])
                                    node_max[k] = max(node_max[k], wp[k])
            # Do not read a normal buffer when the primitive has no NORMAL.
            if 'NORMAL' in attrs:
                normal_seen = True
                ndata, ncomp, nacc = read_accessor(gltf, bin_chunk, attrs['NORMAL'])
                if ncomp != 3:
                    raise ValueError('NORMAL accessor must be VEC3')
                normal_stats['normal_count'] += nacc['count']
                for normal_index in range(nacc['count']):
                    values = ndata[normal_index * 3:normal_index * 3 + 3]
                    non_finite_values = sum(
                        1 for value in values if not _finite_number(value)
                    )
                    if non_finite_values:
                        finite = False
                        normal_stats['non_finite_value_count'] += non_finite_values
                        normal_stats['non_finite_normal_count'] += 1
                        continue
                    length = math.sqrt(sum(value * value for value in values))
                    normal_stats['finite_normal_count'] += 1
                    normal_stats['finite_length_count'] += 1
                    normal_length_sum += length
                    normal_length_min = min(normal_length_min, length)
                    normal_length_max = max(normal_length_max, length)
                    if abs(length - 1.0) > normal_tolerance:
                        normal_stats['outside_near_unit_tolerance_count'] += 1
            if 'indices' in prim:
                iacc, _, _, _, _, _ = _accessor_info(gltf, prim['indices'])
                total_indices += iacc['count']
                entry['index_count'] = iacc['count']
            prims_out.append(entry)
        if full:
            nd = {
                'node': ni,
                'name': node.get('name'),
                'mesh': mesh.get('name'),
                'translation': node.get('translation'),
                'rotation': node.get('rotation'),
                'scale': node.get('scale'),
                'primitives': prims_out,
                'world_determinant': determinant3(wm),
            }
            if not math.isinf(node_min[0]):
                nd['world_min'] = node_min
                nd['world_max'] = node_max
            node_details.append(nd)
        for k in range(3):
            world_min[k] = min(world_min[k], node_min[k])
            world_max[k] = max(world_max[k], node_max[k])

    result['total_positions'] = total_positions
    result['total_indices'] = total_indices
    result['total_triangles'] = total_indices // 3
    result['finite_positions_normals'] = finite
    result['attribute_coverage'] = attr_coverage
    result['attribute_counts'] = attribute_summary
    result['primitive_attribute_coverage'] = primitive_attribute_coverage
    result['negative_scale_nodes'] = negative_scale_nodes
    result['negative_world_determinant_nodes'] = negative_world_nodes
    if normal_seen:
        if normal_stats['finite_length_count']:
            normal_stats['length_minimum'] = normal_length_min
            normal_stats['length_maximum'] = normal_length_max
            normal_stats['length_mean'] = (
                normal_length_sum / normal_stats['finite_length_count']
            )
        result['normal_statistics'] = normal_stats
    if not math.isinf(world_min[0]):
        result['world_bounds'] = {'min': world_min, 'max': world_max}
        result['dimensions'] = [world_max[i] - world_min[i] for i in range(3)]

    result['materials_detail'] = [
        {
            'index': i,
            'name': m.get('name'),
            'baseColorFactor': m.get('pbrMetallicRoughness', {}).get('baseColorFactor'),
            'has_baseColorTexture': 'baseColorTexture' in m.get('pbrMetallicRoughness', {}),
            'doubleSided': m.get('doubleSided', False),
        }
        for i, m in enumerate(materials)
    ]
    result['images_detail'] = [
        {'index': i, 'name': im.get('name'), 'mimeType': im.get('mimeType'),
         'uri': im.get('uri')}
        for i, im in enumerate(images)
    ]
    if full:
        result['nodes_detail'] = node_details
    return result


def main(argv):
    if len(argv) < 2:
        result = {
            'file': None,
            'ok': False,
            'error': 'usage: inspect_glb.py <file.glb> [--full] [--pretty] '
                     '[--normal-tolerance FLOAT]',
        }
        print(json.dumps(result, sort_keys=True))
        return 2
    path = argv[1]
    full = False
    pretty = False
    normal_tolerance = DEFAULT_NORMAL_TOLERANCE
    parse_error = None
    i = 2
    while i < len(argv):
        arg = argv[i]
        if arg == '--full':
            full = True
        elif arg == '--pretty':
            pretty = True
        elif arg == '--normal-tolerance':
            if i + 1 >= len(argv):
                parse_error = '--normal-tolerance requires a value'
                break
            i += 1
            try:
                normal_tolerance = float(argv[i])
            except ValueError:
                parse_error = '--normal-tolerance must be a number'
                break
        elif arg.startswith('--normal-tolerance='):
            try:
                normal_tolerance = float(arg.split('=', 1)[1])
            except ValueError:
                parse_error = '--normal-tolerance must be a number'
                break
        else:
            parse_error = 'unknown option: %s' % arg
            break
        i += 1
    if parse_error is not None:
        result = {'file': path, 'ok': False, 'error': parse_error}
        print(json.dumps(result, sort_keys=True))
        return 2
    try:
        result = inspect(
            path,
            full=full,
            normal_tolerance=normal_tolerance,
        )
    except Exception as exc:  # report, do not traceback
        result = {'file': path, 'ok': False, 'error': str(exc)}
        exit_code = 2
    else:
        exit_code = 0
    indent = 2 if pretty else None
    print(json.dumps(result, indent=indent, sort_keys=True, allow_nan=False))
    return exit_code


if __name__ == '__main__':
    sys.exit(main(sys.argv))
