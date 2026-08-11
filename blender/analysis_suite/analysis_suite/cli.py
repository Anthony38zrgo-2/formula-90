"""Stable CLI entry point for the analysis suite (ANL-001).

Subcommands:
    audit-glb       structural GLB inspection via the existing vehicle inspector
    probe-geometry  transform-aware per-vertex predicate evidence for GLB geometry
    validate-obj    read-only adapter to blender/obj_validator
    probe-blend     headless Blender scene/object mesh and transform facts
    vehicle-measure Formula90s vehicle-specific, config-driven measurements

Every command emits one deterministic JSON object to stdout, optionally writes
an identical report to --report, and never raises a traceback.
"""
import argparse
import json
import os
import sys
import tempfile

from . import contracts
from . import glb_adapter
from . import hashing
from . import obj_adapter
from . import paths
from . import predicates
from . import reporting
from .blender_headless import run_probe
from .contracts import (
    EXIT_CONFIG,
    EXIT_CORRUPT,
    EXIT_FAIL,
    EXIT_PASS,
    ConfigError,
    CorruptAssetError,
)
from .policies import vehicle as vehicle_policy

DEFAULT_BLENDER_PYTHON = (
    r'C:\Program Files\Blender Foundation\Blender 5.2\5.2\python\bin\python.exe'
)

COMMANDS = (
    ('audit-glb', 'structural GLB inspection via the existing vehicle inspector'),
    ('probe-geometry', 'transform-aware per-vertex predicate evidence for GLB geometry'),
    ('validate-obj', 'read-only adapter to blender/obj_validator'),
    ('probe-blend', 'headless Blender scene/object mesh and transform facts'),
    ('vehicle-measure', 'Formula90s vehicle-specific, config-driven measurements'),
)

EXIT_CONTRACT = (
    'Exit contract:\n'
    '  0  PASS    command completed; all measured validations passed\n'
    '  1  FAIL    deterministic validation failure\n'
    '  2  CONFIG  invalid configuration, precondition or input\n'
    '  3  CORRUPT corrupt or unsupported asset'
)


