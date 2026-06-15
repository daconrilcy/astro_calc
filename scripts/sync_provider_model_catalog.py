#!/usr/bin/env python3
"""Synchronise le catalogue provider/modele/caracteristiques LLM en base.

Strategie:
- tente la liste API officielle quand elle existe et qu'une cle est presente
- complete avec un catalogue officiel documente en dur pour prix/limites manquants
- upsert dans `llm_providers`, `llm_provider_models`, `llm_model_characteristics`

Le runtime reste alimente par la base; cette synchro prepare la base avant usage.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ENV_PATH = ROOT / ".env"


@dataclass
class ModelSeed:
    provider: str
    model: str
    display_name: str
    api_model_id: str
    model_code: str
    catalog_notes: str
    usage_tier_code: str | None
    max_context_tokens: int | None
    max_output_tokens: int | None
    supports_reasoning: bool
    supports_temperature: bool
    supports_streaming: bool
    supports_json_schema_strict: bool
    supports_json_object: bool
    structured_output_adapter: str
    storage_disable_supported: bool
    input_price_usd_per_mtok: float | None
    output_price_usd_per_mtok: float | None
    cache_read_price_usd_per_mtok: float | None
    cache_write_price_usd_per_mtok: float | None
    reasoning_price_usd_per_mtok: float | None
    source_kind: str
    source_ref: str


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


def load_env_into_process() -> None:
    for key, value in read_env().items():
        os.environ.setdefault(key, value)


def quote(value: str | None) -> str:
    if value is None:
        return "NULL"
    return "'" + value.replace("'", "''") + "'"


def sql_number(value: int | float | None) -> str:
    return "NULL" if value is None else str(value)


def database_url() -> str | None:
    return os.environ.get("DATABASE_URL") or read_env().get("DATABASE_URL")


def postgres_credentials() -> tuple[str, str]:
    env = read_env()
    user = os.environ.get("POSTGRES_USER") or env.get("POSTGRES_USER") or "postgres"
    db = os.environ.get("POSTGRES_DB") or env.get("POSTGRES_DB") or user
    return user, db


def run_psql(sql: str) -> str:
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
        raise RuntimeError(stderr or "psql failed")
    return result.stdout.strip()


def http_json(url: str, headers: dict[str, str]) -> dict | list:
    request = urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(request, timeout=20) as response:
        return json.loads(response.read().decode("utf-8"))


def official_seed_catalog() -> list[ModelSeed]:
    return [
        ModelSeed(
            provider="openai",
            model="gpt-5-mini",
            display_name="GPT-5 mini",
            api_model_id="gpt-5-mini",
            model_code="gpt-5-mini",
            catalog_notes="Mini frontier pour production et repairs.",
            usage_tier_code="production_candidate",
            max_context_tokens=400_000,
            max_output_tokens=128_000,
            supports_reasoning=True,
            supports_temperature=True,
            supports_streaming=True,
            supports_json_schema_strict=True,
            supports_json_object=True,
            structured_output_adapter="openai_responses_text_format",
            storage_disable_supported=True,
            input_price_usd_per_mtok=0.25,
            output_price_usd_per_mtok=2.0,
            cache_read_price_usd_per_mtok=0.025,
            cache_write_price_usd_per_mtok=None,
            reasoning_price_usd_per_mtok=None,
            source_kind="official_docs",
            source_ref="https://openai.com/api/pricing/",
        ),
        ModelSeed(
            provider="openai",
            model="gpt-5-nano",
            display_name="GPT-5 nano",
            api_model_id="gpt-5-nano",
            model_code="gpt-5-nano",
            catalog_notes="Sous-taches, validation, fort volume.",
            usage_tier_code="subtask_candidate",
            max_context_tokens=400_000,
            max_output_tokens=128_000,
            supports_reasoning=True,
            supports_temperature=True,
            supports_streaming=True,
            supports_json_schema_strict=True,
            supports_json_object=True,
            structured_output_adapter="openai_responses_text_format",
            storage_disable_supported=True,
            input_price_usd_per_mtok=0.05,
            output_price_usd_per_mtok=0.4,
            cache_read_price_usd_per_mtok=0.005,
            cache_write_price_usd_per_mtok=None,
            reasoning_price_usd_per_mtok=None,
            source_kind="official_docs",
            source_ref="https://openai.com/api/pricing/",
        ),
        ModelSeed(
            provider="openai",
            model="gpt-4.1",
            display_name="GPT-4.1",
            api_model_id="gpt-4.1",
            model_code="gpt-4.1",
            catalog_notes="Baseline non-reasoning encore supportee.",
            usage_tier_code="baseline",
            max_context_tokens=1_000_000,
            max_output_tokens=32_000,
            supports_reasoning=False,
            supports_temperature=True,
            supports_streaming=True,
            supports_json_schema_strict=True,
            supports_json_object=True,
            structured_output_adapter="openai_responses_text_format",
            storage_disable_supported=True,
            input_price_usd_per_mtok=2.0,
            output_price_usd_per_mtok=8.0,
            cache_read_price_usd_per_mtok=0.5,
            cache_write_price_usd_per_mtok=None,
            reasoning_price_usd_per_mtok=None,
            source_kind="official_docs",
            source_ref="https://openai.com/api/pricing/",
        ),
        ModelSeed(
            provider="anthropic",
            model="claude-sonnet-4-20250514",
            display_name="Claude Sonnet 4",
            api_model_id="claude-sonnet-4-20250514",
            model_code="claude-sonnet-4-20250514",
            catalog_notes="Anthropic Sonnet 4.",
            usage_tier_code="production_candidate",
            max_context_tokens=200_000,
            max_output_tokens=8_192,
            supports_reasoning=False,
            supports_temperature=True,
            supports_streaming=True,
            supports_json_schema_strict=True,
            supports_json_object=True,
            structured_output_adapter="anthropic_output_config_format",
            storage_disable_supported=False,
            input_price_usd_per_mtok=3.0,
            output_price_usd_per_mtok=15.0,
            cache_read_price_usd_per_mtok=0.3,
            cache_write_price_usd_per_mtok=3.75,
            reasoning_price_usd_per_mtok=None,
            source_kind="official_docs",
            source_ref="https://platform.claude.com/docs/en/about-claude/pricing",
        ),
        ModelSeed(
            provider="mistral",
            model="mistral-large-latest",
            display_name="Mistral Large Latest",
            api_model_id="mistral-large-latest",
            model_code="mistral-large-latest",
            catalog_notes="Mistral Large latest API alias.",
            usage_tier_code="production_candidate",
            max_context_tokens=128_000,
            max_output_tokens=8_192,
            supports_reasoning=False,
            supports_temperature=True,
            supports_streaming=True,
            supports_json_schema_strict=True,
            supports_json_object=True,
            structured_output_adapter="mistral_response_format_json_schema",
            storage_disable_supported=False,
            input_price_usd_per_mtok=2.0,
            output_price_usd_per_mtok=6.0,
            cache_read_price_usd_per_mtok=None,
            cache_write_price_usd_per_mtok=None,
            reasoning_price_usd_per_mtok=None,
            source_kind="official_docs",
            source_ref="https://mistral.ai/pricing/",
        ),
    ]


def fetch_openai_models() -> set[str]:
    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        return set()
    try:
        payload = http_json(
            "https://api.openai.com/v1/models",
            {"Authorization": f"Bearer {api_key}"},
        )
    except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError):
        return set()
    data = payload.get("data", []) if isinstance(payload, dict) else []
    return {item.get("id") for item in data if isinstance(item, dict) and item.get("id")}


def upsert_provider(provider_code: str, label_fr: str, sort_order: int) -> None:
    sql = f"""
