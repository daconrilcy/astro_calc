# REV-W00 plan

- Statut: closed
- Perimetre: gouvernance initiale, documentation cible, tests statiques de frontiere, creation du module `astrology/`.
- Invariants:
  - aucune dependance `domain -> infra`;
  - aucun raccourci runtime/DB dans les couches metier ciblees;
  - `simplified` et `horoscope` n'importent pas `crate::natal::aspects` ni `crate::natal::ephemeris`;
  - les calculs communs reutilisables vivent sous `astral_calculator/src/astrology/`.
- Risques attendus:
  - casser des imports publics historiques;
  - oublier un import interne interdit;
  - deplacer de la logique metier dans une facade de feature.
- Findings restants: Aucun.
