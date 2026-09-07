"""Shared GLB buffer helpers (no Blender dependency)."""
import json
import struct
from pathlib import Path


def read_glb(path):
    raw = Path(path).read_bytes()
    assert raw[:4] == b'glTF'
    (length, _kind) = struct.unpack_from('<II', raw, 12)
    doc = json.loads(raw[20:20 + length])
    off = 20 + length
    (size, _kind) = struct.unpack_from('<II', raw, off)
    return doc, bytearray(raw[off + 8:off + 8 + size])


def write_glb(path, doc, blob):
    js = json.dumps(doc, separators=(',', ':')).encode()
    js += b' ' * ((-len(js)) % 4)
    blob += b'\0' * ((-len(blob)) % 4)
    Path(path).write_bytes(
        struct.pack('<4sII', b'glTF', 2, 28 + len(js) + len(blob))
        + struct.pack('<II', len(js), 0x4E4F534A) + js
        + struct.pack('<II', len(blob), 0x004E4942) + blob
    )


def _vec3_accessors(doc):
    for mesh in doc.get('meshes', []):
        for prim in mesh.get('primitives', []):
            yield prim


def transform_positions_normals(doc, blob, func):
    """Apply func(x, y, z) -> (x2, y2, z2) to POSITION and the linear part to NORMAL.

    func must be an axis-aligned affine map; normals use the inverse-transpose
    (exact for diagonal scales) and are renormalized.
    """
    import math
    # Recover scale factors by probing unit axes through func.
    ox, oy, oz = func(0.0, 0.0, 0.0)
    sx = func(1.0, 0.0, 0.0)[0] - ox
    sy = func(0.0, 1.0, 0.0)[1] - oy
    sz = func(0.0, 0.0, 1.0)[2] - oz
    for mesh in doc.get('meshes', []):
        for prim in mesh.get('primitives', []):
            attrs = prim['attributes']
            for key in ('POSITION', 'NORMAL'):
                if key not in attrs:
                    continue
                acc = doc['accessors'][attrs[key]]
                view = doc['bufferViews'][acc['bufferView']]
                assert acc['componentType'] == 5126 and acc['type'] == 'VEC3'
                base = view.get('byteOffset', 0) + acc.get('byteOffset', 0)
                is_normal = key == 'NORMAL'
                nxs, nys, nzs = [], [], []
                for i in range(acc['count']):
                    x, y, z = struct.unpack_from('<3f', blob, base + i * 12)
                    if is_normal:
                        nx, ny, nz = x / sx, y / sy, z / sz
                        inv = 1.0 / math.sqrt(nx * nx + ny * ny + nz * nz)
                        struct.pack_into('<3f', blob, base + i * 12, nx * inv, ny * inv, nz * inv)
                    else:
                        struct.pack_into('<3f', blob, base + i * 12, *func(x, y, z))
                if not is_normal and 'min' in acc and 'max' in acc:
                    corners = [func(x, y, z)
                               for x in (acc['min'][0], acc['max'][0])
                               for y in (acc['min'][1], acc['max'][1])
                               for z in (acc['min'][2], acc['max'][2])]
                    acc['min'] = [min(c[k] for c in corners) for k in range(3)]
                    acc['max'] = [max(c[k] for c in corners) for k in range(3)]


def flip_triangle_winding(doc, blob):
    for mesh in doc.get('meshes', []):
        for prim in mesh.get('primitives', []):
            idx_acc = doc['accessors'][prim['indices']]
            idx_view = doc['bufferViews'][idx_acc['bufferView']]
            assert idx_acc['componentType'] == 5123 and idx_acc['type'] == 'SCALAR'
            base = idx_view.get('byteOffset', 0) + idx_acc.get('byteOffset', 0)
            for t in range(idx_acc['count'] // 3):
                a, b, c = struct.unpack_from('<3H', blob, base + t * 6)
                struct.pack_into('<3H', blob, base + t * 6, a, c, b)


def mirror_duct_r_to_l(src_path, dst_path):
    """Exact X-mirror of a duct-R GLB: intake stays forward, inboard flips side."""
    doc, blob = read_glb(src_path)
    transform_positions_normals(doc, blob, lambda x, y, z: (-x, y, z))
    flip_triangle_winding(doc, blob)
    # Datum names stay attached to their mirrored positions, which preserves
    # inboard/outboard semantics (mirrored inboard geometry sits at +X).
    for node in doc.get('nodes', []):
        if 'translation' in node:
            node['translation'][0] *= -1
    write_glb(dst_path, doc, blob)
