from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np


PIPELINE_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PIPELINE_DIR))

from add_gltf_normals import _chunks, add_normals, calculate_normals, normal_coverage  # noqa: E402


REPO_ROOT = PIPELINE_DIR.parents[1]
SOURCE = REPO_ROOT / "assets-lowpoly-python/vehicles/canonical/formula_reference_livery_contract_annotated.glb"


class GltfNormalsTests(unittest.TestCase):
    def test_calculate_normals_smooths_a_coplanar_uv_seam(self) -> None:
        positions = np.array(
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            dtype=np.float32,
        )
        normals, metrics = calculate_normals(
            positions, np.arange(6, dtype=np.uint32), crease_angle_degrees=45.0, weld_tolerance=1e-6
        )
        np.testing.assert_allclose(normals, np.tile([0.0, 0.0, 1.0], (6, 1)), atol=1e-7)
        self.assertEqual(0, metrics["fallback_normals"])

    def test_canonical_candidate_gains_complete_normal_coverage(self) -> None:
        before = normal_coverage(SOURCE)
        self.assertGreater(before["triangle_primitives"], 0)
        with tempfile.TemporaryDirectory() as temporary:
            candidate = Path(temporary) / "candidate.glb"
            report = add_normals(SOURCE, candidate)
            self.assertEqual(0, report["coverage"]["missing_normals"])
            self.assertEqual(before["missing_normals"], report["primitives_modified"])

    def test_transform_preserves_every_preexisting_glb_payload(self) -> None:
        source_document, source_buffer = _chunks(SOURCE)
        with tempfile.TemporaryDirectory() as temporary:
            candidate = Path(temporary) / "candidate.glb"
            add_normals(SOURCE, candidate)
            candidate_document, candidate_buffer = _chunks(candidate)

        self.assertEqual(source_buffer, candidate_buffer[: len(source_buffer)])
        self.assertEqual(source_document["accessors"], candidate_document["accessors"][: len(source_document["accessors"])])
        self.assertEqual(
            source_document["bufferViews"],
            candidate_document["bufferViews"][: len(source_document["bufferViews"])],
        )
        for key in (
            "asset",
            "scene",
            "scenes",
            "nodes",
            "materials",
            "textures",
            "images",
            "samplers",
            "extensionsUsed",
            "extensionsRequired",
        ):
            self.assertEqual(source_document.get(key), candidate_document.get(key), key)

        self.assertEqual(len(source_document["meshes"]), len(candidate_document["meshes"]))
        for source_mesh, candidate_mesh in zip(source_document["meshes"], candidate_document["meshes"]):
            self.assertEqual(source_mesh.get("name"), candidate_mesh.get("name"))
            self.assertEqual(len(source_mesh["primitives"]), len(candidate_mesh["primitives"]))
            for source_primitive, candidate_primitive in zip(
                source_mesh["primitives"], candidate_mesh["primitives"]
            ):
                candidate_without_normal = dict(candidate_primitive)
                candidate_attributes = dict(candidate_without_normal["attributes"])
                if "NORMAL" not in source_primitive["attributes"]:
                    candidate_attributes.pop("NORMAL", None)
                candidate_without_normal["attributes"] = candidate_attributes
                self.assertEqual(source_primitive, candidate_without_normal)


if __name__ == "__main__":
    unittest.main()
