#!/usr/bin/env python3
"""Add deterministic smooth vertex normals to GLB triangle primitives.

The transformer preserves positions, indices, UVs, materials, images, nodes,
and transforms.  Only new NORMAL accessors and their buffer data are appended.
Coincident vertices are smoothed across UV seams when their existing geometric
normal lies within the configured crease angle.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import struct
import tempfile
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np


GLB_MAGIC = b"glTF"
JSON_CHUNK = 0x4E4F534A
BIN_CHUNK = 0x004E4942
TRIANGLES = 4

COMPONENT_DTYPES = {
    5121: np.dtype("<u1"),
    5123: np.dtype("<u2"),
    5125: np.dtype("<u4"),
    5126: np.dtype("<f4"),
}
TYPE_WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}


def _chunks(path: Path) -> tuple[dict[str, Any], bytes]:
    data = path.read_bytes()
    if len(data) < 20 or data[:4] != GLB_MAGIC:
        raise ValueError(f"not a GLB file: {path}")
    version, declared_length = struct.unpack_from("<II", data, 4)
    if version != 2 or declared_length != len(data):
        raise ValueError(f"invalid GLB header: version={version}, length={declared_length}")

    offset = 12
    json_payload: bytes | None = None
    bin_payload: bytes | None = None
    while offset < len(data):
        chunk_length, chunk_type = struct.unpack_from("<II", data, offset)
        offset += 8
        payload = data[offset : offset + chunk_length]
        offset += chunk_length
        if chunk_type == JSON_CHUNK:
            if json_payload is not None:
                raise ValueError("GLB contains multiple JSON chunks")
            json_payload = payload
        elif chunk_type == BIN_CHUNK:
            if bin_payload is not None:
                raise ValueError("GLB contains multiple BIN chunks")
            bin_payload = payload
        else:
            raise ValueError(f"unsupported GLB chunk type: 0x{chunk_type:08x}")
    if json_payload is None or bin_payload is None:
        raise ValueError("GLB must contain one JSON and one BIN chunk")
    document = json.loads(json_payload.rstrip(b" \x00").decode("utf-8"))
    if len(document.get("buffers", [])) != 1:
        raise ValueError("only single-buffer GLBs are supported")
    declared_buffer_length = int(document["buffers"][0]["byteLength"])
    if declared_buffer_length > len(bin_payload):
        raise ValueError("declared buffer length exceeds BIN chunk")
    return document, bin_payload[:declared_buffer_length]


def _read_accessor(document: dict[str, Any], buffer: bytes, accessor_index: int) -> np.ndarray:
    accessor = document["accessors"][accessor_index]
    if "sparse" in accessor:
        raise ValueError("sparse accessors are not supported")
    view = document["bufferViews"][accessor["bufferView"]]
    dtype = COMPONENT_DTYPES.get(accessor["componentType"])
    width = TYPE_WIDTHS.get(accessor["type"])
    if dtype is None or width is None:
        raise ValueError(
            f"unsupported accessor layout: component={accessor['componentType']} type={accessor['type']}"
        )
    count = int(accessor["count"])
    byte_offset = int(view.get("byteOffset", 0)) + int(accessor.get("byteOffset", 0))
    packed_stride = dtype.itemsize * width
    stride = int(view.get("byteStride", packed_stride))
    if stride == packed_stride:
        values = np.frombuffer(buffer, dtype=dtype, count=count * width, offset=byte_offset)
        return values.reshape((count, width)).copy()
    values = np.ndarray(
        shape=(count, width),
        dtype=dtype,
        buffer=buffer,
        offset=byte_offset,
        strides=(stride, dtype.itemsize),
    )
    return values.copy()


def _normalize(rows: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    lengths = np.linalg.norm(rows, axis=1)
    result = np.zeros_like(rows, dtype=np.float64)
    valid = lengths > 1e-15
    result[valid] = rows[valid] / lengths[valid, None]
    return result, valid


def calculate_normals(
    positions: np.ndarray,
    indices: np.ndarray,
    crease_angle_degrees: float,
    weld_tolerance: float,
) -> tuple[np.ndarray, dict[str, int]]:
    if len(indices) % 3:
        raise ValueError("triangle index count is not divisible by three")
    triangles = indices.reshape((-1, 3)).astype(np.int64, copy=False)
    if triangles.size and (triangles.min() < 0 or triangles.max() >= len(positions)):
        raise ValueError("triangle indices are outside the POSITION accessor")

    p = positions.astype(np.float64, copy=False)
    cross = np.cross(p[triangles[:, 1]] - p[triangles[:, 0]], p[triangles[:, 2]] - p[triangles[:, 0]])
    face_normals, valid_faces = _normalize(cross)
    weighted = np.zeros((len(p), 3), dtype=np.float64)
    for corner in range(3):
        np.add.at(weighted, triangles[valid_faces, corner], cross[valid_faces])
    base_normals, valid_vertices = _normalize(weighted)

    scale = 1.0 / weld_tolerance
    position_keys = np.rint(p * scale).astype(np.int64)
    groups: dict[tuple[int, int, int], list[int]] = defaultdict(list)
    for vertex_index, key in enumerate(position_keys):
        groups[tuple(int(v) for v in key)].append(vertex_index)

    cosine_limit = math.cos(math.radians(crease_angle_degrees))
    result = base_normals.copy()
    smoothed = 0
    for group in groups.values():
        if len(group) < 2:
            continue
        group_indices = np.asarray(group, dtype=np.int64)
        for vertex_index in group:
            if not valid_vertices[vertex_index]:
                continue
            compatible = group_indices[
                (base_normals[group_indices] @ base_normals[vertex_index]) >= cosine_limit
            ]
            combined = weighted[compatible].sum(axis=0)
            length = float(np.linalg.norm(combined))
            if length <= 1e-15:
                continue
            candidate = combined / length
            if float(np.linalg.norm(candidate - base_normals[vertex_index])) > 1e-7:
                smoothed += 1
            result[vertex_index] = candidate

    unresolved = ~valid_vertices
    if unresolved.any():
        result[unresolved] = np.array([0.0, 1.0, 0.0])
    return result.astype("<f4"), {
        "vertices": int(len(positions)),
        "triangles": int(len(triangles)),
        "degenerate_faces": int((~valid_faces).sum()),
        "fallback_normals": int(unresolved.sum()),
        "uv_seam_vertices_smoothed": smoothed,
    }


def normal_coverage(path: Path) -> dict[str, int]:
    document, _ = _chunks(path)
    primitives = [primitive for mesh in document.get("meshes", []) for primitive in mesh["primitives"]]
    triangle_primitives = [p for p in primitives if int(p.get("mode", TRIANGLES)) == TRIANGLES]
    with_normals = sum("NORMAL" in p["attributes"] for p in triangle_primitives)
    return {
        "triangle_primitives": len(triangle_primitives),
        "with_normals": with_normals,
        "missing_normals": len(triangle_primitives) - with_normals,
    }


def add_normals(
    source: Path,
    output: Path,
    crease_angle_degrees: float = 45.0,
    weld_tolerance: float = 1e-6,
) -> dict[str, Any]:
    document, original_buffer = _chunks(source)
    buffer = bytearray(original_buffer)
    primitive_reports: list[dict[str, Any]] = []

    for mesh_index, mesh in enumerate(document.get("meshes", [])):
        for primitive_index, primitive in enumerate(mesh["primitives"]):
            if int(primitive.get("mode", TRIANGLES)) != TRIANGLES:
                continue
            if "NORMAL" in primitive["attributes"]:
                continue
            positions = _read_accessor(document, original_buffer, primitive["attributes"]["POSITION"])
            if primitive.get("indices") is None:
                indices = np.arange(len(positions), dtype=np.uint32)
            else:
                indices = _read_accessor(document, original_buffer, primitive["indices"]).reshape(-1)
            normals, metrics = calculate_normals(
                positions, indices, crease_angle_degrees, weld_tolerance
            )

            while len(buffer) % 4:
                buffer.append(0)
            byte_offset = len(buffer)
            payload = normals.tobytes(order="C")
            buffer.extend(payload)
            view_index = len(document.setdefault("bufferViews", []))
            document["bufferViews"].append(
                {"buffer": 0, "byteOffset": byte_offset, "byteLength": len(payload)}
            )
            accessor_index = len(document.setdefault("accessors", []))
            document["accessors"].append(
                {
                    "bufferView": view_index,
                    "componentType": 5126,
                    "count": len(normals),
                    "type": "VEC3",
                }
            )
            primitive["attributes"]["NORMAL"] = accessor_index
            primitive_reports.append(
                {
                    "mesh_index": mesh_index,
                    "mesh_name": mesh.get("name", ""),
                    "primitive_index": primitive_index,
                    **metrics,
                }
            )

    document["buffers"][0]["byteLength"] = len(buffer)
    json_payload = json.dumps(document, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    json_payload += b" " * ((-len(json_payload)) % 4)
    bin_payload = bytes(buffer) + b"\x00" * ((-len(buffer)) % 4)
    total_length = 12 + 8 + len(json_payload) + 8 + len(bin_payload)
    glb = bytearray(struct.pack("<4sII", GLB_MAGIC, 2, total_length))
    glb.extend(struct.pack("<II", len(json_payload), JSON_CHUNK))
    glb.extend(json_payload)
    glb.extend(struct.pack("<II", len(bin_payload), BIN_CHUNK))
    glb.extend(bin_payload)

    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        mode="wb", prefix=f"{output.name}.", suffix=".tmp", dir=output.parent, delete=False
    ) as temporary:
        temporary.write(glb)
        temporary_path = Path(temporary.name)
    try:
        os.replace(temporary_path, output)
    finally:
        temporary_path.unlink(missing_ok=True)

    coverage = normal_coverage(output)
    return {
        "source": str(source.resolve()),
        "output": str(output.resolve()),
        "crease_angle_degrees": crease_angle_degrees,
        "weld_tolerance": weld_tolerance,
        "primitives_modified": len(primitive_reports),
        "coverage": coverage,
        "primitive_reports": primitive_reports,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--crease-angle", type=float, default=45.0)
    parser.add_argument("--weld-tolerance", type=float, default=1e-6)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if not 0.0 < args.crease_angle <= 180.0:
        parser.error("--crease-angle must be in (0, 180]")
    if args.weld_tolerance <= 0.0:
        parser.error("--weld-tolerance must be positive")
    report = add_normals(args.source, args.output, args.crease_angle, args.weld_tolerance)
    encoded = json.dumps(report, indent=2) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(encoded, encoding="utf-8")
    print(encoded, end="")
    if report["coverage"]["missing_normals"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
