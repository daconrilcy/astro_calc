# REV-W01 adversarial

- Statut: closed
- Findings controles:
  - P1 potentiel: logique d'aspects reste couplee a `natal/aspects.rs`.
  - P2 potentiel: `simplified` ou `horoscope` importe `crate::natal::aspects`.
  - P2 potentiel: regression de detection des aspects structuraux d'angles.
- Resultat:
  - `astral_calculator/src/natal/aspects.rs` re-exporte `crate::astrology::aspects::*`;
  - les imports interdits ne sont pas presents;
  - les tests `aspects_tests` et `payload_tests` restent couverts par `cargo test -p astral_calculator`.
- Findings restants: Aucun.
