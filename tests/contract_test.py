import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class ContractTests(unittest.TestCase):
    def test_operation_paths_are_unique(self):
        operations = json.loads((ROOT / "contract/operations.json").read_text())["operations"]
        keys = {(op["method"], op["path"]) for op in operations}
        self.assertEqual(len(keys), len(operations))
        self.assertEqual(operations[0], {"id": "health", "method": "GET", "path": "/healthz", "auth": False})

    def test_only_health_is_anonymous(self):
        operations = json.loads((ROOT / "contract/operations.json").read_text())["operations"]
        self.assertEqual([op["id"] for op in operations if not op["auth"]], ["health"])


if __name__ == "__main__":
    unittest.main()
