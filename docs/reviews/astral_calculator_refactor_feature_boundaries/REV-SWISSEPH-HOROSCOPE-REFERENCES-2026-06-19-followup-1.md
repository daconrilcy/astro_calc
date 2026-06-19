Statut: closed

Objet:
- re-review adversariale des frontieres apres correction de la garde Swiss Ephemeris.

Finding revu:
- la premiere version du test de gouvernance signalait a tort un simple appel a `with_swiss_ephemeris_lock(...)` comme un second lock local.

Correction:
- le test ne cible plus que la presence d'une definition locale de lock hors `astrology/swisseph_runtime.rs`.

Conclusion:
- le verrouillage reste bien centralise sous `astrology/`;
- `features/horoscope` reste sans `panic!`;
- aucun couplage de frontiere supplementaire n'a ete introduit.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
