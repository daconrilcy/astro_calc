# REV-017 — Review adversariale post-corrections mineures

Date : 2026-06-06  
Périmètre : 4 écarts mineurs (schéma deprecated, quality_summary dynamique, règles réouverture, REV-012/AGENTS)

## Verdict

**Gate : OK** après corrections R17-01…R17-06.

## Findings

| ID | Sév. | Finding | Statut |
|----|------|---------|--------|
| R17-01 | P0 | `Resolve-SimplifiedOpenAiModelFromCatalog` lisait `product_code`/`role` absents de `GET /v1/providers` → `model` toujours null | **Fixed** — `default_model` + fallback premier modèle OpenAI |
| R17-02 | P0 | `Export-SimplifiedQualitySummary` appelé par E2E sans dot-source → commande introuvable en `-UseReal` | **Fixed** — import `simplified_natal_assertions.ps1` dans `test_natal_simplified_e2e.ps1` |
| R17-03 | P1 | `quality_summary.json` généré seulement si `-NoSaveOutputs` absent | **Fixed** — export hors bloc `$saveOutputs` |
| R17-04 | P1 | `-UseReal -NoSaveOutputs` : répertoire `OutputDir` absent → échec écriture métriques | **Fixed** — création `$OutputDir` si `$UseReal` |
| R17-05 | P1 | Cas HTTP en échec exclus de `quality_metrics` → `cases` ≠ 7 | **Fixed** — enregistrement échec HTTP dans métriques |
| R17-06 | P1 | Regex sévérité P1 trop large (`summary` substring) | **Fixed** — motifs explicites |
| R17-07 | P2 | Message `quality_summary.json` affiché même si fichier absent | **Fixed** — affichage conditionnel |
| R17-08 | P2 | OpenAPI LLM sans mention `partial` / `simplified` reserved | **Fixed** — description endpoint orchestré |
| R17-09 | — | `x-deprecated-enum-values` : extension JSON Schema, pas YAML OpenAPI calculateur (plan optionnel) | **Accepted** — aligné schémas publiés + contrat lecture |
| R17-10 | — | Règles réouverture : critères qualitatifs non automatisés | **Accepted** — monitoring manuel documenté |

## Améliorations livrées

- `quality_summary.json` : champ `gate_passed` (P0=0, P1=0, success=cases).
- Warnings P2 conservés (body/summary proches des seuils) sans faire échouer la recette.

## Gate REV-017

```powershell
cargo test -p astral_llm_api --test contracts_publish_tests
.\scripts\test_natal_simplified_e2e.ps1 -NoSaveOutputs
```

Gate REV-017 : **OK**.
