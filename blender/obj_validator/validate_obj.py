#!/usr/bin/env python3
# -*- coding: utf-8 -*-
from __future__ import annotations

"""
validate_obj.py - Validador de modelos OBJ para Godot Easy Vehicle Physics
===========================================================================
Uso:
    python validate_obj.py <archivo.obj>
    python validate_obj.py <carpeta_con_obj>   # valida todos los .obj encontrados
    python validate_obj.py --help

El resultado se muestra en consola y se guarda automaticamente en:
    ./logs/<nombre_archivo>_<timestamp>.log

Dependencias:
    pip install trimesh numpy

Estructura de checks:
    checks/check_geometry.py    - Triangulos, degenerados, normales, escala
    checks/check_uvs.py         - Coordenadas UV, tiling, cobertura
    checks/check_materials.py   - MTL, rutas de textura absolutas vs relativas
    checks/check_objects.py     - Sub-mallas, deteccion chasis/ruedas, poligonos
    checks/check_godot_compat.py - Nombres, caracteres especiales, duplicados
"""

import sys
import io

# Forzar stdout a UTF-8 en Windows para evitar UnicodeEncodeError
if hasattr(sys.stdout, 'reconfigure'):
    try:
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    except Exception:
        pass
else:
    try:
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    except Exception:
        pass

import argparse
import glob
import os
import sys
import textwrap
import traceback
from datetime import datetime
from typing import List, Optional

import trimesh

# Aseguramos que el paquete "checks" sea encontrado
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from checks.check_geometry    import check_geometry
from checks.check_uvs         import check_uvs
from checks.check_materials   import check_materials
from checks.check_objects     import check_objects
from checks.check_godot_compat import check_godot_compat


# ──────────────────────────────────────────────────────────────────────────────
# Colores ANSI para consola (desactivados si no es TTY)
# ──────────────────────────────────────────────────────────────────────────────
def _supports_color() -> bool:
    return hasattr(sys.stdout, "isatty") and sys.stdout.isatty()


USE_COLOR = _supports_color()

class C:
    RESET  = "\033[0m"  if USE_COLOR else ""
    BOLD   = "\033[1m"  if USE_COLOR else ""
    RED    = "\033[91m" if USE_COLOR else ""
    YELLOW = "\033[93m" if USE_COLOR else ""
    GREEN  = "\033[92m" if USE_COLOR else ""
    CYAN   = "\033[96m" if USE_COLOR else ""
    GREY   = "\033[90m" if USE_COLOR else ""


# ──────────────────────────────────────────────────────────────────────────────
# Logger
# ──────────────────────────────────────────────────────────────────────────────
class Logger:
    def __init__(self, log_path: str):
        self._lines: List[str] = []
        self._log_path = log_path

    def _strip_ansi(self, text: str) -> str:
        import re
        return re.sub(r'\033\[[0-9;]*m', '', text)

    def print(self, msg: str = ""):
        print(msg)
        self._lines.append(self._strip_ansi(msg))

    def save(self):
        os.makedirs(os.path.dirname(self._log_path), exist_ok=True)
        with open(self._log_path, "w", encoding="utf-8") as f:
            f.write("\n".join(self._lines))
        print(f"\n{C.GREY}Log guardado en: {self._log_path}{C.RESET}")


# ──────────────────────────────────────────────────────────────────────────────
# Core
# ──────────────────────────────────────────────────────────────────────────────

