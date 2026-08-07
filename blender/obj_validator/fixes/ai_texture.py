"""
ai_texture.py — Generación de texturas con IA para modelos OBJ
---------------------------------------------------------------
Este módulo genera texturas PBR (albedo) para cada sub-objeto del modelo
usando un pipeline de IA generativa y las aplica actualizando el .mtl.

Pipeline:
  1. Por cada sub-objeto: infiere el tipo de parte (rueda, chasis, aleróne)
  2. Construye un prompt descriptivo basado en el nombre del objeto y el vehículo
  3. Llama a la API de generación de imágenes (configurable)
  4. Guarda la textura en textures/<nombre_objeto>_albedo.png
  5. Actualiza el .mtl con rutas relativas

Modo de integración con Antigravity:
  Este módulo puede ser invocado desde fix_obj.py con --ai-textures,
  y genera los prompts para que el agente llame a generate_image por cada parte.

Configuración:
  - TEXTURE_SIZE: resolución de la textura generada (por defecto 1024)
  - STYLE_PRESET: estilo de renderizado ("F1 racing livery", "realistic carbon fiber", etc.)
"""

from __future__ import annotations

import os
import json
from typing import Dict, Any, List, Optional

# ──────────────────────────────────────────────────────────────────────────────
# Categorías de partes del vehículo y sus prompts base
# ──────────────────────────────────────────────────────────────────────────────

PART_PROMPTS: Dict[str, str] = {
    "chasis":      "Formula 1 car chassis body surface texture, carbon fiber weave pattern, "
                   "glossy dark composite material, PBR albedo map, seamless, top-down UV view, "
                   "red and white racing livery accents, no shadows, flat lighting",

    "aleron":      "Formula 1 aerodynamic wing surface, carbon fiber texture, matte black finish, "
                   "with thin colored racing stripes, PBR albedo map, seamless, flat lighting, "
                   "no shadows, high detail",

    "aleron_del":  "Formula 1 front wing texture, carbon fiber weave, dark grey with metallic sheen, "
                   "red accent stripe, PBR albedo map, seamless, flat lighting",

    "aleron_tras": "Formula 1 rear wing texture, carbon fiber matte black, sponsor logo areas, "
                   "red stripe detail, PBR albedo map, seamless, flat lighting",

    "neumatico":   "Formula 1 racing tire tread texture, Pirelli racing slick, black rubber, "
                   "subtle tread pattern on sides, yellow band marking, PBR albedo map, "
                   "cylindrical UV unwrap style, flat lighting, no shadows",

    "wheel":       "Formula 1 racing wheel rim texture, brushed aluminum alloy, "
                   "dark grey machined finish, Pirelli branding on tire sidewall, "
                   "PBR albedo map, flat lighting",

    "default":     "Formula 1 car part texture, carbon fiber with metallic details, "
                   "racing livery style, PBR albedo map, seamless, flat lighting, no shadows",
}

STYLE_SUFFIX = (
    ", 4K texture map quality, game-ready PBR asset, "
    "no environment reflection, pure material texture"
)

TEXTURE_DIR = "textures"


# ──────────────────────────────────────────────────────────────────────────────
# Inferencia de categoría por nombre del objeto
# ──────────────────────────────────────────────────────────────────────────────

def _infer_part_category(obj_name: str) -> str:
    name = obj_name.lower()
    if any(k in name for k in ["neumatico", "neumático", "tire", "tyre", "wheel_fl", "wheel_fr", "wheel_rl", "wheel_rr"]):
        return "neumatico"
    if any(k in name for k in ["rueda", "wheel", "rim"]):
        return "wheel"
    if any(k in name for k in ["aleron_del", "aleron_front", "front_wing", "aleron_delantero"]):
        return "aleron_del"
    if any(k in name for k in ["aleron_tras", "aleron_rear", "rear_wing", "aleron_trasero"]):
        return "aleron_tras"
    if any(k in name for k in ["aleron", "wing", "spoiler"]):
        return "aleron"
    if any(k in name for k in ["chasis", "chassis", "body", "carroceria", "fuselage", "hull"]):
        return "chasis"
    return "default"


