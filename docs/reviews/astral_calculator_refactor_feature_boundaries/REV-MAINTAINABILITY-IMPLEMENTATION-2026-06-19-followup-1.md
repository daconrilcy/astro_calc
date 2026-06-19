Statut: closed

Objet:
- re-review des frontieres apres correction du handoff transactionnel dans le workflow natal.

Verification:
- pas de relachement du verrou d'idempotence entre lecture des tentatives existantes et insertion du nouveau calcul;
- `runtime`, `astrology` et les ports fins restent conformes aux invariants de couche.

Findings restants: Aucun
