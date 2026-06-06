# REV-014 — Review post-PR2 (OpenAI quality)

Date : 2026-06-06  
Statut : **CLOSED**

## Implémenté

| Item | Statut |
|------|--------|
| `Assert-SimplifiedStrictOpenAiQuality` (-UseReal) | **OK** — P0/P1 JSON astro_basis, regex affirmatives, longueurs 120–650 |
| Artefacts horodatés `output/natal_simplified_openai/{ts}/` | **OK** |
| `quality_summary.json` | **OK** |
| Fake reste gate CI | **OK** — E2E fake 24/24 |

## Gate REV-014 (exécuté)

```powershell
.\scripts\test_natal_simplified_e2e.ps1 -UseReal -SubmitProfile -TimeoutSec 900
```

**Résultat : 2026-06-06T10:05:15Z** — exit 0, **7/7** lectures OpenAI, **0** failure `strict:`.

| Cas | `run_id` | Mots |
|-----|----------|------|
| `date_only` | `1d1082a1-7d4e-4b96-94ea-989f7ad4ae15` | 263 |
| `date_with_location_without_timezone` | `334960bf-deb7-42fc-99a5-60125d87359b` | 286 |
| `date_with_timezone_without_time` | `2d86753b-b0c9-450b-b563-0e9ebb03f5e4` | 244 |
| `date_with_location_and_timezone_without_time` | `a174eecf-b794-4d67-9674-03fd2176122b` | 357 |
| `datetime_without_location` | `b58ae7ed-4030-4192-96e4-0c18f4895371` | 309 |
| `complete_birth_data` | `1006bb48-8baa-4e0b-b362-753555469437` | 319 |
| `date_only_equinox_window` | `6bbc98b7-9f38-4c3e-bb79-f2e03d45447f` | 250 |

Artefacts : `output/natal_simplified_openai/2026-06-06T100348Z/` (`quality_summary.json` : P0=0, P1=0).

## Checklist adversariale (post-smoke)

| ID | Vérification | Statut |
|----|--------------|--------|
| R14-01 | Mentions limitatives ASC autorisées | **OK** |
| R14-02 | astro_basis vérifié en JSON uniquement | **OK** |
| R14-03 | Cas équinoxe `ambiguous_core_identity` | **OK** |
| R14-08 | Doc recette = script réel | **OK** |

Gate REV-014 : **CLOSED**.
