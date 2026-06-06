# REV-002 — Projection et preuves astrologiques

## Findings

| ID | Severity | Finding | Correction | Status |
|---|---|---|---|---|
| FREE-005 | Major | Le module horoscope utilisait des constantes Basic rigides pour slots, shortlist et scoring. | Ajout d'un contexte `service_code` et de fonctions parametrees par service. | fixed |
| FREE-006 | Major | Le slot `day` pouvait etre confondu avec une section publique. | `day` est autorise seulement dans la calculation et l'interpretation request; la reponse Free interdit `slots`. | fixed |
| FREE-007 | Major | Les preuves Free pouvaient ne pas etre alignees avec le slot interne. | Garde `required_evidence_keys` appliquee au slot `day`; evidence inventee rejetee. | fixed |
| FREE-008 | Medium | Le calculateur fake reemettrait toujours le service Basic. | Le calculateur reemet maintenant `request.service_code`. | fixed |

## Verification

- `horoscope_free_daily_interpretation_uses_single_internal_day_slot`
- `horoscope_free_daily_evidence_guard_rejects_invented_key`
- Golden Free calculation et interpretation ajoutes.
