"""Deterministic orthographic semantic SVG compiler."""
from __future__ import annotations

from hashlib import sha256
from html import escape
from pathlib import Path
import re
from typing import Any

from .canonical import canonical_json_bytes
from .domain import VehicleDocument
from .glb_scan import load_existing_inspector


VIEW_AXES = {
    "top": ("+X", "-Z"),
    "side": ("-Z", "+Y"),
    "front": ("+X", "+Y"),
    "rear": ("-X", "+Y"),
}


def compile_cad_views(
    document: VehicleDocument | dict[str, Any], *, repo_root: Path
) -> tuple[dict[str, bytes], dict[str, Any]]:
    vehicle = document if isinstance(document, VehicleDocument) else VehicleDocument.from_dict(document)
    canonical = vehicle.canonical_dict()
    document_hash = sha256(vehicle.canonical_bytes()).hexdigest().upper()
    geometry = _component_geometry(canonical, Path(repo_root).resolve())
    views: dict[str, bytes] = {}
    entries: list[dict[str, Any]] = []
    for kind in ("top", "side", "front", "rear"):
        component_edges = {
            component_id: _project_edges(triangles, VIEW_AXES[kind])
            for component_id, triangles in geometry.items()
        }
        svg = _render_svg(
            kind, component_edges,
            document_hash=document_hash,
            source_hash=canonical["source"]["sha256"],
        )
        name = f"{kind}.svg"
        views[name] = svg
        entries.append({
            "view_id": f"view-{kind}",
            "path": name,
            "sha256": sha256(svg).hexdigest().upper(),
            "component_ids": sorted(component_edges),
            "edge_count": sum(len(edges) for edges in component_edges.values()),
        })
    manifest = {
        "schema_version": "vehicle-cad-views/v1",
        "document_sha256": document_hash,
        "source_sha256": canonical["source"]["sha256"],
        "views": entries,
    }
    return views, manifest


def cad_manifest_bytes(manifest: dict[str, Any]) -> bytes:
    return canonical_json_bytes(manifest)


def _component_geometry(document: dict[str, Any], repo_root: Path) -> dict[str, list[tuple]]:
    inspector = load_existing_inspector(repo_root)
    cache: dict[str, tuple[dict[str, Any], bytes]] = {}
    result: dict[str, list[tuple]] = {}
    for component in document["component_map"]:
        binding = component.get("source_binding") or {}
        source_path = binding.get("source_path")
        if not isinstance(source_path, str):
            result[component["component_id"]] = []
            continue
        absolute = (repo_root / source_path).resolve()
        try:
            absolute.relative_to(repo_root)
        except ValueError as exc:
            raise ValueError("component source escapes repository") from exc
        if source_path not in cache:
            cache[source_path] = inspector.read_glb(str(absolute))
        gltf, binary = cache[source_path]
        object_ids = set(binding.get("object_ids") or [])
        role = component.get("semantic_role", "")
        result[component["component_id"]] = _triangles_for_component(
            inspector, gltf, binary, object_ids, role
        )
    return result


def _triangles_for_component(
    inspector: Any,
    gltf: dict[str, Any],
    binary: bytes,
    object_ids: set[str],
    role: str,
) -> list[tuple]:
    nodes = gltf.get("nodes", [])
    world = _world_matrices(inspector, gltf)
    triangles: list[tuple] = []
    for node_index, node in enumerate(nodes):
        mesh_index = node.get("mesh")
        if not isinstance(mesh_index, int):
            continue
        mesh = gltf.get("meshes", [])[mesh_index]
        names = {_slug(node.get("name", "")), _slug(mesh.get("name", ""))}
        selected = bool(names & object_ids)
        if role.startswith("wheel_"):
            selected = selected or any(name.startswith(f"geo-{role.replace('_', '-')}-") for name in names)
        if not selected:
            continue
        matrix = world[node_index]
        for primitive_index, primitive in enumerate(mesh.get("primitives", [])):
            if int(primitive.get("mode", 4)) != 4:
                continue
            position_accessor = primitive.get("attributes", {}).get("POSITION")
            if not isinstance(position_accessor, int):
                continue
            position_values, position_components, _ = inspector.read_accessor(gltf, binary, position_accessor)
            positions = [position_values[index:index + position_components] for index in range(0, len(position_values), position_components)]
            if isinstance(primitive.get("indices"), int):
                indices, _, _ = inspector.read_accessor(gltf, binary, primitive["indices"])
                order = [int(item[0] if isinstance(item, (list, tuple)) else item) for item in indices]
            else:
                order = list(range(len(positions)))
            transformed = [
                tuple(inspector.transform_point(matrix, list(position)))
                for position in positions
            ]
            for triangle_index in range(0, len(order) - 2, 3):
                triangles.append((
                    node_index, primitive_index, triangle_index // 3,
                    transformed[order[triangle_index]],
                    transformed[order[triangle_index + 1]],
                    transformed[order[triangle_index + 2]],
                ))
    triangles.sort(key=lambda item: item[:3])
    return triangles


