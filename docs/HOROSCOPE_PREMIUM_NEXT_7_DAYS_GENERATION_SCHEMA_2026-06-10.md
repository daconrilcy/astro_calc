# Generation Horoscope Premium 7 Days - 2026-06-10

Schema Mermaid du flux de generation du service `horoscope_premium_next_7_days_natal`, avec les fichiers principaux impliques.

```mermaid
flowchart TD
    A["Payload public<br/>service_code = horoscope_premium_next_7_days_natal<br/><br/>astral_llm_api/src/integration_routes.rs<br/>contracts/llm/horoscope_period_natal_request.schema.json"] --> B["Validation payload<br/>validate_period_public_request<br/><br/>astral_llm_application/src/horoscope/mod.rs"]
    B --> C["Build calculation request<br/>next_7_days + six_hour_7_days<br/><br/>astral_llm_application/src/horoscope/mod.rs<br/>json_db/horoscope_services.json<br/>json_db/horoscope_scan_profiles.json"]
    C --> D["Calculator API<br/><br/>astral_llm_infra/src/calculator_client.rs<br/>astral_calculator/src/runtime/service.rs"]
    D --> E["Calculation response<br/><br/>astral_calculator/src/horoscope/mod.rs<br/>contracts/calculator/horoscope_period_calculation_response.schema.json"]

    E --> E1["period_resolution<br/>7 dates incluses"]
    E --> E2["scan_plan<br/>28 snapshots<br/>00:00 / 06:00 / 12:00 / 18:00"]
    E --> E3["snapshots<br/>transits_to_natal + evidence_key"]

    E1 --> F["Build interpretation request<br/><br/>astral_llm_application/src/horoscope/mod.rs<br/>contracts/llm/horoscope_period_interpretation_request.schema.json"]
    E2 --> F
    E3 --> F

    F --> G["Extraction evidence<br/>period_evidence_from_snapshots<br/><br/>astral_llm_application/src/horoscope/mod.rs<br/>json_db/horoscope_natal_focus_labels.json"]
    G --> H["Scoring events<br/>build_period_events<br/><br/>astral_llm_application/src/horoscope/mod.rs"]
    H --> I["Plans editoriaux<br/><br/>astral_llm_application/src/horoscope/mod.rs"]
    I --> I1["daily_plans"]
    I --> I2["key_days"]
    I --> I3["best_days"]
    I --> I4["watch_days"]
    I --> I5["best_windows"]
    I --> I6["watch_windows"]
    I --> I7["domain_sections<br/><br/>C:/dev/astral_calculation/json_db/horoscope_period_public_themes.json"]
    I --> I8["strategy<br/><br/>C:/dev/astral_calculation/json_db/horoscope_detail_profiles.json"]
    I --> I9["editorial_brief<br/><br/>C:/dev/astral_calculation/json_db/horoscope_period_editorial_roles.json<br/>C:/dev/astral_calculation/json_db/horoscope_period_editorial_arcs.json"]

    I1 --> J["Prompt Premium LLM<br/>period_writer_messages<br/><br/>C:/dev/astral_calculation/astral_llm/crates/astral_llm_application/src/horoscope/mod.rs<br/><br/>Prompt exact = template + $.result.interpretation_request<br/>Le prompt n'est pas persiste comme fichier separe"]
    I2 --> J
    I3 --> J
    I4 --> J
    I5 --> J
    I6 --> J
    I7 --> J
    I8 --> J
    I9 --> J
    G --> J

    J --> K["Provider OpenAI<br/>structured JSON schema<br/><br/>astral_llm_application/src/horoscope/mod.rs<br/>contracts/llm/horoscope_period_response.schema.json"]
    K --> L["Raw horoscope_period_response<br/><br/>contracts/llm/horoscope_period_response.schema.json"]

    L --> M["Repair shape<br/>repair_period_response_shape<br/><br/>astral_llm_application/src/horoscope/mod.rs"]
    M --> N["Postprocess<br/>normalisation, tons publics,<br/>personnalisation, repetitions<br/><br/>astral_llm_application/src/horoscope/mod.rs<br/>C:/dev/astral_calculation/json_db/horoscope_period_style_variants.json"]
    N --> O["Validation schema<br/><br/>astral_llm_application/src/horoscope/mod.rs<br/>contracts/llm/horoscope_period_response.schema.json"]
    O --> P["Validation evidence guard<br/>validate_period_response_evidence<br/><br/>astral_llm_application/src/horoscope/mod.rs"]
    P --> Q["Validation qualite Premium<br/>windows, strategy, domains,<br/>word count, no leaks<br/><br/>astral_llm_application/src/horoscope/mod.rs"]

    Q --> R["Reponse finale<br/><br/>C:/dev/astral_calculation/output/horoscope_premium_period_real/horoscope_premium_next_7_days_real_20260610_113631.json"]
    R --> R1["calculation"]
    R --> R2["interpretation_request complet<br/><br/>JSON path: $.result.interpretation_request"]
    R --> R3["reading final complet<br/><br/>JSON path: $.result.reading"]
```

