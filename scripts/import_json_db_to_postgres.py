#!/usr/bin/env python3
"""Create and populate PostgreSQL tables from json_db/*.json.

The script creates typed columns where the source data is clear, stores nested
objects/lists as jsonb, and materializes valid primary/unique/foreign-key
constraints so database tools can display table relationships.
"""

from __future__ import annotations

import argparse
import hashlib
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
    if path.stem != name:
        raise ValueError(f"{path}: file name must match table name {name!r}")
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

    for key in columns_from_structure(structure):
        if key not in seen:
            columns.append(key)
            seen.add(key)

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
        or "array" in desc
        or desc.startswith("object")
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


def constraint_name(prefix: str, parts: list[str]) -> str:
    raw = "_".join([prefix, *parts])
    safe = re.sub(r"[^A-Za-z0-9_]", "_", raw).lower()
    if len(safe) <= 60:
        return safe
    digest = hashlib.sha1(safe.encode("utf-8")).hexdigest()[:8]
    return f"{safe[:51]}_{digest}"


def value_tuple(row: dict[str, Any], columns: tuple[str, ...]) -> tuple[Any, ...]:
    return tuple(row.get(column) for column in columns)


def unique_non_null(rows: list[dict[str, Any]], columns: tuple[str, ...]) -> bool:
    seen: set[tuple[Any, ...]] = set()
    for row in rows:
        values = value_tuple(row, columns)
        if any(value is None for value in values):
            continue
        marker = tuple(json.dumps(value, sort_keys=True) if isinstance(value, (dict, list)) else value for value in values)
        if marker in seen:
            return False
        seen.add(marker)
    return True


def unique_nulls_not_distinct(rows: list[dict[str, Any]], columns: tuple[str, ...]) -> bool:
    seen: set[tuple[Any, ...]] = set()
    for row in rows:
        values = value_tuple(row, columns)
        marker = tuple(json.dumps(value, sort_keys=True) if isinstance(value, (dict, list)) else value for value in values)
        if marker in seen:
            return False
        seen.add(marker)
    return True


def complete_non_null(rows: list[dict[str, Any]], columns: tuple[str, ...]) -> bool:
    return all(all(row.get(column) is not None for column in columns) for row in rows)


def extract_foreign_keys(table: dict[str, Any], columns: list[str]) -> list[tuple[tuple[str, ...], str, tuple[str, ...]]]:
    table_name = table["name"]
    structure = table.get("structure")
    refs: list[tuple[tuple[str, ...], str, tuple[str, ...]]] = []

    def add(source_columns: tuple[str, ...], target_table: str, target_columns: tuple[str, ...]) -> None:
        if not source_columns or not target_columns or len(source_columns) != len(target_columns):
            return
        if any(column not in columns for column in source_columns):
            return
        refs.append((source_columns, target_table, target_columns))

    if not isinstance(structure, dict):
        return refs

    descriptors: list[tuple[str, str]] = [
        (key, value) for key, value in structure.items() if isinstance(value, str)
    ]
    nested_columns = structure.get("columns")
    if isinstance(nested_columns, dict):
        descriptors.extend(
            (key, value) for key, value in nested_columns.items() if isinstance(value, str)
        )

    for column, descriptor in descriptors:
        for target_table, target_column in re.findall(
            r"FK\s*->\s*([A-Za-z0-9_]+)\.([A-Za-z0-9_]+)",
            descriptor,
        ):
            add((column,), target_table, (target_column,))
        for target_table, target_column in re.findall(
            r"foreign_key:([A-Za-z0-9_]+)\.([A-Za-z0-9_]+)",
            descriptor,
        ):
            add((column,), target_table, (target_column,))

    constraints = structure.get("constraints")
    if isinstance(constraints, dict):
        for foreign_key in constraints.get("foreign_keys", []):
            if not isinstance(foreign_key, dict):
                continue
            source_columns = tuple(foreign_key.get("columns", []))
            reference = foreign_key.get("references")
            if not isinstance(reference, str):
                continue
            match = re.fullmatch(r"([A-Za-z0-9_]+)\(([A-Za-z0-9_,\s]+)\)", reference)
            if not match:
                continue
            target_columns = tuple(column.strip() for column in match.group(2).split(","))
            add(source_columns, match.group(1), target_columns)

    # Stable de-duplication while preserving order.
    deduped: list[tuple[tuple[str, ...], str, tuple[str, ...]]] = []
    seen: set[tuple[tuple[str, ...], str, tuple[str, ...]]] = set()
    for foreign_key in refs:
        if foreign_key not in seen:
            deduped.append(foreign_key)
            seen.add(foreign_key)
    return deduped


