# REV-SCHEMAS-CENTRALIZATION

- Status: `closed`
- Decision: `fix now`

## Scope

- Suppression de `astral_calculator/schemas`.
- Centralisation des schemas actifs calculateur dans `contracts/calculator`.
- Retrait des validations JSON Schema historiques v8 a v12.

## Findings fermes

- Les scripts historiques v8 a v12 appelaient encore des tests `external_payload_matches_json_schema_v*` supprimes.
- Les scripts historiques v9 a v12 regeneraient encore un payload avec le moteur courant, alors que les contrats historiques ne sont plus maintenus.
- Les scripts historiques ne recreaient plus leur repertoire de diff apres retrait du fichier temporaire de validation schema.
- La documentation affirmait encore que les validations schema historiques restaient disponibles.

## Conclusion

- Les findings adversariaux trouves sur cette vague ont ete corriges.
- Les schemas actifs sont servis et testes depuis `contracts/calculator`.
- Les scripts historiques v8 a v12 ne font plus que comparer une projection golden ou un fichier fourni explicitement.
