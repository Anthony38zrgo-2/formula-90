"""Transform-aware predicate evidence tests (pure Python, no bpy)."""
import math
import sys
import tempfile
import unittest
from pathlib import Path

SUITE_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SUITE_DIR))

from analysis_suite import predicates  # noqa: E402
from tests.fixtures.glb_builder import build_glb  # noqa: E402


def _write_fixture(path, nodes, meshes):
    path.write_bytes(build_glb(nodes, meshes))


def _config(**overrides):
    config = {
        'source': 'ignored.glb',
        'sample_size': 10,
        'predicates': [],
        'sections': {},
    }
    config.update(overrides)
    return config


def _probe(path, config):
    sections = predicates.validate_probe_config(config)
    return predicates.probe(path, config, sections)


class PredicateTests(unittest.TestCase):
    def setUp(self):
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.dir = Path(self._temp.name)

    def _write(self, nodes, meshes, name='fixture.glb'):
        path = self.dir / name
        _write_fixture(path, nodes, meshes)
        return path

    def _translate_fixture(self):
        return self._write(
            nodes=[{'name': 'Moved', 'mesh': 0, 'translation': [10, 0, 0]}],
            meshes=[{'name': 'Tri', 'primitives': [{
                'points': [[0, 0, 0], [1, 0, 0], [0, 1, 0], [-5, 0, 0]],
                'indices': [0, 1, 2],
            }]}],
        )

    def test_transformed_bounds(self):
        path = self._translate_fixture()
        evidence = _probe(path, _config())
        self.assertEqual(evidence['bounds']['min'], [5.0, 0.0, 0.0])
        self.assertEqual(evidence['bounds']['max'], [11.0, 1.0, 0.0])
        self.assertEqual(evidence['bounds']['dimensions'], [6.0, 1.0, 0.0])

    def test_counts_per_node_mesh_primitive(self):
        path = self._translate_fixture()
        evidence = _probe(path, _config())
        self.assertEqual(evidence['vertex_count'], 4)
        self.assertEqual(evidence['counts']['node_counts'], [{
            'node': 0, 'name': 'Moved', 'vertices': 4,
        }])
        self.assertEqual(evidence['counts']['mesh_counts'], [{
            'mesh': 0, 'name': 'Tri', 'vertices': 4,
        }])
        self.assertEqual(evidence['counts']['primitive_counts'], [{
            'node': 0, 'primitive': 0, 'vertices': 4,
        }])

    def test_above_and_below_axis(self):
        path = self._translate_fixture()
        evidence = _probe(path, _config(predicates=[
            {'id': 'above', 'type': 'above_axis', 'axis': 'Y', 'threshold': 0.5},
            {'id': 'below', 'type': 'below_axis', 'axis': 'Y', 'threshold': 0.5},
        ]))
        above = next(p for p in evidence['predicates'] if p['id'] == 'above')
        below = next(p for p in evidence['predicates'] if p['id'] == 'below')
        self.assertEqual(above['total_count'], 1)
        self.assertEqual(above['sample'][0]['position'], [10.0, 1.0, 0.0])
        self.assertEqual(below['total_count'], 3)

    def test_ahead_of_axis_negative_sign(self):
        path = self._write(
            nodes=[{'name': 'Moved', 'mesh': 0, 'translation': [0, 0, 5]}],
            meshes=[{'name': 'Tri', 'primitives': [{
                'points': [[0, 0, 0], [0, 0, 1], [0, 0, 2]],
            }]}],
        )
        evidence = _probe(path, _config(predicates=[
            {'id': 'ahead', 'type': 'ahead_of_axis', 'axis': 'Z', 'threshold': 6.0},
            {'id': 'behind', 'type': 'behind_axis', 'axis': '-Z', 'threshold': -7.0},
        ]))
        ahead = next(p for p in evidence['predicates'] if p['id'] == 'ahead')
        behind = next(p for p in evidence['predicates'] if p['id'] == 'behind')
        # ahead of z>6 on world z {5,6,7}: only 7
        self.assertEqual(ahead['total_count'], 1)
        # behind of -z<-7 means z>7: none of {5,6,7}
        self.assertEqual(behind['total_count'], 0)

    def test_inside_and_outside_band(self):
        path = self._translate_fixture()
        evidence = _probe(path, _config(predicates=[
            {'id': 'inside', 'type': 'inside_band', 'axis': 'X',
             'centre': 10.5, 'width': 1.5},
            {'id': 'outside', 'type': 'outside_band', 'axis': 'X',
             'centre': 10.5, 'width': 1.5},
        ]))
        inside = next(p for p in evidence['predicates'] if p['id'] == 'inside')
        outside = next(p for p in evidence['predicates'] if p['id'] == 'outside')
        self.assertEqual(inside['total_count'], 3)
        self.assertEqual(outside['total_count'], 1)

    def test_named_sections_filter_scope(self):
        path = self._write(
            nodes=[{'name': 'Moved', 'mesh': 0, 'translation': [0, 0, 5]}],
            meshes=[{'name': 'Tri', 'primitives': [{
                'points': [[0, 0, 0], [0, 0, 1], [0, 0, 2]],
            }]}],
        )
        evidence = _probe(path, _config(
            sections={'near': {'axis': 'Z', 'max': 6.2}},
            predicates=[
                {'id': 'ahead_near', 'type': 'ahead_of_axis', 'axis': 'Z',
                 'threshold': 0.0, 'section': 'near'},
            ],
        ))
        ahead = evidence['predicates'][0]
        self.assertEqual(ahead['section'], 'near')
        self.assertEqual(ahead['total_count'], 2)  # world z 5 and 6

    def test_radial_distance_statistics(self):
        path = self._translate_fixture()
        evidence = _probe(path, _config(predicates=[
            {'id': 'rad', 'type': 'radial_distance', 'plane_axis': 'X',
             'centre': [10.0, 0.0, 0.0]},
        ]))
        radial = evidence['predicates'][0]
        self.assertEqual(radial['vertex_count'], 4)
        self.assertEqual(radial['radius_min'], 0.0)
        self.assertEqual(radial['radius_max'], 1.0)
        self.assertEqual(radial['radius_mean'], 0.25)

    def test_bounded_sample_is_lexicographically_ordered(self):
        path = self._translate_fixture()
        evidence = _probe(path, _config(predicates=[
            {'id': 'below', 'type': 'below_axis', 'axis': 'Y',
             'threshold': 0.5, 'sample_size': 2},
        ]))
        below = evidence['predicates'][0]
        self.assertEqual(below['total_count'], 3)
        self.assertEqual(below['sample_count'], 2)
        self.assertEqual(len(below['sample']), 2)
        indices = [entry['vertex_index'] for entry in below['sample']]
        self.assertEqual(indices, sorted(indices))

    def test_non_finite_vertex_count(self):
        path = self.dir / 'nan.glb'
        path.write_bytes(build_glb(
            nodes=[{'name': 'Root', 'mesh': 0}],
            meshes=[{'name': 'Tri', 'primitives': [{
                'points': [[0, 0, 0], [1, 0, 0]],
                'nan_point': [float('nan'), 0.0, 0.0],
            }]}],
        ))
        evidence = _probe(path, _config())
        self.assertEqual(evidence['vertex_count'], 3)
        self.assertEqual(evidence['finite_vertex_count'], 2)
        self.assertEqual(evidence['non_finite_vertex_count'], 1)
        self.assertEqual(evidence['bounds']['min'], [0.0, 0.0, 0.0])
        self.assertEqual(evidence['bounds']['max'], [1.0, 0.0, 0.0])


class ProbeConfigValidationTests(unittest.TestCase):
    def test_rejects_unknown_predicate_type(self):
        from analysis_suite.contracts import ConfigError
        with self.assertRaises(ConfigError):
            predicates.validate_probe_config(_config(predicates=[
                {'id': 'x', 'type': 'nope'},
            ]))

    def test_rejects_unknown_section_reference(self):
        from analysis_suite.contracts import ConfigError
        with self.assertRaises(ConfigError):
            predicates.validate_probe_config(_config(predicates=[
                {'id': 'x', 'type': 'above_axis', 'axis': 'Y',
                 'threshold': 0.0, 'section': 'missing'},
            ]))

    def test_rejects_missing_threshold(self):
        from analysis_suite.contracts import ConfigError
        with self.assertRaises(ConfigError):
            predicates.validate_probe_config(_config(predicates=[
                {'id': 'x', 'type': 'above_axis', 'axis': 'Y'},
            ]))


if __name__ == '__main__':
    unittest.main()
