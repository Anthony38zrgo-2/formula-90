"""CLI contract tests: help, exit-code classes, discovery, adapters."""
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest import mock

SUITE_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SUITE_DIR))

from analysis_suite import obj_adapter  # noqa: E402
from analysis_suite import paths  # noqa: E402
from tests.fixtures.glb_builder import build_glb  # noqa: E402

RUN_ANALYSIS = SUITE_DIR / 'scripts' / 'run_analysis.py'


def run_cli(*arguments):
    completed = subprocess.run(
        [sys.executable, str(RUN_ANALYSIS), *arguments],
        capture_output=True, text=True, encoding='utf-8', check=False,
    )
    return completed


def write_glb(path):
    path.write_bytes(build_glb(
        nodes=[{'name': 'Root', 'mesh': 0}],
        meshes=[{'name': 'Mesh', 'primitives': [{
            'points': [[0, 0, 0], [1, 0, 0], [0, 1, 0]],
        }]}],
    ))


class HelpAndDiscoveryTests(unittest.TestCase):
    def test_help_works_without_blender_and_lists_exit_contract(self):
        completed = run_cli('--help')
        self.assertEqual(completed.returncode, 0)
        self.assertIn('Exit contract', completed.stdout)
        for line in ('0  PASS', '1  FAIL', '2  CONFIG', '3  CORRUPT'):
            self.assertIn(line, completed.stdout)

    def test_subcommand_help_lists_common_options(self):
        for command in ('audit-glb', 'probe-geometry', 'validate-obj',
                        'probe-blend', 'vehicle-measure'):
            completed = run_cli(command, '--help')
            self.assertEqual(completed.returncode, 0, command)
            self.assertIn('--config', completed.stdout)
            self.assertIn('--report', completed.stdout)

    def test_repository_root_discovery_from_any_cwd(self):
        original = os.getcwd()
        try:
            os.chdir(tempfile.gettempdir())
            discovered = paths.find_repository_root()
            self.assertEqual(discovered, Path(original))
        finally:
            os.chdir(original)

    def test_repository_root_discovery_from_nested_directory(self):
        root = paths.find_repository_root()
        nested = root / 'blender' / 'analysis_suite' / 'tests'
        self.assertEqual(paths.find_repository_root(nested), root)

    def test_resolve_repo_path_rejects_escape(self):
        root = paths.find_repository_root()
        with self.assertRaises(ValueError):
            paths.resolve_repo_path(root, '..\\..\\outside')

    def test_repo_relative_normalization(self):
        root = paths.find_repository_root()
        absolute = root / 'blender' / 'analysis_suite'
        self.assertEqual(
            paths.repo_relative(root, absolute), 'blender/analysis_suite'
        )
        self.assertIsNone(paths.repo_relative(root, 'C:\\outside'))


