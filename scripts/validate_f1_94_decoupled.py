from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "game/assets/models/vehicles/f1_94/decoupled"
ASSEMBLY = PACKAGE / "geometry/F1_94_geometry.glb"
ALBEDO = PACKAGE / "textures/albedo"

REQUIRED_NODES = {
    "DATUM_VEHICLE_ORIGIN", "DATUM_FRONT_AXLE_CENTER", "DATUM_REAR_AXLE_CENTER",
    "JNT_WHEEL_FL", "JNT_WHEEL_FR", "JNT_WHEEL_RL", "JNT_WHEEL_RR",
    "JNT_FRONT_WING_MOUNT", "JNT_REAR_WING_MOUNT",
    "JNT_SUSP_FL_CHASSIS", "JNT_SUSP_FL_HUB", "JNT_SUSP_FR_CHASSIS", "JNT_SUSP_FR_HUB",
    "JNT_SUSP_RL_CHASSIS", "JNT_SUSP_RL_HUB", "JNT_SUSP_RR_CHASSIS", "JNT_SUSP_RR_HUB",
    "GEO_CHASSIS", "GEO_AERO_FRONT_WING", "GEO_AERO_REAR_WING",
    "GEO_SUSPENSION_FL", "GEO_SUSPENSION_FR", "GEO_SUSPENSION_RL", "GEO_SUSPENSION_RR",
}

def read_glb_json(path: Path) -> dict:
    data = path.read_bytes()
    if data[:4] != b"glTF" or len(data) < 20:
        raise ValueError(f"{path}: invalid GLB header")
    json_length, chunk_type = struct.unpack_from("<II", data, 12)
    if chunk_type != 0x4E4F534A:
        raise ValueError(f"{path}: first chunk is not JSON")
    return json.loads(data[20:20 + json_length].decode("utf-8"))

def main() -> int:
    failures: list[str] = []
    try:
        document = read_glb_json(ASSEMBLY)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"[FAIL] {error}", file=sys.stderr)
        return 1

    for forbidden in ("images", "textures", "samplers"):
        if document.get(forbidden):
            failures.append(f"embedded {forbidden} are forbidden")

    node_names = {node.get("name", "") for node in document.get("nodes", [])}
    for missing in sorted(REQUIRED_NODES - node_names):
        failures.append(f"missing required node {missing}")
    for corner in ("FL", "FR", "RL", "RR"):
        if not any(name.startswith(f"GEO_WHEEL_{corner}_") for name in node_names):
            failures.append(f"missing wheel geometry group GEO_WHEEL_{corner}_*")

    for mesh in document.get("meshes", []):
        for primitive in mesh.get("primitives", []):
            attributes = primitive.get("attributes", {})
            if "NORMAL" not in attributes:
                failures.append(f"{mesh.get('name', '<mesh>')}: missing NORMAL")
            if "TEXCOORD_0" not in attributes:
                failures.append(f"{mesh.get('name', '<mesh>')}: missing TEXCOORD_0")

    geometry_names = sorted(name for name in node_names if name.startswith("GEO_"))
    for name in geometry_names:
        if not (ALBEDO / f"{name}.png").is_file():
            failures.append(f"missing external albedo {name}.png")

    if failures:
        for failure in failures:
            print(f"[FAIL] {failure}", file=sys.stderr)
        return 1
    print(
        f"[PASS] F1-94 GLB: {len(geometry_names)} GEO nodes, required JNT/DATUM nodes, "
        "NORMAL + UV0, external albedos, no embedded images/textures."
    )
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