## Artefacts du run

- Run complet : `C:\dev\astral_calculation\output\horoscope_premium_period_real\horoscope_premium_next_7_days_real_20260610_113631.json`
- Reading final complet : `C:\dev\astral_calculation\output\horoscope_premium_period_real\horoscope_premium_next_7_days_real_20260610_113631.json`, JSON path `$.result.reading`.
- Interpretation request complet du meme run : `C:\dev\astral_calculation\output\horoscope_premium_period_real\horoscope_premium_next_7_days_real_20260610_113631.json`, JSON path `$.result.interpretation_request`.
- Prompt exact envoye au LLM : non persiste comme fichier separe pour ce run. Il est construit par `period_writer_messages` dans `C:\dev\astral_calculation\astral_llm\crates\astral_llm_application\src\horoscope\mod.rs` a partir de `$.result.interpretation_request`.

## Fichiers principaux

- `astral_llm/crates/astral_llm_api/src/integration_routes.rs` : entree HTTP integration et soumission du service.
- `astral_llm/crates/astral_llm_application/src/horoscope/mod.rs` : orchestration, construction des requetes, prompt, repair, postprocess et validations.
- `astral_llm/crates/astral_llm_infra/src/calculator_client.rs` : appel HTTP vers le calculateur.
- `astral_calculator/src/runtime/service.rs` : orchestration runtime du calculateur avec snapshots de transits.
- `astral_calculator/src/horoscope/mod.rs` : calcul period horoscope, snapshots, faits et `evidence_key`.
- `contracts/llm/horoscope_period_natal_request.schema.json` : contrat public d'entree LLM API.
- `contracts/llm/horoscope_period_interpretation_request.schema.json` : contrat de requete interne envoyee au writer LLM.
- `contracts/llm/horoscope_period_response.schema.json` : contrat de sortie de lecture.
- `contracts/calculator/horoscope_period_calculation_request.schema.json` : contrat de requete calculateur.
- `contracts/calculator/horoscope_period_calculation_response.schema.json` : contrat de reponse calculateur.
- `json_db/horoscope_services.json` : declaration du service Premium 7 days.
- `json_db/horoscope_scan_profiles.json` : profil `six_hour_7_days`.
- `C:\dev\astral_calculation\json_db\horoscope_detail_profiles.json` : profondeur Premium, limites de mots et sections activees.
- `C:\dev\astral_calculation\json_db\horoscope_natal_focus_labels.json` : libelles et scenes de personnalisation natale.
- `C:\dev\astral_calculation\json_db\horoscope_period_public_themes.json` : libelles publics, titres de domaines et fenetres.
- `C:\dev\astral_calculation\json_db\horoscope_period_editorial_roles.json` : roles editoriaux des jours.
- `C:\dev\astral_calculation\json_db\horoscope_period_editorial_arcs.json` : arcs editoriaux pour themes repetes.
- `C:\dev\astral_calculation\json_db\horoscope_period_style_variants.json` : variantes de style et termes a eviter.
