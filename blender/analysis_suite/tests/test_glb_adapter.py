"""GLB adapter tests: delegation to the existing inspector and exit codes."""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SUITE_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SUITE_DIR))

from analysis_suite import hashing  # noqa: E402
from tests.fixtures.glb_builder import build_glb  # noqa: E402

RUN_ANALYSIS = SUITE_DIR / 'scripts' / 'run_analysis.py'


def _write_config(temp_dir, payload, name='config.json'):
    path = Path(temp_dir) / name
    path.write_text(json.dumps(payload), encoding='utf-8')
    return path


def _write_glb(path):
    path.write_bytes(build_glb(
        nodes=[{'name': 'Root', 'mesh': 0, 'translation': [1, 2, 3]}],
        meshes=[{'name': 'Mesh', 'primitives': [{
            'points': [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
            'indices': [0, 1, 2],
        }]}],
    ))


def run_cli(*arguments):
    completed = subprocess.run(
        [sys.executable, str(RUN_ANALYSIS), *arguments],
        capture_output=True,
        text=True,
        encoding='utf-8',
        check=False,
    )
    report = json.loads(completed.stdout) if completed.stdout else None
    return completed, report


class GlbAdapterCliTests(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.dir = Path(self._temp.name)

    def _glb(self, name='fixture.glb'):
        path = self.dir / name
        _write_glb(path)
        return path

    def _audit_config(self, glb_path, **extra):
        payload = {'source': str(glb_path)}
        payload.update(extra)
        return _write_config(self.dir, payload)

    def test_audit_glb_delegates_to_existing_inspector(self):
        glb = self._glb()
        config = self._audit_config(glb)
        completed, report = run_cli(
            'audit-glb', '--config', str(config),
        )
        self.assertEqual(completed.returncode, 0)
        self.assertEqual(report['summary']['status'], 'pass')
        evidence = report['evidence']
        # Inspector-owned fields prove delegation instead of recomputation.
        self.assertEqual(evidence['gltf_version'], '2.0')
        self.assertEqual(evidence['generator'], 'analysis-suite test')
        self.assertEqual(evidence['counts']['meshes'], 1)
        self.assertEqual(evidence['total_positions'], 3)
        self.assertEqual(evidence['total_triangles'], 1)
        self.assertEqual(evidence['world_bounds']['min'], [1.0, 2.0, 3.0])
        self.assertEqual(
            report['inputs'][0]['sha256'], hashing.sha256_file(glb)
        )

    def test_audit_glb_writes_identical_report_file(self):
        glb = self._glb()
        config = self._audit_config(glb)
        report_path = Path('blender/generated/analysis_suite/_test_report.json')
        try:
            completed, report = run_cli(
                'audit-glb', '--config', str(config),
                '--report', str(report_path),
            )
            self.assertEqual(completed.returncode, 0)
            written = report_path.read_text(encoding='utf-8').strip()
            self.assertEqual(written, completed.stdout.strip())
            self.assertEqual(json.loads(written), report)
        finally:
            if report_path.exists():
                report_path.unlink()

    def test_malformed_glb_exits_three(self):
        glb = self.dir / 'malformed.glb'
        glb.write_bytes(b'not a GLB at all')
        config = self._audit_config(glb)
        completed, report = run_cli(
            'audit-glb', '--config', str(config),
        )
        self.assertEqual(completed.returncode, 3)
        self.assertEqual(report['summary']['exit_code'], 3)
        self.assertEqual(report['summary']['status'], 'error')
        self.assertIn('CORRUPT', report['findings'][0]['code'])

    def test_missing_source_exits_two(self):
        config = self._audit_config(self.dir / 'missing.glb')
        completed, report = run_cli(
            'audit-glb', '--config', str(config),
        )
        self.assertEqual(completed.returncode, 2)
        self.assertEqual(report['summary']['exit_code'], 2)

    def test_unsupported_extension_exits_two(self):
        obj = self.dir / 'model.obj'
        obj.write_text('', encoding='utf-8')
        config = self._audit_config(obj)
        completed, report = run_cli(
            'audit-glb', '--config', str(config),
        )
        self.assertEqual(completed.returncode, 2)
        self.assertIn('must reference a .glb file',
                      report['findings'][0]['message'])

    def test_invalid_json_config_exits_two_without_traceback(self):
        config = self.dir / 'bad.json'
        config.write_text('{not json', encoding='utf-8')
        completed, report = run_cli(
            'audit-glb', '--config', str(config),
        )
        self.assertEqual(completed.returncode, 2)
        self.assertIn('JSON', report['findings'][0]['message'])
        self.assertNotIn('Traceback', completed.stdout + completed.stderr)

    def test_probe_geometry_corrupt_exits_three(self):
        glb = self.dir / 'corrupt.glb'
        glb.write_bytes(b'glTF\x02\x00\x00\x00garbage')
        config = _write_config(self.dir, {
            'source': str(glb),
            'predicates': [],
        })
        completed, report = run_cli(
            'probe-geometry', '--config', str(config),
        )
        self.assertEqual(completed.returncode, 3)
        self.assertEqual(report['summary']['exit_code'], 3)

    def test_expected_hash_mismatch_is_a_warning_not_a_failure(self):
        glb = self._glb()
        config = self._audit_config(glb, expected_sha256='0' * 64)
        completed, report = run_cli(
            'audit-glb', '--config', str(config),
        )
        self.assertEqual(completed.returncode, 0)
        self.assertFalse(report['inputs'][0]['hash_match'])
        self.assertIn('INPUT-HASH-MISMATCH', [
            finding['code'] for finding in report['findings']
        ])


if __name__ == '__main__':
    unittest.main()
