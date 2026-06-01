# Implementation du payload Basic runtime

Ce document decrit l'implementation actuelle du payload Basic dans le binaire Rust
`rust_sqlx_connection_test`.

## Objectif

L'etape 1A transforme le payload technique initial en payload Basic exploitable
par une future couche de generation texte.

Le runtime conserve la chaine existante :

1. calcul des faits astrologiques ;
2. ecriture des positions, cuspides et aspects calcules ;
3. aggregation des signaux ;
4. filtrage produit Basic ;
5. ecriture du payload final dans `astral_interpretation_generation_payloads`.

Cette etape ne produit pas encore une interpretation redactionnelle finale. Elle
prepare une entree propre, lisible et auditable.

## Fichiers concernes

- `rust_sqlx_connection_test/src/domain.rs` : structures runtime et payload JSON.
- `rust_sqlx_connection_test/src/facts.rs` : helpers de libelles signe/maison.
- `rust_sqlx_connection_test/src/ephemeris.rs` : enrichissement des positions calculees.
- `rust_sqlx_connection_test/src/aspects.rs` : detection des aspects avec libelles objet/aspect.
- `rust_sqlx_connection_test/src/signals.rs` : construction et filtrage des signaux Basic.
- `rust_sqlx_connection_test/src/payload.rs` : assemblage du payload final.
- `rust_sqlx_connection_test/src/repositories.rs` : enrichissement SQL et persistance.
- `rust_sqlx_connection_test/src/runtime.rs` : orchestration et regeneration des anciens payloads.

## Contrat des positions

Chaque position expose maintenant les champs lisibles en plus des IDs :

```json
{
  "object_code": "sun",
  "object_name": "Sun",
  "longitude_deg": 84.8759,
  "sign_id": 3,
  "sign_code": "gemini",
  "sign_name": "Gemini",
  "house_id": 9,
  "house_number": 9,
  "house_name": "Beliefs",
  "motion_state_id": 1
}
```

Les IDs restent presents pour l'audit et les relations DB. Les libelles viennent :

- du calcul runtime pour les nouveaux faits ;
- des joins `astral_signs` et `astral_houses` quand un payload est reconstruit depuis la DB.

## Contrat des signaux

Les signaux actifs du payload Basic sont limites a 12.

Un signal de position contient un titre lisible et les preuves techniques dans
`evidence` :

```json
{
  "signal_key": "object_position:sun",
  "title": "Sun in Gemini, house 9",
  "summary": "Sun is placed in Gemini and the Beliefs house, emphasizing this chart factor through a concrete, readable placement.",
  "priority_score": 100.0,
  "confidence_score": 0.95,
  "evidence": {
    "fact_type": "object_position",
    "chart_object_id": 1,
    "object_code": "sun",
    "sign_id": 3,
    "sign_code": "gemini",
    "house_id": 9,
    "house_number": 9,
    "longitude_deg": 84.8759
  }
}
```

Un signal d'aspect utilise les codes stables dans `signal_key`, mais pas dans le
texte utilisateur :

```json
{
  "signal_key": "aspect:sun:mercury:conjunction",
  "title": "Sun conjunction Mercury",
  "summary": "Sun and Mercury form a conjunction with 1.01 degrees of orb; the phase is separating.",
  "priority_score": 69.92,
  "confidence_score": 0.85,
  "evidence": {
    "fact_type": "aspect",
    "source_chart_object_id": 1,
    "source_object_code": "sun",
    "target_chart_object_id": 3,
    "target_object_code": "mercury",
    "aspect_id": 1,
    "aspect_code": "conjunction",
    "orb_deg": 1.0084,
    "phase_state": "separating",
    "strength_score": 0.874
  }
}
```

## Filtrage Basic

Le filtrage est applique dans `signals.rs` :

- les signaux sont tries par `priority_score` decroissant ;
- les aspects dont `strength_score < 0.4` passent en `suppressed` ;
- seuls les 12 premiers signaux actifs restent eligibles au payload ;
- `payload.rs` applique aussi `.take(12)` comme garde de lecture.

Les signaux supprimes restent persistables dans `astral_interpretation_signals`
avec `suppression_state = 'suppressed'`, mais ne remontent pas dans le payload
Basic final.

## Persistance

Le payload final est serialize et upserte dans :

`astral_interpretation_generation_payloads`

La contrainte fonctionnelle est :

```text
(chart_calculation_id, product_code, language_id)
```

Le runtime ecrit aussi les signaux dans `astral_interpretation_signals`.
Avant chaque reecriture des signaux d'un calcul, les signaux existants du meme
`chart_calculation_id` sont passes en `suppressed`. Les signaux recalcules sont
ensuite re-upsertes avec leur etat courant. Cela evite qu'un ancien signal actif
reste visible apres un changement de format de cle ou de filtrage.

Si un calcul idempotent est deja `completed`, le runtime tente de reutiliser le
payload existant. Il ne le reutilise que si le contrat enrichi est present :

- 12 signaux maximum ;
- positions avec `sign_code` et `sign_name` ;
- signaux avec `evidence`.

Sinon, les signaux sont reconstruits depuis les positions et aspects persistants,
puis le payload est reecrit.

## Verification

Depuis `rust_sqlx_connection_test` :

```powershell
cargo test
cargo test --features swisseph-engine
```

Run complet avec les valeurs d'exemple :

```powershell
$env:ASTRAL_BIRTH_DATETIME_UTC = "2024-06-15T12:00:00Z"
$env:ASTRAL_LATITUDE_DEG = "48.8566"
$env:ASTRAL_LONGITUDE_DEG = "2.3522"
$env:ASTRAL_EPHEMERIS_PATH = "..\ephe\se-2026a"
cargo run --features swisseph-engine
```

Le run attendu doit afficher :

- `product_code = "basic"` ;
- des positions avec `sign_code`, `sign_name`, `house_number`, `house_name` ;
- au plus 12 signaux ;
- des titres sans IDs techniques ;
- des IDs conserves dans `evidence` ;
- une ecriture/upsert dans `astral_interpretation_generation_payloads`.

## Limites connues

- L'Ascendant et le MC ne sont pas encore exposes comme objets de position Basic.
- Les resumes restent des phrases templatees, pas une interpretation finale.
- Le programme consomme les libelles des referentiels tels quels. Il ne gere pas la traduction.
- La redaction LLM doit rester une etape ulterieure.
