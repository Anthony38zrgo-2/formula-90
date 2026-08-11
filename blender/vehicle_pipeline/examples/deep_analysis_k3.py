"""Investigative seed from K3: deep per-vertex semantic analysis of Jordan GLBs.

This example is intentionally not production infrastructure. It has hard-coded
paths, datums and thresholds; ANL-001 must replace those with configuration and
reusable analyzer APIs before relying on it in a validator.
"""

import math
import sys

sys.path.insert(0, r'D:\Formula90s\blender\vehicle\_pipeline')

from inspect_glb import (
    read_glb,
    read_accessor,
    mat_from_trs,
    mat_mul,
    transform_point,
)

SRC = (
    r'D:\Formula90s\game\assets\models\vehicles'
    r'\f1_90s_canonical_1997\source\jordan_191_1995'
)

FRONT_AXLE_Z = -1.4394
REAR_AXLE_Z = 1.4906

# Candidate threshold used by the original K3 investigation:
# 950 mm with chassis root at wheel-centre height 0.3302 m.
HEIGHT_THRESHOLD_GLB_Y = 0.6198

# Half-widths, expressed in metres.
WIDE_FWD_LIMIT = 0.70
REAR_ZONE_WIDTH_LIMIT = 0.50


def iter_node_world_positions(gltf, bin_chunk):
    nodes = gltf.get('nodes', [])
    meshes = gltf.get('meshes', [])

    parents = {}
    for i, node in enumerate(nodes):
        for child in node.get('children', []):
            parents[child] = i

    def world(i):
        matrix = mat_from_trs(nodes[i])
        parent = parents.get(i)

        while parent is not None:
            matrix = mat_mul(mat_from_trs(nodes[parent]), matrix)
            parent = parents.get(parent)

        return matrix

    for node_index, node in enumerate(nodes):
        if 'mesh' not in node:
            continue

        world_matrix = world(node_index)
        mesh = meshes[node['mesh']]

        for primitive in mesh.get('primitives', []):
            attrs = primitive.get('attributes', {})

            if 'POSITION' not in attrs:
                continue

            pdata, ncomp, accessor = read_accessor(
                gltf,
                bin_chunk,
                attrs['POSITION'],
            )

            points = []

            for vertex_index in range(accessor['count']):
                p = [
                    pdata[vertex_index * 3],
                    pdata[vertex_index * 3 + 1],
                    pdata[vertex_index * 3 + 2],
                ]
                points.append(transform_point(world_matrix, p))

            yield node.get('name'), primitive.get('material'), points


def analyze_chassis():
    gltf, bin_chunk = read_glb(
        SRC + r'\jordan_191_1995_chassis_source.glb'
    )

    print('=== CHASSIS per-node evidence ===')

    for name, material, points in iter_node_world_positions(gltf, bin_chunk):
        if not points:
            continue

        ymax = max(p[1] for p in points)
        xmax = max(abs(p[0]) for p in points)
        zmin = min(p[2] for p in points)
        zmax = max(p[2] for p in points)

        tall = [
            p for p in points
            if p[1] > HEIGHT_THRESHOLD_GLB_Y
        ]

        wide_fwd = [
            p for p in points
            if abs(p[0]) > WIDE_FWD_LIMIT
            and p[2] <= REAR_AXLE_Z
        ]

        behind = [
            p for p in points
            if p[2] > REAR_AXLE_Z
        ]

        front_far = [
            p for p in points
            if p[2] < FRONT_AXLE_Z - 0.90
        ]

        line = (
            f"{name:<14} "
            f"ymax={ymax:+.4f} "
            f"xmax={xmax:.4f} "
            f"z[{zmin:+.3f}..{zmax:+.3f}]"
        )

        if tall:
            tx = (
                min(p[0] for p in tall),
                max(p[0] for p in tall),
            )
            tz = (
                min(p[2] for p in tall),
                max(p[2] for p in tall),
            )
            line += (
                f" | TALL>{HEIGHT_THRESHOLD_GLB_Y}: "
                f"n={len(tall)} "
                f"x[{tx[0]:+.3f}..{tx[1]:+.3f}] "
                f"z[{tz[0]:+.3f}..{tz[1]:+.3f}]"
            )

        if wide_fwd:
            wz = (
                min(p[2] for p in wide_fwd),
                max(p[2] for p in wide_fwd),
            )
            wx = max(abs(p[0]) for p in wide_fwd)
            line += (
                f" | WIDE-FWD: "
                f"n={len(wide_fwd)} "
                f"xmax={wx:.4f} "
                f"z[{wz[0]:+.3f}..{wz[1]:+.3f}]"
            )

        if behind:
            bx = max(abs(p[0]) for p in behind)
            bz = max(p[2] for p in behind)
            by = max(p[1] for p in behind)

            wide_behind = [
                p for p in behind
                if abs(p[0]) > REAR_ZONE_WIDTH_LIMIT
            ]

            line += (
                f" | BEHIND: "
                f"xmax={bx:.4f} "
                f"zmax={bz:.4f} "
                f"ymax={by:.4f} "
                f"n_wide={len(wide_behind)}"
            )

        if front_far:
            fx = max(abs(p[0]) for p in front_far)
            fz = min(p[2] for p in front_far)
            line += (
                f" | FRONT>900: "
                f"n={len(front_far)} "
                f"xmax={fx:.4f} "
                f"zmin={fz:.4f}"
            )

        print(line)


def analyze_wheel(tag, filename):
    gltf, bin_chunk = read_glb(SRC + '\\' + filename)

    print(f'=== WHEEL {tag} per-node radial evidence ===')

    for name, material, points in iter_node_world_positions(gltf, bin_chunk):
        if not points:
            continue

        radii = [
            math.sqrt(p[1] * p[1] + p[2] * p[2])
            for p in points
        ]
        xs = [p[0] for p in points]

        print(
            f"{name:<10} "
            f"mat={material} "
            f"r[{min(radii):.4f}..{max(radii):.4f}] "
            f"x[{min(xs):+.4f}..{max(xs):+.4f}] "
            f"n={len(points)}"
        )


if __name__ == '__main__':
    analyze_chassis()
    analyze_wheel(
        'FL',
        'jordan_191_1995_wheel_fl_source.glb',
    )
    analyze_wheel(
        'RL',
        'jordan_191_1995_wheel_rl_source.glb',
    )
