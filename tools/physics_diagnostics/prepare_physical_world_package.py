from __future__ import annotations

import argparse
import hashlib
import json as structured_serialization
import math
import struct
import sys as system_runtime
from pathlib import Path

REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
system_runtime.path.insert(0, str(REPOSITORY_ROOT))
from tools.common.output_policy import validate_output_path


def matrix_product(first, second):
    return [[sum(first[row][inner] * second[inner][column] for inner in range(4))
             for column in range(4)] for row in range(4)]


def node_transform(node):
    if "matrix" in node:
        return [[node["matrix"][column * 4 + row] for column in range(4)] for row in range(4)]
    rotation = node.get("rotation", [0, 0, 0, 1])
    first, second, third, scalar = rotation
    if abs(sum(value * value for value in rotation) - 1) > 1e-6:
        raise ValueError("Invalid node rotation")
    scale = node.get("scale", [1, 1, 1])
    translation = node.get("translation", [0, 0, 0])
    matrix = [[1 - 2 * (second * second + third * third), 2 * (first * second - third * scalar), 2 * (first * third + second * scalar), translation[0]],
              [2 * (first * second + third * scalar), 1 - 2 * (first * first + third * third), 2 * (second * third - first * scalar), translation[1]],
              [2 * (first * third - second * scalar), 2 * (second * third + first * scalar), 1 - 2 * (first * first + second * second), translation[2]],
              [0, 0, 0, 1]]
    for row in range(3):
        for column in range(3):
            matrix[row][column] *= scale[column]
    return matrix


