-- Catalogue services integration API (source canonique)
-- Ops : scripts/manage_integration_services.ps1

CREATE TABLE IF NOT EXISTS llm_integration_services (
    service_code                  TEXT PRIMARY KEY,
    profile_code                  TEXT NOT NULL REFERENCES llm_interpretation_profiles(profile_code),
    product_code                  TEXT NOT NULL DEFAULT 'natal_prompter',

    label_fr                      TEXT NOT NULL,
    description_fr                TEXT NOT NULL,

    orchestration_mode            TEXT NOT NULL,
    calculation_mode              TEXT NOT NULL CHECK (calculation_mode IN (
        'none', 'simplified_natal', 'full_natal'
    )),

    service_request_contract      TEXT NOT NULL DEFAULT 'integration_job_request_v1',
    payload_contract              TEXT NOT NULL,
    service_response_contract     TEXT NOT NULL DEFAULT 'integration_job_status_v1',
    calculation_output_contract   TEXT NULL,
    reading_output_contract       TEXT NOT NULL,

    sync_endpoint                 TEXT NULL,
    async_endpoint                TEXT NOT NULL DEFAULT 'POST /v1/jobs',

    supports_async                BOOLEAN NOT NULL DEFAULT true,
    supports_sync_legacy          BOOLEAN NOT NULL DEFAULT false,
    supports_mercure              BOOLEAN NOT NULL DEFAULT false,

    availability                  TEXT NOT NULL CHECK (availability IN (
        'active', 'beta', 'planned', 'deprecated', 'disabled'
    )),

    example_request_json          JSONB,
    sort_order                    SMALLINT NOT NULL,
    updated_at                    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_llm_integration_services_availability
    ON llm_integration_services (availability, sort_order);
