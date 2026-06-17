# REV-IMPLEMENTATION-004-adversarial

- Statut: closed
- Portee auditee:
  - deplacement physique de `natal`, `simplified` et `horoscope` sous `src/features/`;
  - wrappers publics legacy conserves aux anciens chemins racine;
  - chemins internes rebranches vers `crate::features::*`;
  - gouvernance renforcee pour les frontieres produit et `calculate_chart`.

Findings:
- Aucun finding ouvert.

Notes de verification:
- Les contrats publics restent exposes via `astral_calculator::features::*`.
- Les anciens chemins `astral_calculator::{natal,simplified,horoscope}` restent des facades de compatibilite.
- Les calculs communs restent sous `astrology::*`.
