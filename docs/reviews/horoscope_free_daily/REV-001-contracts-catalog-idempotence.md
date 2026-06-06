# REV-001 — Contrats, catalogue, idempotence

## Findings

| ID | Severity | Finding | Correction | Status |
|---|---|---|---|---|
| FREE-001 | Major | `horoscope_response_v1` etait verrouille sur le service Basic et ne pouvait pas valider une forme Free sans `slots`. | Schema passe en variantes conditionnelles par `service_code`. | fixed |
| FREE-002 | Major | Le payload Free risquait de dupliquer le payload Basic. | Ajout de `horoscope_daily_natal_request_v1` commun pour le service Free, en conservant le contrat Basic historique. | fixed |
| FREE-003 | Major | Un service `beta` sans orchestrateur pourrait etre persiste puis echouer au worker. | `service_has_v1_orchestrator` reconnait explicitement `horoscope_free_daily` avant le passage catalogue en `beta`. | fixed |
| FREE-004 | Medium | Le replay idempotent devait rester porte par `/v1/jobs` et le fingerprint existant. | Aucun mecanisme d'idempotence dedie n'a ete ajoute; le service utilise l'enveloppe integration existante. | fixed |
| FREE-012 | Major | Les schemas calcul/interpretion autorisaient encore des combinaisons incoherentes, par exemple Basic avec un seul slot ou Free avec trois slots. | Ajout de contraintes conditionnelles `service_code` -> nombre de slots sur les schemas calculation request/response et interpretation request. | fixed |
| FREE-013 | Major | Un `service_code` horoscope inconnu pouvait arriver jusqu'a la construction de requete et dependre seulement de l'absence de seeds. | Ajout d'une garde centrale `HOROSCOPE_SERVICE_NOT_IMPLEMENTED` avant construction calculation et chargement reference data. | fixed |

## Verification

- Tests de schema Free / Basic ajoutes dans `horoscope_v1_tests`.
- Tests adversariaux ajoutes pour Basic avec un seul slot, Free avec trois slots et service horoscope inconnu.
- Le conflit idempotence reste couvert par la couche integration existante; aucun fork horoscope n'a ete cree.
