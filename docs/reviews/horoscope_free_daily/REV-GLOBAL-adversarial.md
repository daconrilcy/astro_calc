# REV-GLOBAL — Horoscope Free Daily

## Checklist bloquante

- [x] Aucun nouveau moteur horoscope.
- [x] Aucun nouveau worker.
- [x] Aucune nouvelle table de jobs.
- [x] Aucun nouvel endpoint jobs.
- [x] `chart_calculation_id` obligatoire.
- [x] `birth_data` inline refuse par schema.
- [x] `day` uniquement interne.
- [x] `day`, `slot:day` et les codes techniques sont rejetes dans le texte public.
- [x] Reponse Free sans `slots` public.
- [x] Schemas internes verrouillent Basic a 3 slots et Free a 1 slot.
- [x] Service horoscope inconnu rejete avant construction de requete calculateur.
- [x] Basic non regresse par tests et goldens.
- [x] Smoke HTTP fake Basic passe.
- [x] Smoke HTTP fake Free passe.
- [x] Reviews adversariales documentees.

## Statut

Validation completed.

Checks:

- Public response has no `slots`: PASS.
- No public leakage of `day` / `slot:day`: PASS.
- `advice` present: PASS.
- `evidence_keys` present and non-empty: PASS.
- `quality` present: PASS.
- Basic horoscope non-regression: PASS.
- Horoscope tests: PASS -- 45/45.
- French typography: PASS under current rule set.

Note:
Typographic apostrophe normalization (`'` -> `’`) is not currently part of the
blocking French typography rule. Current checks focus on broken elisions such as
`l impression` and punctuation issues such as `Conseil:`. This can be tracked as
a future editorial polish item, not a V1 blocker.

Decision:
`horoscope_free_daily` can remain in `beta` because all blocking structural,
evidence, public-shape, non-regression and fake/real output checks pass.
