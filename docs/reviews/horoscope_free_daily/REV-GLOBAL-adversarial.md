# REV-GLOBAL — Horoscope Free Daily

## Checklist bloquante

- [x] Aucun nouveau moteur horoscope.
- [x] Aucun nouveau worker.
- [x] Aucune nouvelle table de jobs.
- [x] Aucun nouvel endpoint jobs.
- [x] `chart_calculation_id` obligatoire.
- [x] `birth_data` inline refuse par schema.
- [x] `day` uniquement interne.
- [x] `day`, `slot:day` et les codes techniques sont rejetes dans le texte public.
- [x] Reponse Free sans `slots` public.
- [x] Schemas internes verrouillent Basic a 3 slots et Free a 1 slot.
- [x] Service horoscope inconnu rejete avant construction de requete calculateur.
- [x] Basic non regresse par tests et goldens.
- [x] Smoke HTTP fake Basic passe.
- [x] Smoke HTTP fake Free passe.
- [x] Reviews adversariales documentees.

## Statut

Implementation fake locale couverte par tests unitaires, tests contrats/catalogue/jobs et smokes HTTP Docker.
Le catalogue expose `horoscope_free_daily` en `beta`.
