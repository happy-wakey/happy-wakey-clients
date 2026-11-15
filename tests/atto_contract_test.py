"""AttoLang static API metadata conformance and negative drift tests."""
import json
import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ATTO = ROOT / "clients" / "atto" / "client.atto"
OPS = ROOT / "contract" / "operations.json"
PATH_FUNCTIONS = {
    "health": "health_path",
    "listAlarms": "list_alarms_path",
    "createAlarm": "create_alarm_path",
    "transitionOccurrence": "transition_occurrence_path_template",
    "pullChanges": "pull_changes_path",
    "pushChanges": "push_changes_path",
}


def body_of(source: str, name: str, args: str = "") -> str:
    signature = rf"^pub fnc {re.escape(name)}\({re.escape(args)}\): (?:string|bool)\s*\{{"
    match = re.search(signature, source, re.M)
    if match is None:
        raise ValueError(f"missing Atto function: {name}")
    # Match the first closing brace outside quoted route templates; avoid
    # mistaking the {occurrence_id} literal for a function-body delimiter.
    in_string = False
    escaped = False
    depth = 1
    start = match.end()
    for index in range(start, len(source)):
        char = source[index]
        if in_string and char == "\\" and not escaped:
            escaped = True
            continue
        if char == '"' and not escaped:
            in_string = not in_string
        elif not in_string and char == "{":
            depth += 1
        elif not in_string and char == "}":
            depth -= 1
            if depth == 0:
                return source[start:index]
        escaped = False
    raise ValueError(f"unclosed Atto function: {name}")


def literal_return(body: str) -> str:
    match = re.fullmatch(r'\s*return "([^"]*)";\s*', body)
    if match is None:
        raise ValueError("path must be a single string literal")
    return match.group(1)


def verify_atto(source: str, operations: list[dict]) -> None:
    if set(PATH_FUNCTIONS) != {op["id"] for op in operations}:
        raise ValueError("operation IDs drifted")
    if len({op["id"] for op in operations}) != len(operations):
        raise ValueError("duplicate contract operation ID")
    for op in operations:
        path = literal_return(body_of(source, PATH_FUNCTIONS[op["id"]]))
        if path != op["path"]:
            raise ValueError(f"path drift for {op['id']}")
        if not path.startswith("/") or "://" in path or "?" in path:
            raise ValueError(f"invalid static path for {op['id']}")
    known = body_of(source, "is_known_operation", "string operation")
    admitted = re.findall(r'if operation eq "([^"]+)" then return true; fi', known)
    if admitted != [op["id"] for op in operations] or "return false;" not in known:
        raise ValueError("known-operation allowlist drift")
    auth = body_of(source, "requires_bearer", "string operation")
    if not re.fullmatch(r'\s*if operation eq "health" then return false; fi\s*return true;\s*', auth):
        raise ValueError("bearer fail-closed policy drift")
    if [op["id"] for op in operations if not op["auth"]] != ["health"]:
        raise ValueError("contract auth policy drift")
    if literal_return(body_of(source, "bearer_header_name")) != "Authorization":
        raise ValueError("bearer header drift")
    if literal_return(body_of(source, "telemetry_event_name")) != "happy_wakey.client.request":
        raise ValueError("telemetry event drift")


class AttoContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = ATTO.read_text(encoding="utf-8")
        cls.operations = json.loads(OPS.read_text(encoding="utf-8"))["operations"]

    def test_pinned_operations(self):
        verify_atto(self.source, self.operations)

    def test_route_drift_rejected(self):
        with self.assertRaises(ValueError):
            verify_atto(self.source.replace('return "/v1/sync/pull";', 'return "/v2/sync/pull";'),
                        self.operations)

    def test_path_expression_injection_rejected(self):
        with self.assertRaises(ValueError):
            verify_atto(self.source.replace('return "/healthz";', 'return user_input;'),
                        self.operations)

    def test_unknown_operation_allowlist_drift_rejected(self):
        with self.assertRaises(ValueError):
            verify_atto(self.source.replace('if operation eq "pushChanges" then return true; fi',
                                            'if operation eq "typo" then return true; fi'),
                        self.operations)

    def test_bearer_bypass_rejected(self):
        with self.assertRaises(ValueError):
            verify_atto(self.source.replace('if operation eq "health" then return false; fi',
                                            'if operation eq "createAlarm" then return false; fi'),
                        self.operations)


if __name__ == "__main__":
    unittest.main()
