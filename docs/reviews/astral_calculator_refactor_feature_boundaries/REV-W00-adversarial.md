# REV-W00 adversarial

- Statut initial: closed
- Findings:
  - P1: imports interdits `crate::natal::aspects` et `crate::natal::ephemeris` encore presents dans `simplified` et `horoscope`.
  - P2: absence de test statique verrouillant les nouveaux invariants de frontiere.
  - P2: absence de documentation explicite de la cible `astrology/` dans les regles de workspace.
