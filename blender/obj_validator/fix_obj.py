#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from __future__ import annotations

"""
fix_obj.py — Orquestador de correcciones automáticas de modelos OBJ
====================================================================
Uso:
    python fix_obj.py <archivo.obj> [opciones]

Ejemplos:
    python fix_obj.py modelo.obj --all
    python fix_obj.py modelo.obj --fix-scale auto
    python fix_obj.py modelo.obj --fix-scale 10
    python fix_obj.py modelo.obj --fix-geometry
    python fix_obj.py modelo.obj --fix-symmetry wing_front
    python fix_obj.py modelo.obj --fix-wheels
    python fix_obj.py modelo.obj --fix-names
    python fix_obj.py modelo.obj --decimate chassis 0.5
    python fix_obj.py modelo.obj --ai-textures
    python fix_obj.py modelo.obj --all --ai-textures

El modelo corregido se guarda en:
    fixed/<nombre_original>_fixed.obj
"""

import sys
import io
import argparse
import os
import json
import shutil
import textwrap
from datetime import datetime
from typing import List

if hasattr(sys.stdout, 'reconfigure'):
    try:
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    except Exception:
        pass

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from fixes.obj_io       import load_obj, save_obj
from fixes.fix_scale    import fix_scale
from fixes.fix_geometry import remove_degenerate_faces, merge_vertices_by_distance, recalculate_normals, decimate_object
from fixes.fix_symmetry import analyze_symmetry, force_symmetry
from fixes.fix_wheels   import analyze_wheels, fix_wheels
from fixes.fix_names    import fix_names, fix_mtl_names
from fixes.ai_texture   import build_texture_prompts, generate_prompt_report, update_mtl_with_ai_textures


# ──────────────────────────────────────────────────────────────────────────────
# Colores ANSI
# ──────────────────────────────────────────────────────────────────────────────
USE_COLOR = hasattr(sys.stdout, "isatty") and sys.stdout.isatty()

class C:
    RESET  = "\033[0m"  if USE_COLOR else ""
    BOLD   = "\033[1m"  if USE_COLOR else ""
    RED    = "\033[91m" if USE_COLOR else ""
    YELLOW = "\033[93m" if USE_COLOR else ""
    GREEN  = "\033[92m" if USE_COLOR else ""
    CYAN   = "\033[96m" if USE_COLOR else ""
    GREY   = "\033[90m" if USE_COLOR else ""


def section(title: str) -> None:
    print(f"\n{C.CYAN}{C.BOLD}[{title}]{C.RESET}")


def ok(msg: str) -> None:
    print(f"  {C.GREEN}OK{C.RESET}  {msg}")


def warn(msg: str) -> None:
    print(f"  {C.YELLOW}>>>{C.RESET} {msg}")


def err(msg: str) -> None:
    print(f"  {C.RED}ERR{C.RESET} {msg}")


def info(msg: str) -> None:
    print(f"  {C.GREY}{msg}{C.RESET}")


# ──────────────────────────────────────────────────────────────────────────────
# Preparar directorio de salida
# ──────────────────────────────────────────────────────────────────────────────

