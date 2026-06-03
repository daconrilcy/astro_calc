#!/usr/bin/env python3
"""Add or refresh astral_aspect_families metadata from json_db."""

from __future__ import annotations

import json
import sys
from pathlib import Path

from psql_docker import numeric_equals, run_psql

ROOT = Path(__file__).resolve().parents[1]
JSON_PATH = ROOT / "json_db" / "astral_aspect_families.json"


def main() -> int:
    table = json.loads(JSON_PATH.read_text(encoding="utf-8"))
    rows = table.get("data") or []
    if not isinstance(rows, list):
        print(f"{JSON_PATH}: data invalide", file=sys.stderr)
        return 1

    run_psql(
        "ALTER TABLE astral_aspect_families "
        "ADD COLUMN IF NOT EXISTS expected_aspect_count integer;"
    )
    run_psql(
        "ALTER TABLE astral_aspect_families "
        "ADD COLUMN IF NOT EXISTS max_default_orb_deg numeric;"
    )

    updated = 0
    for row in rows:
        if not isinstance(row, dict):
            continue
        name = row.get("name")
        if not isinstance(name, str):
            continue
        expected = row.get("expected_aspect_count")
        max_orb = row.get("max_default_orb_deg")
        if expected is None:
            expected_sql = "NULL"
        else:
            expected_sql = str(int(expected))
        if max_orb is None:
            max_orb_sql = "NULL"
        else:
            max_orb_sql = str(float(max_orb))
        run_psql(
            "UPDATE astral_aspect_families "
            f"SET expected_aspect_count = {expected_sql}, "
            f"max_default_orb_deg = {max_orb_sql} "
            f"WHERE name = {repr(name)};"
        )
        updated += 1

    major_expected = run_psql(
        "SELECT expected_aspect_count FROM astral_aspect_families WHERE name = 'major';",
    )
    major_max_orb = run_psql(
        "SELECT max_default_orb_deg FROM astral_aspect_families WHERE name = 'major';",
    )
    if major_expected != "5":
        print(
            f"echec: expected_aspect_count pour major = {major_expected!r}, attendu 5",
            file=sys.stderr,
        )
        return 1
    if not numeric_equals(major_max_orb, 15.0):
        print(
            f"echec: max_default_orb_deg pour major = {major_max_orb!r}, attendu 15",
            file=sys.stderr,
        )
        return 1

    print(
        "astral_aspect_families: colonnes assurees, "
        f"{updated} famille(s) mises a jour, major expected=5 max_orb=15 verifies."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
