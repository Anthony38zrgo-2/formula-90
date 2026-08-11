"""Deterministic report serialization and report envelope helpers.

Every CLI command emits one JSON object to stdout with sorted keys, stable
finding order and no timestamps, random identifiers or machine-specific
absolute paths in the payload.
"""
import json
import sys

from . import __version__
from .contracts import SUITE_NAME


def metadata():
    return {'python': '%d.%d.%d' % sys.version_info[:3]}


def serialize(report):
    return json.dumps(
        report,
        sort_keys=True,
        separators=(',', ':'),
        ensure_ascii=False,
        allow_nan=False,
    )


def emit(report):
    print(serialize(report))


def write_report(report, path):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(serialize(report) + '\n', encoding='utf-8')


def make_finding(level, code, message):
    return {'level': level, 'code': code, 'message': message}


def build_envelope(command, inputs, findings, evidence, status, exit_code, reason):
    return {
        'suite': {'name': SUITE_NAME, 'version': __version__},
        'command': command,
        'metadata': metadata(),
        'inputs': inputs,
        'findings': sorted(
            findings,
            key=lambda f: (f['code'], f['level'], f['message']),
        ),
        'evidence': evidence,
        'summary': {'status': status, 'exit_code': exit_code},
        'exit_code_explanation': '%d: %s' % (exit_code, reason),
    }