def extract_check_constraints(table: dict[str, Any]) -> list[tuple[str, str]]:
    structure = table.get("structure")
    if not isinstance(structure, dict):
        return []

    checks: list[tuple[str, str]] = []
    constraints = structure.get("constraints")
    declared_checks = constraints.get("checks", []) if isinstance(constraints, dict) else []
    for index, check in enumerate(declared_checks, start=1):
        if isinstance(check, str):
            name = f"rule_{index}"
            expression = check
        elif isinstance(check, dict):
            name = check.get("name")
            expression = check.get("expression")
        else:
            continue
        if not isinstance(name, str) or not isinstance(expression, str):
            continue
        if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
            raise ValueError(f"{table['name']}: unsafe check constraint name {name!r}")
        if ";" in expression or "--" in expression or "/*" in expression or "*/" in expression:
            raise ValueError(f"{table['name']}: unsafe check expression {expression!r}")
        checks.append((name, expression))

    descriptors: list[tuple[str, str]] = [
        (key, value) for key, value in structure.items() if isinstance(value, str)
    ]
    nested_columns = structure.get("columns")
    if isinstance(nested_columns, dict):
        descriptors.extend(
            (key, value) for key, value in nested_columns.items() if isinstance(value, str)
        )

    for column, descriptor in descriptors:
        existing_names = {name for name, _ in checks}
        if re.search(r"\bsnake_case\b", descriptor, re.IGNORECASE):
            name = f"{column}_snake_case"
            if name not in existing_names:
                checks.append(
                    (
                        name,
                        f"{quote_ident(column)} ~ '^[a-z][a-z0-9_]*$'",
                    )
                )
        enum_match = re.search(r"\benum:([A-Za-z0-9_|]+)", descriptor, re.IGNORECASE)
        if enum_match:
            values = enum_match.group(1).split("|")
            allowed = ", ".join(quote_literal(value) for value in values)
            name = f"{column}_enum"
            if name not in existing_names:
                checks.append((name, f"{quote_ident(column)} IN ({allowed})"))
    return checks


def extract_indexes(table: dict[str, Any], columns: list[str]) -> list[tuple[str, ...]]:
    structure = table.get("structure")
    if not isinstance(structure, dict):
        return []

    constraints = structure.get("constraints")
    if not isinstance(constraints, dict):
        return []

    indexes: list[tuple[str, ...]] = []
    for index in constraints.get("indexes", []):
        if not isinstance(index, list) or not index or not all(isinstance(column, str) for column in index):
            continue
        index_columns = tuple(index)
        if any(column not in columns for column in index_columns):
            continue
        indexes.append(index_columns)
    return list(dict.fromkeys(indexes))


def extract_unique_constraints(table: dict[str, Any], columns: list[str]) -> list[tuple[str, ...]]:
    structure = table.get("structure")
    if not isinstance(structure, dict):
        return []

    candidates: list[Any] = [structure.get("unique_key")]
    descriptors: list[tuple[str, str]] = [
        (key, value) for key, value in structure.items() if isinstance(value, str)
    ]
    nested_columns = structure.get("columns")
    if isinstance(nested_columns, dict):
        descriptors.extend(
            (key, value) for key, value in nested_columns.items() if isinstance(value, str)
        )
    candidates.extend([[column] for column, descriptor in descriptors if re.search(r"\bunique\b", descriptor, re.IGNORECASE)])
    constraints = structure.get("constraints")
    if isinstance(constraints, dict):
        candidates.extend([constraints.get("unique_key"), constraints.get("unique")])

    unique_constraints: list[tuple[str, ...]] = []
    for candidate in candidates:
        if not isinstance(candidate, list) or not candidate:
            continue
        groups = [candidate] if all(isinstance(value, str) for value in candidate) else candidate
        for group in groups:
            if not isinstance(group, list) or not group or not all(isinstance(value, str) for value in group):
                continue
            constraint_columns = tuple(group)
            if any(column not in columns for column in constraint_columns):
                continue
            unique_constraints.append(constraint_columns)

    return list(dict.fromkeys(unique_constraints))


