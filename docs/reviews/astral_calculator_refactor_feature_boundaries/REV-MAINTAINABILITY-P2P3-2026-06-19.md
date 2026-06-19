# Review feature boundaries maintenabilite P2/P3 - 2026-06-19

Perimetre:

- split applicatif natal
- interdiction d’usage interne des aliases historiques
- maintien des frontieres `application` / `features` / `infra`

Verification:

- le split conserve `NatalCalculationService` comme unique point d’entrée public
- les helpers de reuse, snapshot et workflow restent en `pub(super)`
- aucun nouvel import `infra` n’a ete ajoute dans `domain`

Conclusion:

- Aucun finding ouvert.
