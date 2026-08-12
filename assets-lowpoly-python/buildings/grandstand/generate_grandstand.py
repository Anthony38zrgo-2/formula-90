"""Procedural low-poly F1 spectator grandstand module with low-poly spectators.

Classify: GENERATE + EXPORT.

Builds a modular stepped grandstand (repeatable tile) out of stacked box steps
plus seated low-poly humanoids (body + head boxes) on every tread. All colors
are flat per-vertex colors (uint8 RGBA) — no UV textures needed. Geometry and
colors are produced with NumPy + Trimesh, then exported to a single GLB.

Outputs next to this script:
    grandstand_lowpoly.glb
    build_report.json
"""

import json
import os
import struct

import numpy as np
import trimesh

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

WIDTH = 20.0          # x: module width across the track
TREAD_DEPTH = 1.2     # z: depth of one step tread
RISER_HEIGHT = 1.0    # y: rise of one step
ROWS = 10             # number of seating rows
PLINTH_H = 0.3        # base slab height under the steps

# Spectator layout (seated, facing the track = -z)
SPACING_X = 0.7       # meters between spectators along a row
BODY_W, BODY_H, BODY_D = 0.40, 0.60, 0.40
HEAD_W, HEAD_H, HEAD_D = 0.26, 0.28, 0.26
SEAT_OFFSET_Z = 0.35  # distance of the body front-edge from the riser face

RNG = np.random.default_rng(1997)  # deterministic palette spread

# ---------------------------------------------------------------------------
# Palette (uint8 RGB)
# ---------------------------------------------------------------------------

GRAY_TREAD = np.array([205, 205, 210])
GRAY_RISER = np.array([150, 150, 158])
GRAY_BASE = np.array([130, 130, 138])

SHIRTS = [
    np.array([200, 30, 30]),   # red
    np.array([30, 80, 200]),   # blue
    np.array([40, 160, 60]),   # green
    np.array([230, 200, 20]),  # yellow
    np.array([230, 130, 20]),  # orange
    np.array([230, 230, 230]), # white
    np.array([30, 180, 200]),  # cyan
    np.array([150, 60, 180]),  # purple
]
SKIN = [
    np.array([224, 172, 142]),
    np.array([186, 122, 92]),
    np.array([246, 208, 178]),
    np.array([122, 82, 62]),
]
HAIR = [
    np.array([40, 32, 26]),
    np.array([24, 20, 18]),
    np.array([70, 52, 34]),
    np.array([190, 150, 90]),
]


# ---------------------------------------------------------------------------
# Geometry helpers
# ---------------------------------------------------------------------------

def box_at(min_corner, extents, color):
    """Box as a Trimesh with a flat per-vertex RGBA color, positioned by min."""
    center = np.array(min_corner, dtype=float) + np.array(extents, dtype=float) / 2
    tf = trimesh.transformations.translation_matrix(center)
    m = trimesh.creation.box(extents=[float(e) for e in extents], transform=tf)
    rgba = np.concatenate([np.asarray(color, dtype=np.uint8), np.array([255], np.uint8)])
    m.visual = trimesh.visual.ColorVisuals(
        vertex_colors=np.tile(rgba, (len(m.vertices), 1))
    )
    return m


def build():
    parts = []

    # Base plinth slab (full footprint)
    parts.append(box_at(
        (-WIDTH / 2, 0.0, 0.0), (WIDTH, PLINTH_H, ROWS * TREAD_DEPTH), GRAY_BASE))

    # Stepped seating rows (front riser gray, top tread light)
    for i in range(ROWS):
        z0 = i * TREAD_DEPTH
        y0 = PLINTH_H + i * RISER_HEIGHT
        parts.append(box_at(  # riser vertical front band
            (-WIDTH / 2, y0, z0 - 0.02), (WIDTH, RISER_HEIGHT, 0.04), GRAY_RISER))
        parts.append(box_at(  # tread horizontal slab (seat surface)
            (-WIDTH / 2, y0 + RISER_HEIGHT - 0.02, z0),
            (WIDTH, 0.04, TREAD_DEPTH), GRAY_TREAD))

    # Seated spectators on each tread
    xs = np.arange(-WIDTH / 2 + 0.35, WIDTH / 2 - 0.35 + 1e-6, SPACING_X)
    spectator_count = 0
    for i in range(ROWS):
        z0 = i * TREAD_DEPTH
        seat_y = PLINTH_H + (i + 1) * RISER_HEIGHT
        for x in xs:
            body_color = SHIRTS[int(RNG.integers(len(SHIRTS)))]
            skin = SKIN[int(RNG.integers(len(SKIN)))]
            hair = HAIR[int(RNG.integers(len(HAIR)))]

            # seated torso (bottom sits on the tread)
            parts.append(box_at(
                (x - BODY_W / 2, seat_y, z0 + SEAT_OFFSET_Z),
                (BODY_W, BODY_H, BODY_D), body_color))
            # head
            head_y0 = seat_y + BODY_H
            parts.append(box_at(
                (x - HEAD_W / 2, head_y0, z0 + SEAT_OFFSET_Z + (BODY_D - HEAD_D) / 2),
                (HEAD_W, HEAD_H, HEAD_D), skin))
            # hair cap (thin slab on top of the head)
            parts.append(box_at(
                (x - HEAD_W / 2, head_y0 + HEAD_H - 0.02,
                 z0 + SEAT_OFFSET_Z + (BODY_D - HEAD_D) / 2),
                (HEAD_W, 0.05, HEAD_D), hair))
            spectator_count += 1

    mesh = trimesh.util.concatenate(parts)
    mesh.merge_vertices()
    mesh.fix_normals()
    mesh.remove_unreferenced_vertices()
    return mesh, spectator_count


