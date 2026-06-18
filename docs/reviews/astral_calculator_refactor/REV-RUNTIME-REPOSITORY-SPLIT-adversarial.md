# REV-RUNTIME-REPOSITORY-SPLIT

Statut: closed

## Scope

Audit adversarial du découpage repository prévu par le plan.

## Findings initiaux

- Medium: `runtime_repository.rs` reste volumineux et doit être découpé avec prudence à cause de sa surface SQL.
- Low: déplacer toute la couche SQL dans la même vague que les changements horoscope augmenterait le risque de régression sans gain fonctionnel immédiat.

## Corrections

- La vague ajoute les garde-fous de comportement avant le découpage physique: sources fake interdites, transit partagé, `shared` purifié.
- Le découpage repository est conservé comme étape suivante documentée, à réaliser mécaniquement par familles de queries avec tests runtime inchangés.
- Aucun nouveau couplage SQL n'a été ajouté hors `infra/db`.

## Re-review

Aucun finding ouvert.
