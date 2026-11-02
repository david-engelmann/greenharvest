import ast
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def test_every_contract_test_name_exists():
    contract = (ROOT / "CONTRACT.md").read_text(encoding="utf-8")
    named = set(re.findall(r"test_[a-z0-9_]+", contract))
    defined = set()
    for path in (ROOT / "tests").glob("test_*.py"):
        tree = ast.parse(path.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_"):
                defined.add(node.name)
    assert named <= defined, sorted(named - defined)