def validate_obj(obj_path: str, log: Logger) -> bool:
    """
    Ejecuta todos los checks sobre un .obj.
    Devuelve True si el modelo pasa todos los checks críticos.
    """
    obj_path = os.path.abspath(obj_path)
    filename = os.path.basename(obj_path)
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")

    SEP1 = "=" * 70
    SEP2 = "-" * 70
    log.print()
    log.print(f"{C.BOLD}{SEP1}{C.RESET}")
    log.print(f"{C.BOLD}  OBJ Validator -- Godot Easy Vehicle Physics{C.RESET}")
    log.print(f"{C.GREY}  Archivo : {obj_path}{C.RESET}")
    log.print(f"{C.GREY}  Fecha   : {timestamp}{C.RESET}")
    log.print(f"{C.BOLD}{SEP1}{C.RESET}")

    overall_passed = True
    all_errors:   List[str] = []
    all_warnings: List[str] = []

    # ── Cargar el mesh con trimesh ────────────────────────────────────────────
    log.print(f"\n{C.CYAN}[CARGA]{C.RESET} Cargando OBJ con trimesh...")
    scene_or_mesh = None
    meshes: dict = {}

    try:
        loaded = trimesh.load(obj_path, process=False, force=None)
        if isinstance(loaded, trimesh.Scene):
            meshes = loaded.geometry  # dict {name: Trimesh}
            log.print(f"  → Scene con {len(meshes)} geometría(s): {list(meshes.keys())}")
        elif isinstance(loaded, trimesh.Trimesh):
            meshes = {"__single__": loaded}
            log.print(f"  → Mesh único: {len(loaded.faces)} caras")
        else:
            log.print(f"{C.YELLOW}  ⚠ Tipo de carga desconocido: {type(loaded)}{C.RESET}")
    except Exception as e:
        err = f"ERROR CARGANDO OBJ con trimesh: {e}"
        log.print(f"{C.RED}  ✗ {err}{C.RESET}")
        all_errors.append(err)
        overall_passed = False

    # ── CHECK 1: Estructura de objetos (parseado propio, sin trimesh) ─────────
    log.print(f"\n{C.CYAN}[CHECK 1/5]{C.RESET} Estructura de objetos y mallas...")
    try:
        obj_result = check_objects(obj_path)
        _print_result(log, obj_result)
        if not obj_result.passed:
            overall_passed = False
        all_errors   += obj_result.errors
        all_warnings += obj_result.warnings
    except Exception as e:
        _print_exception(log, "check_objects", e)
        overall_passed = False

    # ── CHECK 2: Materiales y texturas ────────────────────────────────────────
    log.print(f"\n{C.CYAN}[CHECK 2/5]{C.RESET} Materiales y texturas (.mtl)...")
    try:
        mat_result = check_materials(obj_path)
        _print_result(log, mat_result)
        if not mat_result.passed:
            overall_passed = False
        all_errors   += mat_result.errors
        all_warnings += mat_result.warnings
    except Exception as e:
        _print_exception(log, "check_materials", e)
        overall_passed = False

    # ── CHECK 3: Compatibilidad Godot ─────────────────────────────────────────
    log.print(f"\n{C.CYAN}[CHECK 3/5]{C.RESET} Compatibilidad con Godot 4.x...")
    try:
        godot_result = check_godot_compat(obj_path)
        _print_result(log, godot_result)
        if not godot_result.passed:
            overall_passed = False
        all_errors   += godot_result.errors
        all_warnings += godot_result.warnings
    except Exception as e:
        _print_exception(log, "check_godot_compat", e)
        overall_passed = False

    # ── CHECK 4 & 5: Geometría y UVs por mesh ────────────────────────────────
    if meshes:
        for mesh_name, mesh in meshes.items():
            if not isinstance(mesh, trimesh.Trimesh):
                continue

            log.print(f"\n{C.CYAN}[CHECK 4/5]{C.RESET} Geometría — '{mesh_name}'...")
            try:
                geo_result = check_geometry(mesh, mesh_name)
                _print_result(log, geo_result)
                if not geo_result.passed:
                    overall_passed = False
                all_errors   += geo_result.errors
                all_warnings += geo_result.warnings
            except Exception as e:
                _print_exception(log, f"check_geometry[{mesh_name}]", e)

            log.print(f"\n{C.CYAN}[CHECK 5/5]{C.RESET} Coordenadas UV — '{mesh_name}'...")
            try:
                uv_result = check_uvs(mesh, mesh_name)
                _print_result(log, uv_result)
                if not uv_result.passed:
                    overall_passed = False
                all_errors   += uv_result.errors
                all_warnings += uv_result.warnings
            except Exception as e:
                _print_exception(log, f"check_uvs[{mesh_name}]", e)

    # ── RESUMEN FINAL ─────────────────────────────────────────────────────────
    log.print(f"\n{C.BOLD}{SEP2}{C.RESET}")
    log.print(f"{C.BOLD}  RESUMEN FINAL -- {filename}{C.RESET}")
    log.print(f"{C.BOLD}{SEP2}{C.RESET}")
    log.print(f"  Errores  : {C.RED}{len(all_errors)}{C.RESET}")
    log.print(f"  Avisos   : {C.YELLOW}{len(all_warnings)}{C.RESET}")

    if overall_passed:
        log.print(f"\n  {C.GREEN}[OK] MODELO APROBADO -- Listo para Godot (con posibles avisos){C.RESET}")
    else:
        log.print(f"\n  {C.RED}[FAIL] MODELO NO APROBADO -- Requiere correcciones antes de usar en Godot{C.RESET}")
        log.print(f"\n  {C.RED}Errores criticos:{C.RESET}")
        for err in all_errors:
            for line in textwrap.wrap(err, 66):
                log.print(f"    {C.RED}>> {line}{C.RESET}")

    log.print(f"{C.BOLD}{SEP1}{C.RESET}")
    return overall_passed