def prepare_package(source_path: Path):
    source = source_path.read_bytes()
    magic, version, length = struct.unpack_from("<III", source)
    if magic != 0x46546C67 or version != 2 or length != len(source):
        raise ValueError("Physical source must be a valid version 2 GLB")
    cursor = 12
    document = None
    binary = None
    while cursor < length:
        chunk_length, chunk_kind = struct.unpack_from("<II", source, cursor)
        cursor += 8
        chunk = source[cursor:cursor + chunk_length]
        if len(chunk) != chunk_length:
            raise ValueError("Truncated GLB chunk")
        if chunk_kind == 0x4E4F534A:
            document = structured_serialization.loads(chunk)
        elif chunk_kind == 0x004E4942:
            binary = chunk
        cursor += chunk_length
    if document is None or binary is None or document.get("extensionsRequired"):
        raise ValueError("Physical source requires plain embedded triangle geometry")

    def accessor_values(identifier):
        accessor = document["accessors"][identifier]
        if "sparse" in accessor or accessor.get("normalized"):
            raise ValueError("Sparse and normalized physical accessors are unsupported")
        view = document["bufferViews"][accessor["bufferView"]]
        if view["buffer"] != 0:
            raise ValueError("External physical geometry buffers are unsupported")
        formats = {5126: "f", 5125: "I", 5123: "H", 5121: "B"}
        components = {"SCALAR": 1, "VEC3": 3}[accessor["type"]]
        unpacker = struct.Struct("<" + formats[accessor["componentType"]] * components)
        stride = view.get("byteStride", unpacker.size)
        start = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        end = view.get("byteOffset", 0) + view["byteLength"]
        if stride < unpacker.size or start + (accessor["count"] - 1) * stride + unpacker.size > end:
            raise ValueError("Physical accessor exceeds its buffer view")
        return [unpacker.unpack_from(binary, start + index * stride) for index in range(accessor["count"])]

    shapes = []
    visited = set()
    active = set()
    identity = [[float(row == column) for column in range(4)] for row in range(4)]

    def visit(identifier, parent_transform):
        if identifier in active or identifier in visited:
            raise ValueError("Physical node hierarchy contains a cycle or shared instance")
        active.add(identifier)
        visited.add(identifier)
        node = document["nodes"][identifier]
        transform = matrix_product(parent_transform, node_transform(node))
        determinant = (transform[0][0] * (transform[1][1] * transform[2][2] - transform[1][2] * transform[2][1])
                       - transform[0][1] * (transform[1][0] * transform[2][2] - transform[1][2] * transform[2][0])
                       + transform[0][2] * (transform[1][0] * transform[2][1] - transform[1][1] * transform[2][0]))
        if abs(determinant) < 1e-12:
            raise ValueError("Singular physical node transform")
        if "mesh" in node:
            metadata = node.get("extras", {})
            if "surface_code" not in metadata or metadata.get("collision_role") not in ["driveable", "solid"]:
                raise ValueError("Physical node lacks supported source surface/role metadata: " + node.get("name", str(identifier)))
            for primitive_index, primitive in enumerate(document["meshes"][node["mesh"]]["primitives"]):
                if primitive.get("mode", 4) != 4:
                    raise ValueError("Physical source must contain triangles")
                positions = accessor_values(primitive["attributes"]["POSITION"])
                indices = [value[0] for value in accessor_values(primitive["indices"])] if "indices" in primitive else list(range(len(positions)))
                if len(indices) % 3 or not indices or any(index >= len(positions) for index in indices):
                    raise ValueError("Invalid physical triangle indices")
                vertices = [[sum(transform[row][column] * position[column] for column in range(3)) + transform[row][3]
                             for row in range(3)] for position in positions]
                if any(not math.isfinite(value) for position in vertices for value in position):
                    raise ValueError("Nonfinite physical vertices")
                triangles = [indices[index:index + 3] for index in range(0, len(indices), 3)]
                if determinant < 0:
                    triangles = [[first, third, second] for first, second, third in triangles]
                shapes.append({"identifier": node.get("name", str(identifier)) + "_primitive_" + str(primitive_index),
                               "surface_code": metadata["surface_code"], "collision_role": metadata["collision_role"],
                               "vertices_world_metres": vertices, "triangle_indices": triangles})
        for child in node.get("children", []):
            visit(child, transform)
        active.remove(identifier)

    for root in document["scenes"][document.get("scene", 0)]["nodes"]:
        visit(root, identity)
    if not shapes:
        raise ValueError("Physical world package is empty")
    shapes.sort(key=lambda shape: shape["identifier"])
    geometry_digest = hashlib.sha256()
    for shape in shapes:
        for label in [shape["identifier"], shape["collision_role"]]:
            encoded = label.encode("utf-8")
            geometry_digest.update(struct.pack("<Q", len(encoded)))
            geometry_digest.update(encoded)
        geometry_digest.update(struct.pack("<IQ", shape["surface_code"], len(shape["vertices_world_metres"])))
        for position in shape["vertices_world_metres"]:
            geometry_digest.update(struct.pack("<ddd", *position))
        geometry_digest.update(struct.pack("<Q", len(shape["triangle_indices"])))
        for triangle in shape["triangle_indices"]:
            geometry_digest.update(struct.pack("<III", *triangle))
    return {"schema_version": 1, "source_path": source_path.resolve().relative_to(REPOSITORY_ROOT.resolve()).as_posix(),
            "geometry_digest": geometry_digest.hexdigest(),
            "source_digest": hashlib.sha256(source).hexdigest(), "coordinate_contract": "right_positive_x_up_positive_y_forward_negative_z_metres",
            "shapes": sorted(shapes, key=lambda shape: shape["identifier"])}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    package = prepare_package(arguments.source)
    destination = validate_output_path(REPOSITORY_ROOT, arguments.output, "preview").path
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(structured_serialization.dumps(package, separators=(",", ":"), allow_nan=False) + "\n", encoding="utf-8")
    print(structured_serialization.dumps({"output": str(destination), "source_digest": package["source_digest"],
        "shape_count": len(package["shapes"]), "triangle_count": sum(len(shape["triangle_indices"]) for shape in package["shapes"])}))


if __name__ == "__main__":
    main()
