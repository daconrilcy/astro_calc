#!/usr/bin/env python3
"""Create and populate PostgreSQL tables from json_db/*.json.

The script intentionally avoids foreign-key creation because the JSON catalog
contains a mix of flat seed tables, compatibility tables, and nested documents.
It creates typed columns where the source data is clear, and stores nested
objects/lists as jsonb.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_JSON_DIR = ROOT / "json_db"
DEFAULT_ENV_FILE = ROOT / ".env"
METADATA_KEYS = {
    "constraints",
    "table_role",
    "unique_key",
    "versioning",
    "schema_version",
    "implementation_notes",
    "resolution_order",
}


def read_env(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.exists():
        return values
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


def quote_ident(identifier: str) -> str:
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", identifier):
        raise ValueError(f"Unsafe SQL identifier: {identifier!r}")
    return f'"{identifier}"'


def quote_literal(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def load_json_table(path: Path) -> dict[str, Any]:
    obj = json.loads(path.read_text(encoding="utf-8"))
    name = obj.get("name")
    if not isinstance(name, str):
        raise ValueError(f"{path}: missing string field 'name'")
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
        raise ValueError(f"{path}: unsafe table name {name!r}")
    return obj


def rows_from_data(data: Any) -> list[dict[str, Any]]:
    if isinstance(data, list):
        rows = data
    elif isinstance(data, dict):
        rows = [data]
    elif data is None:
        rows = []
    else:
        raise ValueError(f"Unsupported data type: {type(data).__name__}")

    normalized: list[dict[str, Any]] = []
    for row in rows:
        if not isinstance(row, dict):
            raise ValueError(f"Rows must be JSON objects, got {type(row).__name__}")
        normalized.append(row)
    return normalized


def descriptor_for(structure: Any, column: str) -> str:
    if isinstance(structure, dict):
        value = structure.get(column)
        if isinstance(value, str):
            return value
        columns = structure.get("columns")
        if isinstance(columns, dict):
            nested = columns.get(column)
            if isinstance(nested, str):
                return nested
    return ""


def columns_from_structure(structure: Any) -> list[str]:
    if not isinstance(structure, dict):
        return []

    columns = structure.get("columns")
    if isinstance(columns, dict):
        return [
            key
            for key, value in columns.items()
            if isinstance(value, str) and key not in METADATA_KEYS
        ]

    result: list[str] = []
    for key, value in structure.items():
        if key in METADATA_KEYS or key.startswith("_"):
            continue
        if isinstance(value, str):
            result.append(key)
    return result


def columns_for_table(rows: list[dict[str, Any]], structure: Any) -> list[str]:
    columns: list[str] = []
    seen: set[str] = set()
    for row in rows:
        for key in row:
            if key not in seen:
                columns.append(key)
                seen.add(key)

    if not columns:
        columns = columns_from_structure(structure)

    for column in columns:
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", column):
            raise ValueError(f"Unsafe column name: {column!r}")
    return columns


def infer_type(column: str, descriptor: str, values: list[Any]) -> str:
    desc = descriptor.lower()
    non_null = [value for value in values if value is not None]

    if (
        column.endswith("_json")
        or "json" in desc
        or "object" in desc
        or "array" in desc
        or any(isinstance(value, (dict, list)) for value in non_null)
    ):
        return "jsonb"
    if "datetime" in desc or "timestamp" in desc:
        return "timestamp with time zone"
    if "boolean" in desc:
        return "boolean"
    if "decimal" in desc or "float" in desc or "number" in desc:
        return "numeric"
    if "integer" in desc:
        return "integer"

    if non_null and all(isinstance(value, bool) for value in non_null):
        return "boolean"
    if non_null and all(isinstance(value, int) and not isinstance(value, bool) for value in non_null):
        return "integer"
    if non_null and all(
        isinstance(value, (int, float)) and not isinstance(value, bool)
        for value in non_null
    ):
        return "numeric"
    return "text"


def sql_value(value: Any, postgres_type: str) -> str:
    if value is None:
        return "NULL"
    if postgres_type == "jsonb":
        return quote_literal(json.dumps(value, ensure_ascii=False, separators=(",", ":"))) + "::jsonb"
    if postgres_type == "boolean":
        if isinstance(value, bool):
            return "TRUE" if value else "FALSE"
        if isinstance(value, int):
            return "TRUE" if value else "FALSE"
        return quote_literal(str(value))
    if postgres_type in {"integer", "numeric"}:
        return str(value)
    return quote_literal(str(value))


def build_sql(json_dir: Path, schema: str) -> tuple[str, int, int]:
    paths = sorted(json_dir.glob("*.json"))
    if not paths:
        raise RuntimeError(f"No JSON files found in {json_dir}")

    tables = [load_json_table(path) for path in paths]
    q_schema = quote_ident(schema)
    lines = [
        "\\set ON_ERROR_STOP on",
        "BEGIN;",
        f"CREATE SCHEMA IF NOT EXISTS {q_schema};",
    ]

    for table in tables:
        lines.append(f"DROP TABLE IF EXISTS {q_schema}.{quote_ident(table['name'])} CASCADE;")

    total_rows = 0
    for table in tables:
        rows = rows_from_data(table.get("data"))
        columns = columns_for_table(rows, table.get("structure"))
        column_types: dict[str, str] = {}
        for column in columns:
            values = [row.get(column) for row in rows]
            column_types[column] = infer_type(
                column,
                descriptor_for(table.get("structure"), column),
                values,
            )

        create_columns = [
            f"  {quote_ident(column)} {column_types[column]}"
            for column in columns
        ]
        lines.append("")
        lines.append(f"CREATE TABLE {q_schema}.{quote_ident(table['name'])} (")
        lines.append(",\n".join(create_columns))
        lines.append(");")

        for row in rows:
            total_rows += 1
            col_sql = ", ".join(quote_ident(column) for column in columns)
            values_sql = ", ".join(sql_value(row.get(column), column_types[column]) for column in columns)
            lines.append(
                f"INSERT INTO {q_schema}.{quote_ident(table['name'])} ({col_sql}) VALUES ({values_sql});"
            )

    lines += ["COMMIT;", ""]
    return "\n".join(lines), len(tables), total_rows


def run_psql(sql: str, env_values: dict[str, str]) -> None:
    user = env_values.get("POSTGRES_USER", os.environ.get("POSTGRES_USER", "postgres"))
    db = env_values.get("POSTGRES_DB", os.environ.get("POSTGRES_DB", user))
    command = [
        "docker",
        "compose",
        "exec",
        "-T",
        "postgres",
        "psql",
        "-U",
        user,
        "-d",
        db,
    ]
    subprocess.run(command, input=sql.encode("utf-8"), cwd=ROOT, check=True)


def main() -> None:
    parser = argparse.ArgumentParser(description="Import json_db/*.json into PostgreSQL.")
    parser.add_argument("--json-dir", type=Path, default=DEFAULT_JSON_DIR)
    parser.add_argument("--schema", default="public")
    parser.add_argument("--dry-run", action="store_true", help="Generate SQL without executing it.")
    parser.add_argument("--output", type=Path, help="Write generated SQL to a file.")
    args = parser.parse_args()

    sql, table_count, row_count = build_sql(args.json_dir, args.schema)
    if args.output:
        args.output.write_text(sql, encoding="utf-8")
    if args.dry_run:
        print(f"Generated SQL for {table_count} tables and {row_count} rows.")
        return

    env_values = read_env(DEFAULT_ENV_FILE)
    run_psql(sql, env_values)
    print(f"Imported {row_count} rows into {table_count} PostgreSQL tables.")


if __name__ == "__main__":
    main()