class ExitCodeClassTests(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.dir = Path(self._temp.name)

    def _config(self, payload, name='config.json'):
        path = self.dir / name
        path.write_text(json.dumps(payload), encoding='utf-8')
        return path

    def test_exit_zero_pass(self):
        glb = self.dir / 'ok.glb'
        write_glb(glb)
        completed = run_cli('audit-glb', '--config',
                            str(self._config({'source': str(glb)})))
        self.assertEqual(completed.returncode, 0)

    def test_exit_one_validation_failure(self):
        glb = self.dir / 'ok.glb'
        write_glb(glb)
        config = self._config({
            'sources': [{'id': 'default', 'path': str(glb)}],
            'coordinate_convention': {'forward': '-Z', 'right': 'X', 'up': 'Y'},
            'units': 'm',
            'datums': {'front_axle': {'axis': 'Z', 'value': -1.0},
                       'rear_axle': {'axis': 'Z', 'value': 1.0}},
            'sections': {},
            'measurements': [{
                'id': 'wheelbase', 'kind': 'datum_distance',
                'from': 'front_axle', 'to': 'rear_axle',
                'expected': {'min': 0.0, 'max': 0.5},
            }],
        })
        completed = run_cli('vehicle-measure', '--config', str(config))
        self.assertEqual(completed.returncode, 1)

    def test_exit_two_invalid_input(self):
        config = self._config({'source': str(self.dir / 'missing.glb')})
        completed = run_cli('audit-glb', '--config', str(config))
        self.assertEqual(completed.returncode, 2)

    def test_exit_three_corrupt_asset(self):
        glb = self.dir / 'corrupt.glb'
        glb.write_bytes(b'garbage')
        completed = run_cli('audit-glb', '--config',
                            str(self._config({'source': str(glb)})))
        self.assertEqual(completed.returncode, 3)

    def test_missing_required_option_prints_usage_without_traceback(self):
        completed = run_cli('audit-glb')
        self.assertEqual(completed.returncode, 2)
        self.assertNotIn('Traceback', completed.stderr)


class ObjAdapterTests(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.dir = Path(self._temp.name)
        self.obj = self.dir / 'model.obj'
        self.obj.write_text('v 0 0 0\n', encoding='utf-8')
        self.python = sys.executable
        self.logs = self.dir / 'logs'

    def _run_validator(self, returncode, stdout='', stderr=''):
        with mock.patch('subprocess.run') as mocked:
            mocked.return_value = SimpleNamespace(
                returncode=returncode, stdout=stdout, stderr=stderr,
            )
            result = obj_adapter.validate(
                self.obj, self.python, str(self.logs)
            )
            self.assertEqual(mocked.call_count, 1)
            command = mocked.call_args.args[0]
        return result, command

    def test_command_never_selects_a_repair_path(self):
        _, command = self._run_validator(0, 'OK')
        self.assertIn('validate_obj.py', command[1])
        for token in ('fix_obj', '--fix', '--analyze-only',
                      '--decimate', '--ai-textures'):
            self.assertNotIn(token, command)

    def test_command_redirects_logs_to_the_suite_temp_directory(self):
        result, executed = self._run_validator(0, 'OK')
        self.assertEqual(executed[-1], str(self.logs))
        # The documented command scrubs the transient temp directory.
        self.assertEqual(result['command'][-2:], ['--logs-dir', '<temp-log-dir>'])

    def test_validator_pass_classifies_pass(self):
        result, _ = self._run_validator(0, 'APROBADO')
        self.assertEqual(result['classification'], 'pass')
        self.assertTrue(result['validation_passed'])
        self.assertFalse(result['machine_readable'])

    def test_validator_failure_classifies_fail(self):
        result, _ = self._run_validator(1, 'ERROR')
        self.assertEqual(result['classification'], 'fail')
        self.assertFalse(result['validation_passed'])

    def test_missing_dependency_classifies_precondition(self):
        result, _ = self._run_validator(
            1, 'Traceback (most recent call last):\n'
               'ModuleNotFoundError: No module named trimesh'
        )
        self.assertEqual(result['classification'], 'precondition')
        self.assertIn('trimesh', result['reason'])

    def test_unexpected_exit_code_classifies_precondition(self):
        result, _ = self._run_validator(5, 'weird')
        self.assertEqual(result['classification'], 'precondition')

    def test_missing_python_executable_classifies_precondition(self):
        result = obj_adapter.validate(
            self.obj, str(self.dir / 'no_such_python.exe'), str(self.logs)
        )
        self.assertEqual(result['classification'], 'precondition')
        self.assertNotIn(str(self.logs), ' '.join(result['command']),
                         'temp paths must not leak into the documented command')
    def test_output_tail_scrubs_temporary_paths(self):
        result, _ = self._run_validator(
            0, 'Log guardado en: %s\\log.txt' % self.logs
        )
        self.assertTrue(any(
            '<temp-log-dir>' in line for line in result['output_tail']
        ))


class ProbeBlendTests(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.dir = Path(self._temp.name)

    def test_missing_blender_executable_exits_two(self):
        blend = self.dir / 'scene.blend'
        blend.write_bytes(b'BLENDER placeholder')
        config = self.dir / 'config.json'
        config.write_text(json.dumps({
            'blend_file': str(blend),
            'blender_executable': str(self.dir / 'no_blender.exe'),
        }), encoding='utf-8')
        completed = run_cli('probe-blend', '--config', str(config))
        self.assertEqual(completed.returncode, 2)
        report = json.loads(completed.stdout)
        self.assertIn('Blender executable not found',
                      report['findings'][0]['message'])

    def test_missing_blend_file_exits_two(self):
        config = self.dir / 'config.json'
        config.write_text(json.dumps({
            'blend_file': str(self.dir / 'missing.blend'),
            'blender_executable': str(self.dir / 'no_blender.exe'),
        }), encoding='utf-8')
        completed = run_cli('probe-blend', '--config', str(config))
        self.assertEqual(completed.returncode, 2)


if __name__ == '__main__':
    unittest.main()
