import copy
import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIPELINE = ROOT / "blender" / "track_pipeline"
sys.path.insert(0, str(PIPELINE))

from safety_barrier_layout import compile_layout, validate_manifest


class SafetyBarrierDeterminismTests(unittest.TestCase):
    def setUp(self):
        self.path = PIPELINE / "manifests" / "la_chutana_safety_barriers.json"
        self.manifest = validate_manifest(json.loads(self.path.read_text()))

    def test_same_input_has_same_hash(self):
        self.assertEqual(compile_layout(self.manifest, 2420.0)["sha256"],
                         compile_layout(self.manifest, 2420.0)["sha256"])

    def test_hash_is_stable_between_processes(self):
        code = ("import json,sys;sys.path.insert(0,r'%s');"
                "from safety_barrier_layout import compile_layout;"
                "m=json.load(open(r'%s'));print(compile_layout(m,2420.0)['sha256'])"
                % (PIPELINE, self.path))
        hashes = [subprocess.check_output([sys.executable, "-c", code], text=True).strip()
                  for _ in range(2)]
        self.assertEqual(hashes[0], hashes[1])

    def test_seed_changes_only_authorized_variations(self):
        changed = copy.deepcopy(self.manifest)
        changed["seed"] += 1
        a = compile_layout(self.manifest, 2420.0)["modules"]
        b = compile_layout(changed, 2420.0)["modules"]
        structural = ("segment_id", "module_index", "type", "side", "fraction",
                      "center_distance_m", "length_m", "collision_profile")
        self.assertEqual([[m[k] for k in structural] for m in a],
                         [[m[k] for k in structural] for m in b])

    def test_segment_change_does_not_change_other_module_hash_data(self):
        changed = copy.deepcopy(self.manifest)
        changed["segments"][0]["center_distance_m"] += 1
        a = compile_layout(self.manifest, 2420.0)["modules"]
        b = compile_layout(changed, 2420.0)["modules"]
        self.assertEqual([m for m in a if m["segment_id"] != "ChicaneLeft"],
                         [m for m in b if m["segment_id"] != "ChicaneLeft"])


if __name__ == "__main__":
    unittest.main()
