"""Vehicle policy tests: pass/fail/not_measurable semantics and exit codes."""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SUITE_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SUITE_DIR))

from analysis_suite import hashing  # noqa: E402
from analysis_suite import paths  # noqa: E402
from analysis_suite.policies import vehicle  # noqa: E402
from tests.fixtures.glb_builder import build_glb  # noqa: E402

RUN_ANALYSIS = SUITE_DIR / 'scripts' / 'run_analysis.py'

FIXTURE_MESH = {
    'points': [[-0.5, 0.0, -2.0], [0.5, 0.0, 2.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
    'indices': [0, 1, 2, 1, 2, 3],
}

BASE_DATUMS = {
    'front_axle': {'axis': 'Z', 'value': -1.0},
    'rear_axle': {'axis': 'Z', 'value': 1.0},
    'wheel_centre': {'point': [0.0, 0.0, 0.0]},
    'far_nose': {'axis': 'Z', 'value': -10.0},
}

BASE_SECTIONS = {
    'cockpit': {'axis': 'Z', 'min': -1.0, 'max': 1.0},
    'front_zone': {'axis': 'Z', 'max': 1.0},
    'rear_zone': {'axis': 'Z', 'min': 1.0},
    'nose': {'axis': 'Z', 'max': -5.0},
}


def build_config(measurements, **overrides):
    config = {
        'sources': [{'id': 'default', 'path': 'fixture.glb'}],
        'coordinate_convention': {'forward': '-Z', 'right': 'X', 'up': 'Y'},
        'units': 'm',
        'datums': BASE_DATUMS,
        'sections': BASE_SECTIONS,
        'measurements': measurements,
    }
    config.update(overrides)
    return config


def write_fixture(path):
    path.write_bytes(build_glb(
        nodes=[{'name': 'Root', 'mesh': 0}],
        meshes=[{'name': 'FixtureMesh', 'primitives': [FIXTURE_MESH]}],
    ))


def model_for(glb_path, measurements):
    config = build_config(measurements)
    config['sources'] = [{'id': 'default', 'path': str(glb_path)}]
    return vehicle.validate_config(config)


def measure(glb_path, model):
    vertices_by_source = {
        source['id']: vehicle.load_vertices(glb_path)
        for source in model['sources']
    }
    return vehicle.measure(model, vertices_by_source)


class VehiclePolicyTests(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.dir = Path(self._temp.name)
        self.glb = self.dir / 'fixture.glb'
        write_fixture(self.glb)

    def test_pass_when_within_expectations(self):
        model = model_for(self.glb, [
            {'id': 'wheelbase', 'kind': 'datum_distance',
             'from': 'front_axle', 'to': 'rear_axle',
             'expected': {'min': 1.9, 'max': 2.1}},
            {'id': 'height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'cockpit', 'expected': {'min': 0.9, 'max': 1.1}},
            {'id': 'front_width', 'kind': 'max_abs_extent', 'axis': 'right',
             'section': 'front_zone', 'expected': {'max': 0.6}},
            {'id': 'overhang', 'kind': 'forward_extent_beyond_datum',
             'datum': 'front_axle', 'expected': {'min': 0.5, 'max': 1.5}},
        ])
        result = measure(self.glb, model)
        self.assertEqual(result['summary']['status'], 'pass')
        for entry in result['measurements']:
            self.assertEqual(entry['status'], 'pass')
        self.assertEqual(
            next(m for m in result['measurements'] if m['id'] == 'wheelbase')
            ['measured'], 2.0,
        )

    def test_fail_when_outside_expectations(self):
        model = model_for(self.glb, [
            {'id': 'height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'cockpit', 'expected': {'max': 0.9}},
        ])
        result = measure(self.glb, model)
        self.assertEqual(result['summary']['status'], 'fail')
        self.assertEqual(result['measurements'][0]['status'], 'fail')
        self.assertEqual(result['measurements'][0]['measured'], 1.0)

    def test_not_measurable_when_section_has_no_vertices(self):
        model = model_for(self.glb, [
            {'id': 'nose_height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'nose', 'expected': {'max': 1.0}},
        ])
        result = measure(self.glb, model)
        self.assertEqual(result['summary']['status'], 'not_measurable')
        entry = result['measurements'][0]
        self.assertEqual(entry['status'], 'not_measurable')
        self.assertIn('no finite vertices', entry['reason'])

    def test_not_measurable_when_datum_is_a_point(self):
        model = model_for(self.glb, [
            {'id': 'distance', 'kind': 'datum_distance',
             'from': 'wheel_centre', 'to': 'rear_axle',
             'expected': {'min': 0.0, 'max': 5.0}},
        ])
        result = measure(self.glb, model)
        self.assertEqual(result['measurements'][0]['status'], 'not_measurable')
        self.assertIn('plane datums', result['measurements'][0]['reason'])

    def test_not_measurable_when_no_vertices_beyond_datum(self):
        model = model_for(self.glb, [
            {'id': 'far_overhang', 'kind': 'forward_extent_beyond_datum',
             'datum': 'far_nose', 'expected': {'min': 0.0, 'max': 1.0}},
        ])
        result = measure(self.glb, model)
        self.assertEqual(result['measurements'][0]['status'], 'not_measurable')
        self.assertIn('no vertices beyond', result['measurements'][0]['reason'])

    def test_not_measurable_when_datum_axis_mismatches_forward(self):
        model = model_for(self.glb, [
            {'id': 'overhang', 'kind': 'forward_extent_beyond_datum',
             'datum': 'wheel_centre', 'expected': {'min': 0.0, 'max': 1.0}},
        ])
        # wheel_centre is a point datum: kind check happens first.
        result = measure(self.glb, model)
        self.assertEqual(result['measurements'][0]['status'], 'not_measurable')

    def test_fail_takes_precedence_over_not_measurable(self):
        model = model_for(self.glb, [
            {'id': 'height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'cockpit', 'expected': {'max': 0.9}},
            {'id': 'nose_height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'nose', 'expected': {'max': 1.0}},
        ])
        result = measure(self.glb, model)
        self.assertEqual(result['summary']['status'], 'fail')

    def test_hash_mismatch_marks_all_measurements_not_measurable(self):
        config = build_config([
            {'id': 'wheelbase', 'kind': 'datum_distance',
             'from': 'front_axle', 'to': 'rear_axle',
             'expected': {'min': 1.9, 'max': 2.1}},
        ], sources=[{
            'id': 'default', 'path': str(self.glb),
            'expected_sha256': '0' * 64,
        }])
        model = vehicle.validate_config(config)
        root = paths.find_repository_root()
        resolved = {'default': self.glb}
        source_evidence, broken, findings = vehicle.hash_evidence(model, resolved)
        self.assertEqual(broken, ['default'])
        self.assertFalse(source_evidence[0]['hash_match'])
        result = vehicle.measure(model, {'default': ([], 0)}, broken)
        self.assertEqual(result['summary']['status'], 'not_measurable')
        self.assertEqual(
            result['measurements'][0]['reason'],
            'source precondition not met (hash mismatch)',
        )

    def test_config_rejects_unknown_datum_and_kind(self):
        from analysis_suite.contracts import ConfigError

        with self.assertRaises(ConfigError):
            vehicle.validate_config(build_config([
                {'id': 'x', 'kind': 'datum_distance', 'from': 'ghost',
                 'to': 'rear_axle', 'expected': {'max': 1.0}},
            ]))
        with self.assertRaises(ConfigError):
            vehicle.validate_config(build_config([
                {'id': 'x', 'kind': 'not_a_kind', 'expected': {'max': 1.0}},
            ]))


class VehiclePolicyCliTests(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.dir = Path(self._temp.name)
        self.glb = self.dir / 'fixture.glb'
        write_fixture(self.glb)

    def _run(self, config, command='vehicle-measure'):
        config_path = self.dir / 'config.json'
        config_path.write_text(json.dumps(config), encoding='utf-8')
        completed = subprocess.run(
            [sys.executable, str(RUN_ANALYSIS), command,
             '--config', str(config_path)],
            capture_output=True, text=True, encoding='utf-8', check=False,
        )
        return completed, json.loads(completed.stdout)

    def _config(self, measurements, **overrides):
        config = build_config(measurements, **overrides)
        if 'sources' not in overrides:
            config['sources'] = [{'id': 'default', 'path': str(self.glb)}]
        return config

    def test_cli_pass_exit_zero(self):
        completed, report = self._run(self._config([
            {'id': 'wheelbase', 'kind': 'datum_distance',
             'from': 'front_axle', 'to': 'rear_axle',
             'expected': {'min': 1.9, 'max': 2.1}},
        ]))
        self.assertEqual(completed.returncode, 0)
        self.assertEqual(report['summary']['status'], 'pass')

    def test_cli_fail_exit_one(self):
        completed, report = self._run(self._config([
            {'id': 'height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'cockpit', 'expected': {'max': 0.9}},
        ]))
        self.assertEqual(completed.returncode, 1)
        self.assertEqual(report['summary']['status'], 'fail')
        self.assertEqual(report['exit_code_explanation'][:1], '1')

    def test_cli_not_measurable_exit_two(self):
        completed, report = self._run(self._config([
            {'id': 'nose_height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'nose', 'expected': {'max': 1.0}},
        ]))
        self.assertEqual(completed.returncode, 2)
        self.assertEqual(report['summary']['status'], 'not_measurable')

    def test_cli_hash_mismatch_exit_two(self):
        completed, report = self._run(self._config(
            [{'id': 'wheelbase', 'kind': 'datum_distance',
              'from': 'front_axle', 'to': 'rear_axle',
              'expected': {'min': 1.9, 'max': 2.1}}],
            sources=[{'id': 'default', 'path': str(self.glb),
                      'expected_sha256': '0' * 64}],
        ))
        self.assertEqual(completed.returncode, 2)
        self.assertIn('VEHICLE-HASH-MISMATCH', [
            f['code'] for f in report['findings']
        ])
        self.assertTrue(all(
            m['status'] == 'not_measurable'
            for m in report['evidence']['measurements']
        ))

    def test_cli_fail_and_not_measurable_prefer_fail(self):
        completed, report = self._run(self._config([
            {'id': 'height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'cockpit', 'expected': {'max': 0.9}},
            {'id': 'nose_height', 'kind': 'max_extent', 'axis': 'up',
             'section': 'nose', 'expected': {'max': 1.0}},
        ]))
        self.assertEqual(completed.returncode, 1)
        self.assertEqual(report['summary']['status'], 'fail')

    def test_cli_source_sha256_is_reported(self):
        completed, report = self._run(self._config([
            {'id': 'wheelbase', 'kind': 'datum_distance',
             'from': 'front_axle', 'to': 'rear_axle',
             'expected': {'min': 1.9, 'max': 2.1}},
        ]))
        self.assertEqual(
            report['evidence']['sources'][0]['sha256'],
            hashing.sha256_file(self.glb),
        )


if __name__ == '__main__':
    unittest.main()