def prepare_output(obj_path: str, out_dir: str) -> tuple:
    """
    Copia el .obj y su .mtl al directorio de salida.
    Devuelve (out_obj_path, out_mtl_path o None).
    """
    os.makedirs(out_dir, exist_ok=True)
    basename = os.path.basename(obj_path)
    stem     = os.path.splitext(basename)[0]
    out_obj  = os.path.join(out_dir, f"{stem}_fixed.obj")

    # Copiar MTL si existe
    obj_dir = os.path.dirname(os.path.abspath(obj_path))
    out_mtl = None

    # Detectar mtllib del .obj
    import re
    with open(obj_path, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            m = re.match(r"mtllib\s+(.+)", line.strip(), re.IGNORECASE)
            if m:
                mtl_name = m.group(1).strip()
                src_mtl = os.path.join(obj_dir, mtl_name)
                if os.path.isfile(src_mtl):
                    out_mtl = os.path.join(out_dir, mtl_name)
                    shutil.copy2(src_mtl, out_mtl)
                    info(f"MTL copiado a: {out_mtl}")
                break

    return out_obj, out_mtl


# ──────────────────────────────────────────────────────────────────────────────
# Main
# ──────────────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Auto-fixer de modelos OBJ para Godot Easy Vehicle Physics",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=textwrap.dedent("""
        Ejemplos:
          python fix_obj.py modelo.obj --all
          python fix_obj.py modelo.obj --fix-scale auto
          python fix_obj.py modelo.obj --fix-scale 10
          python fix_obj.py modelo.obj --fix-geometry --fix-names --fix-wheels
          python fix_obj.py modelo.obj --fix-symmetry aleron_front
          python fix_obj.py modelo.obj --decimate chassis 0.5
          python fix_obj.py modelo.obj --ai-textures
        """)
    )
    parser.add_argument("obj", help="Ruta al archivo .obj a corregir")
    parser.add_argument("--out-dir",      default=None,  help="Carpeta de salida (default: fixed/)")
    parser.add_argument("--all",          action="store_true", help="Aplicar todos los fixes disponibles")
    parser.add_argument("--fix-scale",    metavar="MODE_OR_FACTOR", default=None,
                        help="'auto', 'auto:z', 'auto:x', o un número (ej: 10.0)")
    parser.add_argument("--fix-geometry", action="store_true", help="Eliminar degeneradas, merge verts, recalcular normales")
    parser.add_argument("--fix-symmetry", metavar="OBJ_NAME", default=None, nargs="?", const="__all__",
                        help="Forzar simetría en eje X. Nombre del objeto o '__all__'")
    parser.add_argument("--fix-wheels",   action="store_true", help="Centrar pivotes y renombrar ruedas a FL/FR/RL/RR")
    parser.add_argument("--fix-names",    action="store_true", help="Sanear nombres de objetos y materiales")
    parser.add_argument("--decimate",     nargs=2, metavar=("OBJ_NAME", "RATIO"),
                        help="Reducir polígonos de un objeto. Ej: --decimate chassis 0.5")
    parser.add_argument("--ai-textures",  action="store_true", help="Generar prompts de textura IA y reporte")
    parser.add_argument("--analyze-only", action="store_true", help="Solo analizar sin modificar")

    args = parser.parse_args()

    obj_path = os.path.abspath(args.obj)
    if not os.path.isfile(obj_path):
        err(f"Archivo no encontrado: {obj_path}")
        sys.exit(1)

    script_dir = os.path.dirname(os.path.abspath(__file__))
    out_dir    = args.out_dir or os.path.join(script_dir, "fixed")
    out_obj, out_mtl = prepare_output(obj_path, out_dir)

    print(f"\n{C.BOLD}{'='*65}{C.RESET}")
    print(f"{C.BOLD}  OBJ Auto-Fixer  |  {os.path.basename(obj_path)}{C.RESET}")
    print(f"{C.GREY}  Salida: {out_obj}{C.RESET}")
    print(f"{C.BOLD}{'='*65}{C.RESET}")

    # ── Cargar ────────────────────────────────────────────────────────────────
    section("CARGA")
    try:
        obj_file = load_obj(obj_path)
        ok(f"{len(obj_file.objects)} objeto(s): {', '.join(obj_file.object_names())}")
        ok(f"{len(obj_file.vertices):,} vertices | {len(obj_file.normals):,} normales | {len(obj_file.uvs):,} UVs")
    except Exception as e:
        err(f"No se pudo cargar el OBJ: {e}")
        sys.exit(1)

    report = {
        "source": obj_path,
        "timestamp": datetime.now().isoformat(),
        "objects": obj_file.object_names(),
        "fixes": {}
    }

    if args.analyze_only:
        section("ANALISIS DE RUEDAS")
        wa = analyze_wheels(obj_file)
        print(json.dumps(wa, indent=2, ensure_ascii=False))

        section("ANALISIS DE SIMETRIA (modelo completo)")
        sa = analyze_symmetry(obj_file)
        print(json.dumps(sa, indent=2, ensure_ascii=False))
        sys.exit(0)

    # ── Fix: Nombres (PRIMERO para que los otros fixes usen nombres limpios) ──
    if args.fix_names or args.all:
        section("FIX NOMBRES")
        result = fix_names(obj_file)
        if result["object_renames"]:
            for old, new in result["object_renames"].items():
                ok(f"'{old}' -> '{new}'")
        else:
            ok("Todos los nombres ya son correctos.")
        if result["material_renames"] and out_mtl:
            mtl_result = fix_mtl_names(out_mtl, result["material_renames"])
            info(f"MTL actualizado: {mtl_result['updated']} material(es) renombrado(s).")
        report["fixes"]["names"] = result

    # ── Fix: Escala ───────────────────────────────────────────────────────────
    if args.fix_scale or args.all:
        section("FIX ESCALA")
        scale_arg = args.fix_scale if args.fix_scale else "auto"
        if scale_arg == "auto" or scale_arg.startswith("auto:"):
            axis = scale_arg.split(":")[1] if ":" in scale_arg else "max"
            result = fix_scale(obj_file, mode="auto", axis=axis)
        else:
            try:
                factor = float(scale_arg)
                result = fix_scale(obj_file, mode="factor", factor=factor)
            except ValueError:
                err(f"Factor de escala inválido: {scale_arg}")
                result = {"error": "factor invalido"}

        if "error" in result:
            err(result["error"])
        else:
            size_b = result["bbox_before"]["size"]
            size_a = result["bbox_after"]["size"]
            ok(f"Factor aplicado: {result['factor_applied']:.4f}x")
            ok(f"  Antes: X={size_b[0]:.3f}m  Y={size_b[1]:.3f}m  Z={size_b[2]:.3f}m")
            ok(f"  Ahora: X={size_a[0]:.3f}m  Y={size_a[1]:.3f}m  Z={size_a[2]:.3f}m")
        report["fixes"]["scale"] = result

    # ── Fix: Geometría ────────────────────────────────────────────────────────
    if args.fix_geometry or args.all:
        section("FIX GEOMETRIA")
        r1 = remove_degenerate_faces(obj_file)
        ok(f"Caras degeneradas eliminadas: {r1['total_degenerate_removed']}")

        r2 = merge_vertices_by_distance(obj_file, threshold=1e-5)
        ok(f"Vertices fusionados: {r2['merged']} (de {r2['original']} -> {r2['final']})")

        r3 = recalculate_normals(obj_file)
        ok(f"Normales recalculadas: {r3['faces_updated']} caras, {r3['new_normal_count']} normales unicas")
        report["fixes"]["geometry"] = {"degenerate": r1, "merge": r2, "normals": r3}

    # ── Fix: Ruedas ───────────────────────────────────────────────────────────
    if args.fix_wheels or args.all:
        section("FIX RUEDAS")
        wa = analyze_wheels(obj_file)
        if "error" in wa:
            warn(wa["error"])
            info("Objetos disponibles: " + ", ".join(obj_file.object_names()))
        else:
            info(f"Ruedas detectadas: {wa['wheels_found']}")
            for w in wa["wheels"]:
                info(f"  '{w['name']}' -> clasificada como {w['classified_as'].upper()} "
                     f"(centroide {w['centroid']})")

        result = fix_wheels(obj_file, center_pivots=True, rename=True)
        if "error" in result:
            warn(result["error"])
        else:
            for d in result["details"]:
                ok(f"'{d['original_name']}' -> '{d.get('renamed_to', d['original_name'])}' "
                   f"| pivot centrado: {d.get('pivot_centered', False)}")
        report["fixes"]["wheels"] = result

    # ── Fix: Simetría ─────────────────────────────────────────────────────────
    if args.fix_symmetry or args.all:
        section("FIX SIMETRIA")
        target_objs = (
            obj_file.object_names() if (args.fix_symmetry == "__all__" or args.all)
            else [args.fix_symmetry]
        )
        sym_results = {}
        for obj_name in target_objs:
            analysis = analyze_symmetry(obj_file, obj_name)
            score = analysis.get("symmetry_score", 0)
            info(f"  '{obj_name}': score={score:.2f} — {analysis.get('recommendation','')}")
            if score < 0.95:
                result = force_symmetry(obj_file, obj_name, strategy="auto")
                if "error" in result:
                    warn(f"  {result['error']}")
                else:
                    ok(f"  '{obj_name}': {result['mirrored_faces_added']} caras espejadas "
                       f"({result['mirrored_new_verts']} nuevos verts)")
                sym_results[obj_name] = result
            else:
                ok(f"  '{obj_name}': ya es simetrico (score={score:.2f})")
        report["fixes"]["symmetry"] = sym_results

    # ── Fix: Decimate ─────────────────────────────────────────────────────────
    if args.decimate:
        section("FIX DECIMATE")
        obj_name = args.decimate[0]
        try:
            ratio = float(args.decimate[1])
        except ValueError:
            err(f"Ratio inválido: {args.decimate[1]}")
            ratio = 0.5
        result = decimate_object(obj_file, obj_name, target_ratio=ratio)
        if "error" in result:
            err(result["error"])
        else:
            ok(f"'{obj_name}': {result['original_faces']} -> {result['final_faces']} caras "
               f"({result['reduction_pct']:.1f}% reduccion)")
        report["fixes"]["decimate"] = result

    # ── IA: Texturas ──────────────────────────────────────────────────────────
    if args.ai_textures:
        section("IA TEXTURAS")
        obj_names = obj_file.object_names()
        prompts = build_texture_prompts(obj_names, vehicle_style="Formula 1 1997 Monaco livery")
        prompts_path = os.path.join(out_dir, "ai_texture_prompts.json")
        generate_prompt_report(prompts, prompts_path)
        ok(f"{len(prompts)} prompts generados para: {', '.join(p['obj_name'] for p in prompts)}")
        info(f"Reporte: {prompts_path}")
        info("Siguiente paso: el agente IA leerá este archivo y generará las imagenes.")
        report["fixes"]["ai_textures"] = {"prompts": len(prompts), "report": prompts_path}

    # ── Guardar ───────────────────────────────────────────────────────────────
    section("GUARDANDO")
    try:
        save_obj(obj_file, out_obj)
        ok(f"OBJ corregido guardado en:\n    {out_obj}")
    except Exception as e:
        err(f"Error al guardar: {e}")
        import traceback; traceback.print_exc()
        sys.exit(1)

    # Guardar reporte JSON
    report_path = os.path.join(out_dir, f"{os.path.splitext(os.path.basename(obj_path))[0]}_fix_report.json")
    with open(report_path, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2, ensure_ascii=False, default=str)
    info(f"Reporte guardado en: {report_path}")

    print(f"\n{C.BOLD}{'='*65}{C.RESET}")
    print(f"{C.GREEN}{C.BOLD}  LISTO. Ejecuta validate_obj.py sobre el resultado para verificar.{C.RESET}")
    print(f"{C.GREY}  python validate_obj.py \"{out_obj}\"{C.RESET}")
    print(f"{C.BOLD}{'='*65}{C.RESET}\n")


if __name__ == "__main__":
    main()
