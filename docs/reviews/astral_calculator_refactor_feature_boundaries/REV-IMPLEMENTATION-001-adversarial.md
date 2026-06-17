# REV-IMPLEMENTATION-001 adversarial

- Statut: closed
- Perimetre: review adversariale de l'implementation de correction W0-W4.
- Findings:
  - P2: les nouveaux noms canoniques horoscope n'etaient pas testes directement; un export casse aurait pu passer tant que les anciens wrappers restaient verts.
  - P3: `REV-W00-adversarial.md` listait des findings historiques dans un fichier marque closed sans section de resolution explicite.
- Corrections:
  - ajout d'un test d'equivalence entre noms canoniques et wrappers historiques dans `tests/astral_calculator_api_tests.rs`;
  - utilisation directe des noms canoniques dans plusieurs tests horoscope existants;
  - clarification de `REV-W00-adversarial.md` avec `Findings initiaux` et `Resolution`.
- Findings restants: Aucun.
