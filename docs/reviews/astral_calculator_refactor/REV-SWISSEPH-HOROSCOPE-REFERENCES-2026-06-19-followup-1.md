Statut: closed

Objet:
- re-review adversariale apres corrections de la boucle initiale sur la vague Swiss Ephemeris + horoscope period + references.

Constat:
- la premiere version de la garde `swiss_ephemeris_lock_is_centralized` etait trop large et assimilait un appel legitime au helper canonique a une reintroduction de lock local.

Correction:
- resserrement de la detection sur les seules signatures de definition locale du lock (`fn swiss_ephemeris_lock`, `OnceLock<Mutex>`), sans interdire les appels au helper partage.

Verification:
- `cargo test -p astral_calculator --test refactor_governance_tests`
- `cargo test -p astral_calculator`

Findings restants: Aucun

Aucun finding ouvert.