def _world_matrices(inspector: Any, gltf: dict[str, Any]) -> list[Any]:
    nodes = gltf.get("nodes", [])
    local = [inspector.mat_from_trs(node) for node in nodes]
    world = [None] * len(nodes)
    children = {child for node in nodes for child in node.get("children", [])}
    roots = [
        index for index in gltf.get("scenes", [{}])[gltf.get("scene", 0)].get("nodes", [])
        if isinstance(index, int)
    ] or [index for index in range(len(nodes)) if index not in children]

    def visit(index: int, parent: Any) -> None:
        world[index] = inspector.mat_mul(parent, local[index])
        for child in nodes[index].get("children", []):
            visit(child, world[index])

    for root in roots:
        visit(root, inspector.mat_identity())
    for index in range(len(nodes)):
        if world[index] is None:
            visit(index, inspector.mat_identity())
    return world


def _project_edges(triangles: list[tuple], axes: tuple[str, str]) -> list[tuple[float, float, float, float]]:
    edges: set[tuple[int, int, int, int]] = set()
    for triangle in triangles:
        points = [_project(point, axes) for point in triangle[3:6]]
        for first, second in ((points[0], points[1]), (points[1], points[2]), (points[2], points[0])):
            a = (round(first[0] * 1_000_000), round(-first[1] * 1_000_000))
            b = (round(second[0] * 1_000_000), round(-second[1] * 1_000_000))
            if a != b:
                edges.add((*min(a, b), *max(a, b)))
    return [tuple(value / 1_000_000 for value in edge) for edge in sorted(edges)]


def _project(point: tuple[float, float, float], axes: tuple[str, str]) -> tuple[float, float]:
    values = {"X": point[0], "Y": point[1], "Z": point[2]}
    return tuple((1 if axis[0] == "+" else -1) * values[axis[1]] for axis in axes)  # type: ignore[return-value]


def _render_svg(
    kind: str,
    components: dict[str, list[tuple[float, float, float, float]]],
    *,
    document_hash: str,
    source_hash: str,
) -> bytes:
    all_edges = [edge for edges in components.values() for edge in edges]
    coordinates = [(edge[0], edge[1]) for edge in all_edges] + [(edge[2], edge[3]) for edge in all_edges]
    if coordinates:
        xs, ys = zip(*coordinates)
        min_x, max_x, min_y, max_y = min(xs), max(xs), min(ys), max(ys)
    else:
        min_x, max_x, min_y, max_y = -1.0, 1.0, -1.0, 1.0
    margin = max(max_x - min_x, max_y - min_y, 1.0) * 0.05
    view_box = (min_x - margin, min_y - margin, max_x - min_x + 2 * margin, max_y - min_y + 2 * margin)
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{_numbers(view_box)}" data-schema="vehicle-cad-svg/v1" data-view-id="view-{kind}" data-document-sha256="{document_hash}" data-source-sha256="{source_hash}">',
        f'  <title>Vehicle {kind} orthographic view</title>',
        '  <g id="cad-components" fill="none" stroke="#111827" stroke-width="0.002" vector-effect="non-scaling-stroke">',
    ]
    for component_id in sorted(components):
        path = " ".join(
            f"M {_number(edge[0])} {_number(edge[1])} L {_number(edge[2])} {_number(edge[3])}"
            for edge in components[component_id]
        )
        lines.append(
            f'    <path id="{escape(component_id)}" data-component-id="{escape(component_id)}" d="{path}"/>'
        )
    lines.extend(['  </g>', '</svg>', ''])
    return "\n".join(lines).encode("utf-8")


def _number(value: float) -> str:
    text = f"{value:.6f}".rstrip("0").rstrip(".")
    return "0" if text in {"", "-0"} else text


def _numbers(values: tuple[float, ...]) -> str:
    return " ".join(_number(value) for value in values)


def _slug(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")




