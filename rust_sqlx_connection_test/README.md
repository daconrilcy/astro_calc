# Rust SQLx connection test

Petit binaire Rust pour tester la connexion PostgreSQL du projet et lire la
table `astral_signs`.

## Execution

Depuis la racine du depot :

```powershell
docker compose up -d
cargo run --manifest-path rust_sqlx_connection_test/Cargo.toml
```

Le programme charge le fichier `.env` de la racine. Si `DATABASE_URL` n'est pas
definie, il construit automatiquement une URL depuis `POSTGRES_DB`,
`POSTGRES_USER`, `POSTGRES_PASSWORD` et `POSTGRES_PORT`.

## Swiss Ephemeris

Le moteur Swiss Ephemeris utilise le crate `swiss-eph` via la feature
`swisseph-engine`.

Smoke test sans fichiers `.se1`, en mode Moshier :

```powershell
cargo run --manifest-path rust_sqlx_connection_test/Cargo.toml --features swisseph-engine --bin swe_smoke
```

Execution du calcul natal avec les fichiers Swiss Ephemeris.

Le `.env` a la racine du depot doit contenir les variables `ASTRAL_*` en plus de
`POSTGRES_*` (voir `.env.example`).

### Sortie par defaut (4A)

`cargo run` appelle `calculate_natal_engine` et produit une enveloppe
`astro_engine_response_v1` avec `audit_payload` (v13 brut) et `llm_payload`
(projection LLM selon `ASTRAL_PROJECTION_LEVEL`, defaut `rich`).

Depuis `rust_sqlx_connection_test/` :

```powershell
$env:ASTRAL_EPHEMERIS_PATH = "..\ephe\se-2026a"
$env:ASTRAL_BIRTH_DATETIME_UTC = "1990-01-02T03:04:05Z"
$env:ASTRAL_BIRTH_TIMEZONE = "UTC"
$env:ASTRAL_LATITUDE_DEG = "48.8566"
$env:ASTRAL_LONGITUDE_DEG = "2.3522"
$env:ASTRAL_LOCATION_LABEL = "Paris, France"
$env:ASTRAL_PROJECTION_LEVEL = "rich"
$env:ASTRAL_PRODUCT_CODE = "basic"
cargo run --features swisseph-engine -- --file
```

Fichier genere : `output/astro_engine_response_YYYYMMDD_HHMMSS.json`.

Entree stricte 4A (date, heure, fuseau) :

```powershell
$env:ASTRAL_BIRTH_DATE = "1990-01-02"
$env:ASTRAL_BIRTH_TIME = "03:04:05"
$env:ASTRAL_BIRTH_TIMEZONE = "Europe/Paris"
```

### Sortie audit seule (legacy v13)

Pour les scripts golden v13 ou un export brut sans enveloppe :

```powershell
cargo run --features swisseph-engine -- --audit-only --file
```

Fichier genere : `output/basic_payload_YYYYMMDD_HHMMSS.json` (payload v13 direct).

`calc_ut` attend un Julian Day en UT/UTC. En mode `--audit-only`, fournir
`ASTRAL_BIRTH_DATETIME_UTC`. En mode 4A, preferer `ASTRAL_BIRTH_DATE` +
`ASTRAL_BIRTH_TIME` + `ASTRAL_BIRTH_TIMEZONE`.

Swiss Ephemeris est distribue en double licence AGPL ou licence professionnelle
Swiss Ephemeris. Verifier la licence avant toute distribution ou mise en service
publique.