INSERT INTO llm_providers (provider_code, label_fr, sort_order, is_active)
VALUES ({quote(provider_code)}, {quote(label_fr)}, {sort_order}, true)
ON CONFLICT (provider_code) DO UPDATE SET
    label_fr = EXCLUDED.label_fr,
    sort_order = EXCLUDED.sort_order,
    is_active = true,
    updated_at = NOW();
"""
    run_psql(sql)


def upsert_model(seed: ModelSeed) -> None:
    sql = f"""
WITH provider_row AS (
    SELECT id FROM llm_providers WHERE provider_code = {quote(seed.provider)}
), model_upsert AS (
    INSERT INTO llm_provider_models (
        provider, provider_id, model, model_code, display_name, api_model_id, catalog_notes,
        usage_tier_code, supports_json_schema_strict, supports_json_object,
        supports_reasoning_effort, supports_streaming, max_input_tokens, max_output_tokens,
        structured_output_adapter, storage_disable_supported, is_active, supports_temperature
    )
    SELECT
        {quote(seed.provider)}, id, {quote(seed.model)}, {quote(seed.model_code)},
        {quote(seed.display_name)}, {quote(seed.api_model_id)}, {quote(seed.catalog_notes)},
        {quote(seed.usage_tier_code)}, {str(seed.supports_json_schema_strict).lower()},
        {str(seed.supports_json_object).lower()}, {str(seed.supports_reasoning).lower()},
        {str(seed.supports_streaming).lower()}, {sql_number(seed.max_context_tokens)},
        {sql_number(seed.max_output_tokens)}, {quote(seed.structured_output_adapter)},
        {str(seed.storage_disable_supported).lower()}, true, {str(seed.supports_temperature).lower()}
    FROM provider_row
    ON CONFLICT (provider, model) DO UPDATE SET
        provider_id = EXCLUDED.provider_id,
        model_code = EXCLUDED.model_code,
        display_name = EXCLUDED.display_name,
        api_model_id = EXCLUDED.api_model_id,
        catalog_notes = EXCLUDED.catalog_notes,
        usage_tier_code = EXCLUDED.usage_tier_code,
        supports_json_schema_strict = EXCLUDED.supports_json_schema_strict,
        supports_json_object = EXCLUDED.supports_json_object,
        supports_reasoning_effort = EXCLUDED.supports_reasoning_effort,
        supports_streaming = EXCLUDED.supports_streaming,
        max_input_tokens = EXCLUDED.max_input_tokens,
        max_output_tokens = EXCLUDED.max_output_tokens,
        structured_output_adapter = EXCLUDED.structured_output_adapter,
        storage_disable_supported = EXCLUDED.storage_disable_supported,
        is_active = true,
        supports_temperature = EXCLUDED.supports_temperature,
        updated_at = NOW()
    RETURNING id
), current_model AS (
    SELECT id FROM model_upsert
    UNION ALL
    SELECT id FROM llm_provider_models
    WHERE provider = {quote(seed.provider)} AND model = {quote(seed.model)}
    LIMIT 1
)
UPDATE llm_model_characteristics
SET is_current = false
WHERE model_id IN (SELECT id FROM current_model)
  AND is_current = true;

