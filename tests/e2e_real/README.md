# Real Docker E2E tests

Ces scripts supposent que l'application tourne deja via Docker Compose :

```powershell
docker compose up -d --build
.\scripts\docker_bootstrap.ps1
.\tests\e2e_real\run_real_e2e.ps1
```

Ils appellent les services HTTP publies sur `http://127.0.0.1:8080` et
`http://127.0.0.1:8081`.

## Analyse des scripts existants

Les scripts E2E existaient deja, mais ils etaient melanges avec les scripts de
bootstrap, de seed et de verification golden dans `scripts/` :

- `scripts/docker_compose_smoke.ps1` : smoke calculateur -> LLM fake.
- `scripts/test_integration_jobs_e2e.ps1` : service async `natal_simplified`.
- `scripts/test_natal_from_birth_e2e.ps1` : service async `natal_basic`.
- `scripts/test_horoscope_basic_daily_fake.ps1` : service async horoscope.
- `scripts/docker_premium_openai_e2e.ps1` : parcours OpenAI premium optionnel.

Manques couverts par ce dossier :

- regroupement dedie des E2E reels HTTP Docker ;
- couverture directe du calculateur `POST /v1/calculations/horoscope/daily-natal` ;
- verification dynamique de tous les services `active` / `beta` exposes par
  `GET /v1/services` ;
- runner unique pour lancer toute la suite E2E reelle.

## Scripts

- `01_calculator_services.e2e.ps1` : endpoints calculateur, schemas,
  validation, natal complet, natal simplifie, horoscope calculateur.
- `03_integration_catalog_services.e2e.ps1` : catalogue public et jobs async
  pour chaque service `active` ou `beta`.
- `04_horoscope_premium_daily.e2e.ps1` : service async
  `horoscope_premium_daily_local_2h_slots`, avec verification du contrat, du
  payload Premium local, de `timeline[12]`, de `local_chart` et des slots
  `best/watch`. Le script produit aussi un JSON complet et un Markdown lisible
  du texte reellement genere dans `output/horoscope_premium_daily_real/` par
  defaut, ou dans le dossier fourni via `-OutputDir`.
- `run_real_e2e.ps1` : lance la suite principale jobs/gateway et produit un
  rapport Markdown sous `output/e2e_real_reports/` par defaut.

Les anciennes suites sync ont ete retirees du parcours de test courant.

## Diagnostics

Le runner cree aussi un dossier `output/e2e_real_reports/diagnostics_<timestamp>/`.

- `powershell_transcript.log` contient toute la sortie console de la suite.
- En cas d'echec, les derniers logs Docker sont sauvegardes pour
  `astral_llm_worker`, `astral_llm_api`, `astral_calculator_api` et `postgres`.
- Les timeouts de job incluent le `run_id` et le dernier payload retourne par
  `GET /v1/jobs/{run_id}`.
