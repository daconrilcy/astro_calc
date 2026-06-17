# REV-E - Nettoyage Legacy

- Status: `open`
- Decision: `accept with reason`

## Findings

- Des re-exports legacy restent exposes dans `lib.rs` et `runtime/mod.rs`.

## Notes

- Le nettoyage doit rester guide par `rg` workspace et par la compatibilite publique avant suppression.
