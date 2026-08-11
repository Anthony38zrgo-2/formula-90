"""Read-only adapter to blender/obj_validator/validate_obj.py.

The existing validator owns OBJ/MTL validation behavior. This suite never
parses OBJ itself and never invokes any repair path (fix_obj.py). The
validator's human-readable output is classified, not machine-parsed; its
timestamped log files are redirected to a temporary directory so the
repository is never written to.
"""
import re
import subprocess

from . import contracts

ANSI_RE = re.compile(r'\x1b\[[0-9;]*m')
TEMP_LOG_PLACEHOLDER = '<temp-log-dir>'
OUTPUT_TAIL_LINES = 40
REPAIR_TOKENS = ('fix_obj', '--fix', '--analyze-only', '--decimate', '--ai-textures')


def locate_validator(root):
    path = root / 'blender' / 'obj_validator' / 'validate_obj.py'
    return path if path.exists() else None


def classify_output(completed):
    """Strip ANSI escapes and detect a broken validator (missing dependency)."""
    output = ANSI_RE.sub(
        '', (completed.stdout or '') + '\n' + (completed.stderr or '')
    )
    import_error = any(token in output for token in (
        'ImportError',
        'ModuleNotFoundError',
        'No module named',
        'Traceback (most recent call last)',
    ))
    return output, import_error


def validate(source, python_executable, logs_dir, timeout=120):
    """Run the validator read-only and classify its deterministic result.

    Returns a dict with a ``classification`` of 'pass', 'fail' or
    'precondition'. The documented command is normalized to repository-relative
    paths (the transient temp log directory becomes a placeholder) so the
    payload never leaks machine-specific absolute paths. Raises ConfigError for
    missing prerequisites only.
    """
    from . import paths

    root = paths.find_repository_root()
    validator = locate_validator(root)
    if validator is None:
        raise contracts.ConfigError(
            'reuse boundary broken: blender/obj_validator/validate_obj.py '
            'not found'
        )
    executed_command = [
        python_executable,
        str(validator),
        str(source),
        '--logs-dir',
        str(logs_dir),
    ]
    if any(token in executed_command for token in REPAIR_TOKENS):
        raise AssertionError('repair command selected; this must never happen')
    documented_command = [
        python_executable,
        paths.repo_relative(root, validator) or str(validator),
        paths.repo_relative(root, source) or str(source),
        '--logs-dir',
        TEMP_LOG_PLACEHOLDER,
    ]
    try:
        completed = subprocess.run(
            executed_command,
            capture_output=True,
            text=True,
            encoding='utf-8',
            errors='replace',
            timeout=timeout,
            check=False,
        )
    except FileNotFoundError:
        return {
            'command': documented_command,
            'classification': 'precondition',
            'reason': 'python executable not found: %s' % python_executable,
            'machine_readable': False,
            'validation_passed': None,
            'validator_exit_code': None,
            'output_tail': [],
            'gap': ('validator emits human-readable text and timestamped log '
                    'files only; output is classified, not machine-parsed'),
        }
    output, import_error = classify_output(completed)
    tail = [line for line in output.splitlines() if line.strip()][-OUTPUT_TAIL_LINES:]
    for absolute, placeholder in (
        (str(logs_dir), TEMP_LOG_PLACEHOLDER),
        (str(root), '<repo-root>'),
    ):
        tail = [line.replace(absolute, placeholder) for line in tail]
    if completed.returncode == 0:
        classification = 'pass'
        passed = True
        reason = 'validator reported no critical errors'
    elif import_error:
        classification = 'precondition'
        passed = None
        reason = 'validator cannot start (missing dependency such as trimesh)'
    elif completed.returncode == 1:
        classification = 'fail'
        passed = False
        reason = 'validator reported critical errors'
    else:
        classification = 'precondition'
        passed = None
        reason = 'validator exited unexpectedly (%d)' % completed.returncode
    return {
        'command': documented_command,
        'classification': classification,
        'reason': reason,
        'machine_readable': False,
        'validation_passed': passed,
        'validator_exit_code': completed.returncode,
        'output_tail': tail,
        'gap': ('validator emits human-readable text and timestamped log files '
                'only; output is classified, not machine-parsed'),
    }
