import json
import tempfile
import unittest
from pathlib import Path

from formula_one_2030_object_names import (
    CHASSIS_OBJECT_NAMES,
    OBJECT_NAME_MIGRATION,
    WHEEL_POSITIONS,
    canonical_object_name,
    is_chassis_object,
    is_wheel_object,
    library_object_name,
    normalize_object_name_document,
    wheel_source_object_names,
)
from rename_formula_one_2030_runtime_assets import rename_runtime_document


class FormulaOne2030ObjectNamesTest(unittest.TestCase):
    def test_all_destination_names_are_unique(self):
        self.assertEqual(len(OBJECT_NAME_MIGRATION), len(set(OBJECT_NAME_MIGRATION.values())))

    def test_all_chassis_components_remain_exportable(self):
        for previous_name, current_name in CHASSIS_OBJECT_NAMES.items():
            self.assertTrue(is_chassis_object(previous_name), previous_name)
            self.assertTrue(is_chassis_object(current_name), current_name)
        self.assertFalse(is_chassis_object('StudioGroundPlane'))

    def test_every_wheel_contains_its_brakes_and_five_rotating_components(self):
        for previous_position, current_position in WHEEL_POSITIONS.items():
            names = wheel_source_object_names(current_position)
            self.assertEqual(len(names), 8)
            self.assertEqual(sum(name.endswith(('_CALIPER', '_INBOARD_DUCT_COVER', '_DUCT')) for name in names), 3)
            for name in names:
                self.assertTrue(is_wheel_object(name, previous_position))
                self.assertTrue(is_wheel_object(name, current_position))
            self.assertFalse(is_wheel_object('GEO_WHEEL_FRONT_LEFT_TIRE', 'REAR_LEFT'))

    def test_historical_reports_translate_keys_without_changing_paths_or_hashes(self):
        document = {'GEO_CHASSIS_BODY': {'object': 'GEO_WHEEL_FRONT_L_NUT_05', 'path': 'backup/GEO_CHASSIS_BODY.blend', 'sha256': '1234'}}
        translated = normalize_object_name_document(document)
        record = translated['GEO_BODYWORK_NOSE_SIDEPODS_AND_ENGINE_COVER']
        self.assertEqual(record['object'], 'GEO_BRAKE_FRONT_LEFT_CALIPER')
        self.assertEqual(record['path'], document['GEO_CHASSIS_BODY']['path'])
        self.assertEqual(record['sha256'], '1234')
        self.assertIn('GEO_CHASSIS_BODY', document)

    def test_mesh_suffixes_and_historical_decals_preserve_component_identity(self):
        self.assertEqual(canonical_object_name('GEO_CHASSIS_REAR_WING_MID_MESH.001'), 'GEO_REAR_WING_INTERMEDIATE_ELEMENT_MESH.001')
        self.assertEqual(canonical_object_name('GEO_WHEEL_FRONT_LEFT_NUT_05'), 'GEO_BRAKE_FRONT_LEFT_CALIPER')
        self.assertEqual(canonical_object_name('GEO_WHEEL_FRONT_L_DECAL_PIRELLI'), 'GEO_WHEEL_FRONT_LEFT_DECAL_PIRELLI')

    def test_historical_library_lookup_resolves_the_requested_headrest(self):
        self.assertEqual(library_object_name(['GEO_CHASSIS_CockpitHeadrestPadding'], 'GEO_COCKPIT_HeadrestPadding'), 'GEO_CHASSIS_CockpitHeadrestPadding')

    def test_runtime_migration_preserves_non_name_fields_and_binary_chunks(self):
        project_directory = Path(__file__).resolve().parents[2]
        source_path = project_directory / 'game/assets/models/vehicles/f1-2030/f1_2030_v10_wheel_FL.glb'
        with tempfile.TemporaryDirectory() as temporary_directory:
            output_path = Path(temporary_directory) / source_path.name
            report = rename_runtime_document(source_path, output_path)
            self.assertTrue(report['binary_chunks_preserved'])
            self.assertTrue(report['non_name_document_fields_preserved'])


if __name__ == '__main__':
    unittest.main()
