"""Blender headless adapter for the probe-blend command.

Runs Blender in background mode with the thin bpy implementation
(scripts/blender_scene_probe.py) and merges its JSON result. Never renders,
imports external assets, modifies the scene or saves the file.
"""
import json
import os
import subprocess

from . import contracts

SENTINEL = '__FORMULA90_ANALYSIS_JSON__'
DEFAULT_BLENDER_EXE = r'C:\Program Files\Blender Foundation\Blender 5.2\blender.exe'
DEFAULT_TIMEOUT = 300


def resolve_blender_executable(config):
    blender_exe = config.get('blender_executable', DEFAULT_BLENDER_EXE)
    blender_exe = contracts.require_str(blender_exe, 'blender_executable')
    if not os.path.isfile(blender_exe):
        raise contracts.ConfigError(
            'Blender executable not found: %s' % blender_exe
        )
    return blender_exe


def run_probe(config, root, config_path, timeout=DEFAULT_TIMEOUT):
    """Open a .blend headless, gather per-object facts, return its JSON result.

    Result is a dict with 'ok' and either 'objects' (list of object facts) or
    'class' ('precondition' | 'corrupt') plus 'error'.
    """
    blender_exe = resolve_blender_executable(config)
    script = root / 'blender' / 'analysis_suite' / 'scripts' / 'blender_scene_probe.py'
    if not script.exists():
        raise contracts.ConfigError(
            'blender/analysis_suite/scripts/blender_scene_probe.py not found'
        )
    command = [
        blender_exe,
        '--background',
        '--python',
        str(script),
        '--',
        '--root',
        str(root),
        '--config',
        str(config_path),
    ]
    try:
        completed = subprocess.run(
            command,
            capture_output=True,
            text=True,
            encoding='utf-8',
            errors='replace',
            timeout=timeout,
            check=False,
        )
    except FileNotFoundError:
        raise contracts.ConfigError(
            'Blender executable not found: %s' % blender_exe
        ) from None
    sentinel = None
    for line in (completed.stdout or '').splitlines():
        if line.startswith(SENTINEL):
            sentinel = line[len(SENTINEL):]
            break
    if sentinel is None:
        return {
            'ok': False,
            'class': 'precondition',
            'error': 'blender_scene_probe.py produced no JSON result '
                     '(exit %d)' % completed.returncode,
        }
    try:
        result = json.loads(sentinel)
    except ValueError as exc:
        return {
            'ok': False,
            'class': 'precondition',
            'error': 'malformed JSON from blender_scene_probe.py: %s' % exc,
        }
    if not isinstance(result, dict):
        return {
            'ok': False,
            'class': 'precondition',
            'error': 'blender_scene_probe.py returned a non-object result',
        }
    return result
