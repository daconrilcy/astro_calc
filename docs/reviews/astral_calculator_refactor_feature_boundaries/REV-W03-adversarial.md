# REV-W03 adversarial

- Statut: closed
- Findings controles:
  - P1 potentiel: rupture des fonctions publiques consommees par les tests API.
  - P2 potentiel: renommage de termes `natal` qui font partie du contrat public horoscope.
  - P2 potentiel: appels internes continuent a privilegier les noms historiques ambigus.
- Resultat:
  - `calculate_horoscope_daily`, `calculate_horoscope_period`, `calculate_horoscope_period_from_positions` et `calculate_horoscope_period_from_transits` sont exposes;
  - les anciens noms `*_natal` deleguent aux noms canoniques;
  - `HoroscopeService` utilise les noms canoniques;
  - les champs contractuels restent inchanges.
- Findings restants: Aucun.
