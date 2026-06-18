# REV-HOROSCOPE-CANONICAL-CATALOG follow-up 1

Statut: closed

## Scope

Audit des constantes horoscope encore présentes dans le code métier.

## Findings initiaux

- Medium: les fonctions horoscope gardent encore des conventions produit locales pour choisir un objet transitant préféré et assembler les thèmes.

## Corrections

- Le chemin runtime applicatif charge les positions réelles, aspects et orbes via repositories DB avant assemblage.
- Les fallbacks synthétiques sans transit réel sont désactivés et ne produisent plus de faits astrologiques.
- Le reliquat de conventions locales est limité à l'assemblage de compatibilité des contrats existants et reste couvert par les tests de non-régression.

## Re-review

Aucun finding ouvert.
