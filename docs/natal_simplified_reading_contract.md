# Contrat lecture natal simplifiée (produit)

Document de référence produit — règles de vérité avant implémentation code.

## Objectif

Produire une **interprétation simplifiée** (aussi : lecture partielle, lecture indicative, profil astrologique essentiel) à partir de données de naissance incomplètes, **sans simuler** Ascendant, maisons, secte ni placements en maisons lorsque l'entrée ne le permet pas.

## Quatre notions

| Notion | Rôle |
|--------|------|
| `input_precision` | Ce que l'utilisateur a fourni (audit + UX) |
| `computed_scope` | Ce que le moteur peut calculer honnêtement |
| `reliability` | Fiabilité de chaque fait astrologique |
| Contrôles LLM | `allowed_fact_codes`, `blocked_interpretation_fact_codes`, etc. |

## Matrice `input_precision` × `computed_scope`

| Entrée | `input_precision.level` | `computed_scope` |
|--------|-------------------------|------------------|
| Date seule | `date_only` | `stable_birth_date_profile` |
| Date + lieu sans timezone | `date_with_location_without_timezone` | `stable_birth_date_profile` |
| Date + timezone sans heure | `date_with_timezone_without_time` | `stable_birth_date_profile` |
| Date + lieu + timezone sans heure | `date_with_location_and_timezone_without_time` | `stable_birth_date_profile` |
| Date + heure + timezone sans lieu | `datetime_without_location` | `planetary_positions` |
| Date + heure + timezone + lieu | `complete_birth_data` | `angular_chart` |

## Règles fuseau V1

- **Pas** de résolution automatique fuseau ← coordonnées.
- Lieu sans `timezone` explicite **n'améliore pas** la fenêtre temporelle.
- `timezone` IANA explicite requis dès qu'une heure est fournie.

## Fenêtre d'incertitude

| Mode | Fenêtre UTC |
|------|-------------|
| Date seule sans timezone | ~50 h (UTC+14 → UTC-12 pour la date civile) |
| Date + timezone sans heure | 24 h journée locale |
| Date + heure + timezone | Instant déclaré |

Échantillonnage : 60 min (canonique DB), toujours `start_utc` et `end_utc` inclus.

## Limitations vs effets

- `limitations[]` = **causes** uniquement (`birth_time_missing`, etc.)
- `excluded_features[]` = **effets** (`ascendant`, `houses`, `sect`, `house_placements`)

## Wording UX public

Utiliser : interprétation simplifiée, lecture partielle, lecture indicative, profil astrologique essentiel.

Ne **pas** exposer : dégradée, `degraded`, `minimum_reading_level: degraded`.

Contrat technique API : `reading_completeness: partial | simplified`.

## Entrée API — lieu calculatoire

- `birth.location` : lat/lon obligatoires si présent → sinon 400.
- Libellé UX non calculatoire : `input_metadata.location_label` uniquement.

## Miroir legacy `planets{}`

Smoke `natal_light` uniquement. Rempli seulement si `reliability ∈ [stable_across_uncertainty_window, calculated_from_declared_datetime]`.

## Hiérarchie scopes

```text
stable_birth_date_profile < planetary_positions < angular_chart < full_natal
```

`natal_structured_v13` reste réservé au thème complet (`full_natal`).