INSERT INTO llm_model_characteristics (
    model_id, max_context_tokens, max_output_tokens, supports_reasoning,
    supports_temperature, supports_streaming, supports_json_schema_strict, supports_json_object,
    structured_output_adapter, storage_disable_supported,
    input_price_usd_per_mtok, output_price_usd_per_mtok,
    cache_read_price_usd_per_mtok, cache_write_price_usd_per_mtok, reasoning_price_usd_per_mtok,
    pricing_currency, source_kind, source_ref, observed_at, is_current
)
SELECT
    id, {sql_number(seed.max_context_tokens)}, {sql_number(seed.max_output_tokens)},
    {str(seed.supports_reasoning).lower()}, {str(seed.supports_temperature).lower()},
    {str(seed.supports_streaming).lower()}, {str(seed.supports_json_schema_strict).lower()},
    {str(seed.supports_json_object).lower()}, {quote(seed.structured_output_adapter)},
    {str(seed.storage_disable_supported).lower()},
    {sql_number(seed.input_price_usd_per_mtok)}, {sql_number(seed.output_price_usd_per_mtok)},
    {sql_number(seed.cache_read_price_usd_per_mtok)}, {sql_number(seed.cache_write_price_usd_per_mtok)},
    {sql_number(seed.reasoning_price_usd_per_mtok)},
    'USD', {quote(seed.source_kind)}, {quote(seed.source_ref)}, NOW(), true
FROM llm_provider_models
WHERE provider = {quote(seed.provider)} AND model = {quote(seed.model)}
ON CONFLICT DO NOTHING;
"""
    run_psql(sql)


def main() -> int:
    load_env_into_process()
    if not database_url():
        print("DATABASE_URL absent (.env ou env process).", file=sys.stderr)
        return 1

    providers = [
        ("fake", "Fournisseur de test", 0),
        ("openai", "OpenAI", 10),
        ("anthropic", "Anthropic", 20),
        ("mistral", "Mistral", 30),
    ]
    for provider_code, label_fr, sort_order in providers:
        upsert_provider(provider_code, label_fr, sort_order)

    api_visible_openai = fetch_openai_models()
    seeds = []
    for seed in official_seed_catalog():
        if seed.provider == "openai" and api_visible_openai and seed.api_model_id not in api_visible_openai:
            continue
        seeds.append(seed)

    for seed in seeds:
        upsert_model(seed)
        print(
            f"OK {seed.provider}/{seed.model} "
            f"(source={seed.source_kind}, ref={seed.source_ref})"
        )

    print("Catalogue providers/modeles synchronise.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