def foreign_key_is_valid(
    source_rows: list[dict[str, Any]],
    source_columns: tuple[str, ...],
    target_rows: list[dict[str, Any]],
    target_columns: tuple[str, ...],
) -> bool:
    target_values = {
        value_tuple(row, target_columns)
        for row in target_rows
        if all(row.get(column) is not None for column in target_columns)
    }
    for row in source_rows:
        values = value_tuple(row, source_columns)
        if any(value is None for value in values):
            continue
        if values not in target_values:
            return False
    return True


def build_sql(json_dir: Path, schema: str) -> tuple[str, int, int, int, int]:
    paths = sorted(json_dir.glob("*.json"))
    if not paths:
        raise RuntimeError(f"No JSON files found in {json_dir}")

    tables = [load_json_table(path) for path in paths]
    table_defs: dict[str, dict[str, Any]] = {}
    for table in tables:
        if table["name"] in table_defs:
            raise ValueError(f"Duplicate table name: {table['name']!r}")
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
        table_defs[table["name"]] = {
            "table": table,
            "rows": rows,
            "columns": columns,
            "column_types": column_types,
        }

    q_schema = quote_ident(schema)
    lines = [
        "\\set ON_ERROR_STOP on",
        "BEGIN;",
        f"CREATE SCHEMA IF NOT EXISTS {q_schema};",
        "DO $$",
        "DECLARE",
        "  obsolete_table record;",
        "BEGIN",
        "  FOR obsolete_table IN",
        "    SELECT tablename",
        "    FROM pg_tables",
        f"    WHERE schemaname = {quote_literal(schema)}",
        "      AND (",
        "        tablename ~ 'translations?'",
        "        OR tablename IN (",
        "          'reference_versions',",
        "          'chart_results',",
        "          'astral_chart_planet_dignity_results',",
        "          'astral_house',",
        "          'prediction_rulesets',",
        "          'astral_planets',",
        "          'astral_planet_definitions',",
        "          'astral_planet_natures',",
        "          'astral_planet_motion_states',",
        "          'astral_planet_sign_dignities',",
        "          'astral_planet_interpretation_profiles',",
        "          'astral_planet_category_weights',",
        "          'astral_planet_condition_signal_profiles',",
        "          'astral_prediction_daily_planet_profiles',",
        "          'astral_speed',",
        "          'astral_diginity_score_profiles',",
        "          'astral_structural_reference_catalog'",
        "        )",
        "      )",
        "  LOOP",
        "    EXECUTE format('DROP TABLE IF EXISTS %I.%I CASCADE',",
        f"      {quote_literal(schema)}, obsolete_table.tablename);",
        "  END LOOP;",
        "END $$;",
    ]

    for table in tables:
        lines.append(f"DROP TABLE IF EXISTS {q_schema}.{quote_ident(table['name'])} CASCADE;")

    total_rows = 0
    primary_keys: set[tuple[str, tuple[str, ...]]] = set()
    unique_keys: set[tuple[str, tuple[str, ...]]] = set()
    foreign_keys: list[tuple[str, tuple[str, ...], str, tuple[str, ...]]] = []
    skipped_foreign_keys = 0

    for table in tables:
        table_def = table_defs[table["name"]]
        rows = table_def["rows"]
        columns = table_def["columns"]
        column_types = table_def["column_types"]

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

        id_columns = ("id",)
        if "id" in columns and unique_non_null(rows, id_columns) and complete_non_null(rows, id_columns):
            primary_keys.add((table["name"], id_columns))

    for table_name, table_def in table_defs.items():
        for columns in extract_unique_constraints(table_def["table"], table_def["columns"]):
            if not unique_nulls_not_distinct(table_def["rows"], columns):
                raise ValueError(f"{table_name}: declared unique constraint is violated for columns {columns!r}")
            if (table_name, columns) not in primary_keys:
                unique_keys.add((table_name, columns))

    for source_name, source_def in table_defs.items():
        for source_columns, target_name, target_columns in extract_foreign_keys(
            source_def["table"],
            source_def["columns"],
        ):
            target_def = table_defs.get(target_name)
            if not target_def:
                skipped_foreign_keys += 1
                continue
            if any(column not in target_def["columns"] for column in target_columns):
                skipped_foreign_keys += 1
                continue
            if not unique_non_null(target_def["rows"], target_columns):
                skipped_foreign_keys += 1
                continue
            if not foreign_key_is_valid(
                source_def["rows"],
                source_columns,
                target_def["rows"],
                target_columns,
            ):
                skipped_foreign_keys += 1
                continue
            if (target_name, target_columns) not in primary_keys:
                unique_keys.add((target_name, target_columns))
            foreign_keys.append((source_name, source_columns, target_name, target_columns))

    for table_name, columns in sorted(primary_keys):
        name = constraint_name("pk", [table_name, *columns])
        col_sql = ", ".join(quote_ident(column) for column in columns)
        lines.append(
            f"ALTER TABLE {q_schema}.{quote_ident(table_name)} "
            f"ADD CONSTRAINT {quote_ident(name)} PRIMARY KEY ({col_sql});"
        )

    for table_name, columns in sorted(unique_keys):
        name = constraint_name("uq", [table_name, *columns])
        col_sql = ", ".join(quote_ident(column) for column in columns)
        lines.append(
            f"ALTER TABLE {q_schema}.{quote_ident(table_name)} "
            f"ADD CONSTRAINT {quote_ident(name)} UNIQUE NULLS NOT DISTINCT ({col_sql});"
        )

    for table_name, table_def in sorted(table_defs.items()):
        for check_name, expression in extract_check_constraints(table_def["table"]):
            name = constraint_name("ck", [table_name, check_name])
            lines.append(
                f"ALTER TABLE {q_schema}.{quote_ident(table_name)} "
                f"ADD CONSTRAINT {quote_ident(name)} CHECK ({expression});"
            )

        for columns in extract_indexes(table_def["table"], table_def["columns"]):
            name = constraint_name("idx", [table_name, *columns])
            col_sql = ", ".join(quote_ident(column) for column in columns)
            lines.append(
                f"CREATE INDEX IF NOT EXISTS {quote_ident(name)} "
                f"ON {q_schema}.{quote_ident(table_name)} ({col_sql});"
            )

    for source_name, source_columns, target_name, target_columns in foreign_keys:
        name = constraint_name("fk", [source_name, *source_columns, target_name, *target_columns])
        source_col_sql = ", ".join(quote_ident(column) for column in source_columns)
        target_col_sql = ", ".join(quote_ident(column) for column in target_columns)
        lines.append(
            f"ALTER TABLE {q_schema}.{quote_ident(source_name)} "
            f"ADD CONSTRAINT {quote_ident(name)} FOREIGN KEY ({source_col_sql}) "
            f"REFERENCES {q_schema}.{quote_ident(target_name)} ({target_col_sql});"
        )

        index_name = constraint_name("idx", [source_name, *source_columns])
        lines.append(
            f"CREATE INDEX IF NOT EXISTS {quote_ident(index_name)} "
            f"ON {q_schema}.{quote_ident(source_name)} ({source_col_sql});"
        )

    lines += ["COMMIT;", ""]
    return "\n".join(lines), len(tables), total_rows, len(foreign_keys), skipped_foreign_keys


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

    sql, table_count, row_count, foreign_key_count, skipped_foreign_key_count = build_sql(
        args.json_dir,
        args.schema,
    )
    if skipped_foreign_key_count:
        raise RuntimeError(
            f"Refusing to import with {skipped_foreign_key_count} skipped foreign keys."
        )
    if args.output:
        args.output.write_text(sql, encoding="utf-8")
    if args.dry_run:
        print(
            f"Generated SQL for {table_count} tables, {row_count} rows, "
            f"{foreign_key_count} foreign keys "
            f"({skipped_foreign_key_count} skipped)."
        )
        return

    env_values = read_env(DEFAULT_ENV_FILE)
    run_psql(sql, env_values)
    print(
        f"Imported {row_count} rows into {table_count} PostgreSQL tables "
        f"with {foreign_key_count} foreign keys "
        f"({skipped_foreign_key_count} skipped)."
    )


if __name__ == "__main__":
    main()
