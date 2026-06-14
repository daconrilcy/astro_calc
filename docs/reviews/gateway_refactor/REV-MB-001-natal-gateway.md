# REV-MB-001 Natal Gateway

Statut : closed after corrections

Findings initiaux :

- P1 absence de facade v2 pour `natal` par gamme `free/basic/premium`
- P1 couplage direct ancien entre orchestration `natal` et dispatch legacy
- P2 contrats publics v2 inexistants
- P2 absence de tests dedies a la policy `simplified/full` et `free/basic/premium`

Corrections appliquees :

- ajout des endpoints `v2/natal/simplified/*` et `v2/natal/full/*`
- introduction de `GenerateNatalReadingUseCase`
- mapping typed public -> calculator -> llm -> public
- policies `projection_level`, `audience_level`, `interpretation_profile_code`
- tests racine dedies a la gateway natal v2

Risques residuels :

- les flux `horoscope` n'ont pas encore ete migres vers la gateway
- le dispatch legacy global n'est pas encore supprime