def inject_vertex_color_material(glb_path):
    """Embed a pbrMetallicRoughness material so Godot enables vertex colors.

    The exported GLB has COLOR_0 but no material; Godot then creates a default
    material with `vertex_color_use_as_albedo=false`, so the per-vertex palette
    is hidden. Injecting a white baseColor PBR material and assigning it to the
    primitive makes Godot apply vertex colors as the albedo.
    """
    with open(glb_path, "rb") as f:
        data = f.read()
    assert data[:4] == b"glTF"
    jl, jt = struct.unpack_from("<II", data, 12)
    json_bytes = data[20:20 + jl]
    bin_start = 20 + jl + 8
    gltf = json.loads(json_bytes.decode("utf-8"))

    gltf.setdefault("materials", [])
    if not gltf["materials"]:
        gltf["materials"].append({
            "name": "GrandstandVertexColor",
            "pbrMetallicRoughness": {
                "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 1.0,
            },
            "doubleSided": True,
        })
    for mesh in gltf.get("meshes", []):
        for prim in mesh.get("primitives", []):
            if prim.get("material") is None:
                prim["material"] = 0

    new_json = json.dumps(gltf, separators=(",", ":")).encode("utf-8")
    # pad JSON chunk with spaces to keep 4-byte alignment
    while (len(new_json) % 4) != 0:
        new_json += b" "

    total = 12 + 8 + len(new_json) + 8 + (len(data) - bin_start)
    out = bytearray()
    out += struct.pack("<I", 0x46546C67)  # glTF
    out += struct.pack("<I", 2)
    out += struct.pack("<I", total)
    out += struct.pack("<I", len(new_json))
    out += struct.pack("<I", 0x4E4F534A)  # JSON
    out += new_json
    out += struct.pack("<I", len(data) - bin_start)  # BIN chunk length
    out += struct.pack("<I", 0x4E4942)               # BIN
    out += data[bin_start:]  # BIN payload unchanged
    with open(glb_path, "wb") as f:
        f.write(bytes(out))


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    mesh, spectator_count = build()

    glb_path = os.path.join(here, "grandstand_lowpoly.glb")
    mesh.export(glb_path)
    inject_vertex_color_material(glb_path)

    bounds = mesh.bounds
    dims = bounds[1] - bounds[0]
    report = {
        "asset": "grandstand_lowpoly",
        "format": "GLB",
        "module": "repeatable grandstand tile with seated spectators",
        "dimensions_m": {
            "width_x": float(WIDTH),
            "depth_z": float(ROWS * TREAD_DEPTH),
            "height_y": float(dims[1]),
        },
        "rows": ROWS,
        "spectators": spectator_count,
        "vertices": int(mesh.vertices.shape[0]),
        "faces": int(mesh.faces.shape[0]),
        "bbox": {
            "min": bounds[0].tolist(),
            "max": bounds[1].tolist(),
            "dims": dims.tolist(),
        },
        "center": mesh.centroid.tolist(),
        "vertex_colors": True,
        "uv_textures": False,
        "material": "pbrMetallicRoughness (white baseColor, vertex colors as albedo)",
        "seed": 1997,
        "output": os.path.basename(glb_path),
    }

    report_path = os.path.join(here, "build_report.json")
    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)

    print(f"spectators={spectator_count}")
    print(f"verts={report['vertices']}  faces={report['faces']}")
    print(f"bbox dims={np.round(dims, 3).tolist()}")
    print(f"wrote {glb_path}")
    print(f"wrote {report_path}")


if __name__ == "__main__":
    main()
