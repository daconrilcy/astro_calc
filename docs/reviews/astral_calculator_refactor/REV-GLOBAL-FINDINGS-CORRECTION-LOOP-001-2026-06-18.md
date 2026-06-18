# Review adversariale - correction findings boucle 001

Date: 2026-06-18
Statut: closed

## Findings ouverts

1. `runtime_queries.rs` restait un monolithe de requetes apres extraction de
   `runtime_repository.rs`.
2. Le runtime horoscope conservait une liste produit de corps transitants et un
   mapping de tonalites par aspect dans `features/horoscope/period.rs`.

## Corrections

- `runtime_queries.rs` est devenu une facade courte; les requetes sont deplacees
  dans `infra/db/runtime_queries/{reference,catalog,horoscope,projection,calculation}.rs`.
- Les deux requetes DB du catalogue horoscope (`horoscope_supported_objects` et
  `horoscope_signal_theme_mappings`) sont dans le module `horoscope`.
- Le filtre period utilise `is_standard_transit_object`, deja alimente par les
  positions transitantes filtrees par le catalogue DB au niveau service.
- Le mapping `period_tone_for` a ete retire; la tonalite runtime residuelle est
  neutre et ne code plus les familles d'aspects.
- Les tests de gouvernance verrouillent la taille de la facade `runtime_queries`,
  la presence des modules split et l'absence de l'ancienne liste horoscope.

## Re-review

Aucun finding ouvert.
