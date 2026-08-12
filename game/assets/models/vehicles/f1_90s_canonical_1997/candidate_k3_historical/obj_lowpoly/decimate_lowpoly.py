"""Low-poly generation for F1-90s Jordan 191 historical candidate (k3).

Strategy (built after a fail-fast experiment):
- The OBJ models use flat diffuse `Kd` colors per `usemtl` material group
  (no image textures / UV map). "Preserve the texture" => preserve the per-face
  material assignment so each face keeps its original Kd.
- PyMeshLab's `meshing_decimation_quadric_edge_collapse` preserves the number
  of materials and per-face `usemtl` assignments when the whole mesh is
  decimated at once. It renames materials to `material_0..N` in the order they
  FIRST APPEAR in the input OBJ. So after decimation we text-rewrite the OBJ
  to map `material_N` back to the original usemtl name, ignore pymeshlab's
  auto-generated `.obj.mtl`, and copy the original `.mtl` unchanged. This keeps
  exact Kd colors tied to per-face materials.
- Whole-mesh decimation also reaches the target face count exactly (the earlier
  per-material-chunk approach was structurally limited by `preserveboundary`
  and plateaus at ~31%).
- Target per model: 20% of high-poly face count.

Outputs to <candidate_k3_historical>/obj_lowpoly/:
    <model>.obj      decimated, materials remapped to original names
    <model>.mtl      copied unchanged from the HP source
    decimate_manifest.json   per-model face counts + ratio
"""

import os
import sys
import json
import shutil

import pymeshlab

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

OBJ_DIR = r"D:\Formula90s\game\assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical\obj"
OUT_DIR = r"D:\Formula90s\game\assets\models\vehicles\f1_90s_canonical_1997\candidate_k3_historical\obj_lowpoly"

RATIO = 0.20

MODELS = [
    "jordan_191_candidate_chassis_copy.obj",
    "jordan_191_candidate_wheel_front_copy.obj",
    "jordan_191_candidate_wheel_rear_copy.obj",
]

