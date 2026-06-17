# REV-W02 adversarial

- Statut: closed
- Findings controles:
  - P1 potentiel: `simplified` ou `horoscope` importe encore `crate::natal::ephemeris`.
  - P1 potentiel: appels internes nouveaux utilisent encore `calculate_natal`.
  - P2 potentiel: `astrology/ephemeris.rs` depend de `infra/db` ou ouvre une connexion DB.
- Resultat:
  - les services `natal`, `simplified` et `horoscope` appellent `calculate_chart`;
  - le wrapper `calculate_natal` delegue a `calculate_chart`;
  - aucun import `infra/db` n'est present dans `astrology/ephemeris.rs`.
- Findings restants: Aucun.
