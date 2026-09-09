import bpy
import hashlib
import json
import sys
from pathlib import Path


def mesh_signature(obj):
    if obj.type != "MESH":
        return None
    mesh = obj.data
    digest = hashlib.sha256()
    for vertex in mesh.vertices:
        digest.update(("%.6f,%.6f,%.6f;" % tuple(vertex.co)).encode("ascii"))
    for polygon in mesh.polygons:
        digest.update((",".join(str(index) for index in polygon.vertices) + ";").encode("ascii"))
    return {
        "vertices": len(mesh.vertices),
        "edges": len(mesh.edges),
        "polygons": len(mesh.polygons),
        "loops": len(mesh.loops),
        "sha256": digest.hexdigest(),
    }


def main():
    args = sys.argv[sys.argv.index("--") + 1 :]
    if len(args) != 1:
        raise SystemExit("usage: blender --background FILE --python SCRIPT -- OUTPUT.json")
    output = Path(args[0])
    objects = []
    for obj in sorted(bpy.data.objects, key=lambda item: item.name.casefold()):
        objects.append(
            {
                "name": obj.name,
                "type": obj.type,
                "data_name": obj.data.name if obj.data else None,
                "parent": obj.parent.name if obj.parent else None,
                "collections": sorted(collection.name for collection in obj.users_collection),
                "location": [round(value, 6) for value in obj.location],
                "dimensions": [round(value, 6) for value in obj.dimensions],
                "mesh": mesh_signature(obj),
                "materials": [slot.material.name if slot.material else None for slot in obj.material_slots],
            }
        )
    payload = {
        "blend": bpy.data.filepath,
        "objects": objects,
        "collections": sorted(collection.name for collection in bpy.data.collections),
    }
    output.write_text(json.dumps(payload, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
