"""Pure-stdlib regression tests for inspect_glb.py."""
import json
import math
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


INSPECTOR = Path(__file__).resolve().parents[1] / 'inspect_glb.py'


def _pad4(data, fill):
    return data + fill * ((4 - len(data) % 4) % 4)


def _write_valid_fixture(path):
    binary = bytearray()
    buffer_views = []
    accessors = []

    def add_accessor(payload, component_type, accessor_type, count, **extra):
        while len(binary) % 4:
            binary.append(0)
        offset = len(binary)
        binary.extend(payload)
        buffer_views.append({
            'buffer': 0,
            'byteOffset': offset,
            'byteLength': len(payload),
        })
        accessor = {
            'bufferView': len(buffer_views) - 1,
            'componentType': component_type,
            'count': count,
            'type': accessor_type,
        }
        accessor.update(extra)
        accessors.append(accessor)
        return len(accessors) - 1

    position_accessor = add_accessor(
        struct.pack(
            '<9f',
            -1.0, -2.0, 0.0,
            1.0, 2.0, 3.0,
            0.0, 0.0, -1.0,
        ),
        5126,
        'VEC3',
        3,
        min=[-1.0, -2.0, -1.0],
        max=[1.0, 2.0, 3.0],
    )
    normal_accessor = add_accessor(
        struct.pack(
            '<9f',
            1.0, 0.0, 0.0,
            0.0, 0.0, 2.0,
            math.nan, 0.0, 0.0,
        ),
        5126,
        'VEC3',
        3,
    )
    texcoord_accessor = add_accessor(
        struct.pack('<6f', 0.0, 0.0, 1.0, 0.0, 0.0, 1.0),
        5126,
        'VEC2',
        3,
    )
    index_accessor = add_accessor(
        struct.pack('<3H', 0, 1, 2),
        5123,
        'SCALAR',
        3,
    )

    gltf = {
        'asset': {'version': '2.0', 'generator': 'inspect_glb regression'},
        'scene': 0,
        'scenes': [{'nodes': [0]}],
        'nodes': [
            {
                'name': 'MirroredRoot',
                'mesh': 0,
                'children': [1],
                'translation': [10, 0, 0],
                'scale': [-1, 1, 1],
            },
            {
                'name': 'ExplicitNegativeChild',
                'scale': [1, -1, 1],
            },
        ],
        'meshes': [{
            'name': 'FixtureMesh',
            'primitives': [
                {
                    'attributes': {
                        'POSITION': position_accessor,
                        'NORMAL': normal_accessor,
                        'TEXCOORD_0': texcoord_accessor,
                    },
                    'indices': index_accessor,
                },
                {
                    'attributes': {'POSITION': position_accessor},
                    'indices': index_accessor,
                },
            ],
        }],
        'buffers': [{'byteLength': len(binary)}],
        'bufferViews': buffer_views,
        'accessors': accessors,
    }
    json_chunk = _pad4(
        json.dumps(gltf, separators=(',', ':')).encode('utf-8'),
        b' ',
    )
    bin_chunk = _pad4(bytes(binary), b'\x00')
    total_length = 12 + 8 + len(json_chunk) + 8 + len(bin_chunk)
    glb = b''.join([
        struct.pack('<4sII', b'glTF', 2, total_length),
        struct.pack('<II', len(json_chunk), 0x4E4F534A),
        json_chunk,
        struct.pack('<II', len(bin_chunk), 0x004E4942),
        bin_chunk,
    ])
    path.write_bytes(glb)


def _run_inspector(path, *arguments):
    completed = subprocess.run(
        [sys.executable, str(INSPECTOR), str(path), *arguments],
        capture_output=True,
        text=True,
        check=False,
    )
    return completed, json.loads(completed.stdout)


class InspectGlbTests(unittest.TestCase):
    def test_successful_json_coverage_normals_and_transforms(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / 'fixture.glb'
            _write_valid_fixture(path)
            completed, report = _run_inspector(
                path,
                '--full',
                '--pretty',
                '--normal-tolerance',
                '0.01',
            )

        self.assertEqual(completed.returncode, 0)
        self.assertTrue(report['ok'])
        self.assertEqual(
            report['attribute_coverage'],
            {'POSITION': 2, 'NORMAL': 1, 'TEXCOORD_0': 1},
        )
        self.assertEqual(len(report['primitive_attribute_coverage']), 2)
        self.assertEqual(
            report['primitive_attribute_coverage'][0]['attributes']['NORMAL'],
            {'present': True, 'count': 3},
        )
        self.assertEqual(
            report['primitive_attribute_coverage'][1]['attributes']['NORMAL'],
            {'present': False, 'count': 0},
        )

        normal_stats = report['normal_statistics']
        self.assertEqual(normal_stats['normal_count'], 3)
        self.assertEqual(normal_stats['non_finite_value_count'], 1)
        self.assertEqual(normal_stats['non_finite_normal_count'], 1)
        self.assertEqual(normal_stats['length_minimum'], 1.0)
        self.assertEqual(normal_stats['length_maximum'], 2.0)
        self.assertEqual(normal_stats['length_mean'], 1.5)
        self.assertEqual(normal_stats['outside_near_unit_tolerance_count'], 1)
        self.assertFalse(report['finite_positions_normals'])

        self.assertEqual([entry['node'] for entry in report['negative_scale_nodes']], [0, 1])
        self.assertEqual(
            [entry['node'] for entry in report['negative_world_determinant_nodes']],
            [0],
        )
        self.assertLess(
            report['negative_world_determinant_nodes'][0]['world_determinant'],
            0,
        )
        self.assertEqual(report['world_bounds']['min'], [9.0, -2.0, -1.0])
        self.assertEqual(report['world_bounds']['max'], [11.0, 2.0, 3.0])

    def test_malformed_input_emits_json_and_exits_two(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / 'malformed.glb'
            path.write_bytes(b'not a GLB')
            completed, report = _run_inspector(path)

        self.assertEqual(completed.returncode, 2)
        self.assertFalse(report['ok'])
        self.assertIn('bad magic', report['error'])


if __name__ == '__main__':
    unittest.main()
