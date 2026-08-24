from __future__ import annotations

from pathlib import Path
import json

import bpy


def _image(path: Path):
    key = str(path.resolve())
    existing = bpy.data.images.get(path.name)
    if existing and Path(existing.filepath).resolve() == path.resolve():
        return existing
    return bpy.data.images.load(key, check_existing=True)


def texture_material(name: str, texture_path: Path, roughness: float, metallic: float = 0.0, alpha: bool = False):
    existing = bpy.data.materials.get(name)
    if existing:
        return existing
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    links = mat.node_tree.links
    for node in list(nodes):
        nodes.remove(node)

    out = nodes.new("ShaderNodeOutputMaterial")
    bsdf = nodes.new("ShaderNodeBsdfPrincipled")
    tex = nodes.new("ShaderNodeTexImage")
    tex.image = _image(texture_path)
    tex.interpolation = "Closest"
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    links.new(tex.outputs["Color"], bsdf.inputs["Base Color"])
    links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])

    if alpha:
        links.new(tex.outputs["Alpha"], bsdf.inputs["Alpha"])
        mat.use_backface_culling = False
        if hasattr(mat, "surface_render_method"):
            mat.surface_render_method = "DITHERED"
        elif hasattr(mat, "blend_method"):
            mat.blend_method = "CLIP"
            mat.alpha_threshold = 0.34
        if hasattr(mat, "use_transparency_overlap"):
            mat.use_transparency_overlap = False
    return mat


def flat_material(name: str, color, roughness: float = 0.8, metallic: float = 0.0):
    existing = bpy.data.materials.get(name)
    if existing:
        return existing
    mat = bpy.data.materials.new(name)
    mat.diffuse_color = (*color, 1.0)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    return mat


def build_material_library(texture_dir: str | Path, curb_manifest: dict | None = None) -> dict[str, bpy.types.Material]:
    root = Path(texture_dir)
    manifest_path = root / "active_manifest.json"
    if not manifest_path.exists():
        raise RuntimeError(f"Missing procedural texture manifest: {manifest_path}. Run generate_procedural_textures.py first.")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    forge = manifest.get("forge", {})
    if forge.get("style") != "ps1_rally_clean":
        raise RuntimeError(f"Unexpected texture forge style: {forge}")

    materials = {
        "asphalt": texture_material("F90_Asphalt", root / manifest["shared"]["asphalt"], roughness=0.95),
        "ground": texture_material(f"F90_Ground_{manifest['active_biome']}", root / manifest["terrain"], roughness=1.0),
        "shoulder": texture_material(f"F90_Shoulder_{manifest['active_biome']}", root / manifest["shoulder"], roughness=1.0),
        "bark": texture_material(f"F90_Bark_{manifest['active_biome']}", root / manifest["bark"], roughness=0.92),
        "guardrail": texture_material("F90_Guardrail", root / manifest["shared"]["guardrail"], roughness=0.50, metallic=0.66),
        "start_finish": texture_material("F90_StartFinish", root / manifest["shared"]["start_finish"], roughness=0.72),
        "edge_line": flat_material("F90_EdgeLine", (0.94, 0.94, 0.91), roughness=0.86),
        "roof": flat_material("F90_SimpleRoof", (0.31, 0.30, 0.28), roughness=0.96),
        "tire_barrier": flat_material("F90_TireBarrier", (0.025, 0.022, 0.018), roughness=0.98),
        "spectator": flat_material("F90_SpectatorSilhouette", (0.035, 0.045, 0.055), roughness=1.0),
        "marshal": flat_material("F90_MarshalSilhouette", (0.82, 0.40, 0.08), roughness=0.96),
        "photographer": flat_material("F90_PhotographerSilhouette", (0.08, 0.10, 0.12), roughness=1.0),
        "flag_pole": flat_material("F90_TracksideFlagPole", (0.18, 0.20, 0.22), roughness=0.78, metallic=0.45),
        "flag_navy": flat_material("F90_TracksideFlagNavy", (0.035, 0.10, 0.22), roughness=0.92),
        "flag_white": flat_material("F90_TracksideFlagWhite", (0.88, 0.87, 0.81), roughness=0.92),
        "sign": flat_material("F90_TracksideSign", (0.72, 0.62, 0.30), roughness=0.94),
    }
    for material_id, spec in (curb_manifest or {}).get("palette", {}).items():
        materials[f"curb:{material_id}"] = flat_material(
            f"F90_Curb_{material_id}",
            tuple(float(channel) for channel in spec["color"]),
            roughness=float(spec.get("roughness", 0.84)),
        )
    for asset_id, rel in manifest["assets"].items():
        alpha = ("_tree_" in asset_id or "_bush_" in asset_id or "_grass_" in asset_id)
        materials[f"asset:{asset_id}"] = texture_material(
            f"F90_{asset_id}", root / rel,
            roughness=0.98 if alpha else 0.92,
            alpha=alpha,
        )
    repo_root = root.parents[3]
    trackside_people = repo_root / "blender/assets/texture_sources/la_chutana/trackside/people"
    if trackside_people.exists():
        for png in trackside_people.glob("*_source.png"):
            key = png.stem.replace("_source", "")
            materials[f"card:{key}"] = texture_material(
                f"F90_Card_{key}", png, roughness=1.0, alpha=True,
            )
            materials[f"asset:{key}"] = materials[f"card:{key}"]
    trackside_signs = repo_root / "blender/assets/texture_sources/la_chutana/trackside/signs"
    if trackside_signs.exists():
        for png in trackside_signs.glob("*_source.png"):
            key = png.stem.replace("_source", "")
            materials[f"card:{key}"] = texture_material(
                f"F90_Card_{key}", png, roughness=0.92, alpha=True,
            )
            materials[f"asset:{key}"] = materials[f"card:{key}"]
    materials["active_biome"] = manifest["active_biome"]
    return materials


def add_safety_barrier_materials(
    materials: dict[str, bpy.types.Material],
    palette: dict,
    guardrail_bitmap_path: Path | None = None,
    tire_bitmap_path: Path | None = None,
) -> None:
    specs = {
        "white": (0.82, 0.0),
        "navy": (0.86, 0.0),
        "concrete": (0.96, 0.0),
        "steel": (0.48, 0.68),
        "steel_dark": (0.56, 0.58),
    }
    for key, (roughness, metallic) in specs.items():
        rgba = palette[key]
        materials[f"safety:{key}"] = flat_material(
            f"F90_Safety_{key}", tuple(float(value) for value in rgba[:3]),
            roughness=roughness, metallic=metallic,
        )
    if guardrail_bitmap_path is not None:
        materials["safety:guardrail_card"] = texture_material(
            "F90_Safety_GuardrailCard", guardrail_bitmap_path,
            roughness=0.62, metallic=0.18, alpha=True,
        )
        materials["safety:guardrail_card"].use_backface_culling = False
    if tire_bitmap_path is not None:
        materials["safety:tire_wall"] = texture_material(
            "F90_Safety_TireWall", tire_bitmap_path,
            roughness=0.96, metallic=0.0, alpha=False,
        )
        materials["safety:tire_wall"].use_backface_culling = False
