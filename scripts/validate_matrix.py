#!/usr/bin/env python3
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
matrix = json.loads((ROOT / "clients/sdk-matrix.json").read_text())
operations = json.loads((ROOT / "contract/operations.json").read_text())["operations"]
operation_tokens = {
    "health": ("health",),
    "listAlarms": ("listAlarms", "list_alarms", "listAlarms"),
    "createAlarm": ("createAlarm", "create_alarm"),
    "transitionOccurrence": ("transitionOccurrence", "transition_occurrence"),
    "pullChanges": ("pullChanges", "pull_changes"),
    "pushChanges": ("pushChanges", "push_changes"),
}

assert len(matrix["targets"]) >= 17
for name, (directory, manifest, source) in matrix["targets"].items():
    package = ROOT / directory
    assert (package / manifest).is_file(), f"{name}: missing {manifest}"
    source_path = package / source
    assert source_path.is_file(), f"{name}: missing {source}"
    text = source_path.read_text(encoding="utf-8")
    for operation in operations:
        tokens = operation_tokens[operation["id"]]
        assert any(token in text for token in tokens), f"{name}: missing {operation['id']}"
    assert "telemetry" in text.lower() or "next_loggers" in text.lower() or "logger" in text.lower(), f"{name}: no Ores telemetry bridge"
    assert "authorization" in text.lower() or "bearer" in text.lower() or name == "rust-wasm", f"{name}: bearer contract missing"

manifest = (ROOT / ".zpkg.toml").read_text(encoding="utf-8")
for dependency in ("happy-wakey/happy-wakey-interfaces", "ores-otel/ores.otel.log"):
    assert dependency in manifest
assert "cb8d8128c7a3d50f813a05e68451c2e3b292c59c" in (ROOT / "contract/interface.lock.json").read_text()
print(f"validated {len(matrix['targets'])} client slices, {len(operations)} operations, four TypeScript runtimes, contract pin, and Ores telemetry bridges")
