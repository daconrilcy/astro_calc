#!/usr/bin/env python3
"""Add or refresh astral_aspects.default_orb_deg from json_db without recreating tables."""

from __future__ import annotations

import json
import sys
from pathlib import Path

from psql_docker import run_psql

ROOT = Path(__file__).resolve().parents[1]
JSON_PATH = ROOT / "json_db" / "astral_aspects.json"


def main() -> int:
    table = json.loads(JSON_PATH.read_text(encoding="utf-8"))
    rows = table.get("data") or []
    if not isinstance(rows, list):
        print(f"{JSON_PATH}: data invalide", file=sys.stderr)
        return 1

    run_psql(
        "ALTER TABLE astral_aspects "
        "ADD COLUMN IF NOT EXISTS default_orb_deg numeric;"
    )

    updated = 0
    for row in rows:
        if not isinstance(row, dict):
            continue
        aspect_id = row.get("id")
        orb = row.get("default_orb_deg")
        if aspect_id is None or orb is None:
            continue
        run_psql(
            f"UPDATE astral_aspects SET default_orb_deg = {orb} WHERE id = {int(aspect_id)};",
        )
        updated += 1

    major_expected = run_psql(
        "SELECT expected_aspect_count FROM astral_aspect_families WHERE name = 'major';",
    )
    if not major_expected or major_expected.upper() == "NULL":
        print(
            "echec: astral_aspect_families.expected_aspect_count manquant pour major "
            "(executer scripts/patch_astral_aspect_families_expected_count.py)",
            file=sys.stderr,
        )
        return 1

    major_count = run_psql(
        "SELECT COUNT(*) FROM astral_aspects WHERE family = 'major';",
    )
    if major_count != major_expected:
        print(
            f"echec: attendu {major_expected} aspects family=major, trouve {major_count}",
            file=sys.stderr,
        )
        return 1

    missing = run_psql(
        "SELECT COUNT(*) FROM astral_aspects "
        "WHERE family = 'major' "
        "AND (default_orb_deg IS NULL OR default_orb_deg <= 0);",
    )
    if missing != "0":
        print(
            f"echec: {missing} aspect(s) majeur(s) sans orbe canonique apres patch",
            file=sys.stderr,
        )
        return 1

    excessive = run_psql(
        "SELECT COUNT(*) FROM astral_aspects a "
        "WHERE a.family = 'major' "
        "AND a.default_orb_deg > ("
        "  SELECT f.max_default_orb_deg FROM astral_aspect_families f WHERE f.name = 'major'"
        ");",
    )
    if excessive != "0":
        print(
            f"echec: {excessive} aspect(s) majeur(s) avec default_orb_deg > max famille",
            file=sys.stderr,
        )
        return 1

    print(
        f"astral_aspects.default_orb_deg: colonne assuree, {updated} ligne(s) mises a jour, "
        f"{major_expected} aspects majeurs verifies."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
