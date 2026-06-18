# Review adversariale - frontieres payload/projection reasons v14

Date: 2026-06-18
Statut: closed

## Perimetre

Validation des frontieres de couche pour la vague `natal_structured_v14`:
raisons typees `BasicProjectionReason`, referentiel DB des reason definitions,
validation payload et projection LLM stable.

Hors perimetre: changement de contrat public `llm_projection_natal_v1`,
traductions non anglaises, suppression du schema historique v13.

## Cycle 1

### Findings

- P1 - Le passage de `Vec<String>` a `Vec<BasicProjectionReason>` pouvait
  laisser un fallback implicite dans la projection LLM si une definition
  referentielle manquait, ce qui aurait recree de la logique metier hors DB.
- P1 - Le runtime pouvait devenir dependant d'un seed de test si le
  referentiel `astral_projection_reason_definitions` n'etait pas charge par le
  catalogue DB courant.
- P2 - Le changement de contrat payload pouvait fuir vers
  `llm_projection_natal_v1` si les goldens, l'enveloppe engine et les helpers
  de projection etaient alignes sur v13 par inertie.

### Corrections

- Le catalogue `BasicPayloadCatalog` charge maintenant les
  `ProjectionReasonDefinition` depuis la DB et expose une resolution par
  `reason_code`.
- `test_catalog()` ne garde qu'un fallback de test strictement identique au
  seed `json_db/astral_projection_reason_definitions.json`; aucun fallback
  runtime silencieux n'est introduit.
- Les builders natal et house axes emettent exclusivement des
  `BasicProjectionReason` structures, avec deduplication structurelle.
- La validation payload rejette les reasons inconnues, inactives ou
  incompletes selon `requires_*`, ce qui empeche la projection de compenser une
  derive de referentiel.
- La projection LLM rend les `supporting_factors` depuis les templates
  referentiels tout en conservant le contrat public
  `llm_projection_natal_v1`.

## Cycle 2

### Findings

Aucun finding ouvert.

### Verification adversariale

- Les payloads/goldens courants utilisent `natal_structured_v14`.
- Aucun code combinatoire legacy (`jupiter_exaltation`,
  `sun_luminary_in_house`, `moon_luminary_in_house`) n'est requis pour rendre
  les facteurs lisibles.
- Les frontieres restent nettes: builders -> payload type -> validation ->
  projection, avec definitions canoniques chargees depuis la base.

## Conclusion

Aucun finding ouvert.
