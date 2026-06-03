"""Execute SQL against the project PostgreSQL (Docker Compose by default)."""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ENV_PATH = ROOT / ".env"


def read_env(path: Path = ENV_PATH) -> dict[str, str]:
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


def postgres_credentials() -> tuple[str, str]:
    env = read_env()
    user = os.environ.get("POSTGRES_USER") or env.get("POSTGRES_USER") or "postgres"
    db = os.environ.get("POSTGRES_DB") or env.get("POSTGRES_DB") or user
    return user, db


def database_url() -> str | None:
    env = read_env()
    return os.environ.get("DATABASE_URL") or env.get("DATABASE_URL")


def run_psql(sql: str) -> str:
    env = read_env()
    url = database_url()

    if shutil.which("psql") and url:
        result = subprocess.run(
            ["psql", url, "-v", "ON_ERROR_STOP=1", "-t", "-A", "-c", sql],
            capture_output=True,
            text=True,
            check=False,
            cwd=ROOT,
        )
        if result.returncode == 0:
            return result.stdout.strip()

    user, db = postgres_credentials()
    result = subprocess.run(
        [
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
            "-v",
            "ON_ERROR_STOP=1",
            "-t",
            "-A",
            "-c",
            sql,
        ],
        capture_output=True,
        text=True,
        check=False,
        cwd=ROOT,
    )
    if result.returncode != 0:
        stderr = result.stderr.strip() or result.stdout.strip()
        hint = (
            "Verifiez que le conteneur tourne: docker compose up -d "
            f"(service postgres, user={user!r}, db={db!r})."
        )
        print(stderr or "psql via docker compose a echoue", file=sys.stderr)
        print(hint, file=sys.stderr)
        raise RuntimeError(stderr or "psql failed")
    return result.stdout.strip()


def numeric_equals(value: str, expected: float, tolerance: float = 0.0001) -> bool:
    try:
        return abs(float(value) - expected) <= tolerance
    except ValueError:
        return False