os.makedirs(OUT_DIR, exist_ok=True)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def first_usemtl_order(obj_path):
    """Return list of unique `usemtl` names in the order they first appear."""
    out = []
    seen = set()
    with open(obj_path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            if line.startswith("usemtl "):
                name = line.split()[1].strip()
                if name not in seen:
                    seen.add(name)
                    out.append(name)
    return out


def count_v_f(obj_path):
    v = 0; f = 0
    with open(obj_path, "r", encoding="utf-8", errors="replace") as fh:
        for line in fh:
            if line.startswith("v "):
                v += 1
            elif line.startswith("f "):
                f += 1
    return v, f


# ---------------------------------------------------------------------------
# Per-model pipeline
# ---------------------------------------------------------------------------

def process_model(obj_name):
    src_obj = os.path.join(OBJ_DIR, obj_name)
    base = os.path.splitext(obj_name)[0]
    out_obj = os.path.join(OUT_DIR, obj_name)
    out_mtl = os.path.join(OUT_DIR, base + ".mtl")
    src_mtl = os.path.join(OBJ_DIR, base + ".mtl")

    print(f"\n=== {obj_name} ===", flush=True)

    # 1) get first-encounter materiakemanal order from original OBJ
    orig_order = first_usemtl_order(src_obj)
    print(f"  original materials (first-encounter): {len(orig_order)}", flush=True)

    # 2) load into pymeshlab
    ms = pymeshlab.MeshSet()
    ms.load_new_mesh(src_obj)
    m = ms.current_mesh()
    faces_before = m.face_number()
    verts_before = m.vertex_number()
    print(f"  HP: verts={verts_before} faces={faces_before}", flush=True)

    # 3) whole-mesh decimation
    target = int(faces_before * RATIO)
    if target < faces_before:
        ms.meshing_decimation_quadric_edge_collapse(
            targetfacenum=target,
            targetperc=0.0,
            qualitythr=0.3,
            preserveboundary=False,
            boundaryweight=1.0,
            preservenormal=False,
            preservetopology=False,
            optimalplacement=True,
            planarquadric=False,
            planarweight=0.001,
            qualityweight=False,
            autoclean=True,
            selected=False,
        )
    m2 = ms.current_mesh()
    faces_after = m2.face_number()
    verts_after = m2.vertex_number()
    print(f"  decimated: verts={verts_after} faces={faces_after} (target={target})", flush=True)

    # 4) save (pymeshlab writes <out_obj>.mtl too, we'll discard it)
    ms.save_current_mesh(out_obj)
    auto_mtl = out_obj + ".mtl"
    if os.path.exists(auto_mtl):
        os.remove(auto_mtl)

    # 5) text-rewrite the saved OBJ: remap material_N -> original usemtl name
    #    and fix the mtllib path to point at our copied MTL.
    out_mtl_basename = os.path.basename(out_mtl)
    remap_lines = []
    with open(out_obj, "r", encoding="utf-8", errors="replace") as f:
        text = f.read()

    new_lines = []
    for line in text.splitlines(keepends=True):
        ls = line.lstrip()
        if ls.startswith("mtllib "):
            parts = line.rstrip().split()
            new_lines.append(f"mtllib {out_mtl_basename}\n")
        elif ls.startswith("usemtl "):
            parts = line.rstrip().split(None, 1)
            name = parts[1].strip() if len(parts) > 1 else ""
            # name looks like "material_N"
            if name.startswith("material_"):
                try:
                    nidx = int(name[len("material_"):])
                    if 0 <= nidx < len(orig_order):
                        new_lines.append(f"usemtl {orig_order[nidx]}\n")
                    else:
                        new_lines.append(line)  # unknown, keep
                        remap_lines.append(f"  !! unmaterialized: {name}")
                except ValueError:
                    new_lines.append(line)
            else:
                new_lines.append(line)
        else:
            new_lines.append(line)

    with open(out_obj, "w", encoding="utf-8") as f:
        f.writelines(new_lines)

    # 6) copy original MTL unchanged
    if os.path.exists(src_mtl):
        shutil.copy2(src_mtl, out_mtl)
        mtl_note = f"copied original MTL ({os.path.getsize(out_mtl)} bytes)"
    else:
        mtl_note = "no source mtl"
    print(f"  mtllib/usemtl remapped; mtl: {mtl_note}", flush=True)

    # 7) re-validate by parsing the new OBJ: count v/f + usemtl markers
    nv, nf = count_v_f(out_obj)
    n_usemtl = sum(1 for ln in new_lines if ln.lstrip().startswith("usemtl "))
    n_unique = sorted({ln.split()[1].strip() for ln in new_lines
                       if ln.lstrip().startswith("usemtl ")})
    print(f"  verify: file verts={nv} faces={nf}  usemtl_markers={n_usemtl} "
          f"unique_matls={len(n_unique)}", flush=True)

    actual_ratio = faces_after / faces_before if faces_before else 0.0
    print(f"  ratio: {actual_ratio*100:5.2f}%", flush=True)

    return {
        "model": obj_name,
        "output": out_obj,
        "verts_before": verts_before,
        "verts_after": verts_after,
        "faces_before": faces_before,
        "faces_after": faces_after,
        "target": target,
        "ratio": actual_ratio,
        "material_count": len(orig_order),
        "materials_after": len(n_unique),
        "materials_intact": len(n_unique) == len(orig_order),
        "mtl": mtl_note,
    }


def main():
    chosen = sys.argv[1:] if len(sys.argv) > 1 else MODELS
    report_all = []
    for name in chosen:
        try:
            r = process_model(name)
        except Exception as e:
            import traceback
            print(f"  !!ERROR on {name}:\n{traceback.format_exc()}", flush=True)
            r = {"model": name, "error": repr(e)}
        report_all.append(r)

    tb = sum(r.get("faces_before", 0) for r in report_all)
    ta = sum(r.get("faces_after", 0) for r in report_all)
    summary = {
        "tool": "pymeshlab.meshing_decimation_quadric_edge_collapse",
        "target_ratio": RATIO,
        "texture_preservation": (
            "whole-mesh decimation preserves per-face material assignment; "
            "material_N names remapped to original usemtl; original MTL copied "
            "unchanged (same Kd palette per face)."
        ),
        "decimation_constraints": {
            "preserveboundary": False,
            "preservetopology": False,
            "optimalplacement": True,
            "autoclean": True,
        },
        "models": report_all,
        "total_faces_before": tb,
        "total_faces_after": ta,
        "total_ratio": ta / tb if tb else 0,
    }
    manifest = os.path.join(OUT_DIR, "decimate_manifest.json")
    with open(manifest, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)
    print(f"\n=== SUMMARY ===\nBefore: {tb}\nAfter:  {ta}\nRatio:  "
          f"{(ta/tb*100) if tb else 0:.2f}%\nManifest: {manifest}", flush=True)


if __name__ == "__main__":
    main()