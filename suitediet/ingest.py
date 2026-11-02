"""Reading of the measured record sets suitediet ranks.

A record set stands in one document whose format name is
suitediet-records/1. Reading is exact. A missing field or an unexpected
field or a duplicated name or a repeated key is refused rather than
guessed, because the plan is only as honest as the records it rests on.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from .errors import SuitedietError

RECORDS_FORMAT = "suitediet-records/1"
QUARANTINE_FORMAT = "suitediet-quarantine/1"

_DOCUMENT_FIELDS = {"format", "suite", "tests"}
_RECORD_FIELDS = {"name", "runtime_ms", "caught"}


@dataclass(frozen=True)
class TestRecord:
    name: str
    runtime_ms: int
    caught: tuple[str, ...]


@dataclass(frozen=True)
class RecordSet:
    suite: str
    tests: tuple[TestRecord, ...]

    def by_name(self) -> dict[str, TestRecord]:
        return {test.name: test for test in self.tests}


@dataclass(frozen=True)
class Quarantine:
    suite: str
    tests: tuple[TestRecord, ...]


def _object_without_repeated_keys(pairs: list) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise SuitedietError(f"a JSON object carries the key {key!r} twice")
        result[key] = value
    return result


def _read_json(text: str, what: str) -> object:
    try:
        return json.loads(text, object_pairs_hook=_object_without_repeated_keys)
    except json.JSONDecodeError as exc:
        raise SuitedietError(f"the {what} is not valid JSON: {exc}") from exc


def _read_test_records(raw: object, what: str, allow_empty: bool) -> tuple[TestRecord, ...]:
    if not isinstance(raw, list):
        raise SuitedietError(f"the {what} tests field is not a list")
    if not raw and not allow_empty:
        raise SuitedietError(f"the {what} tests list is empty")
    records = []
    seen_names = set()
    for entry in raw:
        if not isinstance(entry, dict):
            raise SuitedietError(f"a test record in the {what} is not an object")
        if set(entry) != _RECORD_FIELDS:
            raise SuitedietError(
                f"a test record in the {what} carries fields {sorted(entry)} "
                f"rather than {sorted(_RECORD_FIELDS)}"
            )
        name = entry["name"]
        if not isinstance(name, str) or not name:
            raise SuitedietError(f"a test record in the {what} carries an empty or non text name")
        if name in seen_names:
            raise SuitedietError(f"the {what} lists the test name {name} twice")
        seen_names.add(name)
        runtime = entry["runtime_ms"]
        if isinstance(runtime, bool) or not isinstance(runtime, int) or runtime <= 0:
            raise SuitedietError(
                f"the test {name} carries a runtime that is not a positive whole number of milliseconds"
            )
        caught = entry["caught"]
        if not isinstance(caught, list):
            raise SuitedietError(f"the test {name} carries a caught field that is not a list")
        for defect in caught:
            if not isinstance(defect, str) or not defect:
                raise SuitedietError(f"the test {name} catches an empty or non text defect identifier")
        if len(set(caught)) != len(caught):
            raise SuitedietError(f"the test {name} lists one defect identifier twice")
        records.append(TestRecord(name=name, runtime_ms=runtime, caught=tuple(sorted(caught))))
    records.sort(key=lambda record: record.name)
    return tuple(records)


def _read_document(text: str, format_name: str, what: str, allow_empty: bool):
    document = _read_json(text, what)
    if not isinstance(document, dict):
        raise SuitedietError(f"the {what} is not an object")
    if set(document) != _DOCUMENT_FIELDS:
        raise SuitedietError(
            f"the {what} carries fields {sorted(document)} rather than {sorted(_DOCUMENT_FIELDS)}"
        )
    if document["format"] != format_name:
        raise SuitedietError(
            f"the {what} carries the format name {document['format']!r} rather than {format_name!r}"
        )
    suite = document["suite"]
    if not isinstance(suite, str) or not suite:
        raise SuitedietError(f"the {what} carries an empty or non text suite name")
    tests = _read_test_records(document["tests"], what, allow_empty)
    return suite, tests


def read_records_text(text: str) -> RecordSet:
    suite, tests = _read_document(text, RECORDS_FORMAT, "records document", allow_empty=False)
    return RecordSet(suite=suite, tests=tests)


def read_remaining_text(text: str) -> RecordSet:
    suite, tests = _read_document(
        text, RECORDS_FORMAT, "remaining records document", allow_empty=True
    )
    return RecordSet(suite=suite, tests=tests)


def read_quarantine_text(text: str) -> Quarantine:
    suite, tests = _read_document(text, QUARANTINE_FORMAT, "quarantine record", allow_empty=True)
    return Quarantine(suite=suite, tests=tests)


def _read_file(path: str | Path, what: str) -> str:
    try:
        return Path(path).read_text(encoding="utf-8")
    except FileNotFoundError:
        raise SuitedietError(f"the {what} at {path} does not exist") from None
    except OSError as exc:
        raise SuitedietError(f"the {what} at {path} cannot be read: {exc}") from exc


def read_records(path: str | Path) -> RecordSet:
    return read_records_text(_read_file(path, "records document"))


def read_remaining(path: str | Path) -> RecordSet:
    return read_remaining_text(_read_file(path, "remaining records document"))


def read_quarantine(path: str | Path) -> Quarantine:
    return read_quarantine_text(_read_file(path, "quarantine record"))


def record_document(record: TestRecord) -> dict:
    return {"name": record.name, "runtime_ms": record.runtime_ms, "caught": list(record.caught)}


def records_document(record_set: RecordSet) -> dict:
    return {
        "format": RECORDS_FORMAT,
        "suite": record_set.suite,
        "tests": [record_document(test) for test in record_set.tests],
    }


def quarantine_document(quarantine: Quarantine) -> dict:
    return {
        "format": QUARANTINE_FORMAT,
        "suite": quarantine.suite,
        "tests": [record_document(test) for test in quarantine.tests],
    }
