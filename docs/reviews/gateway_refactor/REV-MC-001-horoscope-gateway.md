# REV-MC-001 Horoscope Gateway

Statut : closed after corrections

Findings initiaux :

- P1 la gateway ne pilotait encore que `natal`, pas `horoscope`
- P1 la generation LLM `horoscope` restait seulement accessible via l'orchestrateur legacy
- P2 le client LLM gateway ne remontait pas correctement les erreurs HTTP non-2xx
- P2 le mapping des contrats inférés dans le catalogue pouvait mentir sur le sens request/response

Corrections appliquees :

- ajout des endpoints internes LLM `v1/internal/horoscope/daily/render` et `v1/internal/horoscope/period/render`
- ajout des use cases gateway `GenerateHoroscopeDailyReadingUseCase` et `GenerateHoroscopePeriodReadingUseCase`
- ajout des endpoints publics gateway :
  - `v2/horoscope/daily/free|basic|premium`
  - `v2/horoscope/period/free|basic|premium`
- validation de la chaine `public -> calculator -> llm -> public` avec tests golden
- durcissement des erreurs du client LLM gateway
- correction de l'inference des contrats typed du catalogue

Risques residuels :

- `UnifiedReadingOrchestrator` existe encore pour les jobs legacy et reste un point de dispatch historique
- les preparations calculator `horoscope` sont encore construites depuis `astral_llm_application` et n'ont pas encore ete deplacees vers `astral_calculator`