def _print_result(log: Logger, result) -> None:
    prefix_ok   = f"  {C.GREEN}✔{C.RESET}"
    prefix_warn = f"  {C.YELLOW}⚠{C.RESET}"
    prefix_err  = f"  {C.RED}✗{C.RESET}"

    for w in result.warnings:
        log.print(f"{prefix_warn} {w}")
    for e in result.errors:
        log.print(f"{prefix_err} {e}")
    if result.passed and not result.errors:
        log.print(f"{prefix_ok} OK")


def _print_exception(log: Logger, check_name: str, exc: Exception) -> None:
    log.print(f"  {C.RED}✗ Excepción en {check_name}: {exc}{C.RESET}")
    log.print(f"{C.GREY}{traceback.format_exc()}{C.RESET}")


# ──────────────────────────────────────────────────────────────────────────────
# CLI
# ──────────────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="Validador de modelos OBJ para Godot Easy Vehicle Physics",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=textwrap.dedent("""
            Ejemplos:
              python validate_obj.py modelo.obj
              python validate_obj.py D:\\Modelos\\coche
              python validate_obj.py .
        """)
    )
    parser.add_argument(
        "target",
        help="Ruta a un .obj específico o a una carpeta con modelos .obj"
    )
    parser.add_argument(
        "--logs-dir",
        default=None,
        help="Carpeta donde guardar los logs. Default: <carpeta_del_script>/logs/"
    )
    args = parser.parse_args()

    target = os.path.abspath(args.target)
    script_dir = os.path.dirname(os.path.abspath(__file__))
    logs_base = args.logs_dir or os.path.join(script_dir, "logs")

    # Recopilar archivos OBJ
    if os.path.isfile(target) and target.lower().endswith(".obj"):
        obj_files = [target]
    elif os.path.isdir(target):
        obj_files = glob.glob(os.path.join(target, "**", "*.obj"), recursive=True)
        obj_files += glob.glob(os.path.join(target, "**", "*.OBJ"), recursive=True)
        obj_files = sorted(set(obj_files))
    else:
        print(f"{C.RED}Error: '{target}' no es un .obj ni una carpeta.{C.RESET}")
        sys.exit(1)

    if not obj_files:
        print(f"{C.YELLOW}No se encontraron archivos .obj en: {target}{C.RESET}")
        sys.exit(0)

    print(f"{C.CYAN}Validando {len(obj_files)} archivo(s)...{C.RESET}")

    results = []
    for obj_path in obj_files:
        ts = datetime.now().strftime("%Y%m%d_%H%M%S")
        stem = os.path.splitext(os.path.basename(obj_path))[0]
        log_path = os.path.join(logs_base, f"{stem}_{ts}.log")

        log = Logger(log_path)
        passed = validate_obj(obj_path, log)
        log.save()
        results.append((obj_path, passed))

    # Resumen multi-archivo
    if len(results) > 1:
        print(f"\n{C.BOLD}{'═'*70}{C.RESET}")
        print(f"{C.BOLD}  RESUMEN GLOBAL{C.RESET}")
        print(f"{C.BOLD}{'═'*70}{C.RESET}")
        for path, passed in results:
            status = f"{C.GREEN}✔ APROBADO{C.RESET}" if passed else f"{C.RED}✗ FALLÓ{C.RESET}"
            print(f"  {status}  {os.path.basename(path)}")

    total_failed = sum(1 for _, p in results if not p)
    sys.exit(0 if total_failed == 0 else 1)


if __name__ == "__main__":
    main()