def build_texture_prompts(obj_names: List[str], vehicle_style: str = "Formula 1 1997") -> List[Dict[str, str]]:
    """
    Construye una lista de prompts para generar texturas.
    Devuelve: [{obj_name, category, prompt, output_filename}, ...]
    """
    results = []
    for name in obj_names:
        category = _infer_part_category(name)
        base_prompt = PART_PROMPTS[category]
        full_prompt = f"{vehicle_style} — {base_prompt}{STYLE_SUFFIX}"
        output_file = f"{TEXTURE_DIR}/{name}_albedo.png"
        results.append({
            "obj_name":       name,
            "category":       category,
            "prompt":         full_prompt,
            "output_file":    output_file,
        })
    return results


# ──────────────────────────────────────────────────────────────────────────────
# Actualización del MTL con texturas IA
# ──────────────────────────────────────────────────────────────────────────────

def update_mtl_with_ai_textures(
    mtl_path: str,
    texture_map: Dict[str, str],  # {material_name: relative_texture_path}
    strip_old_maps: bool = True,
) -> Dict[str, Any]:
    """
    Reescribe el archivo .mtl para usar las texturas generadas por IA.

    texture_map: diccionario {nombre_material → ruta_relativa_textura}
    strip_old_maps: si True, elimina map_Kd y map_Bump del original antes de añadir

    Ejemplo:
        texture_map = {
            "Material_chasis": "textures/chasis_albedo.png",
            "Material_neumatico_fl": "textures/wheel_fl_albedo.png",
        }
    """
    if not os.path.isfile(mtl_path):
        return {"error": f"MTL no encontrado: {mtl_path}"}

    with open(mtl_path, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()

    new_lines = []
    current_mat = None
    updated_mats = []
    i = 0

    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        if stripped.startswith("newmtl "):
            current_mat = stripped[7:].strip()
            new_lines.append(line)

        elif stripped.lower().startswith("map_kd") or stripped.lower().startswith("map_bump"):
            if strip_old_maps and current_mat:
                # Omitir las líneas de mapa antiguas
                i += 1
                continue

        elif stripped == "" and current_mat and current_mat in texture_map:
            # Insertar la nueva textura antes de la línea vacía de separación
            rel_path = texture_map[current_mat]
            new_lines.append(f"map_Kd {rel_path}\n")
            updated_mats.append(current_mat)
            texture_map.pop(current_mat)  # evitar duplicar
            new_lines.append(line)
            i += 1
            continue

        new_lines.append(line)
        i += 1

    # Materiales que aún no tuvieron línea vacía al final
    for mat_name, rel_path in texture_map.items():
        new_lines.append(f"\nnewmtl {mat_name}_generated\n")
        new_lines.append(f"map_Kd {rel_path}\n")
        updated_mats.append(mat_name)

    # Backup del MTL original
    backup_path = mtl_path + ".bak"
    if not os.path.exists(backup_path):
        with open(mtl_path, "r", encoding="utf-8", errors="replace") as f:
            orig = f.read()
        with open(backup_path, "w", encoding="utf-8") as f:
            f.write(orig)

    with open(mtl_path, "w", encoding="utf-8") as f:
        f.writelines(new_lines)

    return {
        "mtl_path": mtl_path,
        "backup": backup_path,
        "materials_updated": updated_mats,
    }


# ──────────────────────────────────────────────────────────────────────────────
# Reporte de prompts (para que el agente IA los use)
# ──────────────────────────────────────────────────────────────────────────────

def generate_prompt_report(prompts: List[Dict[str, str]], out_path: str) -> None:
    """
    Guarda un JSON con todos los prompts para que el agente IA pueda procesarlos
    y llamar a generate_image por cada entrada.
    """
    os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(prompts, f, indent=2, ensure_ascii=False)
    print(f"Reporte de prompts guardado en: {out_path}")
