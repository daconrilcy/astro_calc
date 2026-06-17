# Review adversariale - frontieres consommateurs internes calculateur

Date: 2026-06-17
Statut: closed

## Perimetre

Validation des frontieres de couche pour la vague de migration des consommateurs
internes vers `/v1/internal/calculations/*`.

## Cycle 1

### Findings

- P2 - Les mentions de compatibilite du guide Docker avaient ete migrees par
  erreur vers le chemin canonique interne, ce qui brouillait la distinction
  entre facade interne courante et alias legacy.

### Corrections

- Les docs de compatibilite conservent explicitement `/v1/calculations/*`.
- Les scripts, E2E, helpers et outils de dev utilisent les routes canoniques
  internes.
- Aucun appel HTTP n'a ete introduit dans `astral_calculator`; le moteur reste
  independant d'Axum et de la surface API.
- Aucun changement de payload, port Docker, crate, service ou conteneur.

## Cycle 2

### Findings

Aucun finding ouvert.

## Cycle 3

### Findings

Aucun finding ouvert.

### Verification adversariale

- Aucun consommateur interne courant ne depend des routes legacy
  `/v1/calculations/*`.
- Les references legacy restantes documentent la compatibilite ou testent les
  aliases existants.
- Les frontieres restent inchangees : aucune dependance HTTP n'est introduite
  dans `astral_calculator`.

## Conclusion

Aucun finding ouvert.
