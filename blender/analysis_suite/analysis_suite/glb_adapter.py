"""GLB adapter delegating structural inspection to vehicle_pipeline/inspect_glb.py.

The existing inspector owns GLB structure evidence; this suite reuses it
instead of recomputing it. Reuse boundary: inspect_glb.py is imported from
the repository, never copied or modified.
"""
import sys

from . import contracts


def load_inspector():
    """Import blender/vehicle_pipeline/inspect_glb.py from the repo root."""
    from . import paths

    root = paths.find_repository_root()
    pipeline = root / 'blender' / 'vehicle_pipeline'
    if not (pipeline / 'inspect_glb.py').exists():
        raise contracts.ConfigError(
            'reuse boundary broken: blender/vehicle_pipeline/inspect_glb.py '
            'not found'
        )
    if str(pipeline) not in sys.path:
        sys.path.insert(0, str(pipeline))
    import inspect_glb  # noqa: PLC0415 - repo-local, must load at call time

    return inspect_glb


def validate_normal_tolerance(value):
    """Config-side validation so bad tolerance is a config error, not corrupt."""
    value = contracts.require_non_negative_number(value, 'normal_tolerance')
    return value


def audit(path, full=False, normal_tolerance=0.01):
    """Return the existing inspector's evidence for a GLB file.

    Raises CorruptAssetError for structural errors; config errors must be
    validated before calling this function.
    """
    inspector = load_inspector()
    try:
        return inspector.inspect(
            str(path),
            full=full,
            normal_tolerance=normal_tolerance,
        )
    except ValueError as exc:
        raise contracts.CorruptAssetError('GLB structural error: %s' % exc) from exc