def build_parser():
    parser = argparse.ArgumentParser(
        prog='run_analysis.py',
        description='Deterministic headless 3D analysis suite for Formula90s.',
        epilog=EXIT_CONTRACT,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    subparsers = parser.add_subparsers(
        dest='command', required=True, metavar='<subcommand>'
    )
    for name, help_text in COMMANDS:
        sub = subparsers.add_parser(name, help=help_text)
        sub.add_argument(
            '--config',
            required=True,
            help='repository-relative JSON configuration',
        )
        sub.add_argument(
            '--report',
            default=None,
            help='optional repository-relative JSON report path',
        )
    return parser


# ---------------------------------------------------------------------------
# Config and evidence helpers
# ---------------------------------------------------------------------------

def _load_config(args, root):
    config_path = paths.resolve_input(root, args.config, label='config path')
    try:
        with open(config_path, encoding='utf-8') as stream:
            config = json.load(stream)
    except ValueError as exc:
        raise ConfigError('invalid JSON config %s: %s' % (args.config, exc)) from exc
    if not isinstance(config, dict):
        raise ConfigError('config root must be a JSON object')
    return config, config_path


def _input_evidence(root, path, expected_sha256=None):
    digest = hashing.sha256_file(path)
    entry = {
        'path': paths.repo_relative(root, path),
        'sha256': digest,
        'size_bytes': os.path.getsize(path),
    }
    if expected_sha256 is not None:
        expected_sha256 = contracts.require_str(expected_sha256, 'expected_sha256')
        entry['expected_sha256'] = expected_sha256
        entry['hash_match'] = expected_sha256.upper() == digest
    return entry


def _expected_from(config):
    expected = config.get('expected_sha256')
    if expected is not None:
        expected = contracts.require_str(expected, 'expected_sha256')
    return expected


def _write_report(root, report_value, envelope):
    if os.path.isabs(report_value):
        raise ConfigError('--report must be a repository-relative path')
    report_path = paths.resolve_repo_path(root, report_value)
    reporting.write_report(envelope, report_path)


# ---------------------------------------------------------------------------
# Subcommands
# ---------------------------------------------------------------------------

def _cmd_audit_glb(args, root):
    config, _ = _load_config(args, root)
    source = paths.resolve_input(root, config.get('source'), '.glb', 'source')
    full = bool(config.get('full', False))
    tolerance = glb_adapter.validate_normal_tolerance(
        config.get('normal_tolerance', 0.01)
    )
    expected = _expected_from(config)
    evidence = dict(glb_adapter.audit(source, full=full, normal_tolerance=tolerance))
    evidence['file'] = paths.repo_relative(root, source)
    inputs = [_input_evidence(root, source, expected)]
    findings = [reporting.make_finding(
        'info', 'GLB-AUDIT-OK', 'existing inspector completed the structural audit'
    )]
    if expected is not None and not inputs[0]['hash_match']:
        findings.append(reporting.make_finding(
            'warning', 'INPUT-HASH-MISMATCH',
            'source does not match expected SHA-256 (%s)' % inputs[0]['sha256'],
        ))
    return reporting.build_envelope(
        'audit-glb', inputs, findings, evidence,
        contracts.STATUS_PASS, EXIT_PASS, 'structural inspection completed',
    ), EXIT_PASS


def _cmd_probe_geometry(args, root):
    config, _ = _load_config(args, root)
    source = paths.resolve_input(root, config.get('source'), '.glb', 'source')
    sections = predicates.validate_probe_config(config)
    expected = _expected_from(config)
    try:
        evidence = predicates.probe(source, config, sections)
    except ValueError as exc:
        raise CorruptAssetError('GLB structural error: %s' % exc) from exc
    inputs = [_input_evidence(root, source, expected)]
    findings = [reporting.make_finding(
        'info', 'PROBE-GEOMETRY-OK', 'predicate evidence gathered'
    )]
    if expected is not None and not inputs[0]['hash_match']:
        findings.append(reporting.make_finding(
            'warning', 'INPUT-HASH-MISMATCH',
            'source does not match expected SHA-256 (%s)' % inputs[0]['sha256'],
        ))
    return reporting.build_envelope(
        'probe-geometry', inputs, findings, evidence,
        contracts.STATUS_PASS, EXIT_PASS, 'geometry probe completed',
    ), EXIT_PASS


def _cmd_validate_obj(args, root):
    config, _ = _load_config(args, root)
    source = paths.resolve_input(root, config.get('source'), '.obj', 'source')
    python_executable = config.get(
        'python_executable', DEFAULT_BLENDER_PYTHON
    )
    python_executable = contracts.require_str(
        python_executable, 'python_executable'
    )
    if not os.path.isfile(python_executable):
        raise ConfigError('python executable not found: %s' % python_executable)
    with tempfile.TemporaryDirectory(prefix='analysis_suite_obj_') as temp_dir:
        result = obj_adapter.validate(source, python_executable, temp_dir)
    inputs = [_input_evidence(root, source)]
    classification = result['classification']
    if classification == 'pass':
        status, exit_code = contracts.STATUS_PASS, EXIT_PASS
        code, message = 'OBJ-VALIDATE-OK', result['reason']
    elif classification == 'fail':
        status, exit_code = contracts.STATUS_FAIL, EXIT_FAIL
        code, message = 'OBJ-VALIDATE-FAIL', result['reason']
    else:
        status, exit_code = contracts.STATUS_ERROR, EXIT_CONFIG
        code, message = 'OBJ-VALIDATE-PRECONDITION', result['reason']
    findings = [reporting.make_finding(
        'info' if classification == 'pass' else 'warning', code, message
    )]
    return reporting.build_envelope(
        'validate-obj', inputs, findings, result,
        status, exit_code, message,
    ), exit_code


def _cmd_probe_blend(args, root):
    config, config_path = _load_config(args, root)
    source = paths.resolve_input(root, config.get('blend_file'), '.blend', 'blend_file')
    result = run_probe(config, root, config_path)
    inputs = [_input_evidence(root, source)]
    if result.get('ok'):
        status, exit_code = contracts.STATUS_PASS, EXIT_PASS
        findings = [reporting.make_finding(
            'info', 'PROBE-BLEND-OK', 'Blender headless probe completed'
        )]
        message = 'scene probe completed'
    else:
        message = result.get('error', 'Blender headless probe failed')
        if result.get('class') == 'corrupt':
            status, exit_code = contracts.STATUS_ERROR, EXIT_CORRUPT
            findings = [reporting.make_finding(
                'error', 'PROBE-BLEND-CORRUPT', message
            )]
        else:
            status, exit_code = contracts.STATUS_ERROR, EXIT_CONFIG
            findings = [reporting.make_finding(
                'warning', 'PROBE-BLEND-PRECONDITION', message
            )]
    return reporting.build_envelope(
        'probe-blend', inputs, findings, result,
        status, exit_code, message,
    ), exit_code


def _cmd_vehicle_measure(args, root):
    config, _ = _load_config(args, root)
    model = vehicle_policy.validate_config(config)
    resolved_paths = {}
    for source in model['sources']:
        resolved_paths[source['id']] = paths.resolve_input(
            root, source['path'], '.glb', 'source path %s' % source['id']
        )
    source_evidence, broken_ids, findings = vehicle_policy.hash_evidence(
        model, resolved_paths
    )
    vertices_by_source = {}
    for source in model['sources']:
        vertices_by_source[source['id']] = vehicle_policy.load_vertices(
            resolved_paths[source['id']]
        )
    result = vehicle_policy.measure(model, vertices_by_source, broken_ids)
    evidence = {
        'sources': source_evidence,
        'measurements': result['measurements'],
        'summary': result['summary'],
    }
    status = result['summary']['status']
    if status == contracts.STATUS_FAIL:
        exit_code = EXIT_FAIL
    elif status == contracts.STATUS_NOT_MEASURABLE:
        exit_code = EXIT_CONFIG
    else:
        exit_code = EXIT_PASS
    if not findings:
        findings.append(reporting.make_finding(
            'info', 'VEHICLE-MEASURE-OK', 'all measurements evaluated'
        ))
    message = result['summary']['reason']
    return reporting.build_envelope(
        'vehicle-measure', [], findings, evidence,
        status, exit_code, message,
    ), exit_code


DISPATCH = {
    'audit-glb': _cmd_audit_glb,
    'probe-geometry': _cmd_probe_geometry,
    'validate-obj': _cmd_validate_obj,
    'probe-blend': _cmd_probe_blend,
    'vehicle-measure': _cmd_vehicle_measure,
}


def _error_envelope(command, exit_code, message):
    if exit_code == EXIT_CONFIG:
        code = 'INVALID-CONFIG-OR-PRECONDITION'
    else:
        code = 'CORRUPT-OR-UNSUPPORTED-ASSET'
    envelope = reporting.build_envelope(
        command,
        [],
        [reporting.make_finding('error', code, message)],
        None,
        contracts.STATUS_ERROR,
        exit_code,
        message,
    )
    return envelope, exit_code


def main(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        root = paths.find_repository_root()
        envelope, exit_code = DISPATCH[args.command](args, root)
    except ConfigError as exc:
        envelope, exit_code = _error_envelope(
            args.command, EXIT_CONFIG, str(exc)
        )
    except CorruptAssetError as exc:
        envelope, exit_code = _error_envelope(
            args.command, EXIT_CORRUPT, str(exc)
        )
    except Exception as exc:  # never traceback; report deterministically
        envelope, exit_code = _error_envelope(
            args.command, EXIT_CONFIG,
            'unexpected error: %s: %s' % (type(exc).__name__, exc),
        )
    if args.report is not None:
        try:
            _write_report(root, args.report, envelope)
        except (ConfigError, RuntimeError) as exc:
            envelope, exit_code = _error_envelope(
                args.command, EXIT_CONFIG, str(exc)
            )
    reporting.emit(envelope)
    return exit_code
