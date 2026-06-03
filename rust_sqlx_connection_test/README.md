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
`POSTGRES_*` (voir `.env.example`). Sans elles, le binaire echoue avec
`ASTRAL_BIRTH_DATETIME_UTC must be set`.

Depuis `rust_sqlx_connection_test/` :

```powershell
$env:ASTRAL_EPHEMERIS_PATH = "..\ephe\se-2026a"
$env:ASTRAL_BIRTH_DATETIME_UTC = "1990-01-02T03:04:05Z"
$env:ASTRAL_LATITUDE_DEG = "48.8566"
$env:ASTRAL_LONGITUDE_DEG = "2.3522"
$env:ASTRAL_REFERENCE_VERSION_ID = "1"
$env:ASTRAL_PRODUCT_CODE = "basic"
cargo run --features swisseph-engine -- --file
```

Ou ajouter ces lignes dans `../.env` puis lancer `cargo run --features swisseph-engine -- --file`.

`calc_ut` attend un Julian Day en UT/UTC. Les heures locales doivent donc etre
converties en UTC avant de construire `ASTRAL_BIRTH_DATETIME_UTC`.

Swiss Ephemeris est distribue en double licence AGPL ou licence professionnelle
Swiss Ephemeris. Verifier la licence avant toute distribution ou mise en service
publique.
